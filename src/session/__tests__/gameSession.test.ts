// Contrato de GameSession y de sus comandos (Docs/Arquitectura/7_Diseno_GameSession.md). No reimplementa las
// reglas del motor —de eso ya se ocupa `engine/__tests__/`— sino lo que la capa de partida añade encima:
// ResultadoComando en vez de excepciones, versión que solo avanza en éxito, eventos con el `momento`
// inyectado, y que un comando rechazado no deje rastro.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { BALANCE_VERSION } from '../../constants';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { instanteDeTick, isoDeInstante } from '../estado';
import { exito, type ManejadorComando } from '../comandos/tipos';
import { conHeroe } from './fixtures';

const SEED = 42;
const ACTOR = 'jugador-test';

function partidaNueva(): GameSession {
  return GameSession.crear('partida-test', { seed: SEED });
}

/** Partida con una Facción ya creada: el motor exige que exista antes de fundar (`engine/settlement.ts`). */
function partidaConFaccion(): { sesion: GameSession; faccionId: string } {
  const sesion = conHeroe(partidaNueva(), ACTOR);
  const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR });
  if (!r.ok) throw new Error('setup del test: no se pudo crear la Facción');
  return { sesion, faccionId: r.datos!.faccionId };
}

describe('GameSession — despachador', () => {
  it('adopta el estado que devuelve el comando (única vía de mutación de la partida)', () => {
    const sesion = partidaNueva();
    expect(sesion.getState().facciones).toEqual([]);
    sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { actor: ACTOR });
    expect(sesion.getState().facciones).toHaveLength(1);
  });

  it('sin `actor` explícito usa el actor de sistema (operaciones del scheduler, no de un jugador)', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'Ugarit' }, {});
    expect(resultado.ok).toBe(true);
  });
});

describe('comando fundarAsentamiento', () => {
  it('éxito: crea el asentamiento, sube la versión y devuelve su id', () => {
    const { sesion, faccionId } = partidaConFaccion();
    const versionPrevia = sesion.getState().version;

    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });

    expect(resultado.ok).toBe(true);
    expect(resultado.codigoError).toBeUndefined();
    expect(resultado.version).toBe(versionPrevia + 1);
    expect(sesion.getState().asentamientos).toHaveLength(1);
    expect(sesion.getState().asentamientos[0]!.id).toBe(resultado.datos!.asentamientoId);
  });

  it('éxito: el evento lleva el asentamientoId y el `momento` INYECTADO, no un reloj leído dentro', () => {
    const { sesion, faccionId } = partidaConFaccion();
    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });

    expect(resultado.eventos).toHaveLength(1);
    expect(resultado.eventos[0]!.momento).toBe(isoDeInstante(instanteDeTick(0))); // comando a tick 0 → instante = época
    expect(resultado.eventos[0]!.asentamientoId).toBe(resultado.datos!.asentamientoId);
    // Código estable, ya no `'legado'`: los eventos de comando se migraron junto a los 13 subsistemas del
    // tick (A5) para que Fase C pueda filtrarlos por audiencia sin parsear el texto.
    expect(resultado.eventos[0]!.codigo).toBe('fundacion.asentamiento_fundado');
    // Se funda DONDE SE ESTA (Doc 1.3): la posicion ya no la elige el test, sale de la columna con la que
    // apareció el actor — se compara contra el asentamiento resultante, no contra un punto fijo.
    expect(resultado.eventos[0]!.payload).toMatchObject({ faccionId, posicion: sesion.getState().asentamientos[0]!.posicion });
  });

  it('éxito: registra en el historial del actor, que es el único fundador', () => {
    // Desde que `fundarAsentamiento` dejó de fabricar fundadores ficticios (`jugador-<faccionId>-<n>`), el
    // fundador es SIEMPRE el actor que ejecuta el comando — nunca una lista con más gente que él.
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });

    const historial = sesion.getState().historialHeroes;
    expect(Object.keys(historial)).toEqual([ACTOR]);
    expect(historial[ACTOR]![0]!.mensaje).toContain('Funda');
  });

  it('rechazo: fundar dentro de una plaza devuelve codigoError sin mutar nada ni subir la versión', () => {
    // Ya no se puede pedir una posición fuera del mapa (Doc 1.3: se funda donde se está, no donde se elige) —
    // el equivalente reachable es intentarlo desde DENTRO de una plaza, que `puntoDeFundacionDe` rechaza
    // igual que antes rechazaba una posición inválida: sin tocar el estado ni subir versión.
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR }); // el actor queda DENTRO de lo fundado
    const antes = sesion.getState();

    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('movilizacion.invalida');
    expect(resultado.datos).toBeUndefined();
    expect(resultado.eventos).toEqual([]);
    expect(resultado.version).toBe(antes.version);
    // Un rechazo devuelve el MISMO objeto de estado, no una copia equivalente.
    expect(sesion.getState()).toBe(antes);
  });

  it('rechazo: Facción inexistente también se traduce a codigoError, sin excepción sin capturar', () => {
    // Hace falta existir ya en el mundo —con columna— para que la posición pueda derivarse: se le da de alta
    // al actor con la propia Facción real antes de intentarlo contra una que no existe.
    const { sesion } = partidaConFaccion();
    const resultado = sesion.ejecutar(fundarAsentamiento, { faccionId: 'no-existe' }, { actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('fundacion.invalida');
  });
});

