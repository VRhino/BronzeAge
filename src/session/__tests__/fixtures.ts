// Fixtures compartidas por los tests de `session/comandos/*` (revisión de duplicación 2026-08-25):
// `partidaConAsentamiento` estaba clonada casi byte a byte en 4 archivos de test distintos (cargos,
// construcción, militar, expansión) — mismo seed, misma Facción, mismo punto de fundación, y solo cambiaba
// el `gameId` (cosmético, para distinguir sesiones en un log si hiciera falta depurar).
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { comprarCasa } from '../comandos/cargos';

export const MOMENTO = '2026-01-01T00:00:00.000Z';
export const ACTOR = 'jugador-test';
export const OPC = { momento: MOMENTO, actor: ACTOR };

/** Segundo residente del asentamiento de la fixture, que entra comprando casa. */
export const VECINO = 'jugador-vecino';

/**
 * Partida con una Facción y un asentamiento fundado (seed 42, en `{x:500,y:500}`).
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
  const sesion = GameSession.crear(gameId, { seed: 42 });
  const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
  const faccionId = rf.datos!.faccionId;
  const ra = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 } }, OPC);
  const asentamientoId = ra.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  sesion.ejecutar(comprarCasa, { asentamientoId, jugadorId: VECINO }, OPC);
  return { sesion, faccionId, asentamientoId, fundador, vecino: VECINO };
}
