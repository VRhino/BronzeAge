// Grafo de navegación global (ficha `Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md` §3, §7): una
// rejilla 8-conexa sobre TODO el mapa, con el coste de cada arista ya integrado a lo largo de ella —relieve y
// bosque (`Mapa.costeEnPunto`), agua (infranqueable) y ríos (infranqueables salvo por un vado)—. Se calcula una
// vez por mundo y se cachea; `world/rutas.ts` corre el A* encima. Sustituye a la malla que antes se montaba en
// cada búsqueda, que no escalaba al mundo regional (§7: segundos por ruta a 10 km).
//
// Excepción consciente a "nunca rejilla horneada" (`Fase_0_1_Definicion.md`): es una caché DERIVADA del campo
// continuo, nunca se guarda en `MapaGenerado` ni en la partida. Lo que se guarda es la red de caminos (las
// aristas que alguien ha recorrido, `engine/redCaminos.ts`), que es infraestructura de partida.

import type { Point, RioZona } from '../domain/types';
import { NAVEGACION } from '../worldgen/config';
import { distancia } from './geometria';
import type { Mapa } from './mapa';

/** Las 8 direcciones de la rejilla (col, fila). El opuesto de la dirección `d` es `d ^ 1`. */
export const VECINOS: readonly (readonly [number, number])[] = [
  [1, 0], [-1, 0], [0, 1], [0, -1],
  [1, 1], [-1, -1], [1, -1], [-1, 1],
];

interface SegmentoRio {
  a: Point;
  b: Point;
  /** Longitud del cauce desde el nacimiento hasta `a`. */
  s0: number;
}

export interface GrafoNavegacion {
  espaciado: number;
  cols: number;
  filas: number;
  /** Separación de muestras al validar un tramo. */
  paso: number;
  /** Coste de la arista `nodo * 8 + d`; `Infinity` = no se recorre (agua, fuera del mapa). */
  coste: Float64Array;
  /** Aristas que solo corta un río fuera de un vado, con el punto del cruce: las abre una ciudad ribereña. */
  crucesRio: Map<number, Point>;
  /** Segmentos de río por celda de la rejilla, para cortar tramos contra los cauces sin recorrerlos todos. */
  riosPorCelda: Map<number, SegmentoRio[]>;
  separacionVados: number;
  radioPasoCiudad: number;
}

export interface Tramo {
  /** `Infinity` = no se puede recorrer. */
  coste: number;
  /** Cruce de río fuera de vado, si lo hay. */
  cruceRio?: Point;
}

export function posicionNodo(grafo: GrafoNavegacion, nodo: number): Point {
  return { x: (nodo % grafo.cols) * grafo.espaciado, y: Math.floor(nodo / grafo.cols) * grafo.espaciado };
}

/** Clave canónica de una arista entre dos puntos, sin orden. Es la que guarda la red de caminos. */
export function claveArista(a: Point, b: Point): string {
  const ka = `${Math.round(a.x)},${Math.round(a.y)}`;
  const kb = `${Math.round(b.x)},${Math.round(b.y)}`;
  return ka < kb ? `${ka};${kb}` : `${kb};${ka}`;
}

/** Los dos extremos de una `claveArista`. */
export function extremosDeArista(clave: string): [Point, Point] {
  const [a, b] = clave.split(';').map((k) => {
    const [x, y] = k.split(',').map(Number);
    return { x: x!, y: y! };
  });
  return [a!, b!];
}

/** ¿Se cruza el río por aquí? `s` = distancia desde el nacimiento a lo largo del cauce. El primer medio intervalo
 * es arroyo (se cruza por cualquier sitio) y luego hay un vado cada `separacion`, de `semiancho` a cada lado. */
export function hayVado(s: number, separacion: number, semiancho: number): boolean {
  return s <= separacion / 2 || Math.abs(s - Math.round(s / separacion) * separacion) <= semiancho;
}

function indexarRios(rios: readonly RioZona[], lado: number, cols: number): Map<number, SegmentoRio[]> {
  const celdas = new Map<number, SegmentoRio[]>();
  for (const rio of rios) {
    let s0 = 0;
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const a = rio.puntos[i]!;
      const b = rio.puntos[i + 1]!;
      const seg = { a, b, s0 };
      for (let col = Math.floor(Math.min(a.x, b.x) / lado); col <= Math.floor(Math.max(a.x, b.x) / lado); col++) {
        for (let fila = Math.floor(Math.min(a.y, b.y) / lado); fila <= Math.floor(Math.max(a.y, b.y) / lado); fila++) {
          const clave = fila * (cols + 1) + col;
          const lista = celdas.get(clave);
          if (lista) lista.push(seg);
          else celdas.set(clave, [seg]);
        }
      }
      s0 += distancia(a, b);
    }
  }
  return celdas;
}

