// Fixtures compartidas por los tests de `session/comandos/*` (revisión de duplicación 2026-08-25):
// `partidaConAsentamiento` estaba clonada casi byte a byte en 4 archivos de test distintos (cargos,
// construcción, militar, expansión) — mismo seed, misma Facción, mismo punto de fundación, y solo cambiaba
// el `gameId` (cosmético, para distinguir sesiones en un log si hiciera falta depurar).
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { comprarCasa } from '../comandos/cargos';
import { instanteDeTick, isoDeInstante } from '../estado';

/** `EventoDominio.momento` (ISO 8601) de un comando sobre una partida recién creada — instante del tick 0
 * (= `SIMULACION.epocaInicial`). Ya no se inyecta un `momento`, lo deriva `GameSession` del tick (Fase D /
 * doc 10). */
export const MOMENTO = isoDeInstante(instanteDeTick(0));
export const ACTOR = 'jugador-test';
export const OPC = { actor: ACTOR };

/** Segundo residente del asentamiento de la fixture, que entra comprando casa. */
export const VECINO = 'jugador-vecino';

/**
 * Planta la columna de un jugador en un punto concreto, DEVOLVIENDO una sesión nueva (`GameSession` no
 * expone ninguna forma de mover una columna sin pasar por un comando de movimiento real).
 *
 * Existe porque se funda donde se está (Doc 1.3): un test que quiera fundar en un sitio elegido —o construir
 * geometría relativa a un punto fijo, como "una plaza rival a tal distancia de la propia"— tiene que LLEVAR
 * ahí a su fundador antes. Caminar de verdad costaría ticks que ningún test de estos mide.
 */
export function enPie(sesion: GameSession, jugadorId: string, punto: { x: number; y: number }): GameSession {
  const payload = sesion.exportar();
  const columna = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.jugadorId === jugadorId));
  if (!columna) throw new Error(`el fixture esperaba que ${jugadorId} tuviera columna: ¿ha ejecutado algún comando?`);
  return GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      ejercitos: payload.state.ejercitos.map((e) => (e.id === columna.id ? { ...e, posicionActual: punto } : e)),
    },
  });
}

/**
 * Partida con una Facción y un asentamiento fundado (seed 42, en `{x:400,y:400}`).
 *
 * El fundador es el ACTOR de `OPC`: desde que `fundarAsentamiento` dejó de fabricar fundadores ficticios
 * (`jugador-<faccionId>-<n>`), funda quien ejecuta el comando, y con ello se gana casa y ciudadanía. Se
 * devuelve `fundador` igualmente para no obligar a cada test a resolverlo.
 *
 * `vecino` es un SEGUNDO residente que entra por la otra vía que admite el juego: comprar casa (Doc 2.5).
 * Antes salía de una fundación grupal con jugadores inventados; ahora se gana la residencia como lo haría una
 * persona. Lo usan los tests que necesitan distinguir "reside aquí" de "manda aquí".
 */
export function partidaConAsentamiento(gameId = 'test'): {
  sesion: GameSession;
  faccionId: string;
  asentamientoId: string;
  fundador: string;
  vecino: string;
} {
  let sesion = GameSession.crear(gameId, { seed: 42 });
  const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
  const faccionId = rf.datos!.faccionId;
  // Se funda DONDE SE ESTA (Doc 1.3): se lleva al fundador a un punto fijo antes de fundar, ya no es un
  // parámetro del comando. (400,400) y no (500,500): en la seed 42 ese punto es AGUA, y desde que el agua es
  // infranqueable (2026-09-02) fundar ahí ya no es legal — el fixture llevaba fundando en el mar sin que se
  // notara, porque nada dependía del terreno. Este está en llano y es `recomendable`.
  sesion = enPie(sesion, ACTOR, { x: 400, y: 400 });
  const ra = sesion.ejecutar(fundarAsentamiento, { faccionId }, OPC);
  const asentamientoId = ra.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  sesion.ejecutar(comprarCasa, { asentamientoId, jugadorId: VECINO }, OPC);
  return { sesion, faccionId, asentamientoId, fundador, vecino: VECINO };
}
