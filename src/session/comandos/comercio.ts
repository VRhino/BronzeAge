// Comandos de comercio: proponer un trueque entre asentamientos, colocar una orden de mercado y construir
// una caravana comercial.
import type { RecursoTipo } from '../../domain/types';
import { construirCaravanaComercial as construirCaravanaComercialEngine, proponerTrueque as proponerTruequeEngine } from '../../engine/trade';
import { colocarOrdenMercado as colocarOrdenMercadoEngine } from '../../engine/market';
import { asegurarCaminoComercial } from '../../engine/caminos';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento } from './ayudas';
import { evento, eventos as construirEventos, type EventoDeComando } from './eventos';

export interface PayloadTruequePropuesto {
  acuerdoId: string;
  asentamientoAId: string;
  asentamientoBId: string;
  recursoA: RecursoTipo;
  cantidadA: number;
  recursoB: RecursoTipo;
  cantidadB: number;
}
export interface PayloadCaminoComercial {
  asentamientoAId: string;
  asentamientoBId: string;
}
export interface PayloadOrdenColocada {
  ordenId: string;
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: RecursoTipo;
  cantidad: number;
  precioUnitario: number;
}
export interface PayloadCaravanaConstruida {
  caravanaId: string;
  asentamientoId: string;
}

export interface ParamsProponerTrueque {
  asentamientoAId: string;
  recursoA: RecursoTipo;
  cantidadA: number;
  asentamientoBId: string;
  recursoB: RecursoTipo;
  cantidadB: number;
}

export const proponerTrueque = comando<ParamsProponerTrueque, { acuerdoId: string }>((estado, mapa, ctx, params) => {
  const nuevo = proponerTruequeEngine(
    estado.asentamientos,
    params.asentamientoAId,
    params.asentamientoBId,
    params.recursoA,
    params.recursoB,
    params.cantidadA,
    params.cantidadB,
    ctx.instante,
    ctx.ids.siguiente()
  );

  const narrados: EventoDeComando[] = [
    {
      codigo: 'comercio.trueque_propuesto',
      mensaje: `Trueque propuesto: ${nuevo.id}.`,
      payload: { acuerdoId: nuevo.id, ...params } satisfies PayloadTruequePropuesto,
    },
  ];
  let caminos = estado.caminos;

  // Camino Comercial (Doc 1.6, Fase 0.3): se genera al establecer la relación comercial, no en cada
  // trueque — `asegurarCaminoComercial` no hace nada si el par ya tiene uno. Los asentamientos existen seguro
  // llegados aquí (`proponerTruequeEngine` los resuelve y lanza si no), así que estos `exigir` no llegan a
  // rechazar nunca; se usan igualmente para no reintroducir un `find` sin guarda.
  const a = exigirAsentamiento(estado, params.asentamientoAId);
  const b = exigirAsentamiento(estado, params.asentamientoBId);
  const previos = caminos.length;
  caminos = asegurarCaminoComercial(caminos, mapa, a, b);
  if (caminos.length > previos) {
    narrados.push({
      codigo: 'comercio.camino_creado',
      mensaje: `Nuevo camino comercial entre ${a.id} y ${b.id}.`,
      payload: { asentamientoAId: a.id, asentamientoBId: b.id } satisfies PayloadCaminoComercial,
    });
  }

  const siguiente: GameSessionState = { ...estado, acuerdos: [...estado.acuerdos, nuevo], caminos };
  return exito(siguiente, construirEventos(ctx, narrados), { acuerdoId: nuevo.id });
});

export interface ParamsColocarOrdenMercado {
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: RecursoTipo;
  cantidad: number;
  /** Sin precio, el motor usa el de referencia del mercado. */
  precio?: number;
}

export const colocarOrdenMercado = comando<ParamsColocarOrdenMercado, { ordenId: string }>((estado, _mapa, ctx, params) => {
  const nueva = colocarOrdenMercadoEngine(
    estado.asentamientos,
    params.asentamientoId,
    params.tipo,
    params.recurso,
    params.cantidad,
    ctx.instante,
    params.precio,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = { ...estado, ordenes: [...estado.ordenes, nueva] };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'mercado.orden_colocada',
        mensaje: `Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`,
        payload: {
          ordenId: nueva.id,
          asentamientoId: params.asentamientoId,
          tipo: nueva.tipo,
          // `OrdenMercado.recurso` es `string` en el dominio; `params.recurso` es el MISMO valor ya tipado.
          recurso: params.recurso,
          cantidad: nueva.cantidad,
          precioUnitario: nueva.precioUnitario,
        } satisfies PayloadOrdenColocada,
        asentamientoId: params.asentamientoId,
      }),
    ],
    { ordenId: nueva.id }
  );
});

export interface ParamsCrearCaravana {
  asentamientoId: string;
}

export const crearCaravana = comando<ParamsCrearCaravana, { caravanaId: string }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const { asentamiento: actualizado, caravana } = construirCaravanaComercialEngine(
    asentamiento,
    estado.caravanas,
    ctx.instante,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, actualizado),
    caravanas: [...estado.caravanas, caravana],
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_construida',
        mensaje: `Construye una caravana comercial (${caravana.id}).`,
        payload: { caravanaId: caravana.id, asentamientoId: params.asentamientoId } satisfies PayloadCaravanaConstruida,
        asentamientoId: params.asentamientoId,
      }),
    ],
    { caravanaId: caravana.id }
  );
});
