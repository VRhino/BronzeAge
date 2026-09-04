// Contrato de la persistencia de partida (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): guardado
// atómico, carga que reconstruye una sesión operable y en continuidad de RNG, y la red de seguridad de
// versión de concurrencia. Usa un directorio temporal real por test — es la única forma honesta de probar
// "escritura atómica en disco": un fake en memoria no ejercitaría `rename` en absoluto.
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
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

function partidaEnMarcha(seed = 42): GameSession {
  const sesion = GameSession.crear('partida-test', { seed });
  const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, {});
  sesion.ejecutar(
    fundarAsentamiento,
    { faccionId: r.datos!.faccionId, posicion: { x: 500, y: 500 }, numJugadores: 5 },
    {}
  );
  for (let i = 0; i < 5; i++) sesion.avanzarTick();
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
    expect(cargada!.sesion.getState()).toEqual(sesion.getState());
    // D5: `cargarPartida` también devuelve el `guardadoEn` del snapshot — la referencia del catch-up.
    expect(cargada!.guardadoEn).toBe(MOMENTO);
  });

  it('la partida cargada continúa la misma secuencia de RNG, no la reinicia (a través del disco)', async () => {
    // Repite la prueba de continuidad de `session/__tests__/gameSession.test.ts` pero a través del disco de
    // verdad — es el camino que de verdad usará la reconstrucción de un incidente de producción.
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const cargada = (await cargarPartida(directorio, sesion.gameId))!.sesion;

    for (let i = 0; i < 5; i++) {
      sesion.avanzarTick();
      cargada.avanzarTick();
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

    const antesDelConflicto = (await cargarPartida(directorio, sesion.gameId))?.sesion ?? null;

    // Una sesión "atrasada" (por ejemplo, una réplica en memoria que no vio los últimos comandos) intenta
    // guardar una versión anterior a la que ya hay en disco.
    const atrasada = GameSession.importar(sesion.exportar());
    await expect(guardarPartida(directorio, atrasada, MOMENTO)).resolves.toBeUndefined(); // misma versión: ok
    sesion.avanzarTick(); // sesion avanza; `atrasada` se queda atrás
    await guardarPartida(directorio, sesion, MOMENTO); // disco ahora en versión N+1

    await expect(guardarPartida(directorio, atrasada, MOMENTO)).rejects.toThrow(ConflictoDeVersionError);

    // El archivo sigue reflejando la versión N+1 que había antes del intento fallido, no algo a medias.
    const trasElConflicto = (await cargarPartida(directorio, sesion.gameId))?.sesion ?? null;
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

  it('migra un snapshot v1 (campos `*EnTick` en ticks) al modelo temporal actual (`*En` en Instante)', async () => {
    // Snapshot v1 sintético: una partida con una relación, una orden y un asentamiento con política y
    // escuadrón herido, todo fechado en ORDINALES DE TICK como lo hacía la build anterior a la Fase D.
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 1;
    const s = snap.partida.state;
    const a = s.asentamientos[0];
    a.fundadoEn = undefined;
    a.fundadoEnTick = 3;
    a.politicasActivas = [{ id: 'p1', politicaId: 'lineas_produccion', cargo: 'gobernador', activadaEnTick: 3, expiraEnTick: 153 }];
    a.escuadrones = [{ id: 'e1', nombre: 'x', jugadorId: 'j', origen: 'pesants', cantidad: 5, veterania: 0, moral: 100, tropaId: 'milicia_lanceros', heridoHastaTick: 20 }];
    s.relaciones = [
      { id: 'r1', tipo: 'alianza', faccionAId: 'fa', faccionBId: 'fb', creadoEnTick: 7, estado: 'activa' },
      // Vasallaje con tributo: `cantidadPorTick` (v3) -> `cantidadPorMinuto` (v4) — la cadena entera v1→v4.
      { id: 'r2', tipo: 'vasallaje', faccionAId: 'fa', faccionBId: 'fc', creadoEnTick: 1, estado: 'activa', tributo: { recurso: 'trigo', cantidadPorTick: 9 } },
    ];
    s.ordenes = [{ id: 'o1', asentamientoId: a.id, tipo: 'venta', recurso: 'trigo', cantidad: 10, cantidadCumplida: 0, precioUnitario: 1, creadoEnTick: 5, estado: 'activa' }];
    s.bandidosProximoSpawnEn = undefined;
    s.bandidosProximoSpawnTick = 12;
    s.salidasFaccionPorJugador = { ana: '2026-01-01T00:10:00.000Z' };
    s.estadoMapa.regeneraEn = undefined;
    s.estadoMapa.regeneraEnTick = { 'nodo-1': 40 };
    // v1 también guardaba el contador de construcción `ticksRestantes` (D3 lo pasa a `completaEn`).
    a.edificios.push({ id: 'obra-1', tipo: 'granja', posicion: { x: 1, y: 1 }, estado: 'en_construccion', ticksRestantes: 4, ambito: 'asentamiento' });
    a.edificios.push({ id: 'obra-cola', tipo: 'vivienda', posicion: { x: 2, y: 2 }, estado: 'en_cola', ticksRestantes: 8, ambito: 'asentamiento' });
    // v1 fechaba eventos y log con `tick` (v5 lo retira del contrato — solo queda `momento`).
    s.eventosDominio = [{ codigo: 'legado', mensaje: 'algo pasó', momento: '2026-01-01T00:02:00.000Z', tick: 2, version: 1 }];
    s.historialJugadores = { 'jugador-1': [{ tick: 6, mensaje: 'nota' }] };
    delete a.fundadoEn;
    delete s.bandidosProximoSpawnEn;
    delete s.estadoMapa.regeneraEn;
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))?.sesion ?? null;
    const state = cargada!.getState();
    const seg = 60_000; // 1 tick = 1 minuto de mundo
    const epoca = new Date('2026-01-01T00:00:00.000Z').getTime();
    expect(state.asentamientos[0]!.fundadoEn).toBe(epoca + 3 * seg);
    expect(state.asentamientos[0]!.politicasActivas[0]!.expiraEn).toBe(epoca + 153 * seg);
    expect(state.asentamientos[0]!.escuadrones[0]!.heridoHasta).toBe(epoca + 20 * seg);
    expect(state.relaciones[0]!.creadoEn).toBe(epoca + 7 * seg);
    expect(state.ordenes[0]!.creadoEn).toBe(epoca + 5 * seg);
    expect(state.bandidosProximoSpawnEn).toBe(epoca + 12 * seg);
    expect(state.salidasFaccionPorJugador['ana']).toBe(new Date('2026-01-01T00:10:00.000Z').getTime());
    expect(state.estadoMapa.regeneraEn['nodo-1']).toBe(epoca + 40 * seg);
    // Construcción: el contador se volvió fecha absoluta (tick actual + lo que le quedaba); la cola la pierde.
    const obra = state.asentamientos[0]!.edificios.find((e) => e.id === 'obra-1')!;
    expect(obra.completaEn).toBe(epoca + (state.tick + 4) * seg);
    const cola = state.asentamientos[0]!.edificios.find((e) => e.id === 'obra-cola')!;
    expect(cola.completaEn).toBeUndefined();
    // Tributo: renombrado a `cantidadPorMinuto` (mismo valor), en la misma pasada de carga.
    expect(state.relaciones[1]!.tributo).toEqual({ recurso: 'trigo', cantidadPorMinuto: 9 });
    // Evento y log: el `tick` provisional se fue; el log conserva el momento equivalente.
    expect(state.eventosDominio[0]).not.toHaveProperty('tick');
    expect(state.historialJugadores['jugador-1']).toEqual([{ momento: new Date(epoca + 6 * seg).toISOString(), mensaje: 'nota' }]);
    // Ni `*EnTick` ni `ticksRestantes` ni `cantidadPorTick` sobreviven.
    expect(JSON.stringify(cargada!.exportar())).not.toMatch(/EnTick|ticksRestantes|cantidadPorTick/);
  });

  it('migra un snapshot v2 (`Edificio.ticksRestantes`) a v3 (`Edificio.completaEn`)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 2;
    const s = snap.partida.state;
    const a = s.asentamientos[0];
    a.edificios.push({ id: 'obra-2', tipo: 'granja', posicion: { x: 1, y: 1 }, estado: 'en_construccion', ticksRestantes: 6, ambito: 'asentamiento' });
    a.edificios.push({ id: 'lista', tipo: 'vivienda', posicion: { x: 2, y: 2 }, estado: 'activo', ticksRestantes: 0, ambito: 'asentamiento' });
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))?.sesion ?? null;
    const state = cargada!.getState();
    const epoca = new Date('2026-01-01T00:00:00.000Z').getTime();
    const obra = state.asentamientos[0]!.edificios.find((e) => e.id === 'obra-2')!;
    expect(obra.completaEn).toBe(epoca + (state.tick + 6) * 60_000);
    const lista = state.asentamientos[0]!.edificios.find((e) => e.id === 'lista')!;
    expect(lista.completaEn).toBeUndefined();
    expect(JSON.stringify(cargada!.exportar())).not.toMatch(/ticksRestantes/);
  });

  it('migra un snapshot v3 (`tributo.cantidadPorTick`) a v4 (`tributo.cantidadPorMinuto`)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 3;
    snap.partida.state.relaciones = [
      { id: 'r1', tipo: 'vasallaje', faccionAId: 'fa', faccionBId: 'fb', creadoEn: Date.parse(MOMENTO), estado: 'activa', tributo: { recurso: 'madera', cantidadPorTick: 12 } },
    ];
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))!.sesion;
    expect(cargada.getState().relaciones[0]!.tributo).toEqual({ recurso: 'madera', cantidadPorMinuto: 12 });
    expect(JSON.stringify(cargada.exportar())).not.toMatch(/cantidadPorTick/);
  });

  it('migra un snapshot v4 (`tick` en eventos y en el log por jugador) a v5 (solo `momento`)', async () => {
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 4;
    const epoca = Date.parse('2026-01-01T00:00:00.000Z');
    snap.partida.state.eventosDominio = [
      { codigo: 'x', mensaje: 'algo', momento: '2026-01-01T00:05:00.000Z', tick: 5, version: 1 },
    ];
    snap.partida.state.historialJugadores = { 'jugador-1': [{ tick: 8, mensaje: 'compró una casa' }] };
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))!.sesion;
    const state = cargada.getState();
    expect(state.eventosDominio[0]).toEqual({ codigo: 'x', mensaje: 'algo', momento: '2026-01-01T00:05:00.000Z', version: 1 });
    expect(state.historialJugadores['jugador-1']).toEqual([
      { momento: new Date(epoca + 8 * 60_000).toISOString(), mensaje: 'compró una casa' },
    ]);
    // El `tick` de la PARTIDA (unidad interna del motor) sigue ahí; el de los eventos/log, no.
    expect(state.tick).toBeTypeOf('number');
    expect(state.eventosDominio[0]).not.toHaveProperty('tick');
    expect(state.historialJugadores['jugador-1']![0]).not.toHaveProperty('tick');
  });

  it('migra v5 -> v6: un snapshot sin `jugadores` ni `ejercitos` los recibe vacíos', async () => {
    // Movimiento de ejércitos (Doc 5.11/5.12): dos colecciones nuevas. Una partida guardada de antes de la
    // mecánica no tiene ejércitos en campaña, y un jugador sin registro en `jugadores` usa `LIDERAZGO.base`
    // por diseño — así que no hay nada que reconstruir, solo que existan para que nadie lea `undefined`.
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 5;
    delete snap.partida.state.jugadores;
    delete snap.partida.state.ejercitos;
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))!.sesion;
    const state = cargada.getState();
    expect(state.jugadores).toEqual([]);
    expect(state.ejercitos).toEqual([]);
    // Y la partida sigue siendo operable: la migración no toca nada más.
    expect(state.asentamientos.length).toBeGreaterThan(0);
  });

  it('migra v6 -> v7: un snapshot sin `memoriaPorFaccion` la recibe vacía', async () => {
    // Niebla de guerra (Paso 2): la partida no venía guardando qué había visto cada Facción, así que no hay
    // pasado que reconstruir — se empieza a recordar desde el primer tick que corra con la mecánica. Lo que
    // el jugador nota es que el mapa se tapa entero salvo lo que esté viendo, y se destapa jugando.
    const sesion = partidaEnMarcha();
    await guardarPartida(directorio, sesion, MOMENTO);
    const ruta = join(directorio, `${sesion.gameId}.json`);
    const snap = JSON.parse(await readFile(ruta, 'utf-8'));
    snap.formatoVersion = 6;
    delete snap.partida.state.memoriaPorFaccion;
    await writeFile(ruta, JSON.stringify(snap), 'utf-8');

    const cargada = (await cargarPartida(directorio, sesion.gameId))!.sesion;
    expect(cargada.getState().memoriaPorFaccion).toEqual({});
    expect(cargada.getState().asentamientos.length).toBeGreaterThan(0);
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

    expect((await cargarPartida(directorio, a.gameId))!.sesion.getState().asentamientos).toHaveLength(1);
    expect((await cargarPartida(directorio, b.gameId))!.sesion.getState().asentamientos).toHaveLength(0);
  });
});

