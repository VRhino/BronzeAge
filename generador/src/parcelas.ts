import type { Point } from './tipos';
import { anilloEn, FACTOR, type Anillo, type Campamento } from './campamentos';

/**
 * Masas explotables de bosque y sus parcelas de explotación (Doc 1.9c, decidido el 2026-10-13).
 * El bosque regional es un campo continuo de densidad: una masa es una componente conexa de celdas con densidad suficiente,
 * y una parcela es un cupo (punto fijo) de la masa. La Leñera se construye en la ciudad; la parcela solo dice de qué parte
 * del bosque vive. Las parcelas están separadas al menos `2·rp`, así que no se solapan por construcción.
 */
export const PARAMETROS_PARCELAS = {
  /** Radio de una parcela (estratégicas). */
  rp: 4,
  /** Lado de la celda del raster de densidad (estratégicas). */
  celda: 4,
  /** Densidad mínima para contar como bosque. */
  densidadMinima: 0.15,
  /** Una masa de menos celdas que esto se descarta (no cabe ni una parcela). */
  celdasMinimas: 3,
  /** Un bosque continuo mayor que esto se parte en comarcas (radio equivalente ~78): las masas son «pocos grandes», no un continente. */
  celdasMaximas: 1200,
};

export interface MasaBosque { id: string; centro: Point; radio: number; densidad: number; area: number; anillo: Anillo; parcelasIds: string[] }
export interface ParcelaBosque { id: string; masaId: string; centro: Point; densidad: number; anillo: Anillo }

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

export function generarParcelas(
  limites: { ancho: number; alto: number },
  densidadEn: (p: Point) => number,
  campamentos: readonly Campamento[],
  prm = PARAMETROS_PARCELAS,
): { masas: MasaBosque[]; parcelas: ParcelaBosque[] } {
  const lado = prm.celda * FACTOR, nx = Math.ceil(limites.ancho / lado), ny = Math.ceil(limites.alto / lado);
  const centroDe = (i: number): Point => ({ x: ((i % nx) + 0.5) * lado, y: (Math.floor(i / nx) + 0.5) * lado });
  const dens = new Float32Array(nx * ny);
  for (let i = 0; i < dens.length; i++) dens[i] = densidadEn(centroDe(i));

  // Componentes conexas (4 vecinos) de celdas con bosque.
  const masaDe = new Int32Array(nx * ny).fill(-1), celdas: number[][] = [];
  for (let i = 0; i < dens.length; i++) {
    if (masaDe[i]! >= 0 || dens[i]! < prm.densidadMinima) continue;
    const id = celdas.length, pila = [i], lista: number[] = [];
    masaDe[i] = id;
    while (pila.length) {
      const c = pila.pop()!;
      lista.push(c);
      const x = c % nx, y = Math.floor(c / nx);
      for (const v of [x > 0 ? c - 1 : -1, x < nx - 1 ? c + 1 : -1, y > 0 ? c - nx : -1, y < ny - 1 ? c + nx : -1]) {
        if (v >= 0 && masaDe[v]! < 0 && dens[v]! >= prm.densidadMinima) { masaDe[v] = id; pila.push(v); }
      }
    }
    celdas.push(lista);
  }

  // Los bosques continentales se parten en comarcas por bisección de coordenadas y cada trozo se vuelve a separar en componentes.
  const trozos = (lista: number[]): number[][] => {
    if (lista.length <= prm.celdasMaximas) return [lista];
    let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
    for (const c of lista) { const x = c % nx, y = Math.floor(c / nx); x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
    const clave = (c: number) => (x1 - x0 >= y1 - y0 ? c % nx : Math.floor(c / nx)) * nx * ny + c;
    const orden = [...lista].sort((a, b) => clave(a) - clave(b)), mitad = orden.length >> 1;
    return [...trozos(orden.slice(0, mitad)), ...trozos(orden.slice(mitad))];
  };
  const comarcas: number[][] = [];
  for (const comp of celdas) {
    for (const trozo of trozos(comp)) {
      if (trozo === comp) { comarcas.push(trozo); continue; }
      const dentro = new Set(trozo), visto = new Set<number>();
      for (const inicio of trozo) {
        if (visto.has(inicio)) continue;
        const pila = [inicio], lista: number[] = [];
        visto.add(inicio);
        while (pila.length) {
          const c = pila.pop()!;
          lista.push(c);
          const x = c % nx;
          for (const v of [x > 0 ? c - 1 : -1, x < nx - 1 ? c + 1 : -1, c - nx, c + nx]) if (dentro.has(v) && !visto.has(v)) { visto.add(v); pila.push(v); }
        }
        comarcas.push(lista);
      }
    }
  }

  // Parcelas: voraz por densidad, separadas 2·rp (global), al menos una por masa si cabe.
  const sep = 2 * prm.rp * FACTOR, hash = new Map<string, ParcelaBosque[]>();
  const cercana = (p: Point) => {
    const cx = Math.floor(p.x / sep), cy = Math.floor(p.y / sep);
    for (let y = cy - 1; y <= cy + 1; y++) for (let x = cx - 1; x <= cx + 1; x++)
      if ((hash.get(`${x},${y}`) ?? []).some(q => dist(q.centro, p) < sep)) return true;
    return false;
  };
  const masas: MasaBosque[] = [], parcelas: ParcelaBosque[] = [];
  const orden = comarcas.map((c, id) => ({ c, id })).filter(m => m.c.length >= prm.celdasMinimas).sort((a, b) => b.c.length - a.c.length);
  for (const { c } of orden) {
    const masaId = `bosque-${masas.length}`, propias: ParcelaBosque[] = [];
    for (const i of [...c].sort((a, b) => dens[b]! - dens[a]! || a - b)) {
      const p = centroDe(i);
      if (cercana(p)) continue;
      const a = anilloEn(campamentos, p).anillo;
      const parcela: ParcelaBosque = { id: `parcela-${parcelas.length}`, masaId, centro: p, densidad: dens[i]!, anillo: a };
      propias.push(parcela); parcelas.push(parcela);
      const k = `${Math.floor(p.x / sep)},${Math.floor(p.y / sep)}`;
      hash.set(k, [...(hash.get(k) ?? []), parcela]);
    }
    if (!propias.length) continue; // pegada a otra masa y sin sitio propio
    const area = c.length * lado ** 2; // unidades del generador²
    const centro = c.reduce((s, i) => { const q = centroDe(i); return { x: s.x + q.x / c.length, y: s.y + q.y / c.length }; }, { x: 0, y: 0 });
    masas.push({ id: masaId, centro, radio: Math.sqrt(area / Math.PI), densidad: c.reduce((s, i) => s + dens[i]!, 0) / c.length, area,
      anillo: anilloEn(campamentos, centro).anillo, parcelasIds: propias.map(q => q.id) });
  }
  return { masas, parcelas };
}
