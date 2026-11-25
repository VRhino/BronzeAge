// El relieve de los ríos (experimento/rios): una sola rejilla de alturas, fuente de verdad de valles y cauces. Sobre el relieve regional
// muestreado en ella se simula la erosión de la corriente (Braun y Willett 2013: potencia de la corriente implícita, en tiempo lineal) con
// un poco de difusión de ladera; los ríos se leen de ella y sus cauces se tallan en ella, cada uno bajando celda a celda. El relieve continuo
// (`evaluarElevacion`) la interpola en bicúbico (Catmull-Rom, que pasa por cada nodo: un punto de río en el centro de su celda vale lo que
// la celda). Datos planos: viaja a los hilos del paquete de relieve.
import type { Point } from './tipos';

export interface RejillaFluvial {
  /** Lado de la celda (unidades del generador) y la rejilla; alturas normalizadas (0-1) en el centro de cada celda. */
  celda: number;
  columnas: number;
  filas: number;
  altura: Float32Array;
}

/** La erosión: `vueltas` pasos de potencia de la corriente (`K` por paso, en metros: lo que baja un cauce de pendiente 1 y 1 m² de cuenca,
 * con la raíz del área) y de difusión de ladera (`difusion` por paso, estable por debajo de 0,25); el desagüe se rehace cada `rehacerDesague`
 * pasos. */
export const EROSION_CORRIENTE = { celda: 40, vueltas: 40, K: 0.005, difusion: 0.05, rehacerDesague: 4 };

const VECINOS: readonly [number, number][] = [[-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1]];
/** Lo que sube el relleno de una celda a la siguiente (normalizado): las hoyas quedan con una pendiente mínima hacia su desagüe. */
const EPSILON = 1e-6;

/** Catmull-Rom en una dimensión. */
const cr = (a: number, b: number, c: number, d: number, t: number) =>
  b + 0.5 * t * (c - a + t * (2 * a - 5 * b + 4 * c - d + t * (3 * (b - c) + d - a)));

/** La altura de la rejilla en `p`, en bicúbico: en el centro de una celda, exactamente la de la celda. */
export function alturaFluvial(r: RejillaFluvial, p: Point): number {
  const fx = Math.min(Math.max(p.x / r.celda - 0.5, 0), r.columnas - 1), fy = Math.min(Math.max(p.y / r.celda - 0.5, 0), r.filas - 1);
  const i = Math.min(r.columnas - 2, Math.floor(fx)), j = Math.min(r.filas - 2, Math.floor(fy)), tx = fx - i, ty = fy - j;
  const h = (a: number, b: number) => r.altura[Math.min(r.filas - 1, Math.max(0, b)) * r.columnas + Math.min(r.columnas - 1, Math.max(0, a))]!;
  const fila = (b: number) => cr(h(i - 1, b), h(i, b), h(i + 1, b), h(i + 2, b), tx);
  return cr(fila(j - 1), fila(j), fila(j + 1), fila(j + 2), ty);
}

/** Montículo de mínimos por clave (para el Priority-Flood). Sin funciones creadas dentro de los bucles: es lo más caliente del drenaje. */
class Monticulo {
  private readonly i: number[] = [];
  private readonly k: number[] = [];
  get vacio() { return this.i.length === 0; }
  private menor(x: number, y: number): boolean { return this.k[x]! < this.k[y]! || (this.k[x] === this.k[y] && this.i[x]! < this.i[y]!); }
  meter(indice: number, clave: number) {
    let n = this.i.length;
    this.i.push(indice); this.k.push(clave);
    while (n > 0) {
      const p = (n - 1) >> 1;
      if (this.k[p]! < clave || (this.k[p] === clave && this.i[p]! < indice)) break;
      this.i[n] = this.i[p]!; this.k[n] = this.k[p]!; this.i[p] = indice; this.k[p] = clave; n = p;
    }
  }
  sacar(): number {
    const fuera = this.i[0]!, ui = this.i.pop()!, uk = this.k.pop()!;
    if (this.i.length) {
      let n = 0;
      this.i[0] = ui; this.k[0] = uk;
      for (;;) {
        const a = 2 * n + 1, b = a + 1;
        let m = n;
        if (a < this.i.length && this.menor(a, m)) m = a;
        if (b < this.i.length && this.menor(b, m)) m = b;
        if (m === n) break;
        [this.i[n], this.i[m]] = [this.i[m]!, this.i[n]!]; [this.k[n], this.k[m]] = [this.k[m]!, this.k[n]!]; n = m;
      }
    }
    return fuera;
  }
}

