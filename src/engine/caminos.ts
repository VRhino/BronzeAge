// Caminos comerciales (Fase 0.3, Doc 1.6 `1_Sistema_de_Mundo_y_Territorio.md`): se generan automáticamente
// al establecer una relación comercial, y aceleran a las caravanas que los siguen (ver
// `worldgen/config.ts` `COSTE_MOVIMIENTO.factorCamino`, aplicado en `engine/trade.ts`). "MEJORABLE vía
// Políticas" (Doc 1.6) no necesita nada nuevo aquí: `factorVelocidadCaravana` (política "Rutas Rápidas",
// `engine/politicas.ts`) ya es un multiplicador de velocidad de caravana comercial y se sigue aplicando
// encima del bonus de camino sin cambios.

import type { Asentamiento, CaminoComercial } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';

/** Busca el camino entre dos asentamientos, sin importar el orden A/B con el que se guardó. */
export function buscarCamino(caminos: readonly CaminoComercial[], asentamientoAId: string, asentamientoBId: string): CaminoComercial | undefined {
  return caminos.find(
    (c) =>
      (c.asentamientoAId === asentamientoAId && c.asentamientoBId === asentamientoBId) ||
      (c.asentamientoAId === asentamientoBId && c.asentamientoBId === asentamientoAId)
  );
}

/**
 * Garantiza que exista un Camino Comercial entre `asentamientoA` y `asentamientoB` — si ya hay uno, lo deja
 * intacto (el camino es infraestructura física permanente, ver `CaminoComercial`); si no, lo calcula con
 * `calcularRuta` (mismo pathfinding que usa cualquier caravana, ver `world/rutas.ts`) y lo añade.
 */
export function asegurarCaminoComercial(
  caminos: readonly CaminoComercial[],
  mapa: Mapa,
  asentamientoA: Asentamiento,
  asentamientoB: Asentamiento
): CaminoComercial[] {
  if (buscarCamino(caminos, asentamientoA.id, asentamientoB.id)) return [...caminos];

  const nuevo: CaminoComercial = {
    id: `camino-${asentamientoA.id}-${asentamientoB.id}`,
    asentamientoAId: asentamientoA.id,
    asentamientoBId: asentamientoB.id,
    puntos: calcularRuta(mapa, asentamientoA.posicion, asentamientoB.posicion),
  };
  return [...caminos, nuevo];
}
