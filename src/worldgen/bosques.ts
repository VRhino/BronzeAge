import type { Point, ZonaBosque } from '../domain/types';
import { BOSQUE, BOSQUE_TERRENO_PERMITIDO, COLOCACION } from './config';
import type { Limites } from './colocacion';
import { evaluarTerreno } from './elevacion';
import { evaluarFertilidad } from './fertilidad';
import { randRange, type RandomFn } from './rng';
import type { CampoElevacion, CampoFertilidad } from './types';

/** Peso de la fertilidad del suelo frente al azar puro al decidir cuán denso sale un bosque (0-1). Alto a
 * propósito: es la palanca de "realismo" que pidió el diseño (suelo mejor → bosque más frondoso), pero no
 * 1.0 — un bosque sigue necesitando variación natural aunque nazca en el mismo tipo de suelo que otro. */
const PESO_FERTILIDAD_DENSIDAD = 0.7;

/**
 * Bosques: zonas circulares con densidad variable (Doc 1.4) — la fuente de madera del mapa, y lo primero
 * que se genera, porque el resto de capas los usa como zona de exclusión (ver `colocarConEspaciado`).
 * Pueden solaparse entre sí y salirse del borde del mapa: solo su centro está garantizado dentro.
 *
 * El centro se condiciona al terreno (Fase 0.1, `BOSQUE_TERRENO_PERMITIDO`) por rejection-sampling, igual
 * que `colocarConEspaciado`: hasta `COLOCACION.intentosPorPunto` intentos, y si no encuentra hueco se
 * coloca igual (mapa saturado) en vez de bloquear la generación. Radio no cambia; densidad sí (ver abajo).
 */
export function generarBosques(rng: RandomFn, limites: Limites, elevacion: CampoElevacion, fertilidad: CampoFertilidad): ZonaBosque[] {
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

    // Densidad (v6): mezcla de fertilidad del suelo en el centro (suelo mejor → bosque más frondoso, señal
    // de diseño) con un sorteo uniforme (variación natural — dos bosques en suelo igual de fértil no deben
    // salir clónicos). `PESO_FERTILIDAD_DENSIDAD` fija cuánto pesa cada uno; el resultado se remapea al
    // rango [densidadMin, densidadMax] de siempre, así que el invariante de rango no cambia.
    const senalFertilidad = evaluarFertilidad(fertilidad, centroFinal);
    const senalAzar = randRange(rng, 0, 1);
    const mezcla = senalFertilidad * PESO_FERTILIDAD_DENSIDAD + senalAzar * (1 - PESO_FERTILIDAD_DENSIDAD);
    const densidad = BOSQUE.densidadMin + mezcla * (BOSQUE.densidadMax - BOSQUE.densidadMin);

    bosques.push({
      id: `bosque-${i}`,
      centro: centroFinal,
      radio: randRange(rng, BOSQUE.radioMin, BOSQUE.radioMax),
      densidad,
    });
  }
  return bosques;
}
