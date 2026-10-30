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

/** Multiplicador de coste de movimiento en un punto: cuánto frena el terreno que sí se puede pisar
 * (quién puede pisarlo lo decide `esTransitable`). */
export function costeEnPunto(campo: CampoElevacion, p: Point): number {
  const { dx, dy } = gradienteElevacion(campo, p, ESCALA_TERRENO.pasoPendiente);
  const pendiente = Math.hypot(dx, dy) * ESCALA_TERRENO.metrosVerticalesPorElevacion
    / ESCALA_TERRENO.metrosHorizontalesPorUnidad;
  const factorPendiente = 1 + Math.min(2, pendiente * 0.45);
  switch (evaluarTerreno(campo, p)) {
    case 'llano':
      return COSTE_MOVIMIENTO.llano * factorPendiente;
    case 'costa':
      return COSTE_MOVIMIENTO.costa * factorPendiente;
    case 'colina':
      return COSTE_MOVIMIENTO.colina * factorPendiente;
    case 'montana':
      return COSTE_MOVIMIENTO.montana * factorPendiente;
    case 'cima':
      return COSTE_MOVIMIENTO.cima * factorPendiente;
    case 'agua':
      return COSTE_MOVIMIENTO.agua;
  }
}
