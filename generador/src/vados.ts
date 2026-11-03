import type { Point } from './tipos';
import { FACTOR } from './campamentos';

/**
 * Los ríos cortan el paso salvo en los vados (decidido el 2026-10-13). El río se traza como barrera en la rejilla de
 * transitabilidad (la misma del artefacto) y cada vado abre un hueco. Así el grafo y el servidor ven el mismo mundo.
 */
export const PARAMETROS_VADOS = {
  /** Separación entre vados a lo largo de un río (estratégicas); siempre hay al menos uno por río. */
  separacion: 100,
  /** Radio del hueco que abre un vado (unidades del generador): el cauce mide una celda (dos donde va en diagonal), el hueco lo atraviesa de sobra. */
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

/** Las celdas de lado `lado` que pisa el segmento `a`-`b`, en orden y conectadas por un lado (nunca solo por una esquina). */
export function recorrerEn4(a: Point, b: Point, lado: number, marca: (cx: number, cy: number) => void): void {
  let cx = Math.floor(a.x / lado), cy = Math.floor(a.y / lado);
  const fx = Math.floor(b.x / lado), fy = Math.floor(b.y / lado);
  const dx = b.x - a.x, dy = b.y - a.y, sx = Math.sign(dx), sy = Math.sign(dy);
  // Fracción del segmento hasta el siguiente borde vertical y horizontal, y lo que se avanza por celda.
  let tx = sx === 0 ? Infinity : ((sx > 0 ? cx + 1 : cx) * lado - a.x) / dx;
  let ty = sy === 0 ? Infinity : ((sy > 0 ? cy + 1 : cy) * lado - a.y) / dy;
  const px = sx === 0 ? Infinity : lado / Math.abs(dx), py = sy === 0 ? Infinity : lado / Math.abs(dy);
  marca(cx, cy);
  for (let n = Math.abs(fx - cx) + Math.abs(fy - cy); n > 0; n--) {
    if (tx < ty) { cx += sx; tx += px; }
    else if (ty < tx) { cy += sy; ty += py; }
    else { marca(cx + sx, cy); cx += sx; cy += sy; tx += px; ty += py; n--; } // justo por la esquina: también la de al lado
    marca(cx, cy);
  }
}

/**
 * Máscara de celdas bloqueadas por los ríos (1 = bloqueada), en la rejilla de `lado` unidades del generador: las celdas que pisa el
 * trazo del río, conectadas por un lado ([`recorrerEn4`]: así ninguna arista del grafo lo cruza en diagonal), con un hueco en cada vado.
 */
export function bloquearRios(
  limites: { ancho: number; alto: number },
  rios: readonly RioLinea[],
  vados: readonly Point[],
  lado: number,
  radioHueco = PARAMETROS_VADOS.radioHueco,
): Uint8Array {
  const nx = Math.ceil(limites.ancho / lado), ny = Math.ceil(limites.alto / lado);
  // El cauce de una celda (decisión del autor del 2026-11-03): solo las celdas que pisa el río, recorridas en 4 vecinas (al pasar de una
  // celda a la diagonal se marca también una de las dos de al lado), así que una arista diagonal del grafo nunca cruza entre dos esquinas libres.
  // Hasta entonces se engrosaba una celda más a cada lado y la franja prohibida medía unas 12 u frente a un río dibujado de 1,5 u.
  const bloqueo = new Uint8Array(nx * ny);
  const marca = (cx: number, cy: number) => {
    if (cx >= 0 && cy >= 0 && cx < nx && cy < ny) bloqueo[cy * nx + cx] = 1;
  };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) recorrerEn4(rio.puntos[i - 1]!, rio.puntos[i]!, lado, marca);
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
