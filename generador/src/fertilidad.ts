import type { LagoZona, Point, RioZona } from './tipos';
import { FERTILIDAD } from './config';
import { evaluarRuido, generarCampoRuido } from './ruido';
import type { RandomFn } from './rng';
import type { CampoFertilidad } from './types';
import { distanciaARioMasCercano } from './rios';

/**
 * Campo de fertilidad continuo (0-1): ruido fractal de gradiente (ver `ruido.ts`), con menos octavas y
 * formaciones más grandes que la elevación — la fertilidad son manchas amplias, no terreno accidentado.
 * Devuelve los PARÁMETROS del campo, no una función ya cerrada sobre ellos: así el campo es serializable,
 * comparable entre dos generaciones e inspeccionable desde un test. Para consultarlo, `evaluarFertilidad`.
 */
export function generarCampoFertilidad(rng: RandomFn): CampoFertilidad {
  return generarCampoRuido(rng, {
    octavas: FERTILIDAD.octavas,
    frecuenciaBase: FERTILIDAD.escala,
    lacunaridad: FERTILIDAD.lacunaridad,
    persistencia: FERTILIDAD.persistencia,
  });
}

/**
 * Fertilidad del suelo en un punto cualquiera, normalizada 0-1. Función PURA: mismo campo + mismo punto
 * dan siempre el mismo valor. Definida en todo el plano, también fuera de los límites del mapa — los
 * muestreos en anillo del motor (colocación de Granjas) pueden salirse del borde.
 */
export function evaluarFertilidad(campo: CampoFertilidad, p: Point): number {
  return evaluarRuido(campo, p);
}

/** La fertilidad utilizable incorpora humedad; los grandes valles fluviales son áridos lejos del cauce. */
export function evaluarFertilidadEfectiva(
  campo: CampoFertilidad, rios: readonly RioZona[], lagos: readonly LagoZona[], p: Point
): number {
  const troncal = rios.some((r) => r.fin === 'troncal');
  const base = evaluarFertilidad(campo, p);
  const distRio = distanciaARioMasCercano(rios, p);
  const distLago = lagos.reduce((min, lago) =>
    Math.min(min, Math.max(0, Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y) - lago.radio)), Infinity);
  const distAgua = Math.min(distRio, distLago);
  const humedad = Math.max(0, 1 - distAgua / 80);
  const valor = (troncal ? base * 0.72 - 0.06 : base) + humedad * (troncal ? 0.34 : 0.18);
  return Math.min(1, Math.max(0, valor));
}