/** Primer cruce (desde `p`) del segmento p→q con un cauce fuera de vado, o `undefined`. */
function cruceSinVado(grafo: GrafoNavegacion, p: Point, q: Point): Point | undefined {
  if (grafo.riosPorCelda.size === 0) return undefined;
  const lado = grafo.espaciado;
  let mejorT = Infinity;
  let cruce: Point | undefined;
  const rx = q.x - p.x;
  const ry = q.y - p.y;
  for (let col = Math.floor(Math.min(p.x, q.x) / lado); col <= Math.floor(Math.max(p.x, q.x) / lado); col++) {
    for (let fila = Math.floor(Math.min(p.y, q.y) / lado); fila <= Math.floor(Math.max(p.y, q.y) / lado); fila++) {
      for (const seg of grafo.riosPorCelda.get(fila * (grafo.cols + 1) + col) ?? []) {
        const sx = seg.b.x - seg.a.x;
        const sy = seg.b.y - seg.a.y;
        const den = rx * sy - ry * sx;
        if (den === 0) continue;
        const qpx = seg.a.x - p.x;
        const qpy = seg.a.y - p.y;
        const t = (qpx * sy - qpy * sx) / den;
        const u = (qpx * ry - qpy * rx) / den;
        if (t < 0 || t > 1 || u < 0 || u > 1 || t >= mejorT) continue;
        if (hayVado(seg.s0 + u * Math.hypot(sx, sy), grafo.separacionVados, grafo.espaciado)) continue;
        mejorT = t;
        cruce = { x: p.x + rx * t, y: p.y + ry * t };
      }
    }
  }
  return cruce;
}

/** Coste de recorrer el segmento recto a→b: muestrea el agua en los extremos y cada `paso`, e integra el coste
 * de terreno en el punto medio de cada trocito. */
export function evaluarTramo(mapa: Mapa, grafo: GrafoNavegacion, a: Point, b: Point): Tramo {
  const largo = distancia(a, b);
  const n = Math.max(1, Math.ceil(largo / grafo.paso));
  const en = (t: number): Point => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
  for (let k = 0; k <= n; k++) if (!mapa.esTransitable(en(k / n))) return { coste: Infinity };
  let suma = 0;
  for (let k = 0; k < n; k++) suma += mapa.costeEnPunto(en((k + 0.5) / n));
  return { coste: (largo / n) * suma, cruceRio: cruceSinVado(grafo, a, b) };
}

function construir(mapa: Mapa): GrafoNavegacion {
  const { ancho, alto } = mapa.limites;
  const ladoMayor = Math.max(ancho, alto);
  const espaciado = Math.max(NAVEGACION.espaciadoMin, Math.round(ladoMayor / NAVEGACION.nodosPorLado));
  const cols = Math.floor(ancho / espaciado) + 1;
  const filas = Math.floor(alto / espaciado) + 1;
  const grafo: GrafoNavegacion = {
    espaciado,
    cols,
    filas,
    paso: Math.min(NAVEGACION.muestreoMax, espaciado / 3),
    coste: new Float64Array(cols * filas * 8).fill(Infinity),
    crucesRio: new Map(),
    riosPorCelda: indexarRios(mapa.listarRios(), espaciado, cols),
    separacionVados: Math.max(NAVEGACION.separacionVadosMin, ladoMayor / NAVEGACION.vadosPorLado),
    radioPasoCiudad: Math.max(NAVEGACION.radioPasoCiudadMin, espaciado * 1.5),
  };
  for (let fila = 0; fila < filas; fila++) {
    for (let col = 0; col < cols; col++) {
      const nodo = fila * cols + col;
      // Una dirección de cada pareja; la opuesta (d ^ 1) se rellena con el mismo tramo.
      for (const d of [0, 2, 4, 6]) {
        const [dc, df] = VECINOS[d]!;
        const c2 = col + dc;
        const f2 = fila + df;
        if (c2 < 0 || c2 >= cols || f2 < 0 || f2 >= filas) continue;
        const vecino = f2 * cols + c2;
        const tramo = evaluarTramo(mapa, grafo, posicionNodo(grafo, nodo), posicionNodo(grafo, vecino));
        grafo.coste[nodo * 8 + d] = tramo.coste;
        grafo.coste[vecino * 8 + (d ^ 1)] = tramo.coste;
        if (tramo.cruceRio && Number.isFinite(tramo.coste)) {
          grafo.crucesRio.set(nodo * 8 + d, tramo.cruceRio);
          grafo.crucesRio.set(vecino * 8 + (d ^ 1), tramo.cruceRio);
        }
      }
    }
  }
  return grafo;
}

/** Cacheado por mundo (`Mapa.mundo`): todas las fachadas de una partida comparten el mismo grafo. */
const grafosPorMundo = new WeakMap<object, GrafoNavegacion>();

export function grafoDe(mapa: Mapa): GrafoNavegacion {
  let grafo = grafosPorMundo.get(mapa.mundo);
  if (!grafo) {
    grafo = construir(mapa);
    grafosPorMundo.set(mapa.mundo, grafo);
  }
  return grafo;
}
