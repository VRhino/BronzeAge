import type { Point } from './tipos';
import type { GrafoMundo } from './mundoTipos';
import { FACTOR } from './campamentos';

/**
 * Los ríos cortan el paso salvo en los vados (decidido el 2026-10-13). El río se traza como barrera en la rejilla de
 * transitabilidad (la misma del artefacto): el cauce entero, sin huecos. Desde el 2026-11-18 (decisión del autor) el vado es un
 * **punto**, no un hueco de celdas: un nodo del grafo de navegación justo en el punto del vado, enlazado con las orillas de los dos
 * lados ([`enlazarVados`]). Toda ruta que cruza ese río pasa exactamente por ese punto, así que una columna parada en el vado está en el
 * camino de quien cruce. Hasta entonces cada vado abría todas las celdas en un radio de 100 (unas 8 u) y se cruzaba por cualquier
 * sitio de una franja de 16 u.
 */
export const PARAMETROS_VADOS = {
  /** Separación entre vados a lo largo de un río (estratégicas); siempre hay al menos uno por río. */
  separacion: 100,
  /** Hasta dónde busca un vado los nodos de la orilla con los que se enlaza, en espaciados del grafo (1,5 = 6 u); si un lado se
   * queda sin ninguno, se prueba con `radioEnlaceMaximo`. */
  radioEnlace: 1.5,
  radioEnlaceMaximo: 2.5,
  /** Cuántos nodos de cada orilla, como mucho, enlaza un vado (los más cercanos). */
  enlacesPorOrilla: 3,
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
 * trazo del río, conectadas por un lado ([`recorrerEn4`]: así ninguna arista del grafo lo cruza en diagonal). El cauce va entero: los
 * vados no abren celdas, son nodos del grafo ([`enlazarVados`]).
 */
export function bloquearRios(limites: { ancho: number; alto: number }, rios: readonly RioLinea[], lado: number): Uint8Array {
  const nx = Math.ceil(limites.ancho / lado), ny = Math.ceil(limites.alto / lado);
  // El cauce de una celda (decisión del autor del 2026-11-03): solo las celdas que pisa el río, recorridas en 4 vecinas (al pasar de una
  // celda a la diagonal se marca también una de las dos de al lado), así que una arista diagonal del grafo nunca cruza entre dos esquinas libres.
  // Hasta entonces se engrosaba una celda más a cada lado y la franja prohibida medía unas 12 u frente a un río dibujado de 1,5 u.
  const bloqueo = new Uint8Array(nx * ny);
  const marca = (cx: number, cy: number) => {
    if (cx >= 0 && cy >= 0 && cx < nx && cy < ny) bloqueo[cy * nx + cx] = 1;
  };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) recorrerEn4(rio.puntos[i - 1]!, rio.puntos[i]!, lado, marca);
  return bloqueo;
}

/** ¿Se cortan los segmentos `p`-`q` y `a`-`b`? Devuelve la fracción de `p`-`q` donde se cortan (0..1), o `null`. */
function corte(p: Point, q: Point, a: Point, b: Point): number | null {
  const rx = q.x - p.x, ry = q.y - p.y, sx = b.x - a.x, sy = b.y - a.y;
  const den = rx * sy - ry * sx;
  if (den === 0) return null; // paralelos: el enlace sale del río, no va a lo largo de él
  const t = ((a.x - p.x) * sy - (a.y - p.y) * sx) / den, u = ((a.x - p.x) * ry - (a.y - p.y) * rx) / den;
  return t >= 0 && t <= 1 && u >= 0 && u <= 1 ? t : null;
}

export interface OpcionesEnlace {
  /** ¿Se puede pisar `p` sin contar los ríos? (ni agua ni ladera de más de la cuenta). Los enlaces cruzan el cauce del propio vado. */
  pisableSinRios: (p: Point) => boolean;
  /** Coste de movimiento en el nodo `p` de la orilla (1 = llano, como el `coste` de [`construirGrafo`]). */
  coste: (p: Point) => number;
  /** El nodo del grafo en la celda de `p` (por su posición), o -1. */
  nodoEn: (cx: number, cy: number) => number;
}

