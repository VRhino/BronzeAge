// Primeros comandos migrados a GameSession (Docs/Arquitectura/7_Diseno_GameSession.md) — no reimplementa las
// reglas del motor (eso ya lo cubre `engine/__tests__/`), verifica el CONTRATO nuevo: ResultadoComando en vez
// de excepciones/log, version que solo avanza en éxito, y que GameSession sigue siendo reproducible (mismo
// patrón de determinismo que ya protege `engine/__tests__/determinismo.test.ts`).
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';

const SEED = 42;
const MOMENTO = '2026-01-01T00:00:00.000Z';

function partidaNueva(): GameSession {
  return GameSession.crear('partida-test', { seed: SEED });
}

/** Partida con una Facción 'faccion-1' ya creada — la mayoría de los tests de `fundarAsentamiento` la
 * necesitan como requisito previo (el motor exige que la Facción exista, ver `engine/settlement.ts`). */
function partidaConFaccion(): GameSession {
  const sesion = partidaNueva();
  const r = sesion.crearFaccion(MOMENTO, 'faccion-1');
  if (!r.ok) throw new Error('setup del test: no se pudo crear la facción');
  return sesion;
}

describe('GameSession — fundarAsentamiento', () => {
  it('éxito: crea el asentamiento, sube la versión y devuelve el id en `datos`', () => {
    const sesion = partidaConFaccion();
    const faccionId = sesion.getState().facciones[0]!.id;
    const antes = sesion.getState().version;

    const resultado = sesion.fundarAsentamiento(MOMENTO, faccionId, { x: 500, y: 500 }, 3);

    expect(resultado.ok).toBe(true);
    expect(resultado.codigoError).toBeUndefined();
    expect(resultado.datos?.asentamientoId).toBeTruthy();
    expect(resultado.version).toBe(antes + 1);
    expect(sesion.getState().asentamientos).toHaveLength(1);
    expect(sesion.getState().asentamientos[0]!.id).toBe(resultado.datos!.asentamientoId);
  });

  it('éxito: el evento estructurado lleva el asentamientoId y el `momento` inyectado, no un reloj interno', () => {
    const sesion = partidaConFaccion();
    const faccionId = sesion.getState().facciones[0]!.id;
    const resultado = sesion.fundarAsentamiento(MOMENTO, faccionId, { x: 500, y: 500 }, 1);

    expect(resultado.eventos).toHaveLength(1);
    expect(resultado.eventos[0]!.momento).toBe(MOMENTO);
    expect(resultado.eventos[0]!.asentamientoId).toBe(resultado.datos!.asentamientoId);
    expect(resultado.eventos[0]!.codigo).toBe('legado');
  });

  it('rechazo: posición fuera del mapa devuelve codigoError sin mutar nada ni subir la versión', () => {
    const sesion = partidaConFaccion();
    const faccionId = sesion.getState().facciones[0]!.id;
    const antes = sesion.getState();

    const resultado = sesion.fundarAsentamiento(MOMENTO, faccionId, { x: -100, y: -100 }, 1);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fundacion.invalida');
    expect(resultado.datos).toBeUndefined();
    expect(resultado.eventos).toEqual([]);
    expect(resultado.version).toBe(antes.version);
    expect(sesion.getState().asentamientos).toEqual(antes.asentamientos);
  });

  it('rechazo: Facción inexistente también se traduce a codigoError, sin excepción sin capturar', () => {
    const sesion = partidaNueva();
    const resultado = sesion.fundarAsentamiento(MOMENTO, 'no-existe', { x: 500, y: 500 }, 1);
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fundacion.invalida');
  });

  it('un comando rechazado no añade nada al log administrativo', () => {
    const sesion = partidaConFaccion();
    const faccionId = sesion.getState().facciones[0]!.id;
    const logAntes = sesion.getState().log;
    sesion.fundarAsentamiento(MOMENTO, faccionId, { x: -100, y: -100 }, 1);
    expect(sesion.getState().log).toBe(logAntes);
  });
});

