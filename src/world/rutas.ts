// Pathfinding sobre el mapa: una sola función para caravanas, ejércitos y fundaciones. A* sobre el grafo de
// navegación global (`world/grafoNavegacion.ts`), que ya trae integrado el coste de relieve y bosque y sabe qué
// aristas cortan el agua o un río fuera de vado. Vive en `world/` (no en `worldgen/`) porque necesita la
// fachada `Mapa`.
//
// Solo se guarda el resultado (una polilínea): el origen, los nodos del grafo recorridos y el destino. Cada par
// de puntos consecutivos es una arista de la red de caminos (`claveArista`, `engine/redCaminos.ts`).

import type { Point } from '../domain/types';
import { NAVEGACION } from '../worldgen/config';
import { claveArista, evaluarTramo, grafoDe, posicionNodo, VECINOS, type GrafoNavegacion } from './grafoNavegacion';
import { distancia } from './geometria';
import type { Mapa } from './mapa';

export interface OpcionesRuta {
  /** Peso de cada arista de la red de caminos (`pesosDeRed`, `engine/redCaminos.ts`): abarata las aristas con
   * peso, y así el trazado se desvía hacia los caminos principales (Doc 1.6, decisión 5). */
  pesos?: ReadonlyMap<string, number>;
  /** Posiciones de los asentamientos: un río se cruza también por una ciudad ribereña (decisión 2). */
  pasosRio?: readonly Point[];
}

/** Multiplicador de coste de una arista con `peso` rutas (decisión 5). */
export function factorAtraccion(peso: number): number {
  return peso > 0 ? Math.max(NAVEGACION.atraccionMin, 1 - NAVEGACION.atraccionPorPeso * peso) : 1;
}

/** Heap binario mínimo de (prioridad, nodo) en arrays paralelos. */
class ColaPrioridad {
  private readonly prioridades: number[] = [];
  private readonly nodos: number[] = [];

  get vacia(): boolean {
    return this.nodos.length === 0;
  }

  insertar(prioridad: number, nodo: number): void {
    const p = this.prioridades;
    const n = this.nodos;
    p.push(prioridad);
    n.push(nodo);
    let i = n.length - 1;
    while (i > 0) {
      const padre = (i - 1) >> 1;
      if (p[padre]! <= p[i]!) break;
      [p[padre], p[i]] = [p[i]!, p[padre]!];
      [n[padre], n[i]] = [n[i]!, n[padre]!];
      i = padre;
    }
  }

  extraerMinimo(): number {
    const p = this.prioridades;
    const n = this.nodos;
    const raiz = n[0]!;
    const ultimaP = p.pop()!;
    const ultimoN = n.pop()!;
    if (n.length > 0) {
      p[0] = ultimaP;
      n[0] = ultimoN;
      let i = 0;
      for (;;) {
        const izq = i * 2 + 1;
        const der = izq + 1;
        let menor = i;
        if (izq < n.length && p[izq]! < p[menor]!) menor = izq;
        if (der < n.length && p[der]! < p[menor]!) menor = der;
        if (menor === i) break;
        [p[menor], p[i]] = [p[i]!, p[menor]!];
        [n[menor], n[i]] = [n[i]!, n[menor]!];
        i = menor;
      }
    }
    return raiz;
  }
}

function crucePermitido(grafo: GrafoNavegacion, cruce: Point | undefined, pasosRio: readonly Point[]): boolean {
  return !cruce || pasosRio.some((c) => distancia(c, cruce) <= grafo.radioPasoCiudad);
}

/** Nodos a los que se engancha un punto suelto (origen o destino): los 4×4 de su alrededor a los que se llega
 * en recta sin mojarse, con el coste de ese enganche. */
function enganches(mapa: Mapa, grafo: GrafoNavegacion, p: Point, pasosRio: readonly Point[], haciaElNodo: boolean): Map<number, number> {
  const resultado = new Map<number, number>();
  const col0 = Math.floor(p.x / grafo.espaciado);
  const fila0 = Math.floor(p.y / grafo.espaciado);
  for (let fila = fila0 - 1; fila <= fila0 + 2; fila++) {
    for (let col = col0 - 1; col <= col0 + 2; col++) {
      if (col < 0 || col >= grafo.cols || fila < 0 || fila >= grafo.filas) continue;
      const nodo = fila * grafo.cols + col;
      const q = posicionNodo(grafo, nodo);
      const tramo = haciaElNodo ? evaluarTramo(mapa, grafo, p, q) : evaluarTramo(mapa, grafo, q, p);
      if (Number.isFinite(tramo.coste) && crucePermitido(grafo, tramo.cruceRio, pasosRio)) resultado.set(nodo, tramo.coste);
    }
  }
  return resultado;
}

