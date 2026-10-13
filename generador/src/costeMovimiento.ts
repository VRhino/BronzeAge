// Coste de movimiento por terreno (Fase 0.3): multiplicador continuo derivado de `evaluarTerreno`, no un
// lookup por celda — mismo criterio que el resto de `worldgen/` (ver `Fase_0_1_Definicion.md`: "coste de
// movimiento como función continua de la pendiente"). 1 = velocidad normal (llano/costa); más alto = más
// lento. `Mapa.costeEnPunto` (`world/mapa.ts`) es la fachada que el motor/pathfinding consultan de verdad —
// esta función vive en `worldgen/` porque solo depende de `CampoElevacion`, igual que `evaluarTerreno`.

import type { Point } from './tipos';
import { COSTE_MOVIMIENTO, ESCALA_TERRENO } from './config';
import { evaluarTerreno, gradienteElevacion } from './elevacion';
import type { CampoElevacion } from './types';

/**
 * ¿Se puede pisar este punto? (a petición del usuario, 2026-09-02: "el agua es un obstáculo").
 *
 * **Esto revierte una decisión explícita de Fase 0.3**, que decía: "Agua/cima no se prohíben duro (romperían
 * el pathfinding en cualquier mundo donde el camino más corto los roce) — se penalizan lo bastante fuerte
 * para que A* los evite salvo que no haya alternativa real". El coste de 15 hacía que el agua fuera cara pero
 * cruzable, y en la traza del Paso 4 se vio a un ejército arrastrándose sobre ella a 1/14 de su velocidad.
 * Ahora es infranqueable de verdad: nadie camina sobre el mar.
 *
 * La advertencia de entonces sigue siendo válida y hay que asumirla: **un destino puede quedar sin ruta**
 * (una isla, una península cortada). `calcularRuta` ya no cae a la línea recta cuando no encuentra camino —
 * devuelve `undefined`, y cada llamador decide qué significa eso (rechazar la movilización, no despachar la
 * caravana...). Cruzar el agua sería peor que no ir.
 *
 * Las cimas suaves pueden cruzarse; una pendiente que supera el máximo físico no.
 *
 * NO cubre los RÍOS, que en este motor no son terreno sino una entidad aparte (`RioZona`, `worldgen/rios.ts`)
 * que el coste de movimiento nunca ha mirado. Que un río corte el paso es parte del rediseño de rutas de
 * caravana (`Docs/Mecanicas a desarrollar.md` §3) y necesita vados o puentes para no fragmentar el mapa.
 */
export function esTransitable(campo: CampoElevacion, p: Point): boolean {
  if (evaluarTerreno(campo, p) === 'agua') return false;
  const { dx, dy } = gradienteElevacion(campo, p, ESCALA_TERRENO.pasoPendiente);
  return Math.hypot(dx, dy) * ESCALA_TERRENO.metrosVerticalesPorElevacion
    / ESCALA_TERRENO.metrosHorizontalesPorUnidad <= ESCALA_TERRENO.pendienteMaximaTransitable;
}

/**
 * Multiplicador de coste de movimiento en un punto (Fase 0.3): cuánto FRENA el terreno que sí se puede
 * pisar. Quién puede pisarlo lo decide `esTransitable` — el coste del agua se queda declarado por coherencia
 * del catálogo, pero ya no debería consultarse para un punto transitable.
 */
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
