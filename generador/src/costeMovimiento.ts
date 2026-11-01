// Transitabilidad y coste de movimiento: funciones continuas derivadas de `evaluarTerreno` y la pendiente.

import type { Point } from './tipos';
import { COSTE_MOVIMIENTO, ESCALA_TERRENO } from './config';
import { evaluarTerreno, gradienteElevacion, type CampoElevacion } from './elevacion';

/** ¿Se puede pisar este punto? El agua es infranqueable y una pendiente sobre el máximo físico tampoco se cruza.
 * No cubre los ríos: cortan el paso aparte, con vados (ver `vados.ts`). */
export function esTransitable(campo: CampoElevacion, p: Point, altura?: number): boolean {
  if (evaluarTerreno(campo, p, altura) === 'agua') return false;
  const { dx, dy } = gradienteElevacion(campo, p, ESCALA_TERRENO.pasoPendiente);
  return Math.hypot(dx, dy) * ESCALA_TERRENO.metrosVerticalesPorElevacion
    / ESCALA_TERRENO.metrosHorizontalesPorUnidad <= ESCALA_TERRENO.pendienteMaximaTransitable;
}

/** Multiplicador de coste de movimiento en un punto: cuánto frena el terreno que sí se puede pisar (quién puede pisarlo lo
 * decide `esTransitable`). Solo cuenta la pendiente (decisión del autor, 2026-10-31): la altura no frena.
 * `altura` opcional: la elevación ya calculada en `p`, para no recalcularla (mismo resultado). */
export function costeEnPunto(campo: CampoElevacion, p: Point, altura?: number): number {
  if (evaluarTerreno(campo, p, altura) === 'agua') return COSTE_MOVIMIENTO.agua;
  const { dx, dy } = gradienteElevacion(campo, p, ESCALA_TERRENO.pasoPendiente);
  const pendiente = Math.hypot(dx, dy) * ESCALA_TERRENO.metrosVerticalesPorElevacion
    / ESCALA_TERRENO.metrosHorizontalesPorUnidad;
  return 1 + Math.min(COSTE_MOVIMIENTO.topePendiente, COSTE_MOVIMIENTO.porPendiente * pendiente);
}
