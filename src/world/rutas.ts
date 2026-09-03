// Pathfinding sobre el mapa (Fase 0.3): una sola función, reutilizada por caminos comerciales
// (`engine/caminos.ts`) y por el lanzamiento de cualquier caravana (`engine/movimiento.ts`). Vive en
// `world/` (no en `worldgen/`) porque necesita la fachada `Mapa` — `worldgen/` no puede depender de ella
// sin crear un ciclo (`world/mapa.ts` ya depende de `worldgen/`).
//
// La malla de muestreo es un cálculo INTERNO y efímero de esta función: nunca se guarda en `MapaGenerado`
// ni en el estado de partida, solo el resultado (una polilínea de puntos) — coherente con la decisión de
// arquitectura de `Fase_0_1_Definicion.md` ("nunca rejilla horneada ni tiles"). Se ejecuta una vez por ruta
// (al crear un camino comercial o lanzar una caravana), nunca dentro del bucle de un tick.

import type { Point } from '../domain/types';
import type { Mapa } from './mapa';
import { distancia } from './geometria';

/** Separación de la malla de muestreo, en unidades de mapa. Menor = ruta más fiel al terreno pero más nodos
 * que explorar; 45 es un punto medio razonable frente a formaciones de relieve de ~150-600 unidades
 * (ver `ELEVACION` en `worldgen/config.ts`). */
const ESPACIADO_MALLA = 45;

/** Coste mínimo posible por unidad de distancia (ver `COSTE_MOVIMIENTO.llano`, siempre el más barato) —
 * usado como heurística admisible de A* (nunca sobreestima el coste real restante). */
const COSTE_MINIMO_POR_UNIDAD = 1;

/** ¿El segmento recto entre dos puntos se mantiene en tierra? Muestrea a `ESPACIADO_MALLA/3`, suficiente
 * para el único caso que lo usa: tramos más cortos que una celda de la malla. */
function tramoTransitable(mapa: Mapa, a: Point, b: Point): boolean {
  const pasos = Math.max(1, Math.ceil(distancia(a, b) / (ESPACIADO_MALLA / 3)));
  for (let i = 0; i <= pasos; i++) {
    const t = i / pasos;
    if (!mapa.esTransitable({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t })) return false;
  }
  return true;
}

/** Vecinos en 8 direcciones de la malla (col, fila). */
const VECINOS: readonly [number, number][] = [
  [1, 0], [-1, 0], [0, 1], [0, -1],
  [1, 1], [1, -1], [-1, 1], [-1, -1],
];

interface NodoAEstrella {
  clave: string;
  col: number;
  fila: number;
  g: number;
  f: number;
  desde: string | null;
}

/** Heap binario mínimo por `f`, suficiente para las ~1-2 mil aristas típicas de una ruta — sin dependencias
 * externas, mismo criterio que el resto del proyecto. */
class ColaPrioridad {
  private items: NodoAEstrella[] = [];

  get vacia(): boolean {
    return this.items.length === 0;
  }

  insertar(nodo: NodoAEstrella): void {
    this.items.push(nodo);
    let i = this.items.length - 1;
    while (i > 0) {
      const padre = (i - 1) >> 1;
      if (this.items[padre]!.f <= this.items[i]!.f) break;
      [this.items[padre], this.items[i]] = [this.items[i]!, this.items[padre]!];
      i = padre;
    }
  }

  extraerMinimo(): NodoAEstrella {
    const raiz = this.items[0]!;
    const ultimo = this.items.pop()!;
    if (this.items.length > 0) {
      this.items[0] = ultimo;
      let i = 0;
      for (;;) {
        const izq = i * 2 + 1;
        const der = i * 2 + 2;
        let menor = i;
        if (izq < this.items.length && this.items[izq]!.f < this.items[menor]!.f) menor = izq;
        if (der < this.items.length && this.items[der]!.f < this.items[menor]!.f) menor = der;
        if (menor === i) break;
        [this.items[menor], this.items[i]] = [this.items[i]!, this.items[menor]!];
        i = menor;
      }
    }
    return raiz;
  }
}

