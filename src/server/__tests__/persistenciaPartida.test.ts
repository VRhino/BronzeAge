// Contrato de la persistencia de partida (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): guardado
// atómico, carga que reconstruye una sesión operable y en continuidad de RNG, y la red de seguridad de
// versión de concurrencia. Usa un directorio temporal real por test — es la única forma honesta de probar
// "escritura atómica en disco": un fake en memoria no ejercitaría `rename` en absoluto.
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { GameSession } from '../../session/gameSession';
import { instanteDeTick } from '../../session/estado';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { fundarAsentamiento } from '../../session/comandos/fundarAsentamiento';
import {
  cargarPartida,
  ConflictoDeVersionError,
  FormatoSnapshotNoSoportadoError,
  FORMATO_SNAPSHOT_VERSION,
  guardarPartida,
  listarPartidas,
  WorldgenVersionNoCoincideError,
  type SnapshotPartida,
} from '../persistenciaPartida';

const MOMENTO = '2026-01-01T00:00:00.000Z';

const ACTOR = 'jugador-test';

function partidaEnMarcha(seed = 42): GameSession {
  const sesion = GameSession.crear('partida-test', { seed });
  // Con actor real, no de sistema: se funda donde se está (Doc 1.3), y el sistema no está en ningún sitio
  // del mundo — `{}` (actor implícito = sistema) fundaba igual antes de esta regla, porque la posición la
  // traía el parámetro y no la columna de nadie.
  const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR });
  sesion.ejecutar(fundarAsentamiento, { faccionId: r.datos!.faccionId }, { actor: ACTOR });
  for (let i = 0; i < 5; i++) sesion.avanzarTick();
  return sesion;
}

/** El snapshot (formato v13) ya no contiene el historial de eventos —vive en `<gameId>.eventos.jsonl`, que
 * lo escribe `RunnerDePartida`, no `guardarPartida`— así que un round-trip por `guardarPartida`/`cargarPartida`
 * a secas lo pierde a propósito. Se compara el resto. */
function sinHistorial(estado: ReturnType<GameSession['getState']>) {
  return { ...estado, eventosDominio: [] };
}

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-persistencia-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

describe('guardarPartida / cargarPartida', () => {
  it('cargarPartida devuelve null si no hay snapshot para ese gameId', async () => {
    expect(await cargarPartida(almacen, 'no-existe')).toBeNull();
  });

  it('lo que se carga reconstruye el mismo estado que se guardó (incluido el mapa, regenerado de la seed)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);

    const cargada = await cargarPartida(almacen, sesion.gameId);

    expect(cargada).not.toBeNull();
    // El mapa no se guarda (formato v13): `cargarPartida` lo regenera con `generarMapa(config)`. Que esta
    // comparación pase byte a byte es la prueba de que la regeneración es idéntica a lo que corría en memoria.
    expect(cargada!.sesion.getState().mapa).toEqual(sesion.getState().mapa);
    expect(sinHistorial(cargada!.sesion.getState())).toEqual(sinHistorial(sesion.getState()));
  });

  it('el snapshot en disco NO contiene el terreno ni el historial (formato v13)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    const { partida } = JSON.parse(await readFile(join(directorio, `${sesion.gameId}.json`), 'utf-8')) as SnapshotPartida;

    expect(partida.state.eventosDominio).toEqual([]);
    // Solo lo mínimo para regenerar (config) e identificar (version) el mapa — no el terreno.
    expect(Object.keys(partida.state.mapa).sort()).toEqual(['config', 'version']);
  });

  it('la partida cargada continúa la misma secuencia de RNG, no la reinicia (a través del disco)', async () => {
    // Repite la prueba de continuidad de `session/__tests__/gameSession.test.ts` pero a través del disco de
    // verdad — es el camino que de verdad usará la reconstrucción de un incidente de producción.
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    const cargada = (await cargarPartida(almacen, sesion.gameId))!.sesion;

    for (let i = 0; i < 5; i++) {
      sesion.avanzarTick();
      cargada.avanzarTick();
    }

    expect(sinHistorial(cargada.getState())).toEqual(sinHistorial(sesion.getState()));
  });

  it('la escritura es atómica: no queda ningún .tmp tras un guardado exitoso', async () => {
    await guardarPartida(almacen, partidaEnMarcha(), MOMENTO);
    const archivos = await readdir(directorio);
    expect(archivos.every((f) => !f.endsWith('.tmp'))).toBe(true);
  });

  it('guardar dos veces la MISMA versión no lanza — es un reintento válido, no un conflicto', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    await expect(guardarPartida(almacen, sesion, MOMENTO)).resolves.toBeUndefined();
  });

  it('rechaza sobreescribir con una versión MENOR que la que ya hay en disco, y no toca el archivo', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO); // versión N en disco

    const antesDelConflicto = (await cargarPartida(almacen, sesion.gameId))?.sesion ?? null;

    // Una sesión "atrasada" (por ejemplo, una réplica en memoria que no vio los últimos comandos) intenta
    // guardar una versión anterior a la que ya hay en disco.
    const atrasada = GameSession.importar(sesion.exportar());
    await expect(guardarPartida(almacen, atrasada, MOMENTO)).resolves.toBeUndefined(); // misma versión: ok
    sesion.avanzarTick(); // sesion avanza; `atrasada` se queda atrás
    await guardarPartida(almacen, sesion, MOMENTO); // disco ahora en versión N+1

    await expect(guardarPartida(almacen, atrasada, MOMENTO)).rejects.toThrow(ConflictoDeVersionError);

    // El archivo sigue reflejando la versión N+1 que había antes del intento fallido, no algo a medias.
    const trasElConflicto = (await cargarPartida(almacen, sesion.gameId))?.sesion ?? null;
    expect(trasElConflicto!.getState().version).toBe(sesion.getState().version);
    expect(trasElConflicto!.getState().version).toBeGreaterThan(antesDelConflicto!.getState().version);
  });

  it('rechaza un snapshot de un formato de envoltorio que esta build no espera', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.formatoVersion = FORMATO_SNAPSHOT_VERSION + 1;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(cargarPartida(almacen, sesion.gameId)).rejects.toThrow(FormatoSnapshotNoSoportadoError);
  });

  it('un snapshot de un formato ANTERIOR se rechaza — ya no se migra (cadena retirada 2026-09-09)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.formatoVersion = FORMATO_SNAPSHOT_VERSION - 1;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(cargarPartida(almacen, sesion.gameId)).rejects.toThrow(FormatoSnapshotNoSoportadoError);
  });

  it('rechaza un snapshot generado con otra versión del generador de mundo', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(almacen, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.partida.worldgenVersion += 1;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(cargarPartida(almacen, sesion.gameId)).rejects.toThrow(WorldgenVersionNoCoincideError);
  });

  it('dos partidas distintas en el mismo directorio no se pisan', async () => {
    const a = partidaEnMarcha(1);
    const b = GameSession.crear('otra-partida', { seed: 2 });
    await guardarPartida(almacen, a, MOMENTO);
    await guardarPartida(almacen, b, MOMENTO);

    expect((await cargarPartida(almacen, a.gameId))!.sesion.getState().asentamientos).toHaveLength(1);
    expect((await cargarPartida(almacen, b.gameId))!.sesion.getState().asentamientos).toHaveLength(0);
  });
});

