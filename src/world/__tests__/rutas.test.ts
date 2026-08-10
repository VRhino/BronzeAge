// Pathfinding (Fase 0.3, ver `world/rutas.ts`). Dos tipos de caso, a propósito:
//  1. Mapa SINTÉTICO (coste inventado, sin generación real): permite afirmaciones cuantitativas exactas
//     ("la ruta rodea el obstáculo") sin depender de dónde caiga el relieve real de una seed.
//  2. Mapa REAL (`generarMapa`): contrato básico (empieza en origen, termina en destino, dentro del mapa)
//     contra el generador de verdad, mismo criterio que `world/__tests__/mapa.test.ts`.

import { describe, expect, it } from 'vitest';
import type { Point } from '../../domain/types';
import { generarMapa, MAPA_DEFAULT } from '../../worldgen';
import { crearMapa } from '../mapa';
import { calcularRuta } from '../rutas';
import { distancia } from '../geometria';
import type { Mapa } from '../mapa';

/** `Mapa` mínimo con coste de terreno inventado — evita depender de la forma real del relieve para poder
 * afirmar "la ruta rodea este obstáculo concreto" con certeza. Cast necesario: `Mapa` es una clase con
 * campos privados, no una interfaz estructural (ver `world/mapa.ts`). */
function mapaSintetico(ancho: number, alto: number, costeEnPunto: (p: Point) => number): Mapa {
  return { limites: { ancho, alto }, costeEnPunto } as unknown as Mapa;
}

function longitudRuta(puntos: readonly Point[]): number {
  let total = 0;
  for (let i = 0; i < puntos.length - 1; i++) total += distancia(puntos[i]!, puntos[i + 1]!);
  return total;
}

/** Coste total acumulado de recorrer `puntos` (suma de longitud×coste medio de cada tramo) — mismo cálculo
 * que hace `world/rutas.ts` internamente para las aristas de la malla, aplicado aquí a un camino ya hecho.
 * Los tramos de una `ruta` de `calcularRuta` ya son cortos (separados ~`ESPACIADO_MALLA`), así que el
 * promedio de extremos por tramo es una buena aproximación — para una línea recta LARGA de solo 2 puntos
 * (`costeLineaRecta`, abajo) hace falta subdividir primero, o el promedio de extremos ignora el obstáculo
 * que haya en medio. */
function costeTotal(puntos: readonly Point[], costeEnPunto: (p: Point) => number): number {
  let total = 0;
  for (let i = 0; i < puntos.length - 1; i++) {
    const a = puntos[i]!;
    const b = puntos[i + 1]!;
    total += distancia(a, b) * ((costeEnPunto(a) + costeEnPunto(b)) / 2);
  }
  return total;
}

/** Coste de ir en línea recta de `a` a `b`, subdividida en `muestras` tramos — a diferencia de `costeTotal`
 * sobre 2 puntos sueltos, esto sí integra el coste real de lo que haya EN MEDIO del segmento. */
function costeLineaRecta(a: Point, b: Point, costeEnPunto: (p: Point) => number, muestras = 400): number {
  const puntos: Point[] = Array.from({ length: muestras + 1 }, (_, i) => {
    const t = i / muestras;
    return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
  });
  return costeTotal(puntos, costeEnPunto);
}

describe('calcularRuta — mapa sintético', () => {
  const origen: Point = { x: 100, y: 1000 };
  const destino: Point = { x: 1900, y: 1000 };
  // Disco costoso centrado en el punto medio de la línea recta — cualquier camino que pase por ahí es caro.
  const centroObstaculo: Point = { x: 1000, y: 1000 };
  const radioObstaculo = 120;

  function costeConObstaculo(p: Point): number {
    return distancia(p, centroObstaculo) < radioObstaculo ? 30 : 1;
  }

  it('rodea un obstáculo costoso en vez de cruzarlo, cuando hay margen para hacerlo', () => {
    const mapa = mapaSintetico(2000, 2000, costeConObstaculo);
    const ruta = calcularRuta(mapa, origen, destino);

    // Ningún punto de la ruta entra en el disco costoso.
    for (const p of ruta) {
      expect(distancia(p, centroObstaculo)).toBeGreaterThanOrEqual(radioObstaculo);
    }

    // Y el coste real de esa ruta es mejor que el de ir recto atravesando el obstáculo.
    expect(costeTotal(ruta, costeConObstaculo)).toBeLessThan(costeLineaRecta(origen, destino, costeConObstaculo));
  });

  it('en un mapa sin coste variable (todo 1), la ruta es esencialmente la línea recta', () => {
    const mapa = mapaSintetico(2000, 2000, () => 1);
    const ruta = calcularRuta(mapa, origen, destino);
    // Sin ningún incentivo para desviarse, la longitud de la polilínea no debe alejarse mucho de la
    // distancia en línea recta (la malla 8-conexa no es perfectamente diagonal, así que se permite un
    // margen pequeño en vez de exigir igualdad exacta).
    const directa = distancia(origen, destino);
    expect(longitudRuta(ruta)).toBeLessThan(directa * 1.05);
  });

  it('devuelve [origen, destino] cuando ya están casi pegados (por debajo del espaciado de malla)', () => {
    const mapa = mapaSintetico(2000, 2000, costeConObstaculo);
    const cerca: Point = { x: origen.x + 5, y: origen.y + 5 };
    expect(calcularRuta(mapa, origen, cerca)).toEqual([origen, cerca]);
  });

  it('siempre empieza en origen y termina en destino exactos, no en el punto de malla más cercano', () => {
    const mapa = mapaSintetico(2000, 2000, costeConObstaculo);
    const ruta = calcularRuta(mapa, origen, destino);
    expect(ruta[0]).toEqual(origen);
    expect(ruta[ruta.length - 1]).toEqual(destino);
  });
});

describe('calcularRuta — mapa real (generarMapa)', () => {
  const SEEDS = [1, 42, 7];

  it('siempre devuelve un camino que empieza en origen, termina en destino y cae dentro del mapa', () => {
    for (const seed of SEEDS) {
      const generado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed });
      const mapa = crearMapa(generado);
      const origen: Point = { x: 200, y: 200 };
      const destino: Point = { x: 1800, y: 1800 };
      const ruta = calcularRuta(mapa, origen, destino);

      expect(ruta[0]).toEqual(origen);
      expect(ruta[ruta.length - 1]).toEqual(destino);
      for (const p of ruta) {
        expect(p.x).toBeGreaterThanOrEqual(0);
        expect(p.x).toBeLessThanOrEqual(generado.config.ancho);
        expect(p.y).toBeGreaterThanOrEqual(0);
        expect(p.y).toBeLessThanOrEqual(generado.config.alto);
      }
    }
  });

  it('es determinista: mismo mundo, mismo origen/destino -> misma ruta', () => {
    const generado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: 1 });
    const mapa = crearMapa(generado);
    const origen: Point = { x: 300, y: 1600 };
    const destino: Point = { x: 1700, y: 300 };
    expect(calcularRuta(mapa, origen, destino)).toEqual(calcularRuta(mapa, origen, destino));
  });
});
