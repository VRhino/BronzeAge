import type { Point, ZonaBosque } from '../domain/types';
import { BOSQUE, BOSQUE_TERRENO_PERMITIDO, COLOCACION } from './config';
import type { Limites } from './colocacion';
import { evaluarTerreno } from './elevacion';
import { randRange, type RandomFn } from './rng';
import type { CampoElevacion } from './types';

/**
 * Bosques: zonas circulares con densidad variable (Doc 1.4) — la fuente de madera del mapa, y lo primero
 * que se genera, porque el resto de capas los usa como zona de exclusión (ver `colocarConEspaciado`).
 * Pueden solaparse entre sí y salirse del borde del mapa: solo su centro está garantizado dentro.
 *
 * El centro se condiciona al terreno (Fase 0.1, `BOSQUE_TERRENO_PERMITIDO`) por rejection-sampling, igual
 * que `colocarConEspaciado`: hasta `COLOCACION.intentosPorPunto` intentos, y si no encuentra hueco se
 * coloca igual (mapa saturado) en vez de bloquear la generación. Radio y densidad no cambian.
 */
export function generarBosques(rng: RandomFn, limites: Limites, elevacion: CampoElevacion): ZonaBosque[] {
  const bosques: ZonaBosque[] = [];
  for (let i = 0; i < BOSQUE.cantidad; i++) {
    let centro: Point | null = null;
    for (let intento = 0; intento < COLOCACION.intentosPorPunto; intento++) {
      const candidato: Point = { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
      if (BOSQUE_TERRENO_PERMITIDO.includes(evaluarTerreno(elevacion, candidato))) {
        centro = candidato;
        break;
      }
    }
    const centroFinal = centro ?? { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
    bosques.push({
      id: `bosque-${i}`,
      centro: centroFinal,
      radio: randRange(rng, BOSQUE.radioMin, BOSQUE.radioMax),
      densidad: randRange(rng, BOSQUE.densidadMin, BOSQUE.densidadMax),
    });
  }
  return bosques;
}
