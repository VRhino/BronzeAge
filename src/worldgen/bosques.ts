import type { Point, ZonaBosque } from '../domain/types';
import { BOSQUE } from './config';
import type { Limites } from './colocacion';
import { randRange, type RandomFn } from './rng';

/**
 * Bosques: zonas circulares con densidad variable (Doc 1.4) — la fuente de madera del mapa, y lo primero
 * que se genera, porque el resto de capas los usa como zona de exclusión (ver `colocarConEspaciado`).
 * Pueden solaparse entre sí y salirse del borde del mapa: solo su centro está garantizado dentro.
 */
export function generarBosques(rng: RandomFn, limites: Limites): ZonaBosque[] {
  const bosques: ZonaBosque[] = [];
  for (let i = 0; i < BOSQUE.cantidad; i++) {
    const centro: Point = { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
    bosques.push({
      id: `bosque-${i}`,
      centro,
      radio: randRange(rng, BOSQUE.radioMin, BOSQUE.radioMax),
      densidad: randRange(rng, BOSQUE.densidadMin, BOSQUE.densidadMax),
    });
  }
  return bosques;
}
