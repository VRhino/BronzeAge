import type { Point } from '../../domain/types';
import { fundarAsentamiento as fundarAsentamientoEngine } from '../../engine/settlement';
import { FUNDACION } from '../../constants';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando } from './ayudas';
import { evento } from './eventos';

export interface PayloadAsentamientoFundado {
  asentamientoId: string;
  faccionId: string;
  posicion: Point;
  jugadoresIds: string[];
}

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
export const fundarAsentamiento = comando<ParamsFundarAsentamiento, { asentamientoId: string }>((estado, mapa, ctx, params) => {
  // Los ids de jugador los sigue derivando la capa de partida a partir de la Facción, igual que hacía
  // `GameStore`. En Fase C esto cambia: los jugadores serán identidades reales (`Jugador`, doc 5) y llegarán
  // resueltos desde la sesión autenticada, no fabricados aquí.
  const n = Math.min(FUNDACION.maxJugadoresFundacionGrupal, Math.max(1, params.numJugadores || 1));
  const jugadoresIds = Array.from({ length: n }, (_, i) => `jugador-${params.faccionId}-${i + 1}`);

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

  return exito(
    siguiente,
    [
      evento(ctx, estado, {
        codigo: 'fundacion.asentamiento_fundado',
        mensaje: `${nombreFaccion} funda asentamiento en (${Math.round(params.posicion.x)}, ${Math.round(params.posicion.y)}).`,
        payload: {
          asentamientoId: resultado.asentamiento.id,
          faccionId: params.faccionId,
          posicion: params.posicion,
          jugadoresIds,
        } satisfies PayloadAsentamientoFundado,
        asentamientoId: resultado.asentamiento.id,
      }),
    ],
    { asentamientoId: resultado.asentamiento.id }
  );
});