describe('comando crearFaccion', () => {
  it('éxito: crea la facción y devuelve su id', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR });

    expect(resultado.ok).toBe(true);
    expect(resultado.datos?.faccionId).toBeTruthy();
    expect(sesion.getState().facciones.map((f) => f.nombre)).toEqual(['Micenas']);
  });

  it('rechazo: nombre vacío no crea nada', () => {
    const sesion = partidaNueva();
    const resultado = sesion.ejecutar(crearFaccion, { nombre: '   ' }, { actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_vacio');
    expect(sesion.getState().facciones).toEqual([]);
  });

  it('rechazo: nombre duplicado (sin distinguir mayúsculas) no crea una segunda facción', () => {
    const sesion = partidaNueva();
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR });
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'micenas' }, { actor: ACTOR });
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.nombre_duplicado');
    expect(sesion.getState().facciones).toHaveLength(1);
  });
});

describe('operaciones del sistema — avanzarTick', () => {
  it('avanza el tick, sube la versión y fecha los eventos con el instante del tick RESULTANTE', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });
    const versionPrevia = sesion.getState().version;

    const resultado = sesion.avanzarTick();

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().tick).toBe(1);
    expect(resultado.version).toBe(versionPrevia + 1);
    // El tick lleva el mundo de instanteDeTick(0) a instanteDeTick(1) = época + 1 minuto (Fase D, doc 10):
    // sus eventos se fechan con el instante resultante, no con el de partida.
    for (const evento of resultado.eventos) expect(evento.momento).toBe(isoDeInstante(instanteDeTick(1)));
  });

  it('es determinista: mismo seed y misma secuencia producen el mismo estado', () => {
    function correr(): unknown {
      const sesion = GameSession.crear('det', { seed: SEED });
      const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR });
      sesion.ejecutar(fundarAsentamiento, { faccionId: r.datos!.faccionId }, { actor: ACTOR });
      for (let tick = 1; tick <= 20; tick++) sesion.avanzarTick();
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
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });
    const payload = sesion.exportar();
    return GameSession.importar({ ...payload, state: { ...payload.state, faccionesNpcIds: [faccionId] } });
  }

  it('sin Facciones NPC no hace nada y no sube la versión', () => {
    const { sesion } = partidaConFaccion();
    const antes = sesion.getState();
    const resultado = sesion.avanzarFaccionesNpc();
    expect(resultado).toEqual({ ok: true, eventos: [], version: antes.version });
    expect(sesion.getState()).toBe(antes);
  });

  it('con una Facción NPC toma decisiones de gobernanza sobre su asentamiento', () => {
    const sesion = partidaConFaccionNpc();
    const resultado = sesion.avanzarFaccionesNpc();

    expect(resultado.ok).toBe(true);
    // La primera decisión de gobernanza base es asignar Gobernador; no depende de ticks previos.
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBeTruthy();
  });
});

describe('GameSession — el mapa es estado devuelto, no efecto lateral', () => {
  // Prerrequisito de la persistencia de Fase B3: si el tick escribiera el mapa por dentro de la fachada
  // compartida, descartar su estado resultante (por un fallo al guardar) dejaría igualmente los yacimientos
  // vaciados, y la partida en disco no coincidiría con la que sigue en memoria.
  //
  // Se provoca con la REGENERACIÓN y no con la extracción a propósito: extraer exige que el jugador haya
  // levantado un edificio extractor, mientras que un yacimiento agotado hace que el tick le agende su
  // reaparición sin más (`Mapa.avanzarRegeneracion`). Es el mismo camino de escritura.
  function partidaConYacimientoAgotado(): { sesion: GameSession; nodoId: string } {
    const base = partidaNueva();
    const payload = base.exportar();
    const nodo = payload.state.mapa.nodos[0]!;
    payload.state.estadoMapa = { extraido: { [nodo.id]: nodo.cantidadInicial }, regeneraEn: {} };
    return { sesion: GameSession.importar(payload), nodoId: nodo.id };
  }

  it('el tick devuelve el estado del mapa y NO toca el estado anterior', () => {
    const { sesion, nodoId } = partidaConYacimientoAgotado();
    const anterior = sesion.getState();

    sesion.avanzarTick();

    expect(sesion.getState().estadoMapa.regeneraEn[nodoId]).toBeGreaterThan(0);
    expect(anterior.estadoMapa.regeneraEn).toEqual({});
    expect(sesion.getState().estadoMapa).not.toBe(anterior.estadoMapa);
  });

  it('un comando que escribe el mapa y no lo devuelve es un error, no una partida desincronizada', () => {
    const sesion = partidaNueva();
    const nodoId = sesion.getState().mapa.nodos[0]!.id; // intacto: hace falta que `extraer` entregue algo
    const descuidado: ManejadorComando<void, void> = (estado, mapa) => {
      mapa.extraer(nodoId, 1);
      return exito(estado, []); // olvida `estadoMapa` en el estado resultante
    };

    expect(() => sesion.ejecutar(descuidado, undefined, {})).toThrow(/estadoMapa/);
  });
});

