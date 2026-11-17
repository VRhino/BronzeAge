import type { Point } from './tipos';

export interface Limites {
  ancho: number;
  alto: number;
}

export function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** Distancia mínima de un punto a un segmento (proyección clampeada al segmento). */
export function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largoCuadrado = dx * dx + dy * dy;
  if (largoCuadrado === 0) return distancia(p, a);
  const t = Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largoCuadrado));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
}