/**
 * Ruta de coste mínimo entre dos puntos sobre el grafo de navegación: rodea el agua y los ríos (que se cruzan por
 * un vado o por una ciudad ribereña), evita el terreno caro y el bosque, y con `opciones.pesos` se arrima a los
 * caminos ya transitados. Determinista dados el mundo, los puntos y las opciones.
 *
 * Devuelve `[origen, ...nodos, destino]`, o **`undefined` si no hay camino por tierra**: origen o destino sobre
 * agua, o ninguna ruta que los una. Cada llamador decide qué significa eso (rechazar la movilización, no
 * despachar la caravana); ninguno puede seguir como si nada, porque una recta de reserva cruzaría el agua.
 */
export function calcularRuta(mapa: Mapa, origen: Point, destino: Point, opciones: OpcionesRuta = {}): Point[] | undefined {
  if (!mapa.esTransitable(origen) || !mapa.esTransitable(destino)) return undefined;
  const grafo = grafoDe(mapa);
  const pasosRio = opciones.pasosRio ?? [];

  if (distancia(origen, destino) < grafo.espaciado) {
    const directo = evaluarTramo(mapa, grafo, origen, destino);
    return Number.isFinite(directo.coste) && crucePermitido(grafo, directo.cruceRio, pasosRio) ? [origen, destino] : undefined;
  }

  const salidas = enganches(mapa, grafo, origen, pasosRio, true);
  const llegadas = enganches(mapa, grafo, destino, pasosRio, false);
  if (salidas.size === 0 || llegadas.size === 0) return undefined;

  const total = grafo.cols * grafo.filas;
  const META = total;
  const ORIGEN = -2;
  const g = new Float64Array(total + 1).fill(Infinity);
  const previo = new Int32Array(total + 1).fill(-1);
  const cerrado = new Uint8Array(total + 1);
  const abiertos = new ColaPrioridad();
  const pesos = opciones.pesos && opciones.pesos.size > 0 ? opciones.pesos : undefined;
  // Heurística admisible: nunca sobreestima — con atracción una arista puede costar hasta `atraccionMin` por unidad.
  const costeMinimo = pesos ? NAVEGACION.atraccionMin : 1;
  const h = (nodo: number) => distancia(posicionNodo(grafo, nodo), destino) * costeMinimo;

  for (const [nodo, coste] of salidas) {
    if (coste < g[nodo]!) {
      g[nodo] = coste;
      previo[nodo] = ORIGEN;
      abiertos.insertar(coste + h(nodo), nodo);
    }
  }

  while (!abiertos.vacia) {
    const u = abiertos.extraerMinimo();
    if (cerrado[u]) continue;
    cerrado[u] = 1;
    if (u === META) break;

    const llegada = llegadas.get(u);
    if (llegada !== undefined && g[u]! + llegada < g[META]!) {
      g[META] = g[u]! + llegada;
      previo[META] = u;
      abiertos.insertar(g[META]!, META);
    }

    const col = u % grafo.cols;
    const fila = Math.floor(u / grafo.cols);
    for (let d = 0; d < 8; d++) {
      let coste = grafo.coste[u * 8 + d]!;
      if (!Number.isFinite(coste)) continue;
      const v = (fila + VECINOS[d]![1]) * grafo.cols + col + VECINOS[d]![0];
      if (cerrado[v]) continue;
      if (!crucePermitido(grafo, grafo.crucesRio.get(u * 8 + d), pasosRio)) continue;
      if (pesos) coste *= factorAtraccion(pesos.get(claveArista(posicionNodo(grafo, u), posicionNodo(grafo, v))) ?? 0);
      const nuevo = g[u]! + coste;
      if (nuevo < g[v]!) {
        g[v] = nuevo;
        previo[v] = u;
        abiertos.insertar(nuevo + h(v), v);
      }
    }
  }

  if (!cerrado[META]) return undefined;
  const puntos: Point[] = [destino];
  for (let nodo = previo[META]!; nodo !== ORIGEN; nodo = previo[nodo]!) puntos.push(posicionNodo(grafo, nodo));
  puntos.push(origen);
  return puntos.reverse();
}
