import type { Point } from './tipos';
import { createRng, derivarSeed } from './rng';
import { anilloEn, FACTOR, type Anillo, type Campamento } from './campamentos';

/**
 * Estaño y oro por gradiente (Doc 1.9c, decidido el 2026-10-13): foco 5 % (una muestra en el borde del foco, un tercio de
 * cantidad), frontera 25 % (aislados), tierra lejana 70 % (vetas de estaño con 0-2 de oro; el resto del oro, aislado).
 */
export type Metal = 'estano' | 'oro';

export const PARAMETROS_METALES = {
  total: { estano: 64, oro: 24 } as Record<Metal, number>,
  reparto: { foco: 0.05, frontera: 0.25, lejana: 0.7 } as Record<Anillo, number>,
  /** El foco se siembra en su borde exterior (distancia estratégica al campamento). */
  bordeFoco: { min: 100, max: 150 },
  cantidadFoco: 1 / 3,
  /** Separación entre nodos aislados del mismo metal (estratégicas). */
  espacioAislado: 120,
  /** Dentro de una veta: separación mínima entre nodos y radio máximo (estratégicas). */
  espacioEnVeta: 8,
  radioVeta: { min: 20, max: 30 },
  tamanoVeta: { min: 3, max: 6 },
  /** Distancia mínima entre centros de vetas (estratégicas). */
  entreVetas: 60,
};

/** Un sitio apto para un yacimiento (terreno y accesos ya comprobados por quien lo genera) con la calidad de cada metal, 0..1. */
export interface Sitio { posicion: Point; calidad: Record<Metal, number> }

export interface NodoMetal {
  id: string;
  tipo: Metal;
  posicion: Point;
  anillo: Anillo;
  /** Fracción de la cantidad normal del yacimiento. */
  cantidadFactor: number;
  vetaId?: string;
}

export interface Veta { id: string; centro: Point; nodosIds: string[]; campamentoMasCercano: string }

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

/** Reparto por resto mayor: suma exactamente `total`. */
export function cuotasPorAnillo(total: number, reparto: Record<Anillo, number>): Record<Anillo, number> {
  const claves = Object.keys(reparto) as Anillo[];
  const base = claves.map(k => Math.floor(total * reparto[k]));
  const restos = claves.map((k, i) => ({ k, i, r: total * reparto[k] - base[i]! })).sort((a, b) => b.r - a.r);
  let falta = total - base.reduce((s, n) => s + n, 0);
  for (const { i } of restos) { if (falta <= 0) break; base[i]!++; falta--; }
  return Object.fromEntries(claves.map((k, i) => [k, base[i]!])) as Record<Anillo, number>;
}

