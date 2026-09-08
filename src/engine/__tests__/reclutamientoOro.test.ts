// Coste de oro por reclutar (Doc 5.8, bloque "economía del oro", Paso 4): además del equipo, reclutar cuesta
// oro por soldado según el escalón de la tropa. Única excepción: la milicia del Centro Urbano.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { RECLUTAMIENTO_ORO_POR_ESCALON, TROPAS_RECLUTABLES } from '../../constants';
import { reclutarTropa, ReclutamientoInvalidoError } from '../tropas';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

function base(oro: number): Asentamiento {
  const mapa = crearMapaDeterminista(42);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  return {
    ...asentamiento,
    edificios: [
      ...asentamiento.edificios,
      { id: 'b1', tipo: 'barracon', estado: 'activo', nivelInterno: 1, posicion: { x: 0, y: 0 }, ambito: 'interno' } as never,
    ],
    poblacion: { pesants: 300, artesanos: 0, nobleza: 0 },
    almacen: {
      ...asentamiento.almacen,
      oro: { ...asentamiento.almacen['oro']!, cantidad: oro },
      armaMadera: { ...asentamiento.almacen['armaMadera']!, cantidad: 100 },
      trigo: { ...asentamiento.almacen['trigo']!, cantidad: 5000 },
    },
  };
}

const lancerosMimbre = TROPAS_RECLUTABLES.find((t) => t.id === 'lanceros_mimbre')!; // Barracón, escalón 1

describe('reclutamiento en oro', () => {
  it('la milicia del Centro Urbano NO cuesta oro', () => {
    const tras = reclutarTropa(base(0), 'jugador-faccion-1-1', 'milicia_lanceros', 'pesants', 0);
    expect(tras.almacen['oro']!.cantidad).toBe(0);
    expect(tras.escuadrones).toHaveLength(1);
  });

  it('una tropa de Barracón descuenta oro = escalón × nº de soldados', () => {
    const a = base(500);
    const tras = reclutarTropa(a, 'jugador-faccion-1-1', 'lanceros_mimbre', 'pesants', 0);
    const esperado = RECLUTAMIENTO_ORO_POR_ESCALON[lancerosMimbre.escalon]! * lancerosMimbre.unidadesPorDefecto;
    expect(a.almacen['oro']!.cantidad - tras.almacen['oro']!.cantidad).toBeCloseTo(esperado);
  });

  it('sin oro suficiente, reclutar una tropa de Barracón lanza', () => {
    expect(() => reclutarTropa(base(1), 'jugador-faccion-1-1', 'lanceros_mimbre', 'pesants', 0)).toThrow(ReclutamientoInvalidoError);
  });

  it('el coste de oro por escalón es creciente y positivo desde el escalón 1', () => {
    const c = RECLUTAMIENTO_ORO_POR_ESCALON;
    const serie = [c[1]!, c[2]!, c[3]!, c[4]!, c[5]!];
    expect(serie).toEqual([...serie].sort((x, y) => x - y));
    expect(c[1]).toBeGreaterThan(0);
  });
});