export interface Desague {
  /** La superficie rellena (sin hoyas: cada celda baja a su receptor), el receptor de cada celda (-1 en las fijas: mar y borde), el orden
   * de abajo arriba (cada celda después de su receptor) y el área drenada en celdas. */
  lleno: Float64Array;
  receptor: Int32Array;
  orden: Int32Array;
  acumulacion: Float64Array;
}

/** Priority-Flood con épsilon (Barnes 2014) desde las celdas `fijo` (mar y borde), receptores por la mayor bajada sobre lo relleno y área
 * drenada. */
export function desaguar(h: Float64Array, columnas: number, filas: number, fijo: Uint8Array): Desague {
  const n = columnas * filas, lleno = new Float64Array(n), visto = new Uint8Array(n), cola = new Monticulo(), orden = new Int32Array(n);
  for (let k = 0; k < n; k++) if (fijo[k]) { visto[k] = 1; lleno[k] = h[k]!; cola.meter(k, lleno[k]!); }
  let o = 0;
  while (!cola.vacio) {
    const c = cola.sacar(), x = c % columnas, y = (c - x) / columnas;
    orden[o++] = c;
    for (const [dx, dy] of VECINOS) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= columnas || ny >= filas) continue;
      const j = ny * columnas + nx;
      if (visto[j]) continue;
      visto[j] = 1; lleno[j] = Math.max(h[j]!, lleno[c]! + EPSILON); cola.meter(j, lleno[j]!);
    }
  }
  const receptor = new Int32Array(n).fill(-1);
  for (let k = 0; k < n; k++) {
    if (fijo[k]) continue;
    const x = k % columnas, y = (k - x) / columnas;
    let mejor = -1, bajada = 0;
    for (const [dx, dy] of VECINOS) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= columnas || ny >= filas) continue;
      const j = ny * columnas + nx, b = (lleno[k]! - lleno[j]!) / Math.hypot(dx, dy);
      if (b > bajada) { bajada = b; mejor = j; }
    }
    receptor[k] = mejor;
  }
  const acumulacion = new Float64Array(n).fill(1);
  for (let q = n - 1; q >= 0; q--) { const k = orden[q]!, r = receptor[k]!; if (r >= 0) acumulacion[r] = acumulacion[r]! + acumulacion[k]!; }
  return { lleno, receptor, orden, acumulacion };
}

/** La erosión de la corriente sobre `h` (normalizada; se cambia en su sitio). Cada paso: desagüe, potencia de la corriente implícita
 * (de abajo arriba: cada celda baja hacia su receptor ya actualizado, `h = (h + F·h_r)/(1 + F)`, con `F = K·√área/distancia`) y difusión
 * de ladera. Las celdas `fijo` no cambian. */
export function erosionarCorriente(h: Float64Array, columnas: number, filas: number, celda: number, fijo: Uint8Array, o = EROSION_CORRIENTE): void {
  const n = columnas * filas, area = celda * celda, m = new Float64Array(n);
  for (let k = 0; k < n; k++) m[k] = h[k]! * 1000;
  let d: Desague | undefined;
  for (let vuelta = 0; vuelta < o.vueltas; vuelta++) {
    // el desagüe (relleno, receptores y área) cambia poco de un paso al siguiente: se rehace cada `rehacerDesague` pasos
    if (!d || vuelta % o.rehacerDesague === 0) d = desaguar(Float64Array.from(m, v => v / 1000), columnas, filas, fijo);
    for (let q = 0; q < n; q++) {
      const k = d.orden[q]!, r = d.receptor[k]!;
      if (r < 0 || fijo[k]) continue;
      const dist = Math.hypot((k % columnas) - (r % columnas), Math.floor(k / columnas) - Math.floor(r / columnas)) * celda;
      const F = (o.K * Math.sqrt(d.acumulacion[k]! * area)) / dist;
      m[k] = (m[k]! + F * m[r]!) / (1 + F);
    }
    if (o.difusion > 0) {
      const antes = Float64Array.from(m);
      for (let y = 1; y < filas - 1; y++) for (let x = 1; x < columnas - 1; x++) {
        const k = y * columnas + x;
        if (fijo[k]) continue;
        m[k] = antes[k]! + o.difusion * (antes[k - 1]! + antes[k + 1]! + antes[k - columnas]! + antes[k + columnas]! - 4 * antes[k]!);
      }
    }
  }
  for (let k = 0; k < n; k++) h[k] = m[k]! / 1000;
}

