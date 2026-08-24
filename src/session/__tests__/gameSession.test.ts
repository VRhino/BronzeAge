// Contrato de GameSession y de sus comandos (Docs/Arquitectura/7_Diseno_GameSession.md). No reimplementa las
// reglas del motor —de eso ya se ocupa `engine/__tests__/`— sino lo que la capa de partida añade encima:
// ResultadoComando en vez de excepciones, versión que solo avanza en éxito, eventos con el `momento`
// inyectado, y que un comando rechazado no deje rastro.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';

const SEED = 42;
const MOMENTO = '2026-01-01T00:00:00.000Z';
const ACTOR = 'jugador-test';

function partidaNueva(): GameSession {
  return GameSession.crear('partida-test', { seed: SEED });
}

/** Partida con una Facción ya creada: el motor exige que exista antes de fundar (`engine/settlement.ts`). */
function partidaConFaccion(): { sesion: GameSession; faccionId: string } {
  const sesion = partidaNueva();
  const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { momento: MOMENTO, actor: ACTOR });
  if (!r.ok) throw new Error('setup del test: no se pudo crear la Facción');
  return { sesion, faccionId: r.datos!.faccionId };
}

describe('GameSession — despachador', () => {
  it('adopta el estado que devuelve el comando (única vía de mutación de la partida)', () => {
    const sesion = partidaNueva();
    expect(sesion.getState().facciones).toEqual([]);
    sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { momento: MOMENTO, actor: ACTOR });
    expect(sesion.getState().facciones).toHaveLength(1);
  });

  it('sin `actor` explícito usa el actor de sistema (operaciones del scheduler, no de un jugador)', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'Ugarit' }, { momento: MOMENTO });
    expect(resultado.ok).toBe(true);
  });
});

describe('comando fundarAsentamiento', () => {
  it('éxito: crea el asentamiento, sube la versión y devuelve su id', () => {
    const { sesion, faccionId } = partidaConFaccion();
    const versionPrevia = sesion.getState().version;

    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 3 }, { momento: MOMENTO, actor: ACTOR });

    expect(resultado.ok).toBe(true);
    expect(resultado.codigoError).toBeUndefined();
    expect(resultado.version).toBe(versionPrevia + 1);
    expect(sesion.getState().asentamientos).toHaveLength(1);
    expect(sesion.getState().asentamientos[0]!.id).toBe(resultado.datos!.asentamientoId);
  });

  it('éxito: el evento lleva el asentamientoId y el `momento` INYECTADO, no un reloj leído dentro', () => {
    const { sesion, faccionId } = partidaConFaccion();
    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 1 }, { momento: MOMENTO, actor: ACTOR });

    expect(resultado.eventos).toHaveLength(1);
    expect(resultado.eventos[0]!.momento).toBe(MOMENTO);
    expect(resultado.eventos[0]!.asentamientoId).toBe(resultado.datos!.asentamientoId);
    expect(resultado.eventos[0]!.codigo).toBe('legado');
  });

  it('éxito: registra en el historial de cada jugador fundador', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, { momento: MOMENTO, actor: ACTOR });

    const historial = sesion.getState().historialJugadores;
    expect(Object.keys(historial)).toHaveLength(2);
    for (const entradas of Object.values(historial)) expect(entradas[0]!.mensaje).toContain('Funda');
  });

  it('rechazo: posición fuera del mapa devuelve codigoError sin mutar nada ni subir la versión', () => {
    const { sesion, faccionId } = partidaConFaccion();
    const antes = sesion.getState();

    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: -100, y: -100 }, numJugadores: 1 }, { momento: MOMENTO, actor: ACTOR });

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fundacion.invalida');
    expect(resultado.datos).toBeUndefined();
    expect(resultado.eventos).toEqual([]);
    expect(resultado.version).toBe(antes.version);
    // Un rechazo devuelve el MISMO objeto de estado, no una copia equivalente.
    expect(sesion.getState()).toBe(antes);
  });

  it('rechazo: Facción inexistente también se traduce a codigoError, sin excepción sin capturar', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId: 'no-existe', posicion: { x: 500, y: 500 }, numJugadores: 1 }, { momento: MOMENTO, actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fundacion.invalida');
  });
});

