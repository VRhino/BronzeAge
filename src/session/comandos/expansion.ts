// Comandos de expansión: lanzar una Caravana de Fundación hacia un punto del mapa, y desarmarla para
// recuperar su contenido si se cambia de idea antes de que llegue.
import type { Point } from '../../domain/types';
import {
  desarmarCaravanaFundacion as desarmarCaravanaFundacionEngine,
  lanzarCaravanaFundacion as lanzarCaravanaFundacionEngine,
} from '../../engine/expansion';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento, exigirCaravana, exigirFaccionDe } from './ayudas';
import { evento } from './eventos';

export interface PayloadCaravanaFundacionLanzada {
  caravanaId: string;
  origenAsentamientoId: string;
  faccionId: string;
  destino: Point;
  numJugadores: number;
}
export interface PayloadCaravanaFundacionDesarmada {
  caravanaId: string;
  origenAsentamientoId: string;
}

export interface ParamsLanzarCaravanaFundacion {
  origenAsentamientoId: string;
  destino: Point;
  numJugadores: number;
}

export const lanzarCaravanaFundacion = comando<ParamsLanzarCaravanaFundacion, { caravanaId: string }>((estado, mapa, ctx, params) => {
  const origen = exigirAsentamiento(estado, params.origenAsentamientoId);
  const faccion = exigirFaccionDe(estado, origen);

  const resultado = lanzarCaravanaFundacionEngine(
    mapa,
    origen,
    faccion,
    params.destino,
    estado.asentamientos,
    estado.caravanas,
    params.numJugadores,
    ctx.instante,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, resultado.origenActualizado),
    caravanas: [...estado.caravanas, resultado.caravana],
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'expansion.caravana_lanzada',
        mensaje: `Lanza una Caravana de Fundación hacia (${Math.round(params.destino.x)}, ${Math.round(params.destino.y)}).`,
        payload: {
          caravanaId: resultado.caravana.id,
          origenAsentamientoId: origen.id,
          faccionId: faccion.id,
          destino: params.destino,
          numJugadores: params.numJugadores,
        } satisfies PayloadCaravanaFundacionLanzada,
        asentamientoId: origen.id,
      }),
    ],
    { caravanaId: resultado.caravana.id }
  );
});

export interface ParamsDesarmarCaravanaFundacion {
  caravanaId: string;
}

export const desarmarCaravanaFundacion = comando<ParamsDesarmarCaravanaFundacion, void>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId ?? '');

  const actualizado = desarmarCaravanaFundacionEngine(origen, caravana);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, actualizado),
    caravanas: estado.caravanas.filter((c) => c.id !== params.caravanaId),
  };
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'expansion.caravana_desarmada',
      mensaje: `Desarma la Caravana de Fundación ${params.caravanaId} y recupera su contenido.`,
      payload: { caravanaId: params.caravanaId, origenAsentamientoId: origen.id } satisfies PayloadCaravanaFundacionDesarmada,
      asentamientoId: origen.id,
    }),
  ]);
});
