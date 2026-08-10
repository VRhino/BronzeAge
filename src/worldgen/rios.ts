import type { Point, RioZona } from '../domain/types';
import { RIOS } from './config';
import { colocarConEspaciado, distancia, type Limites } from './colocacion';
import { evaluarTerreno, gradienteElevacion } from './elevacion';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';

/**
 * Ríos (Fase 0.1): polilíneas, no celdas — nacen en un punto de montaña y descienden por gradiente de
 * máxima pendiente del campo de elevación hasta agua/borde del mapa, o quedan atrapados en un mínimo local
 * (lago). El único consumo de RNG es la colocación de los nacimientos; el descenso es puramente determinista
 * sobre `elevacion` (ver `Fase_0_1_Definicion.md`: geometría vectorial, nunca rejilla).
 */
export function generarRios(rng: RandomFn, limites: Limites, elevacion: CampoElevacion): RioZona[] {
  // Además de exigir terreno de montaña, el candidato a nacimiento se descarta si su gradiente ya está por
  // debajo de `gradienteMinimo` — nacer exactamente en un extremo local (cima sin pendiente clara) producía
  // ríos de un solo punto, más frecuente cuanto más nacimientos se sortean (Fase 0.1, mapa 2000x2000).
  const nacimientos = colocarConEspaciado(
    rng,
    limites,
    RIOS.cantidad,
    RIOS.espacioMinimoEntreNacimientos,
    [],
    [],
    (p) => {
      if (evaluarTerreno(elevacion, p) !== 'montana') return false;
      const { dx, dy } = gradienteElevacion(elevacion, p, RIOS.pasoGradiente);
      return Math.hypot(dx, dy) >= RIOS.gradienteMinimo;
    }
  );

  return nacimientos.map((fuente, i) => {
    const puntos: Point[] = [fuente];
    let actual = fuente;
    let terminaEnLago = true;

    for (let paso = 0; paso < RIOS.pasosMax; paso++) {
      const { dx, dy } = gradienteElevacion(elevacion, actual, RIOS.pasoGradiente);
      const magnitud = Math.hypot(dx, dy);
      if (magnitud < RIOS.gradienteMinimo) {
        terminaEnLago = true;
        break;
      }

      const siguiente: Point = {
        x: actual.x - (dx / magnitud) * RIOS.pasoDescenso,
        y: actual.y - (dy / magnitud) * RIOS.pasoDescenso,
      };

      if (siguiente.x < 0 || siguiente.x > limites.ancho || siguiente.y < 0 || siguiente.y > limites.alto) {
        const clamped: Point = {
          x: Math.min(Math.max(siguiente.x, 0), limites.ancho),
          y: Math.min(Math.max(siguiente.y, 0), limites.alto),
        };
        puntos.push(clamped);
        terminaEnLago = false;
        break;
      }

      puntos.push(siguiente);
      actual = siguiente;

      if (evaluarTerreno(elevacion, actual) === 'agua') {
        terminaEnLago = false;
        break;
      }
    }

    return { id: `rio-${i}`, puntos, terminaEnLago };
  });
}

/** Distancia mínima de un punto a un segmento (proyección clampeada al segmento). */
function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largoCuadrado = dx * dx + dy * dy;
  if (largoCuadrado === 0) return distancia(p, a);
  const t = Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largoCuadrado));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
}

/**
 * Distancia mínima de un punto a cualquier segmento de cualquier río — proxy de humedad para `evaluarBioma`.
 * O(ríos × puntos): barato con `RIOS.cantidad` pequeño y caminatas acotadas por `RIOS.pasosMax`.
 */
export function distanciaARioMasCercano(rios: readonly RioZona[], p: Point): number {
  let minimo = Infinity;
  for (const rio of rios) {
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const d = distanciaASegmento(p, rio.puntos[i]!, rio.puntos[i + 1]!);
      if (d < minimo) minimo = d;
    }
  }
  return minimo;
}
