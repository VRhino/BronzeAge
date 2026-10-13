import type { Point } from './tipos';
import { FACTOR } from './campamentos';

/**
 * Los ríos cortan el paso salvo en los vados (decidido el 2026-10-13). El río se traza como barrera en la rejilla de
 * transitabilidad (la misma del artefacto) y cada vado abre un hueco. Así el grafo y el servidor ven el mismo mundo.
 */
export const PARAMETROS_VADOS = {
  /** Separación entre vados a lo largo de un río (estratégicas); siempre hay al menos uno por río. */
  separacion: 100,
  /** Radio del hueco que abre un vado (unidades del generador): el cauce engrosado mide ~3 celdas, el hueco las atraviesa. */
  radioHueco: 100,
};

export interface RioLinea { id: string; puntos: Point[] }

/** Vados de un río: uno hacia el primer tramo y luego uno cada `separacion`; si el río es corto, uno en su punto medio. */
export function colocarVados(rios: readonly RioLinea[], separacion = PARAMETROS_VADOS.separacion * FACTOR): Map<string, Point[]> {
  const salida = new Map<string, Point[]>();
  for (const rio of rios) {
    const vados: Point[] = [];
    const lon = rio.puntos.slice(1).map((p, i) => Math.hypot(p.x - rio.puntos[i]!.x, p.y - rio.puntos[i]!.y));
    const total = lon.reduce((s, x) => s + x, 0);
    const en = (d: number): Point => {
      let resto = d;
      for (let i = 0; i < lon.length; i++) {
        if (resto <= lon[i]! || i === lon.length - 1) {
          const t = lon[i]! ? Math.min(1, resto / lon[i]!) : 0, a = rio.puntos[i]!, b = rio.puntos[i + 1]!;
          return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
        }
        resto -= lon[i]!;
      }
      return rio.puntos[0]!;
    };
    if (rio.puntos.length >= 2) {
      for (let d = Math.min(separacion / 2, total / 2); d < total; d += separacion) vados.push(en(d));
    }
    salida.set(rio.id, vados);
  }
  return salida;
}

/**
 * Máscara de celdas bloqueadas por los ríos (1 = bloqueada), en la rejilla de `lado` unidades del generador: el trazo del río
 * engrosado una celda a cada lado (así ninguna arista del grafo lo cruza entre dos muestras) y con un hueco en cada vado.
 */
export function bloquearRios(
  limites: { ancho: number; alto: number },
  rios: readonly RioLinea[],
  vados: readonly Point[],
  lado: number,
  radioHueco = PARAMETROS_VADOS.radioHueco,
): Uint8Array {
  const nx = Math.ceil(limites.ancho / lado), ny = Math.ceil(limites.alto / lado);
  const traza = new Uint8Array(nx * ny);
  const marca = (p: Point) => {
    const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado);
    if (cx >= 0 && cy >= 0 && cx < nx && cy < ny) traza[cy * nx + cx] = 1;
  };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, n = Math.max(1, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / (lado / 4)));
    for (let k = 0; k <= n; k++) marca({ x: a.x + (b.x - a.x) * k / n, y: a.y + (b.y - a.y) * k / n });
  }
  const bloqueo = new Uint8Array(nx * ny);
  for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
    const i = y * nx + x;
    if (traza[i] || (x > 0 && traza[i - 1]) || (x < nx - 1 && traza[i + 1]) || (y > 0 && traza[i - nx]) || (y < ny - 1 && traza[i + nx])) bloqueo[i] = 1;
  }
  for (const v of vados) {
    const r = Math.ceil(radioHueco / lado) + 1, cx = Math.floor(v.x / lado), cy = Math.floor(v.y / lado);
    for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
      const x = cx + dx, y = cy + dy;
      if (x < 0 || y < 0 || x >= nx || y >= ny) continue;
      if (Math.hypot((x + 0.5) * lado - v.x, (y + 0.5) * lado - v.y) <= radioHueco) bloqueo[y * nx + x] = 0;
    }
  }
  return bloqueo;
}