describe('GameSession — exportar / importar', () => {
  it('reconstruye una partida con el mismo estado', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });
    sesion.avanzarTick();

    const reconstruida = GameSession.importar(sesion.exportar());

    expect(reconstruida.getState()).toEqual(sesion.getState());
    expect(reconstruida.gameId).toBe(sesion.gameId);
  });

  it('estampa la BALANCE_VERSION vigente al exportar (Fase C7)', () => {
    const { sesion } = partidaConFaccion();
    expect(sesion.exportar().balanceVersion).toBe(BALANCE_VERSION);
  });

  it('la sesión importada sigue siendo operable', () => {
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });

    const reconstruida = GameSession.importar(sesion.exportar());
    const resultado = reconstruida.avanzarTick();

    expect(resultado.ok).toBe(true);
    expect(reconstruida.getState().tick).toBe(1);
  });

  it('la sesión importada CONTINÚA la secuencia de RNG, no la reinicia desde la seed', () => {
    // Es la razón de ser de `estadoRng` en `PartidaExportada`: sin él, cargar el snapshot de un incidente de
    // producción y avanzar reproduce UNA continuación cualquiera, no la que realmente pasó en el servidor.
    // Aquí se verifica el efecto observable: una partida que sigue en memoria y otra reconstruida en el
    // mismo punto, avanzadas por los mismos ticks, tienen que llegar exactamente al mismo sitio —
    // reclutamiento, ataques de bandidos y demás sistemas que consumen `ctx.rng` incluidos.
    const { sesion, faccionId } = partidaConFaccion();
    sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });
    for (let i = 0; i < 10; i++) sesion.avanzarTick(); // el contador del RNG ya no está en su posición inicial

    const reconstruida = GameSession.importar(sesion.exportar());

    for (let i = 0; i < 10; i++) {
      sesion.avanzarTick();
      reconstruida.avanzarTick();
    }

    expect(reconstruida.getState()).toEqual(sesion.getState());
  });

  it('sin `estadoRng` (formato de archivo v2, anterior a esto) cae de vuelta a la seed del mundo', () => {
    const { sesion } = partidaConFaccion();
    sesion.avanzarTick();
    const payload = sesion.exportar();
    delete payload.estadoRng;

    const reconstruida = GameSession.importar(payload);

    expect(reconstruida.avanzarTick().ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------------------------------------
// El héroe se crea explícitamente (`crearHeroe`, doc 02 §4.2): actuar no da de alta a nadie.
// ---------------------------------------------------------------------------------------------------------

describe('GameSession — el héroe que actúa', () => {
  it('actuar no crea héroe: eso es cosa de `crearHeroe`', () => {
    const sesion = partidaNueva();
    expect(sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ACTOR }).ok).toBe(true);
    expect(sesion.getState().heroes).toEqual([]);
  });

  it('al fundar queda situado DENTRO de su asentamiento', () => {
    const { sesion, faccionId } = partidaConFaccion();

    const r = sesion.ejecutar(fundarAsentamiento, { faccionId }, { actor: ACTOR });
    expect(r.ok, 'setup del test: la fundación tiene que salir').toBe(true);

    const jugador = sesion.getState().heroes.find((j) => j.id === ACTOR)!;
    expect(jugador.ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId: sesion.getState().asentamientos[0]!.id });
  });

  it('un comando RECHAZADO no da de alta a nadie', () => {
    const sesion = partidaNueva();

    // Fundar sin haber actuado nunca se rechaza antes de llegar al motor: no hay columna de la que sacar
    // dónde se está.
    const r = sesion.ejecutar(fundarAsentamiento, { faccionId: 'no-existe' }, { actor: 'fantasma' });

    expect(r.ok).toBe(false);
    expect(sesion.getState().heroes).toEqual([]);
  });

  it('el SISTEMA no es un jugador: el tick no le da registro', () => {
    const sesion = partidaNueva();

    sesion.avanzarTick();

    expect(sesion.getState().heroes).toEqual([]);
  });
});