describe('listarPartidas (Fase C12: descubrimiento)', () => {
  it('directorio inexistente: lista vacía, no un error', async () => {
    expect(await listarPartidas(crearAlmacenEnDisco(join(directorio, 'no-existe-todavia')))).toEqual([]);
  });

  it('directorio vacío: lista vacía', async () => {
    expect(await listarPartidas(almacen)).toEqual([]);
  });

  it('lee TODAS las partidas guardadas, no solo las que hay abiertas en memoria (no hay "memoria" aquí)', async () => {
    const a = partidaEnMarcha(1);
    const b = GameSession.crear('otra-partida', { seed: 2 });
    await guardarPartida(almacen, a, MOMENTO);
    await guardarPartida(almacen, b, MOMENTO);

    const partidas = await listarPartidas(almacen);

    expect(partidas.map((p) => p.gameId).sort()).toEqual([a.gameId, b.gameId].sort());
    const resumenA = partidas.find((p) => p.gameId === a.gameId)!;
    expect(resumenA.instante).toBe(instanteDeTick(a.getState().tick));
    expect(resumenA.version).toBe(a.getState().version);
    expect(resumenA.mapaId).toBeTruthy();
    expect(resumenA.guardadoEn).toBe(MOMENTO);
  });

  it('ignora un .json del directorio que no sea una partida — `identidad.json` vive AQUI', async () => {
    // Reproduce el 500 real de `GET /admin/partidas`: el repositorio de identidad guarda
    // `identidad.json` en el mismo directorio que los snapshots (`server/index.ts`), asi que el filtro por
    // extension lo colaba y `snapshot.partida.state` reventaba. Bastaba con haber iniciado sesion una vez.
    const a = partidaEnMarcha(1);
    await guardarPartida(almacen, a, MOMENTO);
    await writeFile(join(directorio, 'identidad.json'), JSON.stringify({ formatoVersion: 1, datos: { usuarios: [] } }), 'utf-8');

    const partidas = await listarPartidas(almacen);

    expect(partidas.map((p) => p.gameId)).toEqual([a.gameId]);
  });

  it('un snapshot ILEGIBLE se excluye del listado en vez de tumbarlo entero (Fase E2)', async () => {
    // Misma familia que el caso de `identidad.json` de arriba, encontrada al escribir la pasada de
    // mantenimiento: un `.json` que no parsea (truncado por un corte a mitad de escritura, o basura) hacia
    // que `JSON.parse` lanzara y `GET /admin/partidas` devolviera 500 para TODAS las partidas, no solo para
    // la rota. Ahora se excluye la rota y las demas se listan — con un grito por consola, porque a
    // diferencia de `identidad.json` esto si es un problema que alguien tiene que mirar.
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    const a = partidaEnMarcha(1);
    await guardarPartida(almacen, a, MOMENTO);
    await writeFile(join(directorio, 'rota.json'), '{"formatoVersion":7,"parti', 'utf-8');

    const partidas = await listarPartidas(almacen);

    expect(partidas.map((p) => p.gameId)).toEqual([a.gameId]);
    expect(error).toHaveBeenCalled();
    error.mockRestore();
  });
});
