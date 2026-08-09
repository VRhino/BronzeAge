import type { Point } from '../domain/types';
import { FERTILIDAD } from './config';
import { randRange, type RandomFn } from './rng';
import type { CampoFertilidad } from './types';

/**
 * Campo de fertilidad continuo (0-1) por suma de senos con fase aleatoria — sin dependencias externas.
 * Devuelve los PARÁMETROS del campo, no una función ya cerrada sobre ellos: así el campo es serializable,
 * comparable entre dos generaciones e inspeccionable desde un test. Para consultarlo, `evaluarFertilidad`.
 */
export function generarCampoFertilidad(rng: RandomFn): CampoFertilidad {
  const octavas = Array.from({ length: FERTILIDAD.octavas }, (_, i) => ({
    freq: FERTILIDAD.escala * (i + 1),
    faseX: randRange(rng, 0, Math.PI * 2),
    faseY: randRange(rng, 0, Math.PI * 2),
    peso: 1 / (i + 1),
  }));
  const pesoTotal = octavas.reduce((acc, o) => acc + o.peso, 0);
  return { octavas, pesoTotal };
}

/**
 * Fertilidad del suelo en un punto cualquiera, normalizada 0-1. Función PURA: mismo campo + mismo punto
 * dan siempre el mismo valor. Definida en todo el plano, también fuera de los límites del mapa — los
 * muestreos en anillo del motor (colocación de Granjas) pueden salirse del borde.
 */
export function evaluarFertilidad(campo: CampoFertilidad, p: Point): number {
  let valor = 0;
  for (const o of campo.octavas) {
    const s = Math.sin(p.x * o.freq + o.faseX) * Math.cos(p.y * o.freq + o.faseY);
    valor += ((s + 1) / 2) * o.peso;
  }
  return valor / campo.pesoTotal;
}
