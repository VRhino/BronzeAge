// Política "Presión Fiscal" (Tesorero, bloque "economía del oro", Doc 4.4): sube la recaudación de oro por
// población a cambio de frenar el crecimiento de las 3 clases. Sin política activa, el comportamiento por
// defecto no cambia (invariante del Paso 1).
import { describe, expect, it } from 'vitest';
import type { PoliticaActiva } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import { IMPUESTOS, POLITICA_CATALOGO } from '../../constants';
import { crecerPoblacion, recaudacionOro } from '../population';
import { createRng } from '../../worldgen';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';

function asentamientoDeTest() {
  const mapa = crearMapaDeterminista(42);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);
  return asentamiento;
}

const presionFiscal: PoliticaActiva = {
  id: 'pol-test-0',
  politicaId: 'presion_fiscal',
  cargo: 'tesorero',
  activadaEn: instante(0),
  expiraEn: instante(9_999_999_999),
};

const def = POLITICA_CATALOGO.find((p) => p.id === 'presion_fiscal') as { factorRecaudacion: number; factorCrecimientoPoblacion: number };

describe('Presión Fiscal', () => {
  it('multiplica la recaudación de oro por `factorRecaudacion`', () => {
    const base = { ...asentamientoDeTest(), poblacion: { pesants: 200, artesanos: 20, nobleza: 0 } };
    const conPolitica = { ...base, politicasActivas: [presionFiscal] };
    const bruto = 200 * IMPUESTOS.tasaPesants + 20 * IMPUESTOS.tasaArtesanos;
    expect(recaudacionOro(base)).toBeCloseTo(bruto);
    expect(recaudacionOro(conPolitica)).toBeCloseTo(bruto * def.factorRecaudacion);
  });

  it('frena el crecimiento de población frente a no tenerla (acumulado sobre varios ticks)', () => {
    const base = asentamientoDeTest();
    const crecer20Ticks = (politicasActivas: PoliticaActiva[]) => {
      let a = { ...base, politicasActivas, poblacion: { pesants: 10, artesanos: 0, nobleza: 0 } };
      const rng = createRng(1);
      for (let i = 0; i < 20; i++) a = { ...a, poblacion: crecerPoblacion(a, rng).poblacion };
      return a.poblacion.pesants;
    };
    expect(crecer20Ticks([presionFiscal])).toBeLessThan(crecer20Ticks([]));
  });

  it('sin política fiscal activa, recaudación y crecimiento no cambian (factor 1)', () => {
    const a = { ...asentamientoDeTest(), poblacion: { pesants: 200, artesanos: 20, nobleza: 3 } };
    expect(recaudacionOro(a)).toBeCloseTo(
      200 * IMPUESTOS.tasaPesants + 20 * IMPUESTOS.tasaArtesanos + 3 * IMPUESTOS.tasaNobleza
    );
    expect(def.factorRecaudacion).toBeGreaterThan(1);
    expect(def.factorCrecimientoPoblacion).toBeLessThan(1);
  });
});
