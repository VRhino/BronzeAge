// Geometría pura de mapa: sin conocimiento de asentamientos, edificios ni facciones.
// Extraída de `engine/zones.ts` porque la necesitan los dos lados — el motor para recortar zonas de
// influencia, y la fachada `Mapa` para responder consultas espaciales sobre un polígono ya recortado.
// `zones.ts` la sigue reexportando para no obligar a sus consumidores a cambiar de import.

import type { Point } from '../domain/types';

export function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

export function pointInPolygon(p: Point, polygon: Point[]): boolean {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const pi = polygon[i]!;
    const pj = polygon[j]!;
    const intersects =
      pi.y > p.y !== pj.y > p.y && p.x < ((pj.x - pi.x) * (p.y - pi.y)) / (pj.y - pi.y) + pi.x;
    if (intersects) inside = !inside;
  }
  return inside;
}

/** Caja envolvente de un polígono, para poder acotar qué celdas del índice espacial hay que mirar. */
export function boundingBox(polygon: Point[]): { minX: number; minY: number; maxX: number; maxY: number } | null {
  if (polygon.length === 0) return null;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const p of polygon) {
    if (p.x < minX) minX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.x > maxX) maxX = p.x;
    if (p.y > maxY) maxY = p.y;
  }
  return { minX, minY, maxX, maxY };
}
