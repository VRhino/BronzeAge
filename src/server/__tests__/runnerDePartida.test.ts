// Contrato de RunnerDePartida (Docs/Arquitectura/7_Diseno_GameSession.md §4, §8.4): cola serial, ciclo
// "aplicar -> persistir -> confirmar" con descarte en fallo, y el reloj de mundo con catch-up (D5, doc 10).
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { fundarAsentamiento } from '../../session/comandos/fundarAsentamiento';
import { alternarFaccionNpc } from '../../session/comandos/alternarFaccionNpc';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { RunnerDePartida } from '../runnerDePartida';
import { cargarPartida, type SnapshotPartida } from '../persistenciaPartida';

const MOMENTO = '2026-01-01T00:00:00.000Z';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-runner-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

function runner(gameId = 'partida-runner', seed = 7): RunnerDePartida {
  return RunnerDePartida.crear(gameId, { seed }, { almacen, ahora: () => MOMENTO });
}

describe('RunnerDePartida — cola serial', () => {
  it('tres comandos lanzados sin esperar se aplican en el orden en que llegaron', async () => {
    const r = runner();

    // Sin `await` entre medias: las tres promesas se lanzan "a la vez" desde la perspectiva del llamador.
    // Tres actores distintos: un jugador solo puede crear una Facción (Doc 2 "Entidades"), así que el mismo
    // actor para las tres habría rechazado la segunda y la tercera por `faccion.ya_pertenece`.
    const p1 = r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    const p2 = r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2');
    const p3 = r.ejecutar(crearFaccion, { nombre: 'Ugarit' }, 'jugador-3');
    await Promise.all([p1, p2, p3]);

    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Troya', 'Ugarit']);
    // La versión sube de uno en uno con cada comando aceptado — si se hubieran intercalado o perdido alguno,
    // no llegaría a 3.
    expect(r.getState().version).toBe(3);
  });

  it('un comando rechazado no bloquea ni desordena los que vienen después', async () => {
    const r = runner();
    const p1 = r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    // `fundarAsentamiento` con una facción inexistente se rechaza (error de dominio) sin lanzar.
    const p2 = r.ejecutar(fundarAsentamiento, { faccionId: 'no-existe', posicion: { x: 0, y: 0 } });
    const p3 = r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2');

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

    const cargada = await cargarPartida(almacen, 'g-persistido');
    expect(cargada!.sesion.getState().facciones).toHaveLength(1);
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
    // Actor distinto del de Micenas: si reutilizara el mismo, el rechazo de dominio (`faccion.ya_pertenece`)
    // llegaría antes que el de persistencia que este test quiere ejercitar, y nunca lanzaría.
    await expect(r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2')).rejects.toThrow();

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
    // Actor distinto del de Micenas, mismo motivo que el test anterior.
    await expect(r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2')).rejects.toThrow();

    // El "atacante" deja de escribir versiones adelantadas: el siguiente comando legítimo debe volver a
    // funcionar con normalidad, sin arrastrar nada del intento fallido. Tercer actor: ni el de Micenas ni el
    // de Troya (ambos ya tienen Facción).
    await writeFile(ruta, JSON.stringify({ ...snapshot, partida: { ...snapshot.partida, state: { ...snapshot.partida.state, version: 1 } } }), 'utf-8');
    const resultado = await r.ejecutar(crearFaccion, { nombre: 'Ugarit' }, 'jugador-3');

    expect(resultado.ok).toBe(true);
    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Ugarit']);
  });
});

describe('RunnerDePartida.avanzarTick — bundlea auto-comercio y turno NPC', () => {
  it('el turno del NPC de gobernanza ocurre DENTRO de avanzarTick, sin un comando aparte', async () => {
    const r = runner('g-npc');
    const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    const faccionId = creada.datos!.faccionId;
    await r.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 } }, 'jugador-1');
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
    const r = await RunnerDePartida.cargarOCrear('g-nueva', { seed: 1 }, { almacen, ahora: () => MOMENTO });
    expect(r.getState().tick).toBe(0);
    expect(r.getState().facciones).toEqual([]);
  });

  it('una partida nueva se persiste antes de devolverla (Fase C12): sobrevive a un "reinicio" sin ningún comando de por medio', async () => {
    await RunnerDePartida.cargarOCrear('g-recien-creada', { seed: 1 }, { almacen, ahora: () => MOMENTO });

    // "Reinicio del proceso" simulado: pedirla de nuevo debe RETOMARLA (mismo tick/estado), no crear otra
    // desde cero — solo pasa si la primera llamada la escribió a disco sin que se ejecutara ningún comando.
    const retomada = await RunnerDePartida.cargarOCrear('g-recien-creada', { seed: 99 }, { almacen, ahora: () => MOMENTO });
    expect(retomada.getState().mapa.config.seed).toBe(1); // la seed de la PRIMERA llamada, no la 99 de esta
  });

  it('con snapshot previo, retoma la partida guardada — incluida la continuidad de RNG', async () => {
    const original = runner('g-retomada');
    await original.ejecutar(crearFaccion, { nombre: 'Micenas' });
    await original.avanzarTick();

    const retomado = await RunnerDePartida.cargarOCrear('g-retomada', { seed: 1 }, { almacen, ahora: () => MOMENTO });
    await original.avanzarTick();
    await retomado.avanzarTick();

    expect(retomado.getState()).toEqual(original.getState());
  });
});

