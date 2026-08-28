// Movimiento de caravanas sobre polilínea con coste de terreno (Fase 0.3, ver `engine/movimiento.ts`).

import { describe, expect, it } from 'vitest';
import type { Point } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { avanzarPosicionEnRuta, longitudPolilinea, puntoEnPolilinea } from '../movimiento';

/** `Mapa` mínimo con coste de terreno inventado — ver mismo patrón en `world/__tests__/rutas.test.ts`. */
function mapaSintetico(costeEnPunto: (p: Point) => number): Mapa {
  return { costeEnPunto } as unknown as Mapa;
}

describe('longitudPolilinea / puntoEnPolilinea', () => {
  const linea: Point[] = [
    { x: 0, y: 0 },
    { x: 100, y: 0 },
    { x: 100, y: 100 },
  ];

  it('longitudPolilinea suma cada tramo', () => {
    expect(longitudPolilinea(linea)).toBe(200);
  });

  it('puntoEnPolilinea a mitad de camino cae en el primer tramo si la mitad está ahí', () => {
    expect(puntoEnPolilinea(linea, 50)).toEqual({ x: 50, y: 0 });
  });

  it('puntoEnPolilinea avanza al segundo tramo pasado el largo del primero', () => {
    expect(puntoEnPolilinea(linea, 150)).toEqual({ x: 100, y: 50 });
  });

  it('puntoEnPolilinea clampea al último punto si se pide más distancia de la que hay', () => {
    expect(puntoEnPolilinea(linea, 9999)).toEqual({ x: 100, y: 100 });
  });

  it('puntoEnPolilinea clampea al primer punto con distancia negativa', () => {
    expect(puntoEnPolilinea(linea, -50)).toEqual({ x: 0, y: 0 });
  });
});

describe('avanzarPosicionEnRuta', () => {
  const ruta: Point[] = [
    { x: 0, y: 0 },
    { x: 1000, y: 0 },
  ];

  it('a coste 1 en todo el trayecto, avanza exactamente velocidadBase de distancia en un tick', () => {
    const mapa = mapaSintetico(() => 1);
    const avance = avanzarPosicionEnRuta(mapa, ruta, 0, 20);
    expect(avance.posicion).toEqual({ x: 20, y: 0 });
    expect(avance.progreso).toBeCloseTo(20 / 1000, 10);
  });

  it('terreno costoso avanza menos por tick que terreno barato, a igual velocidadBase', () => {
    const mapaBarato = mapaSintetico(() => 1);
    const mapaCaro = mapaSintetico(() => 4);
    const enBarato = avanzarPosicionEnRuta(mapaBarato, ruta, 0, 20);
    const enCaro = avanzarPosicionEnRuta(mapaCaro, ruta, 0, 20);
    expect(enCaro.posicion.x).toBeLessThan(enBarato.posicion.x);
    expect(enCaro.posicion.x).toBeCloseTo(5, 10); // 20 / 4
  });

  it('factorCosteExtra < 1 (bonus de Camino Comercial) hace avanzar más rápido', () => {
    const mapa = mapaSintetico(() => 2);
    const sinBonus = avanzarPosicionEnRuta(mapa, ruta, 0, 20);
    const conBonus = avanzarPosicionEnRuta(mapa, ruta, 0, 20, 0.5);
    expect(conBonus.posicion.x).toBeGreaterThan(sinBonus.posicion.x);
  });

  it('el progreso nunca pasa de 1 ni la posición del final de la ruta', () => {
    const mapa = mapaSintetico(() => 1);
    const avance = avanzarPosicionEnRuta(mapa, ruta, 0.999, 500);
    expect(avance.progreso).toBe(1);
    expect(avance.posicion).toEqual({ x: 1000, y: 0 });
  });
});
