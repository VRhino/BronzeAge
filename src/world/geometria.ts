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

/** Distancia mínima de un punto a un segmento (proyección clampeada al segmento) — mismo cálculo que
 * `distanciaASegmento` en `worldgen/colocacion.ts` (`worldgen/` no puede importar de `world/`, así que se
 * duplica esta primitiva pequeña en vez de romper esa capa, ver `Fase_0_1_Definicion.md`). */
export function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largoCuadrado = dx * dx + dy * dy;
  if (largoCuadrado === 0) return distancia(p, a);
  const t = Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largoCuadrado));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
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
