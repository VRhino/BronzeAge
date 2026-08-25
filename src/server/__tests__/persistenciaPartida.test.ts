// Contrato de la persistencia de partida (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): guardado
// atómico, carga que reconstruye una sesión operable y en continuidad de RNG, y la red de seguridad de
// versión de concurrencia. Usa un directorio temporal real por test — es la única forma honesta de probar
// "escritura atómica en disco": un fake en memoria no ejercitaría `rename` en absoluto.
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { fundarAsentamiento } from '../../session/comandos/fundarAsentamiento';
import {
  cargarPartida,
  ConflictoDeVersionError,
  FormatoSnapshotNoSoportadoError,
  FORMATO_SNAPSHOT_VERSION,
  guardarPartida,
  WorldgenVersionNoCoincideError,
  type SnapshotPartida,
} from '../persistenciaPartida';

const MOMENTO = '2026-01-01T00:00:00.000Z';

function partidaEnMarcha(seed = 42): GameSession {
  const sesion = GameSession.crear('partida-test', { seed });
  const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { momento: MOMENTO });
  sesion.ejecutar(
    fundarAsentamiento,
    { faccionId: r.datos!.faccionId, posicion: { x: 500, y: 500 }, numJugadores: 5 },
    { momento: MOMENTO }
  );
  for (let i = 0; i < 5; i++) sesion.avanzarTick(MOMENTO);
  return sesion;
}

let directorio: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-persistencia-'));
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

describe('guardarPartida / cargarPartida', () => {
  it('cargarPartida devuelve null si no hay snapshot para ese gameId', async () => {
    expect(await cargarPartida(directorio, 'no-existe')).toBeNull();
  });

  it('lo que se carga reconstruye el mismo estado que se guardó', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);

    const cargada = await cargarPartida(directorio, sesion.gameId);

    expect(cargada).not.toBeNull();
    expect(cargada!.getState()).toEqual(sesion.getState());
  });

  it('la partida cargada continúa la misma secuencia de RNG, no la reinicia (a través del disco)', async () => {
    // Repite la prueba de continuidad de `session/__tests__/gameSession.test.ts` pero a través del disco de
    // verdad — es el camino que de verdad usará la reconstrucción de un incidente de producción.
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const cargada = (await cargarPartida(directorio, sesion.gameId))!;

    for (let i = 0; i < 5; i++) {
      sesion.avanzarTick(MOMENTO);
      cargada.avanzarTick(MOMENTO);
    }

    expect(cargada.getState()).toEqual(sesion.getState());
  });

  it('la escritura es atómica: no queda ningún .tmp tras un guardado exitoso', async () => {
    await guardarPartida(directorio, partidaEnMarcha(), MOMENTO);
    const archivos = await readdir(directorio);
    expect(archivos.every((f) => !f.endsWith('.tmp'))).toBe(true);
  });

  it('guardar dos veces la MISMA versión no lanza — es un reintento válido, no un conflicto', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    await expect(guardarPartida(directorio, sesion, MOMENTO)).resolves.toBeUndefined();
  });

  it('rechaza sobreescribir con una versión MENOR que la que ya hay en disco, y no toca el archivo', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO); // versión N en disco

    const antesDelConflicto = await cargarPartida(directorio, sesion.gameId);

    // Una sesión "atrasada" (por ejemplo, una réplica en memoria que no vio los últimos comandos) intenta
    // guardar una versión anterior a la que ya hay en disco.
    const atrasada = GameSession.importar(sesion.exportar());
    await expect(guardarPartida(directorio, atrasada, MOMENTO)).resolves.toBeUndefined(); // misma versión: ok
    sesion.avanzarTick(MOMENTO); // sesion avanza; `atrasada` se queda atrás
    await guardarPartida(directorio, sesion, MOMENTO); // disco ahora en versión N+1

    await expect(guardarPartida(directorio, atrasada, MOMENTO)).rejects.toThrow(ConflictoDeVersionError);

    // El archivo sigue reflejando la versión N+1 que había antes del intento fallido, no algo a medias.
    const trasElConflicto = await cargarPartida(directorio, sesion.gameId);
    expect(trasElConflicto!.getState().version).toBe(sesion.getState().version);
    expect(trasElConflicto!.getState().version).toBeGreaterThan(antesDelConflicto!.getState().version);
  });

  it('rechaza un snapshot de un formato de envoltorio que esta build no espera', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.formatoVersion = FORMATO_SNAPSHOT_VERSION + 1;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(cargarPartida(directorio, sesion.gameId)).rejects.toThrow(FormatoSnapshotNoSoportadoError);
  });

  it('rechaza un snapshot generado con otra versión del generador de mundo', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.partida.worldgenVersion += 1;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(cargarPartida(directorio, sesion.gameId)).rejects.toThrow(WorldgenVersionNoCoincideError);
  });

  it('dos partidas distintas en el mismo directorio no se pisan', async () => {
    const a = partidaEnMarcha(1);
    const b = GameSession.crear('otra-partida', { seed: 2 });
    await guardarPartida(directorio, a, MOMENTO);
    await guardarPartida(directorio, b, MOMENTO);

    expect((await cargarPartida(directorio, a.gameId))!.getState().asentamientos).toHaveLength(1);
    expect((await cargarPartida(directorio, b.gameId))!.getState().asentamientos).toHaveLength(0);
  });
});
