import type { Asentamiento, OrdenMercado } from '../domain/types';
import { COMISION, PRECIO_BASE, PRECIO_REFERENCIA } from '../constants';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { factorComisionExterna } from './politicas';

export class OrdenInvalidaError extends Error {}

/** Precio de referencia por defecto según escasez/abundancia GLOBAL del recurso (Doc 3.4, sin componente de distancia). */
export function calcularPrecioReferencia(recurso: string, asentamientos: Asentamiento[]): number {
  const base = PRECIO_BASE[recurso];
  if (base === undefined) return 0; // oro no cotiza contra sí mismo
  const stockGlobal = asentamientos.reduce((acc, a) => acc + cantidadDisponible(a.almacen, recurso), 0);
  const factor = PRECIO_REFERENCIA.stockObjetivoGlobal / Math.max(1, stockGlobal);
  const factorClamped = Math.max(PRECIO_REFERENCIA.factorMin, Math.min(PRECIO_REFERENCIA.factorMax, factor));
  return base * factorClamped;
}

/** Coloca una orden de compra/venta (Doc 3.3). Si no se especifica precio, se usa el de referencia vigente. */
export function colocarOrdenMercado(
  asentamientos: Asentamiento[],
  asentamientoId: string,
  tipo: 'compra' | 'venta',
  recurso: string,
  cantidad: number,
  tickActual: number,
  precioUnitario?: number,
  contador = 0
): OrdenMercado {
  const asentamiento = asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) throw new OrdenInvalidaError('El asentamiento no existe.');
  if (cantidad <= 0) throw new OrdenInvalidaError('La cantidad debe ser mayor que 0.');
  if (tipo === 'venta' && cantidadDisponible(asentamiento.almacen, recurso) < cantidad) {
    throw new OrdenInvalidaError('No hay suficiente stock para vender esa cantidad.');
  }

  return {
    id: `orden-${asentamientoId}-${tickActual}-${contador}`,
    asentamientoId,
    tipo,
    recurso,
    cantidad,
    cantidadCumplida: 0,
    precioUnitario: precioUnitario ?? calcularPrecioReferencia(recurso, asentamientos),
    creadoEnTick: tickActual,
    estado: 'activa',
  };
}

/** Tasa base (Doc 3.5); si es externa, el Tesorero del vendedor puede modularla vía política (Doc 4.4). */
function tasaComision(vendedor: Asentamiento, comprador: Asentamiento): number {
  if (vendedor.faccionId === comprador.faccionId) return COMISION.tasaMismaFaccion;
  return COMISION.tasaExterna * factorComisionExterna(vendedor);
}

/**
 * Clearing simplificado del mercado abierto (Doc 3.3): empareja órdenes de venta y compra compatibles del
 * mismo recurso entre CUALQUIER par de asentamientos. Simplificación de Fase 0: la transacción se liquida al
 * instante (sin caravana física) — a diferencia del trueque, el diseño no exige transporte para estas órdenes,
 * así que modelar rutas aquí solo duplicaría la mecánica de caravanas sin validar una regla nueva.
 * La comisión (Doc 3.5) la cobra el asentamiento vendedor.
 */
export function avanzarMercado(
  asentamientos: Asentamiento[],
  ordenes: OrdenMercado[]
): { asentamientos: Asentamiento[]; ordenes: OrdenMercado[]; eventos: string[] } {
  const eventos: string[] = [];
  // Copias de trabajo locales: se mutan libremente dentro de esta función, pero nunca los objetos del caller.
  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, { ...a }]));
  const ordenesTrabajo = ordenes.map((o) => (o.estado === 'activa' ? { ...o } : o));
  const recursosEnJuego = new Set(ordenesTrabajo.filter((o) => o.estado === 'activa').map((o) => o.recurso));

  for (const recurso of recursosEnJuego) {
    const ventas = ordenesTrabajo
      .filter((o) => o.estado === 'activa' && o.tipo === 'venta' && o.recurso === recurso)
      .sort((a, b) => a.precioUnitario - b.precioUnitario);
    const compras = ordenesTrabajo
      .filter((o) => o.estado === 'activa' && o.tipo === 'compra' && o.recurso === recurso)
      .sort((a, b) => b.precioUnitario - a.precioUnitario);

    for (const compra of compras) {
      for (const venta of ventas) {
        if (compra.cantidadCumplida >= compra.cantidad) break;
        if (venta.cantidadCumplida >= venta.cantidad) continue;
        if (compra.precioUnitario < venta.precioUnitario) continue; // comprador no paga lo que pide el vendedor
        if (compra.asentamientoId === venta.asentamientoId) continue;

        const vendedor = asentamientosPorId.get(venta.asentamientoId);
        const comprador = asentamientosPorId.get(compra.asentamientoId);
        if (!vendedor || !comprador) continue;

        const cantidadRestanteVenta = venta.cantidad - venta.cantidadCumplida;
        const cantidadRestanteCompra = compra.cantidad - compra.cantidadCumplida;
        const stockVendedor = cantidadDisponible(vendedor.almacen, recurso);
        const oroComprador = cantidadDisponible(comprador.almacen, 'oro');
        const precio = venta.precioUnitario; // se liquida al precio pedido por el vendedor
        const maxPorOro = precio > 0 ? oroComprador / precio : Number.POSITIVE_INFINITY;
        const cantidad = Math.min(cantidadRestanteVenta, cantidadRestanteCompra, stockVendedor, maxPorOro);
        if (cantidad <= 0) continue;

        const valor = cantidad * precio;
        // La comisión (Doc 3.5) no sale del bolsillo del comprador: es oro adicional que "enriquece al
        // asentamiento donde ocurre" la transacción, aquí el vendedor (más bajo si es la misma Facción).
        const comision = valor * tasaComision(vendedor, comprador);

        asentamientosPorId.set(vendedor.id, {
          ...vendedor,
          almacen: agregarRecurso(descontarRecursos(vendedor.almacen, { [recurso]: cantidad }), 'oro', valor + comision),
        });
        asentamientosPorId.set(comprador.id, {
          ...comprador,
          almacen: agregarRecurso(descontarRecursos(comprador.almacen, { oro: valor }), recurso, cantidad),
        });

        venta.cantidadCumplida += cantidad;
        compra.cantidadCumplida += cantidad;
        if (venta.cantidadCumplida >= venta.cantidad) venta.estado = 'cumplida';
        if (compra.cantidadCumplida >= compra.cantidad) compra.estado = 'cumplida';

        eventos.push(
          `Mercado: ${comprador.id} compra ${cantidad.toFixed(1)} ${recurso} a ${vendedor.id} por ${valor.toFixed(1)} oro (comisión ${comision.toFixed(1)}).`
        );
      }
    }
  }

  return {
    asentamientos: asentamientos.map((a) => asentamientosPorId.get(a.id)!),
    ordenes: ordenesTrabajo,
    eventos,
  };
}