/**
 * Añade al grafo un nodo por vado, justo en su punto, y lo enlaza con los nodos de las dos orillas (aristas `vado`). Un enlace sale del
 * vado y llega a un nodo a menos de `radioEnlace` espaciados sin volver a cortar ningún río (el del propio vado solo en su arranque) y
 * pisando a lo largo solo lo que se pisa sin contar los ríos. Cada orilla (el lado del trazo, por el producto vectorial con su tramo) se
 * queda con sus `enlacesPorOrilla` nodos más cercanos. Los ids nuevos siguen a los de la retícula, en el orden de `vados`. Determinista.
 * Devuelve cuántos vados se quedaron sin alguna de las dos orillas (no sirven para cruzar).
 */
export function enlazarVados(
  grafo: GrafoMundo,
  vados: readonly { rio: string; punto: Point }[],
  rios: readonly RioLinea[],
  o: OpcionesEnlace,
  p = PARAMETROS_VADOS,
): number {
  const lado = grafo.espaciado, todos = rios.flatMap(r => r.puntos.slice(1).map((b, i) => ({ a: r.puntos[i]!, b })));
  const posDe = (id: number): Point => grafo.nodos[id]!;
  let cojos = 0;
  for (const { rio, punto: v } of vados) {
    // El tramo del río donde está el vado da el sentido para decidir la orilla.
    const tramos = rios.find(r => r.id === rio)!.puntos;
    let dir = { x: 1, y: 0 }, mejor = Infinity;
    for (let i = 1; i < tramos.length; i++) {
      const a = tramos[i - 1]!, b = tramos[i]!, d = distanciaASegmento(v, a, b);
      if (d < mejor) { mejor = d; dir = { x: b.x - a.x, y: b.y - a.y }; }
    }
    const orillas = (radio: number) => {
      const cerca: { id: number; d: number; lado: number }[] = [];
      const r = Math.ceil(radio), cx = Math.floor(v.x / lado), cy = Math.floor(v.y / lado);
      for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
        const id = o.nodoEn(cx + dx, cy + dy);
        if (id < 0) continue;
        const n = posDe(id), d = Math.hypot(n.x - v.x, n.y - v.y);
        if (d > radio * lado || d === 0) continue;
        // Sin volver a cortar un río: solo vale el corte del arranque (el vado está en el trazo).
        if (todos.some(s => { const t = corte(v, n, s.a, s.b); return t !== null && t * d > 1; })) continue;
        const pasos = Math.ceil(d / (lado / 4));
        let libre = true;
        for (let k = 1; k <= pasos && libre; k++) libre = o.pisableSinRios({ x: v.x + (n.x - v.x) * k / pasos, y: v.y + (n.y - v.y) * k / pasos });
        if (!libre) continue;
        const cruz = dir.x * (n.y - v.y) - dir.y * (n.x - v.x);
        if (cruz !== 0) cerca.push({ id, d, lado: Math.sign(cruz) });
      }
      cerca.sort((a, b) => a.d - b.d || a.id - b.id);
      return [1, -1].map(s => cerca.filter(c => c.lado === s).slice(0, p.enlacesPorOrilla));
    };
    let [izq, der] = orillas(p.radioEnlace);
    if (izq!.length === 0 || der!.length === 0) [izq, der] = orillas(p.radioEnlaceMaximo);
    if (izq!.length === 0 || der!.length === 0) cojos++;
    const id = grafo.nodos.length;
    grafo.nodos.push({ id, x: v.x, y: v.y });
    // El tramo cuesta lo del terreno de su orilla: el cauce del vado no tiene coste propio (su celda está bloqueada y el agua no se pisa).
    for (const c of [...izq!, ...der!]) {
      grafo.aristas.push({ a: c.id, b: id, costeMilesimas: Math.max(1, Math.round(1000 * o.coste(posDe(c.id)) * (c.d / lado))), vado: true });
    }
  }
  return cojos;
}

function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x, dy = b.y - a.y, l2 = dx * dx + dy * dy;
  const t = l2 === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}
