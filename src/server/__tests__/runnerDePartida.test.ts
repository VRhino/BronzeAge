// Contrato de RunnerDePartida (Docs/Arquitectura/7_Diseno_GameSession.md §4, §8.4): cola serial, ciclo
// "aplicar -> persistir -> confirmar" con descarte en fallo, y el scheduler de ticks automáticos.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { fundarAsentamiento } from '../../session/comandos/fundarAsentamiento';
import { alternarFaccionNpc } from '../../session/comandos/alternarFaccionNpc';
import { RunnerDePartida } from '../runnerDePartida';
import { cargarPartida, type SnapshotPartida } from '../persistenciaPartida';

const MOMENTO = '2026-01-01T00:00:00.000Z';

let directorio: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-runner-'));
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

function runner(gameId = 'partida-runner', seed = 7): RunnerDePartida {
  return RunnerDePartida.crear(gameId, { seed }, { directorio, ahora: () => MOMENTO });
}

describe('RunnerDePartida — cola serial', () => {
  it('tres comandos lanzados sin esperar se aplican en el orden en que llegaron', async () => {
    const r = runner();

    // Sin `await` entre medias: las tres promesas se lanzan "a la vez" desde la perspectiva del llamador.
    const p1 = r.ejecutar(crearFaccion, { nombre: 'Micenas' });
    const p2 = r.ejecutar(crearFaccion, { nombre: 'Troya' });
    const p3 = r.ejecutar(crearFaccion, { nombre: 'Ugarit' });
    await Promise.all([p1, p2, p3]);

    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Troya', 'Ugarit']);
    // La versión sube de uno en uno con cada comando aceptado — si se hubieran intercalado o perdido alguno,
    // no llegaría a 3.
    expect(r.getState().version).toBe(3);
  });

  it('un comando rechazado no bloquea ni desordena los que vienen después', async () => {
    const r = runner();
    const p1 = r.ejecutar(crearFaccion, { nombre: 'Micenas' });
    // `fundarAsentamiento` con una facción inexistente se rechaza (error de dominio) sin lanzar.
    const p2 = r.ejecutar(fundarAsentamiento, { faccionId: 'no-existe', posicion: { x: 0, y: 0 }, numJugadores: 1 });
    const p3 = r.ejecutar(crearFaccion, { nombre: 'Troya' });

    const [r1, r2, r3] = await Promise.all([p1, p2, p3]);
    expect(r1.ok).toBe(true);
    expect(r2.ok).toBe(false);
    expect(r3.ok).toBe(true);
    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Troya']);
  });
});

describe('RunnerDePartida — aplicar -> persistir -> confirmar', () => {
  it('cada comando aceptado queda guardado en disco antes de resolver', async () => {
    const r = runner('g-persistido');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' });

    const cargada = await cargarPartida(directorio, 'g-persistido');
    expect(cargada!.getState().facciones).toHaveLength(1);
  });

  it('si la persistencia falla, el comando se descarta: GameSession vuelve a como estaba antes', async () => {
    const r = runner('g-conflicto');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' }); // versión 1, guardada

    // Se fuerza el conflicto de versión a mano: alguien (un bug, otro proceso) deja en disco una versión más
    // avanzada de la que el runner cree tener.
    const ruta = join(directorio, 'g-conflicto.json');
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.partida.state.version = 999;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    const estadoAntes = r.getState();
    await expect(r.ejecutar(crearFaccion, { nombre: 'Troya' })).rejects.toThrow();

    // El comando se descartó: ni la facción nueva ni la versión avanzada quedaron en memoria.
    expect(r.getState().version).toBe(estadoAntes.version);
    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas']);
  });

  it('tras un fallo de persistencia, el runner sigue operable para el siguiente comando', async () => {
    const r = runner('g-recupera');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' });

    const ruta = join(directorio, 'g-recupera.json');
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.partida.state.version = 999;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');
    await expect(r.ejecutar(crearFaccion, { nombre: 'Troya' })).rejects.toThrow();

    // El "atacante" deja de escribir versiones adelantadas: el siguiente comando legítimo debe volver a
    // funcionar con normalidad, sin arrastrar nada del intento fallido.
    await writeFile(ruta, JSON.stringify({ ...snapshot, partida: { ...snapshot.partida, state: { ...snapshot.partida.state, version: 1 } } }), 'utf-8');
    const resultado = await r.ejecutar(crearFaccion, { nombre: 'Ugarit' });

    expect(resultado.ok).toBe(true);
    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Ugarit']);
  });
});