/** Las celdas que recorre una polilínea, en orden y vecinas entre sí (8 vecinos), sin repetir: un lazo que vuelve a una celda ya pasada
 * se corta ahí (a esta escala, el río lo atajaría). La primera y la última, las de sus extremos. */
export function celdasDeLinea(puntos: readonly Point[], celda: number, columnas: number, filas: number): number[] {
  const indice = (p: Point) => Math.min(filas - 1, Math.max(0, Math.floor(p.y / celda))) * columnas + Math.min(columnas - 1, Math.max(0, Math.floor(p.x / celda)));
  const salida: number[] = [], donde = new Map<number, number>();
  const poner = (k: number) => {
    const ya = donde.get(k);
    if (ya !== undefined) { for (const q of salida.splice(ya + 1)) donde.delete(q); return; }
    // rellenar el hueco con un paso recto (Bresenham) si la celda nueva no es vecina de la anterior
    const ultima = salida[salida.length - 1];
    if (ultima !== undefined) {
      let x0 = ultima % columnas, y0 = (ultima - x0) / columnas;
      const x1 = k % columnas, y1 = (k - x1) / columnas;
      while (Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0)) > 1) {
        x0 += Math.sign(x1 - x0); y0 += Math.sign(y1 - y0);
        const c = y0 * columnas + x0;
        if (donde.has(c)) { for (const q of salida.splice(donde.get(c)! + 1)) donde.delete(q); continue; }
        donde.set(c, salida.length); salida.push(c);
      }
    }
    donde.set(k, salida.length); salida.push(k);
  };
  for (let i = 0; i < puntos.length; i++) {
    poner(indice(puntos[i]!));
    if (i + 1 < puntos.length) {
      const a = puntos[i]!, b = puntos[i + 1]!, pasos = Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / (celda / 4));
      for (let s = 1; s < pasos; s++) poner(indice({ x: a.x + ((b.x - a.x) * s) / pasos, y: a.y + ((b.y - a.y) * s) / pasos }));
    }
  }
  return salida;
}

