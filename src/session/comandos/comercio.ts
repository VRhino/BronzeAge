// Comandos de comercio: proponer un trueque entre asentamientos, colocar una orden de mercado y construir
// una caravana comercial.
//
// ⚠️ `crearCaravana` arrastraba el mismo `.find(...)!` que los comandos de cargos: un asentamiento
// inexistente producía un TypeError en vez de un rechazo. Corregido aquí.
import type { RecursoTipo } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { construirCaravanaComercial as construirCaravanaComercialEngine, proponerTrueque as proponerTruequeEngine } from '../../engine/trade';
import { colocarOrdenMercado as colocarOrdenMercadoEngine } from '../../engine/market';
import { asegurarCaminoComercial } from '../../engine/caminos';
import { eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';

export interface ParamsProponerTrueque {
  asentamientoAId: string;
  recursoA: RecursoTipo;
  cantidadA: number;
  asentamientoBId: string;
  recursoB: RecursoTipo;
  cantidadB: number;
}

export function proponerTrueque(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsProponerTrueque
): TransicionComando<{ acuerdoId: string }> {
  try {
    const nuevo = proponerTruequeEngine(
      estado.asentamientos,
      params.asentamientoAId,
      params.asentamientoBId,
      params.recursoA,
      params.recursoB,
      params.cantidadA,
      params.cantidadB,
      estado.tick,
      ctx.ids.siguiente()
    );

    const eventos = [eventoLegado(ctx.momento, estado.tick, `Trueque propuesto: ${nuevo.id}.`)];
    let caminos = estado.caminos;

    // Camino Comercial (Doc 1.6, Fase 0.3): se genera al establecer la relación comercial, no en cada
    // trueque — `asegurarCaminoComercial` no hace nada si el par ya tiene uno. Los asentamientos existen
    // seguro llegados aquí: `proponerTruequeEngine` los resuelve y lanza si no.
    const a = estado.asentamientos.find((s) => s.id === params.asentamientoAId);
    const b = estado.asentamientos.find((s) => s.id === params.asentamientoBId);
    if (a && b) {
      const previos = caminos.length;
      caminos = asegurarCaminoComercial(caminos, mapa, a, b);
      if (caminos.length > previos) {
        eventos.push(eventoLegado(ctx.momento, estado.tick, `Nuevo camino comercial entre ${a.id} y ${b.id}.`));
      }
    }

    const siguiente: GameSessionState = { ...estado, acuerdos: [...estado.acuerdos, nuevo], caminos };
    return exito(siguiente, eventos, { acuerdoId: nuevo.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsColocarOrdenMercado {
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: RecursoTipo;
  cantidad: number;
  /** Sin precio, el motor usa el de referencia del mercado. */
  precio?: number;
}

export function colocarOrdenMercado(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsColocarOrdenMercado
): TransicionComando<{ ordenId: string }> {
  try {
    const nueva = colocarOrdenMercadoEngine(
      estado.asentamientos,
      params.asentamientoId,
      params.tipo,
      params.recurso,
      params.cantidad,
      estado.tick,
      params.precio,
      ctx.ids.siguiente()
    );
    const siguiente: GameSessionState = { ...estado, ordenes: [...estado.ordenes, nueva] };
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`,
      params.asentamientoId
    );
    return exito(siguiente, [evento], { ordenId: nueva.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsCrearCaravana {
  asentamientoId: string;
}

export function crearCaravana(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsCrearCaravana
): TransicionComando<{ caravanaId: string }> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, CODIGOS_ERROR.asentamientoNoExiste);

  try {
    const { asentamiento: actualizado, caravana } = construirCaravanaComercialEngine(
      asentamiento,
      estado.caravanas,
      estado.tick,
      ctx.ids.siguiente()
    );
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)),
      caravanas: [...estado.caravanas, caravana],
    };
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${params.asentamientoId}: construye una caravana comercial (${caravana.id}).`,
      params.asentamientoId
    );
    return exito(siguiente, [evento], { caravanaId: caravana.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
