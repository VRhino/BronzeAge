// Hambruna (Doc 4.1, a petición del usuario): efecto negativo de no poder mantener a la población con trigo.
// Espejo deliberado de la moral de tropas por ración (`avanzarMantenimientoTropas`, engine/tropas.ts) — estos
// tests verifican el mismo contrato: sin trigo la nutrición colapsa en un número de ticks fijo (100/20 = 5,
// igual que la moral militar) y, sostenida en 0, empieza a costar población real (nobleza protegida).
import { describe, expect, it } from 'vitest';
import { OCUPACION, POBLACION } from '../../constants';
import { crecerPoblacion, avanzarNutricionPoblacion } from '../population';
import { createRng } from '../../worldgen';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest, posicionRecomendable } from './fixtures';

function asentamientoDeTest() {
  const mapa = crearMapaDeterminista(42);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);
  return asentamiento;
}

describe('Hambruna: nutrición de la población', () => {
  it('con trigo de sobra la nutrición se queda en 100 y no hay bajas', () => {
    let asentamiento = asentamientoDeTest();
    asentamiento = { ...asentamiento, almacen: { ...asentamiento.almacen, trigo: { cantidad: 1000, capacidad: 2000 } } };
    const poblacionInicial = asentamiento.poblacion;

    for (let i = 0; i < 10; i++) {
      const { asentamiento: siguiente, eventos } = avanzarNutricionPoblacion(asentamiento);
      asentamiento = siguiente;
      expect(eventos).toEqual([]);
    }

    expect(asentamiento.nutricionPoblacion).toBe(100);
    expect(asentamiento.poblacion).toEqual(poblacionInicial);
  });

  it('sin trigo la nutrición colapsa a 0 en 100/degradacionSinComida ticks (mismo ritmo que la moral de tropas)', () => {
    let asentamiento = asentamientoDeTest();
    asentamiento = { ...asentamiento, almacen: { ...asentamiento.almacen, trigo: { cantidad: 0, capacidad: 2000 } } };

    const ticksEsperados = POBLACION.hambre.nutricionInicial / POBLACION.hambre.degradacionSinComida;
    for (let i = 0; i < ticksEsperados - 1; i++) {
      ({ asentamiento } = avanzarNutricionPoblacion(asentamiento));
      expect(asentamiento.nutricionPoblacion).toBeGreaterThan(0);
    }
    ({ asentamiento } = avanzarNutricionPoblacion(asentamiento));
    expect(asentamiento.nutricionPoblacion).toBe(0);
  });

  it('hambre sostenida (nutrición en 0) cuesta pesants/artesanos por tick, nunca nobleza', () => {
    let asentamiento = asentamientoDeTest();
    asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, trigo: { cantidad: 0, capacidad: 2000 } },
      poblacion: { pesants: 100, artesanos: 20, nobleza: 5 },
      nutricionPoblacion: 0,
    };

    const { asentamiento: siguiente, eventos } = avanzarNutricionPoblacion(asentamiento);

    expect(siguiente.poblacion.nobleza).toBe(5);
    const perdidos = asentamiento.poblacion.pesants + asentamiento.poblacion.artesanos - (siguiente.poblacion.pesants + siguiente.poblacion.artesanos);
    expect(perdidos).toBeGreaterThan(0);
    expect(eventos.some((e) => typeof e !== 'string' && e.codigo === 'poblacion.hambruna_muerte')).toBe(true);
  });

  it('la nutrición baja frena el crecimiento de población (factorCrecimientoMinimo en vez de un booleano trigo>0?1:0.2)', () => {
    let base = asentamientoDeTest();
    // Vivienda extra (clonada de la real, ya que fundar da 3) para que quede cupo de sobra y el techo de
    // vivienda no tape la diferencia que se quiere medir (solo la nutrición debe variar entre ambos casos).
    const viviendaBase = base.edificios.find((e) => e.tipo === 'vivienda')!;
    const viviendasExtra = Array.from({ length: 10 }, (_, i) => ({ ...viviendaBase, id: `vivienda-extra-${i}` }));
    base = { ...base, edificios: [...base.edificios, ...viviendasExtra], poblacion: { pesants: 100, artesanos: 0, nobleza: 0 } };

    const bienAlimentado = { ...base, nutricionPoblacion: 100 };
    const hambriento = { ...base, nutricionPoblacion: 0 };

    // Cota determinista con RNG con seed fija: con este cupo, el esperado de crecimiento bien alimentado
    // (comidaFactor=1) es >=2 de sobra por encima del hambriento (comidaFactor=factorCrecimientoMinimo=0.2),
    // así que la diferencia se sostiene sin importar el redondeo estocástico de `crecimientoEstocastico`.
    const { poblacion: crecidoBien } = crecerPoblacion(bienAlimentado, createRng(1));
    const { poblacion: crecidoHambriento } = crecerPoblacion(hambriento, createRng(1));

    expect(crecidoHambriento.pesants - hambriento.poblacion.pesants).toBeLessThan(crecidoBien.pesants - bienAlimentado.poblacion.pesants);
  });

  it('bajo ocupación reciente el crecimiento se frena × OCUPACION.factorCrecimiento (Ocupacion §2.4)', () => {
    // Mismo patrón que el test de "Presión Fiscal frena el crecimiento": acumulado sobre muchos ticks con
    // semilla fija, para que el redondeo estocástico no tape la diferencia del factor.
    const base = { ...asentamientoDeTest(), nutricionPoblacion: 100 };
    const crecer40Ticks = (ocupado: boolean) => {
      let a = { ...base, poblacion: { pesants: 10, artesanos: 0, nobleza: 0 } };
      const rng = createRng(1);
      for (let i = 0; i < 40; i++) {
        a = { ...a, poblacion: crecerPoblacion(a, rng, ocupado ? instanteDeTest(i + 1) : undefined).poblacion };
        if (ocupado) a = { ...a, ocupacionHasta: instanteDeTest(1000) };
      }
      return a.poblacion.pesants;
    };
    expect(OCUPACION.factorCrecimiento).toBeLessThan(1);
    expect(crecer40Ticks(true)).toBeLessThan(crecer40Ticks(false));
  });
});