/** Baja a `nivel(punto más cercano)` las celdas a menos de `medio(punto)` de la polilínea (una vega, un lecho de grava): nunca sube nada. */
export function aplanarFranja(h: Float64Array, columnas: number, filas: number, celda: number, puntos: readonly Point[], medio: readonly number[], nivel: readonly number[]): void {
  const R = Math.max(...medio) + celda;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const p of puntos) { x0 = Math.min(x0, p.x); y0 = Math.min(y0, p.y); x1 = Math.max(x1, p.x); y1 = Math.max(y1, p.y); }
  for (let y = Math.max(0, Math.floor((y0 - R) / celda)); y <= Math.min(filas - 1, Math.floor((y1 + R) / celda)); y++)
    for (let x = Math.max(0, Math.floor((x0 - R) / celda)); x <= Math.min(columnas - 1, Math.floor((x1 + R) / celda)); x++) {
      const c = { x: (x + 0.5) * celda, y: (y + 0.5) * celda };
      let mejor = Infinity, cerca = -1, t = 0;
      for (let i = 0; i + 1 < puntos.length; i++) {
        const a = puntos[i]!, b = puntos[i + 1]!, dx = b.x - a.x, dy = b.y - a.y;
        const u = Math.max(0, Math.min(1, ((c.x - a.x) * dx + (c.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
        const d = Math.hypot(a.x + dx * u - c.x, a.y + dy * u - c.y);
        if (d < mejor) { mejor = d; cerca = i; t = u; }
      }
      if (cerca < 0 || mejor > medio[cerca]! + (medio[cerca + 1]! - medio[cerca]!) * t) continue;
      const k = y * columnas + x, v = nivel[cerca]! + (nivel[cerca + 1]! - nivel[cerca]!) * t;
      if (v < h[k]!) h[k] = v;
    }
}

/** Los cauces finos: el lecho de cada río publicado, encima de la rejilla (que a 40 m no lo resuelve). Cada tramo lleva su cota por punto,
 * que baja siempre, y su medio ancho; el relieve se queda con lo más bajo de la rejilla y el cauce. En el eje del río la altura es la cota
 * del cauce, así que el río publicado baja por construcción; a los lados, la orilla sube al relieve a lo largo de `rampa` medios anchos.
 * Datos planos (los ven los hilos del paquete de relieve): por segmento, `ax ay bx by za zb medioA medioB`. */
export interface CaucesFinos { celda: number; segmentos: number[]; indice: Record<string, number[]> }

export const CAUCE_FINO = { celdaIndice: 300, rampa: 1, rampaMinima: 20,
  /** La orilla se alarga para no pasar de `pendiente` (m/m) donde el cauce queda muy por debajo del relieve, hasta `rampaMaxima` m. */
  pendiente: 0.35, rampaMaxima: 200 };

export function crearCaucesFinos(rios: readonly { puntos: readonly Point[]; nivel: readonly number[]; medio: readonly number[] }[]): CaucesFinos {
  const C = CAUCE_FINO, segmentos: number[] = [], indice: Record<string, number[]> = {};
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, k = segmentos.length / 8;
    segmentos.push(a.x, a.y, b.x, b.y, rio.nivel[i - 1]!, rio.nivel[i]!, rio.medio[i - 1]!, rio.medio[i]!);
    const alcance = Math.max(rio.medio[i - 1]!, rio.medio[i]!) + Math.max(C.rampaMinima, Math.max(rio.medio[i - 1]!, rio.medio[i]!) * C.rampa, C.rampaMaxima);
    for (let y = Math.floor((Math.min(a.y, b.y) - alcance) / C.celdaIndice); y <= Math.floor((Math.max(a.y, b.y) + alcance) / C.celdaIndice); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - alcance) / C.celdaIndice); x <= Math.floor((Math.max(a.x, b.x) + alcance) / C.celdaIndice); x++) (indice[`${x},${y}`] ??= []).push(k);
  }
  return { celda: C.celdaIndice, segmentos, indice };
}

/** El relieve `e` con los cauces finos: lo más bajo de él y de cada cauce cercano (su cota, y la orilla que sube hasta `e`). */
export function aplicarCauces(c: CaucesFinos, p: Point, e: number): number {
  const lista = c.indice[`${Math.floor(p.x / c.celda)},${Math.floor(p.y / c.celda)}`];
  if (!lista) return e;
  let salida = e;
  for (const k of lista) {
    const s = c.segmentos, ax = s[8 * k]!, ay = s[8 * k + 1]!, dx = s[8 * k + 2]! - ax, dy = s[8 * k + 3]! - ay;
    const t = Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / (dx * dx + dy * dy || 1)));
    const z = s[8 * k + 4]! + (s[8 * k + 5]! - s[8 * k + 4]!) * t;
    if (z >= salida) continue;
    // la orilla, en smoothstep: su pendiente más fuerte es 1,5 veces la media (la altura en normalizada: 1 = 1000 m)
    const medio = s[8 * k + 6]! + (s[8 * k + 7]! - s[8 * k + 6]!) * t, F = CAUCE_FINO;
    const rampa = Math.min(F.rampaMaxima, Math.max(F.rampaMinima, medio * F.rampa, (1.5 * (e - z) * 1000) / F.pendiente));
    const d = Math.hypot(p.x - ax - t * dx, p.y - ay - t * dy);
    if (d >= medio + rampa) continue;
    const u = Math.max(0, (d - medio) / rampa), orilla = u * u * (3 - 2 * u);
    salida = Math.min(salida, z + (e - z) * orilla);
  }
  return salida;
}
