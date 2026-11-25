import type { LagoZona, Point, RioZona } from './tipos';
import { distancia } from './geometria';
import type { RandomFn } from './rng';
import { puntoEnLago } from './aguaRegional';
import type { Desague } from './rejillaFluvial';

/** Una hoya rellena se tiene por lago si tiene al menos esta hondura (elevación normalizada: 0,004 = 4 m) y estas celdas. */
export const LAGO_POR_RELLENO = { hondura: 0.004, celdas: 4, minimo: 1e-5 };
const VECINOS: readonly [number, number][] = [[-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1]];

/** La rejilla sobre la que se drena (la de `rejillaFluvial.ts`, ya erosionada): alturas, agua, montaña y su desagüe. */
export interface MallaDrenaje {
  columnas: number; filas: number; celda: number;
  altura: Float64Array; agua: Uint8Array; montana: Uint8Array;
  desague: Desague;
}

/** Un río tal como sale del drenaje: sus celdas, de nacimiento a fin. */
export interface Traza { celdas: number[]; fin: RioZona['fin'] }

const centro = (m: MallaDrenaje, k: number): Point => ({ x: ((k % m.columnas) + 0.5) * m.celda, y: (Math.floor(k / m.columnas) + 0.5) * m.celda });

/** Hasta cuántas celdas se mira si hay otro río ya trazado al lado, para juntarse con él. */
export const UNION_DE_RIOS = 2;

/** El río desde `origen` por los receptores. En el fondo casi llano de un valle, los receptores dejan corrientes paralelas que no se
 * encuentran nunca; un río que llega a `UNION_DE_RIOS` celdas de otro ya trazado (`ocupada`), más bajo y sin loma en medio, se
 * junta con él: desde ahí sigue su cauce (confluyen). */
function trazar(m: MallaDrenaje, origen: number, ocupada: Uint8Array): Traza {
  const celdas = [origen], lleno = m.desague.lleno, R = UNION_DE_RIOS;
  let actual = origen;
  for (let paso = 0; paso < m.altura.length; paso++) {
    if (m.agua[actual]) return { celdas, fin: 'agua' };
    if (!ocupada[actual]) {
      const x = actual % m.columnas, y = (actual - x) / m.columnas;
      let union = -1;
      for (let dy = -R; dy <= R; dy++) for (let dx = -R; dx <= R; dx++) {
        const nx = x + dx, ny = y + dy, j = ny * m.columnas + nx;
        if (nx < 0 || ny < 0 || nx >= m.columnas || ny >= m.filas || !ocupada[j] || lleno[j]! >= lleno[actual]!) continue;
        // sin loma en medio: la celda de en medio no queda por encima de la de partida
        const medio = Math.round(y + dy / 2) * m.columnas + Math.round(x + dx / 2);
        if (lleno[medio]! > lleno[actual]! + 1e-9 && medio !== j) continue;
        if (union < 0 || lleno[j]! < lleno[union]!) union = j;
      }
      if (union >= 0) { actual = union; celdas.push(actual); continue; }
    }
    const r = m.desague.receptor[actual]!;
    if (r < 0) return { celdas, fin: 'borde' };
    actual = r;
    celdas.push(actual);
  }
  throw new Error('Drenaje excede el número de celdas: posible ciclo');
}

/** Cauces por drenaje sobre la rejilla: todos llegan al mar o al borde. Nacen en la montaña, de la cima hacia abajo, separados
 * `espacioNacimientos`; los `forzados` (los troncales) nacen siempre. Donde atraviesan una hoya honda, `lago` decide si es un lago (con su
 * orilla) y el río se parte: entra en él hasta su orilla y vuelve a salir por el desagüe. Si `lago` lo rechaza, el río cruza la hoya. */
