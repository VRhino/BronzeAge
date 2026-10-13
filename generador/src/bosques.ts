import type { LagoZona, Point, RioZona, ZonaBosque } from './tipos';
import { BOSQUE, BOSQUE_TERRENO_PERMITIDO, COLOCACION } from './config';
import type { Limites } from './colocacion';
import { evaluarTerreno } from './elevacion';
import { evaluarFertilidadEfectiva } from './fertilidad';
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
export function generarBosques(
  rng: RandomFn, limites: Limites, elevacion: CampoElevacion, fertilidad: CampoFertilidad,
  rios: readonly RioZona[], lagos: readonly LagoZona[]
): ZonaBosque[] {
  const bosques: ZonaBosque[] = [];
  let sueloApto = 0;
  for (let y = 0; y < 40; y++) for (let x = 0; x < 40; x++) {
    const p = { x: ((x + 0.5) * limites.ancho) / 40, y: ((y + 0.5) * limites.alto) / 40 };
    if (BOSQUE_TERRENO_PERMITIDO.includes(evaluarTerreno(elevacion, p))
      && evaluarFertilidadEfectiva(fertilidad, rios, lagos, p) >= 0.38) sueloApto++;
  }
  const objetivo = Math.min(BOSQUE.cantidad, Math.ceil(BOSQUE.cantidad * sueloApto / (40 * 40 * 0.65)));
  const archipielago = elevacion.estructura?.perfil === 'archipielago';
  for (let i = 0; i < objetivo; i++) {
    let centro: Point | null = null;
    const radio = randRange(rng, archipielago ? 20 : BOSQUE.radioMin, archipielago ? 70 : BOSQUE.radioMax);
    for (let intento = 0; intento < COLOCACION.intentosPorPunto * 4; intento++) {
      const candidato: Point = { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
      if (candidato.x < radio || candidato.y < radio || candidato.x > limites.ancho - radio || candidato.y > limites.alto - radio) continue;
      if (BOSQUE_TERRENO_PERMITIDO.includes(evaluarTerreno(elevacion, candidato))
        && evaluarFertilidadEfectiva(fertilidad, rios, lagos, candidato) >= 0.38
        && Array.from({ length: 8 }, (_, k) => {
          const angulo = (k * Math.PI) / 4;
          return { x: candidato.x + Math.cos(angulo) * radio, y: candidato.y + Math.sin(angulo) * radio };
        }).every((p) => BOSQUE_TERRENO_PERMITIDO.includes(evaluarTerreno(elevacion, p)))) {
        centro = candidato;
        break;
      }
    }
    if (!centro) continue;

    // Densidad (v6): mezcla de fertilidad del suelo en el centro (suelo mejor → bosque más frondoso, señal
    // de diseño) con un sorteo uniforme (variación natural — dos bosques en suelo igual de fértil no deben
    // salir clónicos). `PESO_FERTILIDAD_DENSIDAD` fija cuánto pesa cada uno; el resultado se remapea al
    // rango [densidadMin, densidadMax] de siempre, así que el invariante de rango no cambia.
    const senalFertilidad = evaluarFertilidadEfectiva(fertilidad, rios, lagos, centro);
    const senalAzar = randRange(rng, 0, 1);
    const mezcla = senalFertilidad * PESO_FERTILIDAD_DENSIDAD + senalAzar * (1 - PESO_FERTILIDAD_DENSIDAD);
    const densidad = BOSQUE.densidadMin + mezcla * (BOSQUE.densidadMax - BOSQUE.densidadMin);

    bosques.push({
      id: `bosque-${bosques.length}`,
      centro,
      radio,
      densidad,
    });
  }
  return bosques;
}