export function repartirMetales(
  seed: number,
  campamentos: readonly Campamento[],
  sitios: readonly Sitio[],
  p = PARAMETROS_METALES,
): { nodos: NodoMetal[]; vetas: Veta[] } {
  const rng = createRng(derivarSeed(seed, 'metales-1'));
  const info = sitios.map(s => ({ s, ...anilloEn(campamentos, s.posicion), azar: rng() }));
  const nodos: NodoMetal[] = [];
  const vetas: Veta[] = [];
  const cuota = { estano: cuotasPorAnillo(p.total.estano, p.reparto), oro: cuotasPorAnillo(p.total.oro, p.reparto) };
  const libreDe = (tipo: Metal, q: Point, minimo: number) => nodos.every(n => n.tipo !== tipo || dist(n.posicion, q) >= minimo);
  const otroCerca = (q: Point) => nodos.every(n => dist(n.posicion, q) >= 3 * FACTOR);
  const valor = (c: typeof info[number], t: Metal) => (0.2 + 0.8 * c.s.calidad[t]) * (0.4 + 0.6 * c.azar);
  const porCampamento = new Map<string, number>();
  const sumar = (id: string) => porCampamento.set(id, (porCampamento.get(id) ?? 0) + 1);
  const nuevoId = (tipo: Metal, parte: string) => `${tipo}-${parte}-${nodos.filter(n => n.tipo === tipo).length}`;

  // Foco: una muestra en el borde exterior, repartida entre campamentos distintos mientras se pueda.
  for (const tipo of ['estano', 'oro'] as Metal[]) {
    const usados = new Set<string>();
    for (let n = 0; n < cuota[tipo].foco; n++) {
      const aptos = info.filter(c => c.anillo === 'foco' && c.distancia >= p.bordeFoco.min && c.distancia <= p.bordeFoco.max
        && libreDe(tipo, c.s.posicion, p.espacioAislado * FACTOR) && otroCerca(c.s.posicion));
      const preferidos = aptos.filter(c => !usados.has(c.campamento));
      const mejor = (preferidos.length ? preferidos : aptos).sort((a, b) => valor(b, tipo) - valor(a, tipo))[0];
      if (!mejor) throw new Error(`Metales: sin sitio en el borde del foco para ${tipo} (${n + 1}/${cuota[tipo].foco})`);
      usados.add(mejor.campamento);
      nodos.push({ id: nuevoId(tipo, 'foco'), tipo, posicion: mejor.s.posicion, anillo: 'foco', cantidadFactor: p.cantidadFoco });
    }
  }

  // Frontera: aislados, con una pequeña penalización por los que ya cuelgan del mismo campamento.
  for (const tipo of ['estano', 'oro'] as Metal[]) {
    porCampamento.clear();
    nodos.filter(n => n.tipo === tipo && n.anillo === 'frontera').forEach(n => sumar(anilloEn(campamentos, n.posicion).campamento));
    for (let n = 0; n < cuota[tipo].frontera; n++) {
      const mejor = info.filter(c => c.anillo === 'frontera' && libreDe(tipo, c.s.posicion, p.espacioAislado * FACTOR) && otroCerca(c.s.posicion))
        .sort((a, b) => valor(b, tipo) / (1 + 0.3 * (porCampamento.get(b.campamento) ?? 0)) - valor(a, tipo) / (1 + 0.3 * (porCampamento.get(a.campamento) ?? 0)))[0];
      if (!mejor) throw new Error(`Metales: sin sitio en la frontera para ${tipo} (${n + 1}/${cuota[tipo].frontera})`);
      sumar(mejor.campamento);
      nodos.push({ id: nuevoId(tipo, 'frontera'), tipo, posicion: mejor.s.posicion, anillo: 'frontera', cantidadFactor: 1 });
    }
  }

  // Tierra lejana: vetas de estaño (3-6 nodos) con 0-2 de oro cada una; el oro que sobra, aislado.
  const lejanos = info.filter(c => c.anillo === 'lejana');
  const estanoLejos = cuota.estano.lejana, oroLejos = cuota.oro.lejana;
  const numVetas = Math.max(1, Math.ceil(estanoLejos / ((p.tamanoVeta.min + p.tamanoVeta.max) / 2)));
  const tamanos = Array.from({ length: numVetas }, (_, i) => Math.floor(estanoLejos / numVetas) + (i < estanoLejos % numVetas ? 1 : 0));
  if (tamanos.some(t => t < p.tamanoVeta.min || t > p.tamanoVeta.max)) throw new Error(`Metales: ${estanoLejos} de estaño no caben en ${numVetas} vetas de ${p.tamanoVeta.min}-${p.tamanoVeta.max}`);
  const orosEnVeta = tamanos.map((_, i) => Math.min(2, i % 3 === 0 ? 2 : i % 3 === 1 ? 1 : 0));
  let oroEnVetas = 0;
  const radioMax = p.radioVeta.max * FACTOR, espacio = p.espacioEnVeta * FACTOR;

  // Centros de veta: donde más sitios aptos de estaño hay alrededor, separados entre sí.
  const centros = lejanos.map(c => ({ c, n: lejanos.filter(o => dist(o.s.posicion, c.s.posicion) <= radioMax).length * (0.6 + 0.4 * c.azar) }))
    .sort((a, b) => b.n - a.n);
  const elegidos: Point[] = [];
  for (const { c } of centros) {
    if (elegidos.length === numVetas) break;
    if (elegidos.some(e => dist(e, c.s.posicion) < p.entreVetas * FACTOR)) continue;
    const i = elegidos.length, necesita = tamanos[i]! + orosEnVeta[i]!;
    const cerca = lejanos.filter(o => dist(o.s.posicion, c.s.posicion) <= radioMax);
    const id = `veta-${i}`, propios: NodoMetal[] = [];
    const sitiosVeta = [...cerca].sort((a, b) => valor(b, 'estano') - valor(a, 'estano'));
    for (const o of sitiosVeta) {
      if (propios.length === necesita) break;
      if (propios.every(n => dist(n.posicion, o.s.posicion) >= espacio) && otroCerca(o.s.posicion)) {
        const tipo: Metal = propios.filter(n => n.tipo === 'estano').length < tamanos[i]! ? 'estano' : 'oro';
        propios.push({ id: '', tipo, posicion: o.s.posicion, anillo: 'lejana', cantidadFactor: 1, vetaId: id });
      }
    }
    if (propios.length < necesita) continue; // no cabe: otro centro
    elegidos.push(c.s.posicion);
    for (const n of propios) { n.id = nuevoId(n.tipo, id); nodos.push(n); }
    oroEnVetas += orosEnVeta[i]!;
    vetas.push({ id, centro: c.s.posicion, nodosIds: propios.map(n => n.id), campamentoMasCercano: c.campamento });
  }
  if (vetas.length < numVetas) throw new Error(`Metales: solo caben ${vetas.length} de ${numVetas} vetas`);

  for (let n = oroEnVetas; n < oroLejos; n++) {
    const mejor = lejanos.filter(c => libreDe('oro', c.s.posicion, p.espacioAislado * FACTOR) && otroCerca(c.s.posicion))
      .sort((a, b) => valor(b, 'oro') - valor(a, 'oro'))[0];
    if (!mejor) throw new Error(`Metales: sin sitio lejano para el oro aislado (${n + 1}/${oroLejos})`);
    nodos.push({ id: nuevoId('oro', 'lejos'), tipo: 'oro', posicion: mejor.s.posicion, anillo: 'lejana', cantidadFactor: 1 });
  }
  return { nodos, vetas };
}
