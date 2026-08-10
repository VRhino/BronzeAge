import type { Point } from '../domain/types';
import { FERTILIDAD } from './config';
import { evaluarRuido, generarCampoRuido } from './ruido';
import type { RandomFn } from './rng';
import type { CampoFertilidad } from './types';

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
