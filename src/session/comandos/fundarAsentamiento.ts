import type { Point } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { fundarAsentamiento as fundarAsentamientoEngine } from '../../engine/settlement';
import { FUNDACION } from '../../constants';
import { conHistorialDeJugador, eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';

export interface ParamsFundarAsentamiento {
  faccionId: string;
  posicion: Point;
  numJugadores: number;
}

/**
 * Funda un asentamiento nuevo para una Facción existente.
 *
 * Autorización (matriz del doc 5, pendiente de implementar en Fase C): rol `jugador`, y la Facción objetivo
 * debe ser la propia del actor.
 */
export function fundarAsentamiento(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsFundarAsentamiento
): TransicionComando<{ asentamientoId: string }> {
  // Los ids de jugador los sigue derivando la capa de partida a partir de la Facción, igual que hacía
  // `GameStore`. En Fase C esto cambia: los jugadores serán identidades reales (`Jugador`, doc 5) y llegarán
  // resueltos desde la sesión autenticada, no fabricados aquí.
  const n = Math.min(FUNDACION.maxJugadoresFundacionGrupal, Math.max(1, params.numJugadores || 1));
  const jugadoresIds = Array.from({ length: n }, (_, i) => `jugador-${params.faccionId}-${i + 1}`);

  try {
    const resultado = fundarAsentamientoEngine(
      mapa,
      estado.facciones,
      params.faccionId,
      params.posicion,
      jugadoresIds,
      estado.asentamientos,
      estado.tick
    );

    let siguiente: GameSessionState = {
      ...estado,
      asentamientos: [...estado.asentamientos, resultado.asentamiento],
      facciones: resultado.facciones,
    };

    const nombreFaccion = resultado.facciones.find((f) => f.id === params.faccionId)?.nombre ?? params.faccionId;
    for (const jugadorId of jugadoresIds) {
      siguiente = conHistorialDeJugador(siguiente, jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
    }

    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${nombreFaccion} funda asentamiento en (${Math.round(params.posicion.x)}, ${Math.round(params.posicion.y)}).`,
      resultado.asentamiento.id
    );

    return exito(siguiente, [evento], { asentamientoId: resultado.asentamiento.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