describe('RunnerDePartida.avanzarTick — bundlea auto-comercio y turno NPC', () => {
  it('el turno del NPC de gobernanza ocurre DENTRO de avanzarTick, sin un comando aparte', async () => {
    const r = runner('g-npc');
    const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' });
    const faccionId = creada.datos!.faccionId;
    await r.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 1 });
    await r.ejecutar(alternarFaccionNpc, { faccionId, activo: true });

    // Antes de este fix, `avanzarTick()` del runner solo aplicaba el tick puro — la primera decisión de
    // gobernanza del NPC (asignar Gobernador, determinista, no depende de ticks previos) no llegaba a
    // ocurrir sin una llamada aparte a `avanzarFaccionesNpc`.
    await r.avanzarTick();

    expect(r.getState().asentamientos[0]!.cargos.gobernadorId).toBeTruthy();
  });
});

describe('RunnerDePartida.cargarOCrear', () => {
  it('sin snapshot previo, crea una partida nueva', async () => {
    const r = await RunnerDePartida.cargarOCrear('g-nueva', { seed: 1 }, { directorio, ahora: () => MOMENTO });
    expect(r.getState().tick).toBe(0);
    expect(r.getState().facciones).toEqual([]);
  });

  it('con snapshot previo, retoma la partida guardada — incluida la continuidad de RNG', async () => {
    const original = runner('g-retomada');
    await original.ejecutar(crearFaccion, { nombre: 'Micenas' });
    await original.avanzarTick();

    const retomado = await RunnerDePartida.cargarOCrear('g-retomada', { seed: 1 }, { directorio, ahora: () => MOMENTO });
    await original.avanzarTick();
    await retomado.avanzarTick();

    expect(retomado.getState()).toEqual(original.getState());
  });
});

// Timers REALES a propósito, no `vi.useFakeTimers()`: cada tick del scheduler dispara E/S real a disco
// (`avanzarTick` -> `guardarPartida`), y avanzar un reloj falso no adelanta una escritura de archivo de
// verdad — con timers falsos, la mayoría de los ticks programados nunca llegaban a completar su persistencia
// antes de que el test comprobara el resultado (flakiness verificada al escribir este archivo). El coste es
// unos pocos cientos de ms reales de duración de test; a cambio, prueba el mecanismo de verdad.
function esperar(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('RunnerDePartida — ticks automáticos', () => {
  it('avanza ticks mientras está activo, y deja de avanzar al detenerlo', async () => {
    const r = runner('g-scheduler');
    r.iniciarTicksAutomaticos(10);
    await esperar(200);
    r.detenerTicksAutomaticos();
    // `detener` impide que se ENCOLE trabajo nuevo, pero no cancela el último tick que ya se hubiera
    // disparado y estuviera a medio persistir — se espera a que ese resto drene antes de fijar la
    // referencia, o un backlog bajo carga (suite completa, no este archivo solo) se leería como que el
    // scheduler seguía corriendo.
    await r.esperarColaVacia();
    const tickTrasParar = r.getState().tick;

    expect(tickTrasParar).toBeGreaterThan(0);

    // Si `detenerTicksAutomaticos` no hubiera parado el intervalo de verdad, seguiría subiendo aquí.
    await esperar(200);
    await r.esperarColaVacia();
    expect(r.getState().tick).toBe(tickTrasParar);
  });

  it('llamar dos veces a iniciar no deja un segundo intervalo huérfano sin detener', async () => {
    const r = runner('g-scheduler-doble');
    r.iniciarTicksAutomaticos(10);
    r.iniciarTicksAutomaticos(10); // si esto creara un SEGUNDO intervalo, `detener` (que solo limpia uno) lo dejaría corriendo para siempre
    await esperar(200);
    r.detenerTicksAutomaticos();
    await r.esperarColaVacia();
    const tickTrasParar = r.getState().tick;

    expect(tickTrasParar).toBeGreaterThan(0);

    await esperar(200);
    await r.esperarColaVacia();
    expect(r.getState().tick).toBe(tickTrasParar);
  });
});
