// Comandos de expansión: lanzar una Caravana de Fundación hacia un punto del mapa, y desarmarla para
// recuperar su contenido si se cambia de idea antes de que llegue.
//
// ⚠️ Mismo `.find(...)!` de siempre en ambos, sobre asentamiento y caravana.
import type { Point } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import {
  desarmarCaravanaFundacion as desarmarCaravanaFundacionEngine,
  lanzarCaravanaFundacion as lanzarCaravanaFundacionEngine,
} from '../../engine/expansion';
import { eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';

export interface ParamsLanzarCaravanaFundacion {
  origenAsentamientoId: string;
  destino: Point;
  numJugadores: number;
}

export function lanzarCaravanaFundacion(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsLanzarCaravanaFundacion
): TransicionComando<{ caravanaId: string }> {
  const origen = estado.asentamientos.find((a) => a.id === params.origenAsentamientoId);
  if (!origen) return rechazo(estado, CODIGOS_ERROR.asentamientoNoExiste);
  const faccion = estado.facciones.find((f) => f.id === origen.faccionId);
  if (!faccion) return rechazo(estado, CODIGOS_ERROR.faccionNoExiste);

  try {
    const resultado = lanzarCaravanaFundacionEngine(
      mapa,
      origen,
      faccion,
      params.destino,
      estado.asentamientos,
      estado.caravanas,
      params.numJugadores,
      estado.tick,
      ctx.ids.siguiente()
    );
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === origen.id ? resultado.origenActualizado : a)),
      caravanas: [...estado.caravanas, resultado.caravana],
    };
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${origen.id}: lanza una Caravana de Fundación hacia (${Math.round(params.destino.x)}, ${Math.round(params.destino.y)}).`,
      origen.id
    );
    return exito(siguiente, [evento], { caravanaId: resultado.caravana.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsDesarmarCaravanaFundacion {
  caravanaId: string;
}

export function desarmarCaravanaFundacion(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsDesarmarCaravanaFundacion
): TransicionComando<void> {
  const caravana = estado.caravanas.find((c) => c.id === params.caravanaId);
  if (!caravana) return rechazo(estado, CODIGOS_ERROR.caravanaNoExiste);
  const origen = estado.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);
  if (!origen) return rechazo(estado, CODIGOS_ERROR.asentamientoNoExiste);

  try {
    const actualizado = desarmarCaravanaFundacionEngine(origen, caravana);
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === origen.id ? actualizado : a)),
      caravanas: estado.caravanas.filter((c) => c.id !== params.caravanaId),
    };
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${origen.id}: desarma la Caravana de Fundación ${params.caravanaId} y recupera su contenido.`,
      origen.id
    );
    return exito(siguiente, [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
