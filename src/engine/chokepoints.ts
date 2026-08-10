// Control de chokepoints y peaje (Fase 0.3, Doc 1.5 `1_Sistema_de_Mundo_y_Territorio.md`). Alcance
// confirmado con el usuario: geometría + control por zona de influencia + peaje en oro a Facciones no
// controladoras — SIN bloqueo/escolta militar todavía (no existe un concepto de "guerra activa" entre
// Facciones al que enganchar un bloqueo real, ver `Preguntas_Abiertas.md`).

import type { Asentamiento, Chokepoint, Point, ZonaInfluencia } from '../domain/types';
import { distanciaASegmento, pointInPolygon } from '../world/geometria';

/** Asentamiento que controla un chokepoint: aquel cuya zona de influencia lo cubre. `null` si ningún
 * asentamiento lo tiene dentro (mismo criterio que las fronteras "vivas" de Doc 1.2). */
export function controladorDeChokepoint(chokepoint: Chokepoint, zonas: readonly ZonaInfluencia[]): string | null {
  return zonas.find((z) => pointInPolygon(chokepoint.posicion, z.poligono))?.asentamientoId ?? null;
}

/** Distancia mínima de un punto a cualquier tramo de una polilínea — mismo patrón que
 * `distanciaARioMasCercano` en `worldgen/rios.ts`, de uso general aquí (rutas de caravana). */
function distanciaAPolilinea(p: Point, puntos: readonly Point[]): number {
  let minimo = Infinity;
  for (let i = 0; i < puntos.length - 1; i++) {
    const d = distanciaASegmento(p, puntos[i]!, puntos[i + 1]!);
    if (d < minimo) minimo = d;
  }
  return minimo;
}

export interface PeajeChokepoint {
  chokepoint: Chokepoint;
  controladorId: string;
}

/**
 * Chokepoints que `ruta` atraviesa (a `chokepoint.radio` o menos de algún tramo) Y que están controlados
 * por un asentamiento de una Facción DISTINTA a `faccionOrigenId` — los únicos que cobran peaje. Se llama
 * en la LLEGADA de la caravana (`engine/trade.ts`), con la zona de influencia y el control tal como están
 * en ese momento — no en el lanzamiento, para que el peaje refleje quién domina el paso de verdad al
 * completarse el viaje, no al empezarlo.
 */
export function chokepointsDePeajeEnRuta(
  chokepoints: readonly Chokepoint[],
  zonas: readonly ZonaInfluencia[],
  ruta: readonly Point[],
  faccionOrigenId: string,
  asentamientosPorId: ReadonlyMap<string, Asentamiento>
): PeajeChokepoint[] {
  if (ruta.length < 2) return [];
  const resultado: PeajeChokepoint[] = [];
  for (const chokepoint of chokepoints) {
    if (distanciaAPolilinea(chokepoint.posicion, ruta) > chokepoint.radio) continue;
    const controladorId = controladorDeChokepoint(chokepoint, zonas);
    if (!controladorId) continue;
    const controlador = asentamientosPorId.get(controladorId);
    if (!controlador || controlador.faccionId === faccionOrigenId) continue;
    resultado.push({ chokepoint, controladorId });
  }
  return resultado;
}