describe('comando crearFaccion', () => {
  it('éxito: crea la facción y devuelve su id', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { momento: MOMENTO, actor: ACTOR });

    expect(resultado.ok).toBe(true);
    expect(resultado.datos?.faccionId).toBeTruthy();
    expect(sesion.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas']);
  });

  it('rechazo: nombre vacío no crea nada', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: '   ' }, { momento: MOMENTO, actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_vacio');
    expect(sesion.getState().facciones).toEqual([]);
  });

  it('rechazo: nombre duplicado (sin distinguir mayúsculas) no crea una segunda facción', () => {
    const sesion = partidaNueva();
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { momento: MOMENTO, actor: ACTOR });
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'micenas' }, { momento: MOMENTO, actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_duplicado');
    expect(sesion.getState().facciones).toHaveLength(1);
  });
});

describe('operaciones del sistema — avanzarTick', () => {
  it('avanza el tick, sube la versión y produce eventos con el `momento` correcto', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 1 }, { momento: MOMENTO, actor: ACTOR });
    const versionPrevia = sesion.getState().version;

    const resultado = sesion.avanzarTick(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().tick).toBe(1);
    expect(resultado.version).toBe(versionPrevia + 1);
    for (const evento of resultado.eventos) expect(evento.momento).toBe(MOMENTO);
  });

  it('es determinista: mismo seed y misma secuencia producen el mismo estado', () => {
    function correr(): unknown {
      const sesion = GameSession.crear('det', { seed: SEED });
      const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { momento: MOMENTO, actor: ACTOR });
      sesion.ejecutar(fundarAsentamiento, { faccionId: r.datos!.faccionId, posicion: { x: 500, y: 500 }, numJugadores: 3 }, { momento: MOMENTO, actor: ACTOR });
      for (let tick = 1; tick <= 20; tick++) sesion.avanzarTick(`2026-01-01T00:${String(tick).padStart(2, '0')}:00.000Z`);
      return sesion.getState();
    }

    expect(correr()).toEqual(correr());
  });
});

describe('operaciones del sistema — avanzarFaccionesNpc', () => {
  // Todavía no existe el comando administrativo para fijar `faccionesNpcIds` (pendiente, ver doc 4), así que
  // se construye el estado vía `importar()`, que acepta cualquier `GameSessionState` válido.
  function partidaConFaccionNpc(): GameSession {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 1 }, { momento: MOMENTO, actor: ACTOR });
    const payload = sesion.exportar();
    return GameSession.importar({ ...payload, state: { ...payload.state, faccionesNpcIds: [faccionId] } });
  }

  it('sin Facciones NPC no hace nada y no sube la versión', () => {
    const { sesion } = partidaConFaccion();
    const antes = sesion.getState();
    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);
    expect(resultado).toEqual({ ok: true, eventos: [], version: antes.version });
    expect(sesion.getState()).toBe(antes);
  });

  it('con una Facción NPC toma decisiones de gobernanza sobre su asentamiento', () => {
    const sesion = partidaConFaccionNpc();
    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);

    expect(resultado.ok).toBe(true);
    // La primera decisión de gobernanza base es asignar Gobernador; no depende de ticks previos.
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBeTruthy();
  });
});

describe('GameSession — exportar / importar', () => {
  it('reconstruye una partida con el mismo estado', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, { momento: MOMENTO, actor: ACTOR });
    sesion.avanzarTick(MOMENTO);

    const reconstruida = GameSession.importar(sesion.exportar());

    expect(reconstruida.getState()).toEqual(sesion.getState());
    expect(reconstruida.gameId).toBe(sesion.gameId);
  });

  it('la sesión importada sigue siendo operable', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, { momento: MOMENTO, actor: ACTOR });

    const reconstruida = GameSession.importar(sesion.exportar());
    const resultado = reconstruida.avanzarTick(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(reconstruida.getState().tick).toBe(1);
  });
});
