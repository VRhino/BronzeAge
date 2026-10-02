// El historial de órdenes cerradas se acota por plaza (`MERCADO.historialPorPlaza`): sin tope, 219 362 órdenes (65 MB)
// a las 5 semanas de batch (`Consideraciones/Auditoria_Tick_Eventos.md`).
import { describe, expect, it } from 'vitest';
import type { OrdenMercado } from '../../domain/types';
import { MERCADO } from '../../constants';
import { instanteDeTest } from './fixtures';
import { anexarAlHistorialDeOrdenes } from '../market';

const orden = (id: string, asentamientoId: string): OrdenMercado => ({
  id,
  asentamientoId,
  tipo: 'venta',
  recurso: 'madera',
  cantidad: 10,
  precioUnitario: 1,
  estado: 'cumplida',
  creadaEn: instanteDeTest(0),
  expiraEn: instanteDeTest(10),
} as unknown as OrdenMercado);

describe('anexarAlHistorialDeOrdenes', () => {
  it('sin nada que añadir devuelve el mismo historial', () => {
    const h = [orden('a', 'p1')];
    expect(anexarAlHistorialDeOrdenes(h, [])).toBe(h);
    expect(anexarAlHistorialDeOrdenes(undefined, [])).toBeUndefined();
  });

  it('cada plaza recuerda solo sus últimas, y una plaza activa no borra las de otra', () => {
    const tope = MERCADO.historialPorPlaza;
    let h: OrdenMercado[] | undefined;
    for (let i = 0; i < tope + 50; i++) h = anexarAlHistorialDeOrdenes(h, [orden(`a${i}`, 'p1')]);
    h = anexarAlHistorialDeOrdenes(h, [orden('b0', 'p2')]);

    const deP1 = h!.filter((o) => o.asentamientoId === 'p1');
    expect(deP1).toHaveLength(tope);
    expect(deP1[0]!.id, 'se conservan las últimas, no las primeras').toBe('a50');
    expect(deP1.at(-1)!.id).toBe(`a${tope + 49}`);
    expect(h!.filter((o) => o.asentamientoId === 'p2').map((o) => o.id)).toEqual(['b0']);
  });
});
