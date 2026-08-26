// Copia de `distanciaARioMasCercano` (`src/worldgen/rios.ts`) y `distanciaASegmento` (`src/world/geometria.ts`)
// del backend — la parte de consulta pura, no la generación de ríos (RNG, server-only).
import type { Point, RioZona } from './tipos';

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largoCuadrado = dx * dx + dy * dy;
  if (largoCuadrado === 0) return distancia(p, a);
  const t = Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largoCuadrado));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
}

export function distanciaARioMasCercano(rios: readonly RioZona[], p: Point): number {
  let minimo = Infinity;
  for (const rio of rios) {
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const d = distanciaASegmento(p, rio.puntos[i]!, rio.puntos[i + 1]!);
      if (d < minimo) minimo = d;
    }
  }
  return minimo;
}
