import type { Point } from './tipos';
import { FACTOR } from './campamentos';
import type { Limites } from './geometria';
import type { AristaGrafo, GrafoMundo, NodoGrafo } from './mundoTipos';

/** Espaciado de la retícula: 8 unidades estratégicas (Doc 1, contrato de mundo §2). En unidades del generador. */
export const ESPACIADO_NAVEGACION = 4 * FACTOR;
/** Separación máxima entre muestras al comprobar un segmento (unidades del generador). */
const PASO_MUESTREO = 30;

export interface OpcionesGrafo {
  espaciado?: number;
  /** Coste de la arista como la media de sus dos extremos, sin muestrear el punto medio (con el espaciado de una celda,
   * el punto medio cae en el borde entre dos celdas y no añade información). */
  costePorExtremos?: boolean;
  /** Distancia máxima de un vado al segmento de una arista para marcarla como vado. Por defecto, medio espaciado. */
  radioVado?: number;
}

const DIRECCIONES: readonly (readonly [number, number])[] = [[1, 0], [0, 1], [1, 1], [-1, 1]];
// Solo las 4 direcciones "hacia delante" (E, S, SE, SO): cada arista no dirigida sale una sola vez, con a < b.

function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x, dy = b.y - a.y, l2 = dx * dx + dy * dy;
  const t = l2 === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/**
 * Grafo de navegación (`GrafoMundo`): la única autoridad de rutas.
 *
 * - Nodos: retícula de `espaciado` (por defecto 48 = 4 estratégicas) con el primer nodo a medio espaciado del borde, sobre
 *   los puntos donde `transitable` es cierto. Ids consecutivos desde 0 en orden fila-mayor (y, luego x): estables.
 * - Aristas: vecinos en 8 direcciones, solo si TODO el segmento es transitable (muestreo cada <= 30 unidades).
 * - `costeMilesimas` = coste medio de movimiento a lo largo del segmento (1000 = llano; `coste` se evalúa en los dos nodos y en el
 *   punto medio, y se integra por Simpson; con `costePorExtremos`, la media de los dos nodos) multiplicado por la longitud relativa: las diagonales cuestan √2 (1414 en llano). Así la suma de costes de
 *   una ruta es proporcional a su tiempo real de marcha (el coste de una arista = velocidad inversa × distancia en
 *   unidades de espaciado) y A* no prefiere zigzaguear en diagonal.
 * - `vado`: la arista pasa a menos de `radioVado` de algún punto de `vados`. La transitabilidad la decide solo
 *   `transitable`: un río que corta el paso lo expresa quien llama, dejando transitable el tramo del vado.
 *
 * Determinista (sin aleatoriedad) y lineal en nodos: índices por celda para los vados, sin comparaciones O(n²).
 */
export function construirGrafo(
  limites: Limites,
  transitable: (p: Point) => boolean,
  coste: (p: Point) => number,
  vados: readonly Point[] = [],
  opciones: OpcionesGrafo = {},
): GrafoMundo {
  const espaciado = opciones.espaciado ?? ESPACIADO_NAVEGACION;
  const radioVado = opciones.radioVado ?? espaciado / 2;
  const cols = Math.floor(limites.ancho / espaciado), filas = Math.floor(limites.alto / espaciado);
  const pos = (c: number, f: number): Point => ({ x: (c + 0.5) * espaciado, y: (f + 0.5) * espaciado });

  // Nodos
  const idDe = new Int32Array(cols * filas).fill(-1);
  const nodos: NodoGrafo[] = [], costeNodo: number[] = [];
  for (let f = 0; f < filas; f++) for (let c = 0; c < cols; c++) {
    const p = pos(c, f);
    if (!transitable(p)) continue;
    costeNodo.push(coste(p));
    idDe[f * cols + c] = nodos.length;
    nodos.push({ id: nodos.length, x: p.x, y: p.y });
  }

  // Índice de vados por celda de tamaño `espaciado`
  const celdaVados = new Map<number, Point[]>();
  const clave = (cx: number, cy: number) => cy * 65536 + cx; // cx < 65536; los negativos solo se consultan, nunca se guardan
  for (const v of vados) {
    const k = clave(Math.floor(v.x / espaciado), Math.floor(v.y / espaciado));
    const lista = celdaVados.get(k);
    if (lista) lista.push(v); else celdaVados.set(k, [v]);
  }
  const alcance = Math.ceil(radioVado / espaciado);
  const esVado = (a: Point, b: Point): boolean => {
    if (celdaVados.size === 0) return false;
    const x0 = Math.floor(Math.min(a.x, b.x) / espaciado) - alcance, x1 = Math.floor(Math.max(a.x, b.x) / espaciado) + alcance;
    const y0 = Math.floor(Math.min(a.y, b.y) / espaciado) - alcance, y1 = Math.floor(Math.max(a.y, b.y) / espaciado) + alcance;
    for (let cy = y0; cy <= y1; cy++) for (let cx = x0; cx <= x1; cx++)
      for (const v of celdaVados.get(clave(cx, cy)) ?? []) if (distanciaASegmento(v, a, b) <= radioVado) return true;
    return false;
  };

  // Aristas: ids a < b por construcción (solo direcciones hacia delante en orden fila-mayor; la SO va a la fila siguiente)
  const aristas: AristaGrafo[] = [];
  for (let f = 0; f < filas; f++) for (let c = 0; c < cols; c++) {
    const a = idDe[f * cols + c]!;
    if (a < 0) continue;
    const pa = nodos[a]!;
    for (const [dc, df] of DIRECCIONES) {
      const c2 = c + dc, f2 = f + df;
      if (c2 < 0 || c2 >= cols || f2 >= filas) continue;
      const b = idDe[f2 * cols + c2]!;
      if (b < 0) continue;
      const pb = nodos[b]!;
      const largo = Math.hypot(pb.x - pa.x, pb.y - pa.y), tramos = Math.ceil(largo / PASO_MUESTREO);
      let libre = true;
      for (let i = 1; i < tramos && libre; i++) libre = transitable({ x: pa.x + (pb.x - pa.x) * i / tramos, y: pa.y + (pb.y - pa.y) * i / tramos });
      if (!libre) continue;
      // Coste medio por Simpson (extremos en caché + 1 muestra en el punto medio): exacto hasta tercer grado.
      const medioCoste = opciones.costePorExtremos ? (costeNodo[a]! + costeNodo[b]!) / 2
        : (costeNodo[a]! + 4 * coste({ x: (pa.x + pb.x) / 2, y: (pa.y + pb.y) / 2 }) + costeNodo[b]!) / 6;
      const costeMilesimas = Math.max(1, Math.round(1000 * medioCoste * (largo / espaciado)));
      aristas.push({ a, b, costeMilesimas, vado: esVado(pa, pb) });
    }
  }
  return { nodos, aristas, espaciado };
}