export function generarRiosPorDrenaje(rng: RandomFn, m: MallaDrenaje,
  opciones: { cantidad: number; largoMinimo: number; espacioNacimientos: number; forzados: readonly number[];
    lago: (centro: Point, nivel: number) => LagoZona | undefined }): { trazas: Traza[]; lagos: LagoZona[] } {
  const n = m.columnas * m.filas, lleno = m.desague.lleno;
  const candidatos = [...opciones.forzados.map(i => ({ i, forzado: true, desempate: 0 })),
    ...Array.from({ length: n }, (_, i) => ({ i, forzado: false, desempate: rng() }))
      .filter(({ i }) => m.montana[i] && !m.agua[i])
      .sort((a, b) => m.altura[b.i]! - m.altura[a.i]! || a.desempate - b.desempate)];
  // Las hoyas: componentes de celdas rellenas; por cada una, si es lago (y cuál) o no.
  const hoya = new Int32Array(n).fill(-1);
  const lagoDeHoya: (LagoZona | null)[] = [];
  const hoyaDe = (i: number): number => {
    if (lleno[i]! - m.altura[i]! <= LAGO_POR_RELLENO.minimo) return -1;
    if (hoya[i]! >= 0) return hoya[i]!;
    const id = lagoDeHoya.length, pila = [i], celdas: number[] = [];
    hoya[i] = id;
    while (pila.length) {
      const c = pila.pop()!, x = c % m.columnas, y = Math.floor(c / m.columnas);
      celdas.push(c);
      for (const [dx, dy] of VECINOS) {
        const nx = x + dx, ny = y + dy, j = ny * m.columnas + nx;
        if (nx < 0 || nx >= m.columnas || ny < 0 || ny >= m.filas || hoya[j]! >= 0 || lleno[j]! - m.altura[j]! <= LAGO_POR_RELLENO.minimo) continue;
        hoya[j] = id; pila.push(j);
      }
    }
    const fondo = celdas.reduce((a, c) => (m.altura[c]! < m.altura[a]! ? c : a), celdas[0]!);
    const hondura = Math.max(...celdas.map((c) => lleno[c]! - m.altura[c]!));
    lagoDeHoya.push(hondura >= LAGO_POR_RELLENO.hondura && celdas.length >= LAGO_POR_RELLENO.celdas
      ? opciones.lago(centro(m, fondo), m.altura[fondo]! + hondura) ?? null : null);
    return id;
  };
  const trazas: Traza[] = [], lagos: LagoZona[] = [], nacimientos: Point[] = [], ocupada = new Uint8Array(n);
  let libres = 0;
  for (const { i, forzado } of candidatos) {
    if (!forzado && libres >= opciones.cantidad) break;
    const p = centro(m, i);
    if (!forzado && nacimientos.some((q) => distancia(q, p) < opciones.espacioNacimientos)) continue;
    const traza = trazar(m, i, ocupada);
    if (traza.celdas.length < 2 || (!forzado && traza.celdas.length * m.celda < opciones.largoMinimo)) continue;
    for (const k of traza.celdas) ocupada[k] = 1;
    nacimientos.push(p);
    if (!forzado) libres++;
    // Dentro de la hoya, el río baja hasta la orilla del lago y acaba ahí; lo que queda de hoya (del lago al desagüe) no lleva río.
    let trozo: number[] = [], mojado = false;
    const cerrar = (fin: RioZona['fin']) => { if (trozo.length >= 2) trazas.push({ celdas: trozo, fin }); trozo = []; };
    for (const j of traza.celdas) {
      const h = hoyaDe(j), lago = h >= 0 ? lagoDeHoya[h] : null;
      if (!lago) { mojado = false; trozo.push(j); continue; }
      if (!lagos.includes(lago)) lagos.push(lago);
      if (mojado) continue;
      if (puntoEnLago(lago, centro(m, j))) { mojado = true; if (trozo.length) trozo.push(j); cerrar('lago'); }
      else if (trozo.length) trozo.push(j);
    }
    cerrar(traza.fin);
  }
  return { trazas, lagos };
}