/**
 * Ruta de coste mínimo entre dos puntos, evitando terreno costoso (`Mapa.costeEnPunto`) y **rodeando el
 * agua, que es infranqueable** (`Mapa.esTransitable`, a petición del usuario 2026-09-02). A* sobre una malla
 * 8-conexa muestreada solo dentro de la caja origen-destino (con margen) — nunca sobre el mapa completo.
 * Determinista dados `origen`/`destino`/el mundo, pero NO forma parte del contrato de generación (no consume
 * el PRNG, no depende de la seed más que a través del propio `Mapa`).
 *
 * Devuelve `[origen, ..., destino]`, o **`undefined` si no hay camino por tierra**: origen o destino sobre
 * agua, o ninguna ruta que los una sin cruzarla (una isla, una península cortada).
 *
 * **Antes caía a la línea recta cuando no encontraba camino**, y eso ya no vale: con el agua infranqueable,
 * una recta de reserva sería precisamente una ruta a través del mar. Cada llamador decide qué significa "no
 * hay ruta" — rechazar la movilización, no despachar la caravana, no trazar el camino comercial— pero
 * ninguno puede seguir adelante como si nada.
 */
export function calcularRuta(mapa: Mapa, origen: Point, destino: Point): Point[] | undefined {
  // Ni el punto de partida ni el de llegada pueden estar en el agua, se busque la ruta que se busque.
  if (!mapa.esTransitable(origen) || !mapa.esTransitable(destino)) return undefined;

  const distanciaDirecta = distancia(origen, destino);
  if (distanciaDirecta < ESPACIADO_MALLA) {
    // Demasiado cerca para mallar, pero el tramo recto todavía puede cruzar una lengua de agua: se comprueba
    // muestreando, que es más barato que montar la malla para tan poca distancia.
    return tramoTransitable(mapa, origen, destino) ? [origen, destino] : undefined;
  }

  // Primero la caja ajustada de siempre, que es la barata y resuelve el caso normal.
  const ajustada = buscarEnCaja(mapa, origen, destino, Math.max(200, distanciaDirecta * 0.15));
  if (ajustada) return ajustada;

  // Y si ahí no aparece, se reintenta sobre el MAPA ENTERO antes de rendirse.
  //
  // Hace falta desde que el agua es infranqueable: un rodeo legítimo puede salirse de la caja ajustada —una
  // bahía que obliga a subir mucho más al norte que el margen— y antes daba igual, porque no encontrar
  // camino caía a la línea recta. Ahora "no encontrado" significa rechazar el viaje, así que hay que estar
  // seguro de que de verdad no lo hay. Es barato: a `ESPACIADO_MALLA` = 45, un mapa de 2000×2000 son ~1.900
  // celdas, y solo se paga cuando la búsqueda ajustada ya ha fallado.
  const { ancho, alto } = mapa.limites;
  return buscarEnCaja(mapa, origen, destino, Math.max(ancho, alto));
}

/** A* sobre la malla muestreada en la caja origen-destino ampliada por `margen`. `undefined` si no hay
 * camino POR TIERRA dentro de esa caja — no distingue "no existe" de "no cabe en la caja"; eso lo resuelve
 * `calcularRuta` reintentando sobre el mapa entero. */