// Timers REALES a propósito, no `vi.useFakeTimers()`: cada tick del reloj de mundo dispara E/S real a disco
// (`avanzarTick` -> `guardarPartida`), y avanzar un reloj falso no adelanta una escritura de archivo de
// verdad — con timers falsos, la mayoría de los ticks nunca llegaban a completar su persistencia antes de
// que el test comprobara el resultado (flakiness verificada al escribir este archivo).
function esperar(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Runner con un reloj de PARED controlable: `avanzar(ms)` mueve el reloj hacia adelante, que es lo que el
 * reloj de mundo consulta para saber cuántos ticks se adeudan. La referencia se ancla al ARRANCAR el reloj
 * (`iniciarRelojDeMundo`), así que hay que avanzar el reloj DESPUÉS de arrancarlo para que se adeude algo. */
function runnerConReloj(gameId: string, seed = 7): { r: RunnerDePartida; avanzar: (ms: number) => void } {
  let relojMs = Date.parse(MOMENTO);
  const r = RunnerDePartida.crear(gameId, { seed }, { almacen, ahora: () => new Date(relojMs).toISOString() });
  return { r, avanzar: (ms) => (relojMs += ms) };
}

describe('RunnerDePartida — reloj de mundo (D5)', () => {
  it('arrancar el reloj NO ejecuta nada de lo que pasó antes — el mundo no avanza con el reloj parado', async () => {
    // El corazón de la decisión del 2026-09-05. Antes esto ejecutaba 5 ticks en ráfaga (catch-up); ahora la
    // referencia se ancla a "ahora" al arrancar, así que el tiempo con el reloj parado sencillamente no
    // cuenta — que es lo mismo que pasa cuando el proceso está caído.
    const { r, avanzar } = runnerConReloj('g-catchup');
    avanzar(5 * 60_000); // 5 minutos reales con el reloj PARADO
    r.iniciarRelojDeMundo(60_000);
    await r.esperarColaVacia();
    r.detenerRelojDeMundo();

    expect(r.getState().tick).toBe(0);
  });

  it('parar y reanudar el reloj re-ancla: el tiempo con el reloj parado no cuenta, ni el resto', async () => {
    // Antes de la decisión del 2026-09-05 este test probaba lo CONTRARIO: que un resto sub-intervalo
    // sobrevivía a una parada, para que el catch-up no perdiera fracciones. Con el mundo congelado mientras
    // el reloj no corre, ese resto se va con el resto del tiempo parado — y es coherente, no una regresión:
    // parar el reloj es exactamente lo que pasa cuando el proceso se apaga.
    //
    // El coste real está acotado: se pierde menos de un intervalo por arranque, y el reloj se arranca una vez
    // por apertura de partida.
    // Intervalo corto para que el `setInterval` REAL dispare dentro del test; el reloj de PARED que decide
    // cuántos ticks se deben sigue siendo el inyectado, así que el escenario es el mismo.
    const { r, avanzar } = runnerConReloj('g-resto');
    r.iniciarRelojDeMundo(50);
    avanzar(2 * 50 + 30); // 2 intervalos + un resto, con el reloj EN MARCHA
    await esperar(150);
    r.detenerRelojDeMundo();
    await r.esperarColaVacia();
    expect(r.getState().tick).toBe(2);

    // Con el reloj parado el tiempo no cuenta, y al reanudar la referencia vuelve a ser "ahora": el resto
    // que faltaba para el tercer tick tampoco se arrastra.
    avanzar(30);
    r.iniciarRelojDeMundo(50);
    await r.esperarColaVacia();
    r.detenerRelojDeMundo();
    expect(r.getState().tick).toBe(2);
  });

  it('sigue avanzando con el tiempo y deja de avanzar al detenerlo', async () => {
    const { r, avanzar } = runnerConReloj('g-continuo');
    r.iniciarRelojDeMundo(20);
    avanzar(60); // 3 intervalos: por debajo de `MAX_TICKS_POR_PASADA`, así que se recuperan
    await esperar(120); // deja que el setInterval real dispare
    await r.esperarColaVacia();
    r.detenerRelojDeMundo();
    const tickTrasParar = r.getState().tick;
    expect(tickTrasParar).toBeGreaterThan(0);

    avanzar(60); // más "tiempo real"... pero el reloj ya está detenido
    await esperar(120);
    await r.esperarColaVacia();
    expect(r.getState().tick).toBe(tickTrasParar);
  });

  it('llamar dos veces a iniciar no deja un segundo intervalo huérfano', async () => {
    const { r, avanzar } = runnerConReloj('g-doble');
    r.iniciarRelojDeMundo(20);
    r.iniciarRelojDeMundo(20); // si creara un SEGUNDO intervalo, `detener` (que limpia uno) dejaría el otro vivo
    avanzar(60);
    await esperar(120);
    await r.esperarColaVacia();
    r.detenerRelojDeMundo();
    const tickTrasParar = r.getState().tick;
    expect(tickTrasParar).toBeGreaterThan(0);

    avanzar(60);
    await esperar(120);
    await r.esperarColaVacia();
    expect(r.getState().tick).toBe(tickTrasParar);
  });

  it('reabrir una partida guardada hace 3 "horas" la reanuda donde estaba: el mundo no avanzó', async () => {
    // Lo contrario de lo que hacía D5, y a propósito (decisión del usuario, 2026-09-05). Una caída del
    // servidor no consume tiempo de mundo: lo que estuviera en construcción sigue igual de lejos de acabarse
    // que cuando se cayó, y nadie encuentra su partida saltada tres horas al volver.
    await RunnerDePartida.crearYPersistir('g-reinicio', { seed: 3 }, { almacen, ahora: () => MOMENTO });

    const tresHorasDespues = new Date(Date.parse(MOMENTO) + 3 * 60 * 60_000).toISOString();
    const reabierta = await RunnerDePartida.cargarOCrear('g-reinicio', { seed: 3 }, { almacen, ahora: () => tresHorasDespues });
    reabierta.iniciarRelojDeMundo(60_000);
    await reabierta.esperarColaVacia();
    reabierta.detenerRelojDeMundo();

    expect(reabierta.getState().tick).toBe(0);
  });

  it('un atraso GRANDE con el reloj en marcha se descarta y se cuenta, en vez de ejecutarse', async () => {
    // El caso "proceso vivo pero congelado" (host suspendido, salto de reloj por NTP): se parece a la deriva
    // del temporizador vista desde aquí, y `MAX_TICKS_POR_PASADA` es donde se traza la frontera. Por encima
    // se descarta el tiempo, igual que en una caída, y queda constancia en `ticksOmitidos`.
    const { r, avanzar } = runnerConReloj('g-congelado');
    r.iniciarRelojDeMundo(50);
    avanzar(60 * 60_000); // una hora de mundo adeudada de golpe
    await esperar(150); // deja que el setInterval real dispare
    r.detenerRelojDeMundo();
    await r.esperarColaVacia();

    expect(r.getState().tick).toBe(0);
    expect(r.metricas().ticksOmitidos).toBeGreaterThan(0);
  });

  it('un atraso PEQUEÑO sí se recupera: es la deriva del temporizador, no una caída', async () => {
    // Sin esto el mundo correría más lento que el tiempo real: `setInterval` no dispara exacto y las décimas
    // se acumulan hasta valer un tick entero.
    const { r, avanzar } = runnerConReloj('g-deriva');
    r.iniciarRelojDeMundo(50);
    avanzar(3 * 50); // 3 intervalos de atraso, por debajo del umbral
    await esperar(150);
    // Drenar ANTES de parar, no después: la ráfaga comprueba `this.relojDeMundo` en cada iteración, así que
    // `detenerRelojDeMundo` la ABORTA a medias (deliberado — un apagado no debe seguir avanzando el mundo).
    // Con el orden inverso este test salía intermitente bajo carga.
    await r.esperarColaVacia();
    r.detenerRelojDeMundo();

    expect(r.getState().tick).toBeGreaterThanOrEqual(3);
    expect(r.metricas().ticksOmitidos).toBe(0);
  });
});

describe('RunnerDePartida.ejecutar — idempotencia (Fase C5)', () => {
  it('sin idempotencyKey, dos llamadas se aplican dos veces (comportamiento de siempre)', async () => {
    const r = runner('g-sin-clave');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    // Actor distinto: un jugador ya no puede crear una segunda Facción (`faccion.ya_pertenece`).
    await r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2');
    expect(r.getState().facciones).toHaveLength(2);
  });

  it('la misma idempotencyKey y el mismo actor: el segundo ejecutar NO vuelve a aplicar el comando', async () => {
    const r = runner('g-idem');
    const primero = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1', 'clave-1');
    const segundo = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1', 'clave-1');

    expect(r.getState().facciones).toHaveLength(1); // no dos
    expect(r.getState().version).toBe(1); // la segunda llamada no subió la versión
    expect(segundo).toEqual(primero); // literalmente el mismo resultado, no uno equivalente
  });

  it('deduplica también si el reintento llega ANTES de que el primero termine (misma promesa)', async () => {
    const r = runner('g-idem-concurrente');
    // Sin `await` entre medias: el segundo `ejecutar` llega mientras el primero sigue en la cola.
    const p1 = r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1', 'clave-1');
    const p2 = r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1', 'clave-1');
    await Promise.all([p1, p2]);

    expect(r.getState().facciones).toHaveLength(1);
  });

  it('la misma clave con actores distintos NO colisiona: cada actor tiene su propio espacio de claves', async () => {
    const r = runner('g-idem-actores');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1', 'clave-1');
    await r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-2', 'clave-1');

    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Troya']);
  });

  it('una clave reutilizada tras un RECHAZO de dominio también devuelve el mismo rechazo, sin reintentar de verdad', async () => {
    const r = runner('g-idem-rechazo');
    const primero = await r.ejecutar(fundarAsentamiento, { faccionId: 'no-existe', posicion: { x: 0, y: 0 } }, 'jugador-1', 'clave-1');
    const segundo = await r.ejecutar(fundarAsentamiento, { faccionId: 'no-existe', posicion: { x: 0, y: 0 } }, 'jugador-1', 'clave-1');

    expect(primero.ok).toBe(false);
    expect(segundo).toEqual(primero);
  });

  it('si la persistencia falla, la clave NO queda cacheada: un reintento legítimo puede volver a intentarlo', async () => {
    const r = runner('g-idem-fallo');
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' }); // versión 1, guardada

    const ruta = join(directorio, 'g-idem-fallo.json');
    const snapshot = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshot.partida.state.version = 999;
    await writeFile(ruta, JSON.stringify(snapshot), 'utf-8');

    await expect(r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-1', 'clave-1')).rejects.toThrow();

    // Se arregla el conflicto y se reintenta con la MISMA clave: si hubiera quedado cacheado el fallo, esto
    // devolvería el mismo rechazo en vez de aplicar de verdad.
    const snapshotArreglado = JSON.parse(await readFile(ruta, 'utf-8')) as SnapshotPartida;
    snapshotArreglado.partida.state.version = 1;
    await writeFile(ruta, JSON.stringify(snapshotArreglado), 'utf-8');

    const resultado = await r.ejecutar(crearFaccion, { nombre: 'Troya' }, 'jugador-1', 'clave-1');
    expect(resultado.ok).toBe(true);
    expect(r.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas', 'Troya']);
  });
});

describe('RunnerDePartida — preciosReferencia (doc 9: entrada privilegiada, solo servidor)', () => {
  /** Reloj mutable — a diferencia de `runner()` de arriba (fijo), aquí hace falta poder AVANZARLO para
   * probar el TTL de un minuto real sin depender de `Date.now()` de verdad. */
  function runnerConReloj(momentoInicial: string) {
    let momento = momentoInicial;
    const r = RunnerDePartida.crear('partida-precios', { seed: 1 }, { almacen, ahora: () => momento });
    return { r, avanzarMs: (ms: number) => (momento = new Date(new Date(momento).getTime() + ms).toISOString()) };
  }

  it('sin asentamientos, el stock global es 0 y el factor se clampa al máximo (Doc 3.4)', () => {
    const { r } = runnerConReloj(MOMENTO);
    // PRECIO_BASE.madera = 1, PRECIO_REFERENCIA.factorMax = 3 -> 1 * 3 = 3.
    expect(r.preciosReferencia().madera).toBe(3);
  });

  it('trae un precio para cada recurso de PRECIO_BASE, ni uno más ni uno menos', () => {
    const { r } = runnerConReloj(MOMENTO);
    expect(Object.keys(r.preciosReferencia()).sort()).toEqual(['cobre', 'estano', 'livestock', 'madera', 'piedra', 'trigo'].sort());
  });

  it('dentro del minuto de TTL, dos lecturas devuelven el MISMO objeto — no se recalcula de más', () => {
    const { r, avanzarMs } = runnerConReloj(MOMENTO);
    const primera = r.preciosReferencia();
    avanzarMs(30_000); // 30s: dentro del TTL de 60s
    expect(r.preciosReferencia()).toBe(primera); // misma referencia: no hubo recálculo
  });

  it('pasado el minuto de TTL, la siguiente lectura recalcula', async () => {
    const { r, avanzarMs } = runnerConReloj(MOMENTO);
    const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    const rf = await r.ejecutar(fundarAsentamiento, { faccionId: creada.datos!.faccionId, posicion: { x: 500, y: 500 } }, 'jugador-1');
    expect(rf.ok).toBe(true);

    const primera = r.preciosReferencia(); // con un asentamiento recién fundado (stock inicial > 0)
    avanzarMs(60_000); // exactamente el TTL: >= dispara recálculo
    const segunda = r.preciosReferencia();

    expect(segunda).not.toBe(primera); // objeto nuevo: sí se recalculó
    expect(segunda.madera).toEqual(primera.madera); // el estado no cambió entre medias, el valor sí coincide
  });

  it('el stock de un asentamiento baja el precio frente al caso sin asentamientos', async () => {
    const sinAsentamientos = runnerConReloj(MOMENTO).r.preciosReferencia().madera!;

    const { r } = runnerConReloj(MOMENTO);
    const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    await r.ejecutar(fundarAsentamiento, { faccionId: creada.datos!.faccionId, posicion: { x: 500, y: 500 } }, 'jugador-1');

    expect(r.preciosReferencia().madera).toBeLessThanOrEqual(sinAsentamientos);
  });
});

describe('RunnerDePartida — geometriaAsentamientos (Fase C10: zonas/trazado por frame, entrada privilegiada)', () => {
  it('sin asentamientos, todo vacío', () => {
    const r = runner();
    expect(r.geometriaAsentamientos()).toEqual({ zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} });
  });

  it('sin cambios de estado, dos lecturas devuelven el MISMO objeto — memoizado por referencia, sin TTL', () => {
    const r = runner();
    const primera = r.geometriaAsentamientos();
    expect(r.geometriaAsentamientos()).toBe(primera);
  });

  it('tras fundar un asentamiento, se recalcula y trae su zona y su trazado', async () => {
    const r = runner();
    const antes = r.geometriaAsentamientos();

    const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'jugador-1');
    const fundada = await r.ejecutar(fundarAsentamiento, { faccionId: creada.datos!.faccionId, posicion: { x: 500, y: 500 } }, 'jugador-1');
    expect(fundada.ok).toBe(true);
    const asentamientoId = fundada.datos!.asentamientoId;

    const despues = r.geometriaAsentamientos();
    expect(despues).not.toBe(antes); // la referencia de `asentamientos` cambió: no es el mismo objeto cacheado
    expect(despues.zonas.map((z) => z.asentamientoId)).toEqual([asentamientoId]);
    expect(despues.zonasFusionadas).toEqual([{ faccionId: creada.datos!.faccionId, contornos: expect.any(Array) }]);
    expect(despues.trazadoPorAsentamiento[asentamientoId]).toBeDefined();
  });

  it('un comando que NO toca asentamientos deja la misma referencia cacheada (crearFaccion sola)', async () => {
    const r = runner();
    const primera = r.geometriaAsentamientos();
    await r.ejecutar(crearFaccion, { nombre: 'Micenas' }); // no funda: `estado.asentamientos` sigue siendo el mismo array
    expect(r.geometriaAsentamientos()).toBe(primera);
  });
});

describe('RunnerDePartida — el reloj de pared NO entra en el estado (Fase D / doc 10 §7, regresión del bug)', () => {
  /** Corre la misma secuencia con un `ahora` (reloj de pared) distinto y devuelve el snapshot resultante y el
   * historial de eventos (JSONL hermano, formato v13), ignorando `guardadoEn` (que SÍ es reloj de pared, y a
   * propósito — es metadato del archivo). */
  async function snapshotTras(ahora: () => string): Promise<{ partida: SnapshotPartida['partida']; eventos: unknown[] }> {
    const dir = await mkdtemp(join(tmpdir(), 'bronzeage-reloj-'));
    try {
      const r = RunnerDePartida.crear('g', { seed: 7 }, { almacen: crearAlmacenEnDisco(dir), ahora });
      const creada = await r.ejecutar(crearFaccion, { nombre: 'Micenas' }, 'ana');
      await r.ejecutar(fundarAsentamiento, { faccionId: creada.datos!.faccionId, posicion: { x: 500, y: 500 } }, 'ana');
      await r.avanzarTick();
      await r.avanzarTick();
      const { partida } = JSON.parse(await readFile(join(dir, 'g.json'), 'utf-8')) as SnapshotPartida;
      const jsonl = await readFile(join(dir, 'g.eventos.jsonl'), 'utf-8');
      const eventos = jsonl.split('\n').filter((l) => l.trim() !== '').map((l) => JSON.parse(l));
      return { partida, eventos };
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  }

  it('mismo comando + misma seed con relojes de pared distintos → snapshot Y eventos IDÉNTICOS', async () => {
    const enEnero = await snapshotTras(() => '2026-01-01T00:00:00.000Z');
    const enJulio = await snapshotTras(() => '2026-07-15T12:34:56.000Z');
    // Antes de D1 esto fallaba: `ctx.momento` era `ahora()` y viajaba a `eventosDominio[].momento`.
    expect(enJulio.partida).toEqual(enEnero.partida);
    expect(enJulio.eventos).toEqual(enEnero.eventos);
  });

  it('los eventos se fechan con el instante de MUNDO (derivado del tick), no con el reloj de pared', async () => {
    const { eventos } = await snapshotTras(() => '2099-12-31T23:59:59.000Z');
    expect(eventos.length).toBeGreaterThan(0);
    for (const evento of eventos as Array<{ momento: string }>) {
      expect(evento.momento.startsWith('2026-01-01T00:0')).toBe(true); // época + unos pocos minutos, no 2099
    }
  });
});
