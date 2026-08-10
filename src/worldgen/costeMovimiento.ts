// Coste de movimiento por terreno (Fase 0.3): multiplicador continuo derivado de `evaluarTerreno`, no un
// lookup por celda — mismo criterio que el resto de `worldgen/` (ver `Fase_0_1_Definicion.md`: "coste de
// movimiento como función continua de la pendiente"). 1 = velocidad normal (llano/costa); más alto = más
// lento. `Mapa.costeEnPunto` (`world/mapa.ts`) es la fachada que el motor/pathfinding consultan de verdad —
// esta función vive en `worldgen/` porque solo depende de `CampoElevacion`, igual que `evaluarTerreno`.

import type { Point } from '../domain/types';
import { COSTE_MOVIMIENTO } from './config';
import { evaluarTerreno } from './elevacion';
import type { CampoElevacion } from './types';

/**
 * Multiplicador de coste de movimiento en un punto (Fase 0.3). Agua/cima no se prohíben duro (romperían el
 * pathfinding en cualquier mundo donde el camino más corto los roce) — se penalizan lo bastante fuerte para
 * que A* los evite salvo que no haya alternativa real.
 */
export function costeEnPunto(campo: CampoElevacion, p: Point): number {
  switch (evaluarTerreno(campo, p)) {
    case 'llano':
      return COSTE_MOVIMIENTO.llano;
    case 'costa':
      return COSTE_MOVIMIENTO.costa;
    case 'colina':
      return COSTE_MOVIMIENTO.colina;
    case 'montana':
      return COSTE_MOVIMIENTO.montana;
    case 'cima':
      return COSTE_MOVIMIENTO.cima;
    case 'agua':
      return COSTE_MOVIMIENTO.agua;
  }
}
