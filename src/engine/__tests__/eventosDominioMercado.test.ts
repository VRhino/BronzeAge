// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `market.ts` migrado.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, OrdenMercado } from '../../domain/types';
import { avanzarMercado } from '../market';
import type { PayloadMercadoCompra } from '../market';
import { almacenSintetico } from './tradeFixtures';

function asentamiento(id: string, faccionId: string, recursos: Record<string, number>): Asentamiento {
  return { id, faccionId, almacen: almacenSintetico(recursos), politicasActivas: [] } as unknown as Asentamiento;
}

describe('eventos de dominio — market.ts', () => {
  it('una venta y una compra compatibles del mismo recurso producen mercado.compra', () => {
    const vendedor = asentamiento('vendedor', 'f-vendedor', { madera: 100, oro: 0 });
    const comprador = asentamiento('comprador', 'f-comprador', { madera: 0, oro: 1000 });
    const venta: OrdenMercado = {
      id: 'orden-venta',
      asentamientoId: 'vendedor',
      tipo: 'venta',
      recurso: 'madera',
      cantidad: 50,
      cantidadCumplida: 0,
      precioUnitario: 2,
      creadoEnTick: 0,
      estado: 'activa',
    };
    const compra: OrdenMercado = {
      id: 'orden-compra',
      asentamientoId: 'comprador',
      tipo: 'compra',
      recurso: 'madera',
      cantidad: 50,
      cantidadCumplida: 0,
      precioUnitario: 2,
      creadoEnTick: 0,
      estado: 'activa',
    };

    const resultado = avanzarMercado([vendedor, comprador], [venta, compra]);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('mercado.compra');
    const p = evento.payload as PayloadMercadoCompra;
    expect(p.compradorId).toBe('comprador');
    expect(p.vendedorId).toBe('vendedor');
    expect(p.recurso).toBe('madera');
    expect(p.cantidad).toBe(50);
  });
});