function buscarEnCaja(mapa: Mapa, origen: Point, destino: Point, margen: number): Point[] | undefined {
  const { ancho, alto } = mapa.limites;
  const minX = Math.max(0, Math.min(origen.x, destino.x) - margen);
  const minY = Math.max(0, Math.min(origen.y, destino.y) - margen);
  const maxX = Math.min(ancho, Math.max(origen.x, destino.x) + margen);
  const maxY = Math.min(alto, Math.max(origen.y, destino.y) + margen);

  const colFilaAPunto = (col: number, fila: number): Point => ({ x: minX + col * ESPACIADO_MALLA, y: minY + fila * ESPACIADO_MALLA });
  const puntoAColFila = (p: Point): [number, number] => [
    Math.round((p.x - minX) / ESPACIADO_MALLA),
    Math.round((p.y - minY) / ESPACIADO_MALLA),
  ];

  const colMax = Math.floor((maxX - minX) / ESPACIADO_MALLA);
  const filaMax = Math.floor((maxY - minY) / ESPACIADO_MALLA);
  const dentroDeLaMalla = (col: number, fila: number) => col >= 0 && col <= colMax && fila >= 0 && fila <= filaMax;

  const [colOrigen, filaOrigen] = puntoAColFila(origen);
  const [colDestino, filaDestino] = puntoAColFila(destino);
  const claveDestino = `${colDestino},${filaDestino}`;

  const costeCache = new Map<string, number>();
  const costeDeCelda = (col: number, fila: number): number => {
    const clave = `${col},${fila}`;
    let c = costeCache.get(clave);
    if (c === undefined) {
      c = mapa.costeEnPunto(colFilaAPunto(col, fila));
      costeCache.set(clave, c);
    }
    return c;
  };

  const transitableCache = new Map<string, boolean>();
  const transitableDeCelda = (col: number, fila: number): boolean => {
    const clave = `${col},${fila}`;
    let t = transitableCache.get(clave);
    if (t === undefined) {
      t = mapa.esTransitable(colFilaAPunto(col, fila));
      transitableCache.set(clave, t);
    }
    return t;
  };

  const abiertos = new ColaPrioridad();
  const mejorG = new Map<string, number>();
  const cameFrom = new Map<string, string | null>();
  const posiciones = new Map<string, [number, number]>();

  const claveOrigen = `${colOrigen},${filaOrigen}`;
  mejorG.set(claveOrigen, 0);
  posiciones.set(claveOrigen, [colOrigen, filaOrigen]);
  abiertos.insertar({
    clave: claveOrigen,
    col: colOrigen,
    fila: filaOrigen,
    g: 0,
    f: distancia(origen, destino) * COSTE_MINIMO_POR_UNIDAD,
    desde: null,
  });

  let encontrado = false;
  const visitados = new Set<string>();

  while (!abiertos.vacia) {
    const actual = abiertos.extraerMinimo();
    if (visitados.has(actual.clave)) continue;
    visitados.add(actual.clave);
    cameFrom.set(actual.clave, actual.desde);

    if (actual.clave === claveDestino) {
      encontrado = true;
      break;
    }

    const costeActual = costeDeCelda(actual.col, actual.fila);
    for (const [dCol, dFila] of VECINOS) {
      const col = actual.col + dCol;
      const fila = actual.fila + dFila;
      if (!dentroDeLaMalla(col, fila)) continue;
      const clave = `${col},${fila}`;
      if (visitados.has(clave)) continue;
      // El agua no se cruza: la celda no entra en la frontera, en vez de entrar con coste alto.
      if (!transitableDeCelda(col, fila)) continue;

      const distanciaPaso = Math.hypot(dCol, dFila) * ESPACIADO_MALLA;
      const costeVecino = costeDeCelda(col, fila);
      const g = actual.g + distanciaPaso * ((costeActual + costeVecino) / 2);

      if (g < (mejorG.get(clave) ?? Infinity)) {
        mejorG.set(clave, g);
        posiciones.set(clave, [col, fila]);
        const h = distancia(colFilaAPunto(col, fila), destino) * COSTE_MINIMO_POR_UNIDAD;
        abiertos.insertar({ clave, col, fila, g, f: g + h, desde: actual.clave });
      }
    }
  }

  // Sin camino por tierra. NO se cae a la recta: sería una ruta por el agua (ver cabecera).
  if (!encontrado) return undefined;

  const puntos: Point[] = [];
  let claveActual: string | null = claveDestino;
  while (claveActual !== null) {
    const [col, fila] = posiciones.get(claveActual)!;
    puntos.push(colFilaAPunto(col, fila));
    claveActual = cameFrom.get(claveActual) ?? null;
  }
  puntos.reverse();
  puntos[0] = origen;
  puntos[puntos.length - 1] = destino;
  return puntos;
}
