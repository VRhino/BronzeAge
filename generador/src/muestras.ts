import type { Point } from './tipos';
import { evaluarElevacion, gradienteElevacion, type CampoElevacion } from './elevacion';
import { humedadRegionalEn } from './regional';

/** Lo que el relieve da en un punto y que varias funciones piden a la vez; calcularlo una vez evita repetir el gradiente. */
export interface MuestraRelieve { altura: number; pendiente: number; humedad: number }

/** Altura, pendiente (×1000, gradiente con paso 2) y humedad regional en `p`: las mismas expresiones que usan sus consumidores. */
export function muestrearRelieve(e: CampoElevacion, p: Point): MuestraRelieve {
  const g = gradienteElevacion(e, p, 2);
  return { altura: evaluarElevacion(e, p), pendiente: Math.hypot(g.dx, g.dy) * 1000, humedad: humedadRegionalEn(e.regional, p) };
}
