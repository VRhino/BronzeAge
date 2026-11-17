import type { LagoZona, Point } from './tipos';

/** La misma orilla para representación, vegetación y transitabilidad. */
export function puntoEnLago(lago: LagoZona, p: Point): boolean {
  if (Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y) >= lago.radio) return false;
  if (!lago.contorno) return true;
  let dentro = false;
  const ps = lago.contorno;
  for (let i = 0, j = ps.length - 1; i < ps.length; j = i++) {
    const a = ps[i]!, b = ps[j]!;
    if ((a.y > p.y) !== (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x) dentro = !dentro;
  }
  return dentro;
}
