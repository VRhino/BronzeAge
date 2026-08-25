// Fixtures compartidas por los tests de `session/comandos/*` (revisión de duplicación 2026-08-25):
// `partidaConAsentamiento` estaba clonada casi byte a byte en 4 archivos de test distintos (cargos,
// construcción, militar, expansión) — mismo seed, misma Facción, mismo punto de fundación, y solo cambiaba
// el `gameId` (cosmético, para distinguir sesiones en un log si hiciera falta depurar).
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';

export const MOMENTO = '2026-01-01T00:00:00.000Z';
export const ACTOR = 'jugador-test';
export const OPC = { momento: MOMENTO, actor: ACTOR };

/**
 * Partida con una Facción y un asentamiento fundado (seed 42, 2 jugadores en `{x:500,y:500}`). Devuelve
 * también el id del primer jugador fundador — es ciudadano de la Facción, requisito del motor para varios
 * cargos (Rey, Gobernador...) — así que la mayoría de los comandos de prueba lo pueden usar directamente sin
 * tener que resolverlo ellos mismos.
 */
export function partidaConAsentamiento(gameId = 'test'): {
  sesion: GameSession;
  faccionId: string;
  asentamientoId: string;
  fundador: string;
} {
  const sesion = GameSession.crear(gameId, { seed: 42 });
  const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
  const faccionId = rf.datos!.faccionId;
  const ra = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, OPC);
  const asentamientoId = ra.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  return { sesion, faccionId, asentamientoId, fundador };
}