export interface Componentes {
  /** Componente de cada nodo (indexado por id), numeradas por orden de aparición. */
  componenteDe: Int32Array;
  /** Nodos de cada componente. */
  tamanos: number[];
}

/** Componentes conexas del grafo (unión-búsqueda). Los ids de nodo son los índices de `grafo.nodos`. */
export function componentes(grafo: GrafoMundo): Componentes {
  const padre = Int32Array.from({ length: grafo.nodos.length }, (_, i) => i);
  const raiz = (i: number): number => { while (padre[i] !== i) { padre[i] = padre[padre[i]!]!; i = padre[i]!; } return i; };
  for (const { a, b } of grafo.aristas) { const ra = raiz(a), rb = raiz(b); if (ra !== rb) padre[Math.max(ra, rb)] = Math.min(ra, rb); }
  const numero = new Map<number, number>(), tamanos: number[] = [];
  const componenteDe = new Int32Array(padre.length);
  for (let i = 0; i < padre.length; i++) {
    const r = raiz(i);
    let n = numero.get(r);
    if (n === undefined) { n = tamanos.length; numero.set(r, n); tamanos.push(0); }
    componenteDe[i] = n;
    tamanos[n]!++;
  }
  return { componenteDe, tamanos };
}

export interface EstadisticasGrafo {
  nodos: number;
  aristas: number;
  aristasVado: number;
  componentes: number;
  /** Índice (en `Componentes`) de la componente con más nodos; -1 si no hay nodos. */
  mayor: number;
  nodosMayor: number;
  /** nodosMayor / nodos (1 si no hay nodos). */
  fraccionMayor: number;
}

export function estadisticasGrafo(grafo: GrafoMundo, comp: Componentes = componentes(grafo)): EstadisticasGrafo {
  let mayor = -1;
  comp.tamanos.forEach((t, i) => { if (mayor < 0 || t > comp.tamanos[mayor]!) mayor = i; });
  const nodosMayor = mayor < 0 ? 0 : comp.tamanos[mayor]!;
  return {
    nodos: grafo.nodos.length,
    aristas: grafo.aristas.length,
    aristasVado: grafo.aristas.filter(a => a.vado).length,
    componentes: comp.tamanos.length,
    mayor,
    nodosMayor,
    fraccionMayor: grafo.nodos.length === 0 ? 1 : nodosMayor / grafo.nodos.length,
  };
}

/** Índice por celda de `espaciado` (clave numérica → posiciones en `grafo.nodos`, en orden), uno por grafo. */
const indicesDeNodos = new WeakMap<GrafoMundo, Map<number, number[]>>();
const claveCelda = (cx: number, cy: number) => cy * 1048576 + cx;

/** Nodo de la retícula más cercano a `p` (o undefined si no hay ninguno a menos de un espaciado).
 * Mira solo las celdas vecinas (a ±2 por si el redondeo de la división mueve un nodo del borde; un nodo a un espaciado o menos cae en ±1) y recorre los candidatos en el orden
 * de `grafo.nodos` con la misma comparación `<=` que la búsqueda lineal, así que devuelve exactamente el mismo nodo. */
export function nodoMasCercano(grafo: GrafoMundo, p: Point): NodoGrafo | undefined {
  let indice = indicesDeNodos.get(grafo);
  if (!indice) {
    indice = new Map();
    grafo.nodos.forEach((n, k) => {
      const c = claveCelda(Math.floor(n.x / grafo.espaciado), Math.floor(n.y / grafo.espaciado));
      const lista = indice!.get(c);
      if (lista) lista.push(k); else indice!.set(c, [k]);
    });
    indicesDeNodos.set(grafo, indice);
  }
  const cx = Math.floor(p.x / grafo.espaciado), cy = Math.floor(p.y / grafo.espaciado);
  const candidatos: number[] = [];
  for (let y = cy - 2; y <= cy + 2; y++) for (let x = cx - 2; x <= cx + 2; x++) candidatos.push(...indice.get(claveCelda(x, y)) ?? []);
  candidatos.sort((a, b) => a - b);
  let mejor: NodoGrafo | undefined, d2 = grafo.espaciado * grafo.espaciado;
  for (const k of candidatos) { const n = grafo.nodos[k]!, d = (n.x - p.x) ** 2 + (n.y - p.y) ** 2; if (d <= d2) { d2 = d; mejor = n; } }
  return mejor;
}