describe('listarPartidas (Fase C12: descubrimiento)', () => {
  it('directorio inexistente: lista vacía, no un error', async () => {
    expect(await listarPartidas(join(directorio, 'no-existe-todavia'))).toEqual([]);
  });

  it('directorio vacío: lista vacía', async () => {
    expect(await listarPartidas(directorio)).toEqual([]);
  });

  it('lee TODAS las partidas guardadas, no solo las que hay abiertas en memoria (no hay "memoria" aquí)', async () => {
    const a = partidaEnMarcha(1);
    const b = GameSession.crear('otra-partida', { seed: 2 });
    await guardarPartida(directorio, a, MOMENTO);
    await guardarPartida(directorio, b, MOMENTO);

    const partidas = await listarPartidas(directorio);

    expect(partidas.map((p) => p.gameId).sort()).toEqual([a.gameId, b.gameId].sort());
    const resumenA = partidas.find((p) => p.gameId === a.gameId)!;
    expect(resumenA.instante).toBe(instanteDeTick(a.getState().tick));
    expect(resumenA.version).toBe(a.getState().version);
    expect(resumenA.mapaId).toBeTruthy();
    expect(resumenA.guardadoEn).toBe(MOMENTO);
  });

  it('ignora un .json del directorio que no sea una partida — `identidad.json` vive AQUI', async () => {
    // Reproduce el 500 real de `GET /admin/partidas`: `crearRepositorioIdentidadEnDisco` guarda
    // `identidad.json` en el mismo directorio que los snapshots (`server/index.ts`), asi que el filtro por
    // extension lo colaba y `snapshot.partida.state` reventaba. Bastaba con haber iniciado sesion una vez.
    const a = partidaEnMarcha(1);
    await guardarPartida(directorio, a, MOMENTO);
    await writeFile(join(directorio, 'identidad.json'), JSON.stringify({ formatoVersion: 1, datos: { usuarios: [] } }), 'utf-8');

    const partidas = await listarPartidas(directorio);

    expect(partidas.map((p) => p.gameId)).toEqual([a.gameId]);
  });
});