describe('GameSession — crearFaccion', () => {
  it('éxito: crea la facción y devuelve su id', () => {
    const sesion = partidaNueva();
    const resultado = sesion.crearFaccion(MOMENTO, 'Micenas');

    expect(resultado.ok).toBe(true);
    expect(resultado.datos?.faccionId).toBeTruthy();
    expect(sesion.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas']);
  });

  it('rechazo: nombre vacío no crea nada', () => {
    const sesion = partidaNueva();
    const resultado = sesion.crearFaccion(MOMENTO, '   ');
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_vacio');
    expect(sesion.getState().facciones).toEqual([]);
  });

  it('rechazo: nombre duplicado (sin distinguir mayúsculas) no crea una segunda facción', () => {
    const sesion = partidaNueva();
    sesion.crearFaccion(MOMENTO, 'Micenas');
    const resultado = sesion.crearFaccion(MOMENTO, 'micenas');
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_duplicado');
    expect(sesion.getState().facciones).toHaveLength(1);
  });
});

describe('GameSession — avanzarTick', () => {
  it('avanza el tick, sube la versión y produce eventosDominio con el `momento` correcto', () => {
    const sesion = partidaConFaccion();
    sesion.fundarAsentamiento(MOMENTO, sesion.getState().facciones[0]!.id, { x: 500, y: 500 }, 1);
    const versionPrevia = sesion.getState().version;

    const resultado = sesion.avanzarTick(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().tick).toBe(1);
    expect(resultado.version).toBe(versionPrevia + 1);
    for (const evento of resultado.eventos) expect(evento.momento).toBe(MOMENTO);
  });

  it('es determinista: mismo seed y misma secuencia de comandos/ticks producen el mismo estado', () => {
    function correr(): unknown {
      const sesion = GameSession.crear('det', { seed: SEED });
      sesion.crearFaccion(MOMENTO, 'faccion-1');
      sesion.fundarAsentamiento(MOMENTO, sesion.getState().facciones[0]!.id, { x: 500, y: 500 }, 3);
      for (let tick = 1; tick <= 20; tick++) sesion.avanzarTick(`2026-01-01T00:${String(tick).padStart(2, '0')}:00.000Z`);
      return sesion.getState();
    }

    expect(correr()).toEqual(correr());
  });
});

describe('GameSession — avanzarFaccionesNpc', () => {
  // Todavía no existe un comando administrativo para fijar `faccionesNpcIds` (pendiente, ver Docs/
  // Arquitectura/4_Plan_Evolucion_Tareas.md) — se construye el estado directamente vía `importar()`, que sí
  // acepta cualquier `GameSessionState` válido, para poder ejercitar el método sin ese comando.
  function partidaConFaccionNpc(): GameSession {
    const base = partidaConFaccion();
    const faccionId = base.getState().facciones[0]!.id;
    base.fundarAsentamiento(MOMENTO, faccionId, { x: 500, y: 500 }, 1);
    const payload = base.exportar();
    return GameSession.importar({ ...payload, state: { ...payload.state, faccionesNpcIds: [faccionId] } });
  }

  it('sin `faccionesNpcIds` no hace nada (early return, mismo patrón que `GameStore`)', () => {
    const sesion = partidaConFaccion();
    const antes = sesion.getState();
    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);
    expect(resultado).toEqual({ ok: true, eventos: [], version: antes.version });
    expect(sesion.getState()).toEqual(antes);
  });

  it('con una Facción NPC, el turno puede tomar decisiones sobre su asentamiento (gobernanza base)', () => {
    const sesion = partidaConFaccionNpc();
    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);

    expect(resultado.ok).toBe(true);
    // La primera decisión de gobernanza base es asignar Gobernador — no depende de ticks previos.
    const asentamiento = sesion.getState().asentamientos[0]!;
    expect(asentamiento.cargos.gobernadorId).toBeTruthy();
  });
});

describe('GameSession — exportar / importar', () => {
  it('reconstruye una partida con el mismo estado tras exportar e importar', () => {
    const original = partidaConFaccion();
    original.fundarAsentamiento(MOMENTO, original.getState().facciones[0]!.id, { x: 500, y: 500 }, 2);
    original.avanzarTick(MOMENTO);

    const reconstruida = GameSession.importar(original.exportar());

    expect(reconstruida.getState()).toEqual(original.getState());
  });

  it('la sesión importada sigue siendo operable (puede avanzar tick sin errores)', () => {
    const original = partidaConFaccion();
    original.fundarAsentamiento(MOMENTO, original.getState().facciones[0]!.id, { x: 500, y: 500 }, 2);

    const reconstruida = GameSession.importar(original.exportar());
    const resultado = reconstruida.avanzarTick(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(reconstruida.getState().tick).toBe(1);
  });
});
