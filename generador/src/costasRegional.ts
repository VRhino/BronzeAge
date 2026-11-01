import type { Point, RioZona } from './tipos';
import type { RelieveRegional } from './regional';
import { createRng, derivarSeed } from './rng';
import { evaluarRuido, generarCampoRuido, type CampoRuido } from './ruido';
import { FACTOR } from './campamentos';
import { enPaso } from './relieveDetalle';

/**
 * Costas (revisión 2, decidida por el autor el 2026-11-01 sobre el «Boceto de costa» de la mesa de escalas 3D: «si el generador
 * crea este tipo de costas a lo largo del mapa, perfecto»). La línea de costa base es la del relieve regional (donde cruza el
 * nivel del mar), muestreada cada `muestreo`; encima, a lo largo de todo el mapa:
 *  - tres tipos que se alternan: acantilado (de 30 a 55 m, sube en 2,4 u, y detrás la tierra baja despacio), costa rocosa (de 7 a 12 m, sube de golpe y es
 *    quebrada) y playa (berma llana, dunas y una llanura que sube despacio). Antes una máscara fija ponía la roca al noroeste y
 *    al norte y el resto era playa en rampa; ahora el tipo sale de un ruido a lo largo de la costa, de si la costa es cabo o
 *    bahía (el cabo, roca; la bahía, playa) y de lo alta que es la tierra de detrás;
 *  - el contorno: ondas grandes (cabos y ensenadas), medianas y finas en la roca y el acantilado; la playa, casi lisa;
 *  - calas en la roca y el acantilado, con una playita al fondo y el barranco que baja a ella;
 *  - farallones al pie de los cabos de acantilado;
 *  - estuarios: el río se abre en sus últimas 25 u hasta la boca;
 *  - el fondo del mar: somero frente a la playa y hondo bajo el acantilado.
 * Todo son datos puros (se clonan a los hilos del relieve fino) y `aplicarCostaRegional` es pura.
 */
export const COSTAS_REGIONALES = {
  revision: 'costas-2',
  nivelMar: 0.375,
  /** Muestreo de la línea de costa base y lado de la celda de los índices, en unidades del generador. */
  muestreo: 100,
  celdaIndice: 300,
  /** Hasta dónde llega la costa (u), a cada lado de la línea base: más allá, el relieve no se toca. */
  alcance: 40,
  /** Parte de la longitud de costa de cada tipo; la playa es el resto. */
  reparto: { acantilado: 0.3, roca: 0.25 },
  /** Ancho de la transición entre tipos, en la puntuación de la costa (0-1). */
  transicionTipo: 0.05,
  /** Longitud de onda (u) del ruido que alterna los tipos a lo largo de la costa. */
  ondaTipo: 110,
  /** Radio (u) en el que se mide si la costa es cabo (poca tierra alrededor) o bahía (mucha), y a qué distancia tierra
   * adentro se mira la altura (la costa alta da acantilado). */
  radioForma: 30,
  tierraAdentro: 25,
  /** Pesos de la puntuación: ruido, cabo y altura de detrás. */
  pesosTipo: { ruido: 0.5, cabo: 0.25, altura: 0.25 },
  /** Desembocaduras: sin roca ni acantilado a menos de `min` u, del todo desde `max` u. */
  desembocadura: { min: 8, max: 30 },
  /** Contorno (u): ondas y amplitudes. Las grandes hacen cabos y ensenadas; se apagan entre 15 y 25 u de la línea base. */
  contorno: { ondaGrande: 45, grande: 9, ondaMedia: 9, media: 2.5, ondaFina: 3.5, fina: 1, ondaPlaya: 25, playa: 2, apagado: [15, 25] },
  /** Alturas (m) y ancho (u) en el que suben. */
  acantilado: { min: 30, max: 55, ancho: 2.4 },
  roca: { min: 7, max: 12, ancho: 3 },
  playa: { berma: 1.6, anchoBerma: 8, finDunas: 16, dunas: [1.5, 5], subida: 0.25, ondaDunas: 15,
    /** Ancho (u) de la franja llana antes de que empiece la ladera de dentro, y cuánto (m, como mucho) se retira esa ladera:
     * la playa no allana el relieve de detrás, lo aparta. */
    franja: [6, 14], retiro: 30,
    /** Ancho (u) de la rampa que lleva la ladera de dentro hasta el agua donde esta entra en tierra alta. */
    rampa: 8 },
  /** Los tipos se mezclan entre tramos cercanos (u): sin esto, donde cambia el tramo más cercano saldría un escalón. */
  mezclaTipos: 5,
  /** Dónde (u) la costa vuelve al relieve de dentro, por tipo, y el mar a su fondo. */
  vuelta: { playa: [30, 36], roca: [4, 22], acantilado: [5, 37], mar: [15, 35] },
  /** Calas: separación (u) entre ellas, probabilidad en cada sitio apto, semiejes (u) a lo largo y tierra adentro. */
  calas: { separacion: 45, probabilidad: 0.75, ancho: [5, 9], fondo: [7, 12] },
  /** Farallones: separación (u) entre grupos, probabilidad, cuántos, a qué distancia (u) de la orilla, radio (u) y altura (m). */
  farallones: { separacion: 40, probabilidad: 0.6, cuantos: [1, 4], distancia: [2, 7], radio: [0.8, 2.2], altura: 30 },
  /** Estuarios: largo (u) desde la boca y ancho (u) del río y de la boca. */
  estuario: { largo: 25, anchoRio: 0.5, anchoBoca: 4, tierraBaja: [0.39, 0.405] },
  /** Las formas (calas y estuarios) se apagan entre estas distancias (u) de la línea base, tierra adentro. */
  apagadoFormas: [20, 28],
  /** Junto a los ríos (u: del todo hasta el primero, nada desde el segundo) y en los pasos de las barreras, la costa no
   * tiene contorno, roca, acantilado ni dunas: los ríos siguen bajando hasta el mar y los pasos siguen secos y transitables. */
  corredorRio: [3, 8],
} as const;

export type TipoCosta = 'acantilado' | 'roca' | 'playa';
export interface VerticeCostero extends Point {
  /** Normal hacia tierra y cuánto sube el relieve regional por unidad del generador. */
  nx: number; ny: number; g: number;
  /** Peso de acantilado, roca y playa (suman 1). */
  pesos: [number, number, number];
  /** Altura del acantilado y de la costa rocosa en este punto (m). */
  alto: number; bajo: number;
}
export interface TramoCostero { id: string; a: VerticeCostero; b: VerticeCostero; tipo: TipoCosta }
/** Cala: centro, eje a lo largo de la costa (tx, ty), eje tierra adentro (nx, ny) y semiejes, en unidades del generador. */
export interface Cala { x: number; y: number; tx: number; ty: number; nx: number; ny: number; rx: number; ry: number }
export interface Farallon { x: number; y: number; r: number }
export interface CampoCostero {
  revision: string; seed: number; config: typeof COSTAS_REGIONALES;
  tramos: TramoCostero[]; indice: Record<string, number[]>;
  calas: Cala[]; indiceCalas: Record<string, number[]>;
  farallones: Farallon[]; indiceFarallones: Record<string, number[]>;
  /** Tramos de estuario aplanados: ax, ay, bx, by, ta, tb (t de 0 aguas arriba a 1 en la boca). */
  estuarios: number[]; indiceEstuarios: Record<string, number[]>;
  /** Tramos de río cerca de la costa aplanados (ax, ay, bx, by) y pasos de las barreras (como en `relieveDetalle`). */
  rios: number[]; indiceRios: Record<string, number[]>; pasos: number[];
  ruido: { tipo: CampoRuido; grande: CampoRuido; media: CampoRuido; fina: CampoRuido; playa: CampoRuido; dunas: CampoRuido };
}

const suave = (v: number) => { const t = Math.max(0, Math.min(1, v)); return t * t * (3 - 2 * t); };
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const entre = (r: () => number, [a, b]: readonly [number, number] | readonly number[]) => a! + (b! - a!) * r();
/** De -1 a 1 (el ruido normalizado va de 0 a 1). */
const centrado = (c: CampoRuido, p: Point) => (evaluarRuido(c, p) - 0.5) * 2;

function indexar(indice: Record<string, number[]>, k: number, minX: number, minY: number, maxX: number, maxY: number, celda: number) {
  for (let y = Math.floor(minY / celda); y <= Math.floor(maxY / celda); y++)
    for (let x = Math.floor(minX / celda); x <= Math.floor(maxX / celda); x++) (indice[`${x},${y}`] ??= []).push(k);
}

/** Desplazamiento del contorno (u) con los pesos de tipo dados: lo que la orilla avanza (positivo) o retrocede. */
function desplazamiento(campo: CampoCostero, p: Point, rugoso: number, playa: number, conGrande = true): number {
  const C = campo.config.contorno, r = campo.ruido;
  let d = 0;
  if (rugoso > 0) d += rugoso * ((conGrande ? C.grande * centrado(r.grande, p) : 0) + C.media * centrado(r.media, p) + C.fina * centrado(r.fina, p));
  if (playa > 0) d += playa * C.playa * centrado(r.playa, p);
  return d;
}

/** The continuous geographic field is constructed before forest, soils and resources.
 * Sampling callbacks must omit any previous coast field: rebuilding is idempotent. */
export function crearCostasRegional(seed: number, regional: RelieveRegional, alturaBase: (p: Point) => number, rios: readonly RioZona[] = []): CampoCostero {
  const config = COSTAS_REGIONALES, U = FACTOR, rng = createRng(derivarSeed(seed, 'costas-regional-2'));
  const onda = (u: number) => 1 / (u * U);
  const ruido = {
    tipo: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.ondaTipo), lacunaridad: 2.03, persistencia: 0.5 }),
    grande: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.contorno.ondaGrande), lacunaridad: 2.03, persistencia: 0.4 }),
    media: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.contorno.ondaMedia), lacunaridad: 2.03, persistencia: 0.5 }),
    fina: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.contorno.ondaFina), lacunaridad: 2.03, persistencia: 0.5 }),
    playa: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.contorno.ondaPlaya), lacunaridad: 2.03, persistencia: 0.4 }),
    dunas: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(config.playa.ondaDunas), lacunaridad: 2.03, persistencia: 0.5 }),
  };
  const campo: CampoCostero = { revision: config.revision, seed, config, tramos: [], indice: {}, calas: [], indiceCalas: {},
    farallones: [], indiceFarallones: {}, estuarios: [], indiceEstuarios: {}, rios: [], indiceRios: {},
    pasos: regional.barreras.flatMap(b => b.pasos.flatMap(p => [p.centro.x, p.centro.y, p.tangente.x, p.tangente.y, p.radio, b.ancho])), ruido };
  const { ancho, alto } = regional.limites, nx = Math.ceil(ancho / config.muestreo), ny = Math.ceil(alto / config.muestreo);
  const mar = (p: Point) => alturaBase(p) < config.nivelMar;
  const heights = new Float64Array((nx + 1) * (ny + 1));
  for (let y = 0; y <= ny; y++) for (let x = 0; x <= nx; x++) heights[y * (nx + 1) + x] = alturaBase({ x: Math.min(ancho, x * config.muestreo), y: Math.min(alto, y * config.muestreo) });

  // 1) Vértices de la línea base (bisección en cada arista que cruza el nivel del mar) y tramos.
  const vertices = new Map<string, VerticeCostero>();
  function vertex(a: Point, b: Point, ha: number): VerticeCostero {
    const key = [a.x, a.y, b.x, b.y].join(','); const cached = vertices.get(key); if (cached) return cached;
    let low = 0, high = 1;
    for (let i = 0; i < 16; i++) { const t = (low + high) / 2, h = alturaBase({ x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) }); if ((h >= config.nivelMar) === (ha >= config.nivelMar)) low = t; else high = t; }
    const t = (low + high) / 2, p = { x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) };
    const gx = alturaBase({ x: p.x + 15, y: p.y }) - alturaBase({ x: p.x - 15, y: p.y });
    const gy = alturaBase({ x: p.x, y: p.y + 15 }) - alturaBase({ x: p.x, y: p.y - 15 }), g = Math.hypot(gx, gy) || 1e-9;
    const v: VerticeCostero = { ...p, nx: gx / g, ny: gy / g, g: g / 30, pesos: [0, 0, 1], alto: 0, bajo: 0 };
    vertices.set(key, v); return v;
  }
  function segment(a: VerticeCostero, b: VerticeCostero) {
    if (Math.hypot(a.x - b.x, a.y - b.y) < .001) return;
    campo.tramos.push({ id: `costa-${campo.tramos.length}`, a, b, tipo: 'playa' });
  }
  for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
    const x0 = x * config.muestreo, x1 = Math.min(ancho, (x + 1) * config.muestreo), y0 = y * config.muestreo, y1 = Math.min(alto, (y + 1) * config.muestreo);
    const ps = [{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }];
    const hs = [heights[y * (nx + 1) + x]!, heights[y * (nx + 1) + x + 1]!, heights[(y + 1) * (nx + 1) + x + 1]!, heights[(y + 1) * (nx + 1) + x]!];
    const edges = [[0, 1], [1, 2], [3, 2], [0, 3]], cross: VerticeCostero[] = [];
    for (const [a, b] of edges as [number, number][]) if ((hs[a]! >= config.nivelMar) !== (hs[b]! >= config.nivelMar)) cross.push(vertex(ps[a]!, ps[b]!, hs[a]!));
    if (cross.length === 2) segment(cross[0]!, cross[1]!);
    else if (cross.length === 4) {
      const center = alturaBase({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 }) >= config.nivelMar;
      if (center === (hs[0]! >= config.nivelMar)) { segment(cross[0]!, cross[1]!); segment(cross[2]!, cross[3]!); } else { segment(cross[0]!, cross[3]!); segment(cross[1]!, cross[2]!); }
    }
  }

  // 2) Tipo de cada vértice: puntuación (ruido a lo largo de la costa, cabo o bahía, altura de detrás) y umbrales por longitud.
  const lista = [...vertices.values()], largo = new Map<VerticeCostero, number>();
  for (const t of campo.tramos) { const l = Math.hypot(t.a.x - t.b.x, t.a.y - t.b.y) / 2; largo.set(t.a, (largo.get(t.a) ?? 0) + l); largo.set(t.b, (largo.get(t.b) ?? 0) + l); }
  const radio = config.radioForma * U, rodea = 16, tierraDe = new Map<VerticeCostero, number>();
  const alMar = rios.filter(r => r.fin === 'agua' && !r.terminaEnLago && r.puntos.length > 1 && alturaBase(r.puntos[r.puntos.length - 1]!) < config.nivelMar + 0.02);
  const bocas = alMar.map(r => r.puntos[r.puntos.length - 1]!);
  const puntuacion = lista.map(v => {
    let tierra = 0;
    for (let k = 0; k < rodea; k++) { const a = 2 * Math.PI * k / rodea; if (!mar({ x: v.x + radio * Math.cos(a), y: v.y + radio * Math.sin(a) })) tierra++; }
    tierraDe.set(v, tierra / rodea);
    const detras = alturaBase({ x: v.x + v.nx * config.tierraAdentro * U, y: v.y + v.ny * config.tierraAdentro * U });
    const P = config.pesosTipo;
    return P.ruido * evaluarRuido(ruido.tipo, v) + P.cabo * Math.max(0, Math.min(1, (0.5 - tierra / rodea) * 2 + 0.5)) + P.altura * suave((detras - 0.39) / 0.06);
  });
  const total = lista.reduce((s, v) => s + (largo.get(v) ?? 0), 0);
  const umbral = (fraccion: number) => {
    const orden = lista.map((v, i) => ({ s: puntuacion[i]!, l: largo.get(v) ?? 0 })).sort((a, b) => b.s - a.s);
    let acum = 0; for (const o of orden) { acum += o.l; if (acum >= fraccion * total) return o.s; } return -Infinity;
  };
  const uA = umbral(config.reparto.acantilado), uR = umbral(config.reparto.acantilado + config.reparto.roca), b = config.transicionTipo;
  for (const [i, v] of lista.entries()) {
    const s = puntuacion[i]!;
    let pa = suave((s - uA) / (2 * b) + 0.5), roca = suave((s - uR) / (2 * b) + 0.5);
    let pr = Math.max(0, roca - pa);
    for (const m of bocas) { const f = suave((Math.hypot(v.x - m.x, v.y - m.y) / U - config.desembocadura.min) / (config.desembocadura.max - config.desembocadura.min)); pa *= f; pr *= f; }
    v.pesos = [pa, pr, Math.max(0, 1 - pa - pr)];
    const variacion = evaluarRuido(ruido.tipo, { x: v.x * 3.1 + 77, y: v.y * 3.1 }), detalle = evaluarRuido(ruido.media, { x: v.x * 0.37, y: v.y * 0.37 + 91 });
    v.alto = lerp(config.acantilado.min, config.acantilado.max, Math.min(1, Math.max(0, variacion * 0.65 + detalle * 0.35)));
    v.bajo = lerp(config.roca.min, config.roca.max, detalle);
  }
  for (const t of campo.tramos) {
    const w = t.a.pesos.map((x, k) => x + t.b.pesos[k]!), mayor = w.indexOf(Math.max(...w));
    t.tipo = (['acantilado', 'roca', 'playa'] as const)[mayor]!;
  }
  const alcance = config.alcance * U;
  for (const [k, t] of campo.tramos.entries())
    indexar(campo.indice, k, Math.min(t.a.x, t.b.x) - alcance, Math.min(t.a.y, t.b.y) - alcance, Math.max(t.a.x, t.b.x) + alcance, Math.max(t.a.y, t.b.y) + alcance, config.celdaIndice);

  // Ríos cerca de la costa: su corredor queda protegido.
  const corredor = config.corredorRio[1] * U;
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, bb = rio.puntos[i]!;
    const cerca = [a, bb].some(q => campo.indice[`${Math.floor(q.x / config.celdaIndice)},${Math.floor(q.y / config.celdaIndice)}`]);
    if (!cerca) continue;
    const k = campo.rios.length / 4; campo.rios.push(a.x, a.y, bb.x, bb.y);
    indexar(campo.indiceRios, k, Math.min(a.x, bb.x) - corredor, Math.min(a.y, bb.y) - corredor, Math.max(a.x, bb.x) + corredor, Math.max(a.y, bb.y) + corredor, config.celdaIndice);
  }
  const protegido = (q: Point) => proteccion(campo, q, `${Math.floor(q.x / config.celdaIndice)},${Math.floor(q.y / config.celdaIndice)}`) > 0;

  // 3) Calas y farallones, en sitios elegidos en orden aleatorio; su posición tiene en cuenta cuánto se mueve allí la orilla.
  const azar = createRng(derivarSeed(seed, 'costas-formas-2')), orden = lista.map(v => ({ v, r: azar() })).sort((a, b) => a.r - b.r).map(o => o.v);
  const lejosDeBocas = (p: Point, u: number) => bocas.every(m => Math.hypot(p.x - m.x, p.y - m.y) >= u * U);
  for (const v of orden) {
    const [pa, pr, pp] = v.pesos, tierra = tierraDe.get(v)!, C = config.calas;
    if (pa + pr < 0.6 || tierra < 0.5 || !lejosDeBocas(v, 30) || azar() >= C.probabilidad || protegido(v)) continue;
    if (campo.calas.some(c => Math.hypot(c.x - v.x, c.y - v.y) < C.separacion * U)) continue;
    const rx = entre(azar, C.ancho) * U, ry = entre(azar, C.fondo) * U, detras = { x: v.x + v.nx * (ry + 15 * U), y: v.y + v.ny * (ry + 15 * U) };
    if (mar(detras) || protegido(detras) || protegido({ x: v.x + v.nx * ry, y: v.y + v.ny * ry })) continue;
    // Algo girada respecto a la normal y con la boca más estrecha que el fondo (el centro, más adentro que la orilla).
    const orilla = -desplazamiento(campo, v, pa + pr, pp) * U, giro = (azar() * 2 - 1) * 0.45, cg = Math.cos(giro), sg = Math.sin(giro);
    const nx = v.nx * cg - v.ny * sg, ny = v.nx * sg + v.ny * cg, dentro = orilla + 0.6 * ry;
    const c: Cala = { x: v.x + v.nx * dentro, y: v.y + v.ny * dentro, tx: -ny, ty: nx, nx, ny, rx, ry };
    const k = campo.calas.length, alc = 3.5 * Math.max(rx, ry) + 2 * U;
    campo.calas.push(c); indexar(campo.indiceCalas, k, c.x - alc, c.y - alc, c.x + alc, c.y + alc, config.celdaIndice);
  }
  const grupos: Point[] = [];
  for (const v of orden) {
    const [pa, pr, pp] = v.pesos, F = config.farallones;
    if (pa < 0.5 || tierraDe.get(v)! >= 0.45 || azar() >= F.probabilidad) continue;
    if (grupos.some(g => Math.hypot(g.x - v.x, g.y - v.y) < F.separacion * U)) continue;
    grupos.push(v);
    const orilla = -desplazamiento(campo, v, pa + pr, pp) * U, n = Math.round(entre(azar, F.cuantos));
    for (let i = 0; i < n; i++) {
      const d = orilla - entre(azar, F.distancia) * U, lado = (azar() * 2 - 1) * 5 * U, r = entre(azar, F.radio) * U;
      const f: Farallon = { x: v.x + v.nx * d - v.ny * lado, y: v.y + v.ny * d + v.nx * lado, r };
      if (!mar(f)) continue;
      const k = campo.farallones.length, alc = r + 4 * U;
      campo.farallones.push(f); indexar(campo.indiceFarallones, k, f.x - alc, f.y - alc, f.x + alc, f.y + alc, config.celdaIndice);
    }
  }

  // 4) Estuarios: las últimas `largo` u de cada río que llega al mar, y un poco más allá de su último punto.
  const E = config.estuario;
  for (const rio of alMar) {
    const fin = rio.puntos[rio.puntos.length - 1]!, previo = rio.puntos[rio.puntos.length - 2]!;
    const l = Math.hypot(fin.x - previo.x, fin.y - previo.y) || 1;
    const puntos = [...rio.puntos, { x: fin.x + (fin.x - previo.x) / l * 3 * U, y: fin.y + (fin.y - previo.y) / l * 3 * U }];
    let desdeBoca = -3 * U;
    for (let i = puntos.length - 1; i > 0 && desdeBoca < E.largo * U; i--) {
      const a = puntos[i - 1]!, bb = puntos[i]!, lon = Math.hypot(bb.x - a.x, bb.y - a.y);
      const tb = 1 - Math.max(0, desdeBoca) / (E.largo * U), ta = 1 - Math.max(0, desdeBoca + lon) / (E.largo * U);
      const k = campo.estuarios.length / 6, alc = (E.anchoBoca + 20) * U;
      campo.estuarios.push(a.x, a.y, bb.x, bb.y, Math.max(0, ta), Math.min(1, tb));
      indexar(campo.indiceEstuarios, k, Math.min(a.x, bb.x) - alc, Math.min(a.y, bb.y) - alc, Math.max(a.x, bb.x) + alc, Math.max(a.y, bb.y) + alc, config.celdaIndice);
      desdeBoca += lon;
    }
  }
  return campo;
}

/** Cuánto protege `p` (0-1) el corredor de un río o el paso de una barrera. */
function proteccion(campo: CampoCostero, p: Point, celda: string): number {
  const [r0, r1] = campo.config.corredorRio, U = FACTOR;
  let dr = Infinity;
  for (const k of campo.indiceRios[celda] ?? []) {
    const o = 4 * k, ax = campo.rios[o]!, ay = campo.rios[o + 1]!, dx = campo.rios[o + 2]! - ax, dy = campo.rios[o + 3]! - ay;
    const l = dx * dx + dy * dy, t = l ? Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / l)) : 0;
    dr = Math.min(dr, Math.hypot(p.x - ax - t * dx, p.y - ay - t * dy));
  }
  // El paso entero, de lado a lado de la barrera (±2,8 anchos), con un margen: ni calas, ni acantilado, ni contorno.
  let dp = Infinity;
  for (let i = 0; i < campo.pasos.length; i += 6) {
    const cx = campo.pasos[i]!, cy = campo.pasos[i + 1]!, nx = -campo.pasos[i + 3]!, ny = campo.pasos[i + 2]!, largo = 2.8 * campo.pasos[i + 5]!;
    const t = Math.max(-largo, Math.min(largo, (p.x - cx) * nx + (p.y - cy) * ny));
    dp = Math.min(dp, Math.hypot(p.x - cx - t * nx, p.y - cy - t * ny));
  }
  return Math.max(1 - suave((dr / U - r0) / (r1 - r0)), 1 - suave((dp / U - r0 * 2) / (r1 * 2 - r0 * 2)), campo.pasos.length ? enPaso(campo.pasos, p) : 0);
}

/** Distancia de una forma (cala, estuario) que crece el triple a partir de 6 u: así su efecto se acaba antes del borde de su índice. */
const lejos = (d: number) => d + 2 * Math.max(0, d - 6);

/** Same globally indexed field for terrain, slope, forest and exported geometry. */
export function aplicarCostaRegional(campo: CampoCostero, p: Point, base: number): number {
  const c = campo.config, U = FACTOR, celda = `${Math.floor(p.x / c.celdaIndice)},${Math.floor(p.y / c.celdaIndice)}`;
  const ids = campo.indice[celda];
  if (!ids) return base;
  let nearest = Infinity, best: TramoCostero | undefined, tBest = 0;
  const cerca: number[] = [];
  for (const id of ids) {
    const s = campo.tramos[id]!, dx = s.b.x - s.a.x, dy = s.b.y - s.a.y;
    const t = Math.max(0, Math.min(1, ((p.x - s.a.x) * dx + (p.y - s.a.y) * dy) / (dx * dx + dy * dy)));
    const d = Math.hypot(p.x - lerp(s.a.x, s.b.x, t), p.y - lerp(s.a.y, s.b.y, t));
    cerca.push(id, t, d);
    if (d < nearest) { nearest = d; best = s; tBest = t; }
  }
  if (!best || nearest >= c.alcance * U) return base;
  const A = best.a, B = best.b, dU = nearest / U;

  // Distancia con signo a la línea base (u, positiva en tierra): cerca de la línea, la del relieve (continua y con el signo
  // exacto); más lejos, la del trazo.
  const sTrazo = base >= c.nivelMar ? dU : -dU;
  const sLineal = Math.max(-dU - 0.5, Math.min(dU + 0.5, (base - c.nivelMar) / lerp(A.g, B.g, tBest) / U));
  const sBase = lerp(sLineal, sTrazo, suave((dU - 2) / 2));

  // Tipo, alturas y pendiente de la costa: media de los tramos cercanos, pesada por lo que cada uno se aleja del más cercano.
  let pa = 0, pr = 0, pp = 0, alto = 0, bajo = 0, G = 0, suma = 0;
  const sigma = c.mezclaTipos * U;
  for (let i = 0; i < cerca.length; i += 3) {
    const e = (cerca[i + 2]! - nearest) / sigma; if (e > 3) continue;
    const w = Math.exp(-e * e), tr = campo.tramos[cerca[i]!]!, t = cerca[i + 1]!, a = tr.a, b = tr.b;
    pa += w * lerp(a.pesos[0], b.pesos[0], t); pr += w * lerp(a.pesos[1], b.pesos[1], t); pp += w * lerp(a.pesos[2], b.pesos[2], t);
    alto += w * lerp(a.alto, b.alto, t); bajo += w * lerp(a.bajo, b.bajo, t); G += w * lerp(a.g, b.g, t); suma += w;
  }
  pa /= suma; pr /= suma; pp /= suma; alto /= suma; bajo /= suma; G = G / suma * 1000 * U;
  // En el corredor de un río o en un paso: playa sin contorno, dunas ni retiro de la ladera.
  const protege = proteccion(campo, p, celda), libre = 1 - protege;
  if (protege > 0) { pp += protege * (pa + pr); pa *= libre; pr *= libre; }
  const apagado = suave((sBase - c.apagadoFormas[0]) / (c.apagadoFormas[1] - c.apagadoFormas[0]));
  const unir = (s: number, forma: number) => forma < s ? forma + (s - forma) * apagado : s;

  // Calas: un hueco elíptico (con el borde quebrado) y, al fondo, la playita y su barranco, que pasan la roca a playa.
  let sCala = Infinity;
  for (const k of campo.indiceCalas[celda] ?? []) {
    const cl = campo.calas[k]!, dx = p.x - cl.x, dy = p.y - cl.y;
    const u = (dx * cl.tx + dy * cl.ty) / cl.rx, v = (dx * cl.nx + dy * cl.ny) / cl.ry;
    sCala = Math.min(sCala, lejos((Math.hypot(u, v) - 1) * Math.min(cl.rx, cl.ry) / U));
    // El barranco se estrecha tierra adentro y sus lados no son rectos.
    const anchoBarranco = lerp(0.55, 0.18, suave(v / 3)) + 0.12 * centrado(campo.ruido.media, p);
    const q = (1 - suave((Math.abs(u) - anchoBarranco) / 0.45)) * suave((v + 0.3) / 0.5) * (1 - suave((v - 1.2) / 2)) * (1 - apagado);
    if (q > 0) { pp += q * (pa + pr); pa *= 1 - q; pr *= 1 - q; }
  }
  // Contorno: cabos, ensenadas y orilla quebrada, que se apagan lejos de la línea base.
  const C = c.contorno, fade = 1 - suave((Math.abs(sBase) - C.apagado[0]) / (C.apagado[1] - C.apagado[0]));
  let s = fade * libre > 0 ? sBase + fade * libre * desplazamiento(campo, p, pa + pr, pp) : sBase;
  if (sCala < Infinity) s = unir(s, sCala + (pa + pr) * C.fina * centrado(campo.ruido.fina, p));
  // Estuarios: el río, cada vez más ancho hacia la boca.
  const E = c.estuario;
  for (const k of campo.indiceEstuarios[celda] ?? []) {
    const o = 6 * k, ax = campo.estuarios[o]!, ay = campo.estuarios[o + 1]!, dx = campo.estuarios[o + 2]! - ax, dy = campo.estuarios[o + 3]! - ay;
    const l = dx * dx + dy * dy, tt = l ? Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / l)) : 0;
    const dr = Math.hypot(p.x - ax - tt * dx, p.y - ay - tt * dy) / U, te = lerp(campo.estuarios[o + 4]!, campo.estuarios[o + 5]!, tt);
    // Solo en tierra baja: el estuario no corta la tierra alta (ni se queda como una garganta inundada).
    s = unir(s, lejos(dr - (E.anchoRio + (E.anchoBoca - E.anchoRio) * te * te)) + 40 * suave((base - E.tierraBaja[0]) / (E.tierraBaja[1] - E.tierraBaja[0])));
  }
  // Farallones: columnas en el mar, con un bajío alrededor.
  let sFarallon = -Infinity;
  for (const k of campo.indiceFarallones[celda] ?? []) { const f = campo.farallones[k]!; sFarallon = Math.max(sFarallon, 3 * (f.r - Math.hypot(p.x - f.x, p.y - f.y)) / U); }
  const enFarallon = sFarallon > s;
  if (enFarallon) s = sFarallon;

  const V = c.vuelta, vuelta = (de: readonly number[], d: number) => suave((d - de[0]!) / (de[1]! - de[0]!));
  if (s >= 0) {
    const d = s, mar = c.nivelMar, tierra = Math.max(base, mar + 0.002);
    if (enFarallon) return mar + c.farallones.altura * suave(d / 3 / 0.9) / 1000;
    // Acantilado y roca: su perfil, que vuelve al relieve de dentro.
    const hA = mar + (alto * suave(d / c.acantilado.ancho) ** 0.7 + 0.15 * d) / 1000;
    const hR = mar + (bajo * (1 - (1 - Math.min(1, d / c.roca.ancho)) ** 2)
      + 1.6 * (0.5 + 0.5 * centrado(campo.ruido.fina, { x: p.x * 1.7, y: p.y * 1.7 })) * Math.min(1, d) + 0.2 * d) / 1000;
    // Playa: berma, dunas y una llanura que sube despacio; la ladera de dentro se aparta (sin pasar de `retiro`) y vuelve.
    const P = c.playa, duna = libre * lerp(P.dunas[0], P.dunas[1], evaluarRuido(campo.ruido.dunas, p));
    // La llanura (berma y lo que sube detrás) nunca pasa del relieve de dentro: allana, no levanta. Las dunas van encima.
    const llano = mar + (d < P.anchoBerma ? P.berma * d / P.anchoBerma : P.berma + P.subida * Math.max(0, d - P.finDunas)) / 1000;
    const dunas = d > P.anchoBerma && d < P.finDunas ? duna * Math.sin(Math.PI * (d - P.anchoBerma) / (P.finDunas - P.anchoBerma)) ** 2 / 1000 : 0;
    const franja = lerp(P.franja[0], P.franja[1], evaluarRuido(campo.ruido.playa, { x: p.y, y: p.x })), finRetiro = V.playa[0];
    const retiro = libre * Math.min(G * franja, P.retiro) / 1000 * (1 - suave((d - franja - 4) / Math.max(1, finRetiro - franja - 4)));
    // La ladera de dentro, apartada, y nunca más alta que una rampa desde el agua: donde el agua entra en tierra alta (un
    // estuario, una ensenada) la orilla baja hasta el mar en vez de quedar como un muro.
    const rampa = mar + (tierra - mar) * suave(d / P.rampa);
    const playa = Math.max(Math.min(llano, tierra), Math.min(tierra - retiro, rampa)) + dunas;
    return pa * lerp(hA, tierra, vuelta(V.acantilado, d)) + pr * lerp(hR, tierra, vuelta(V.roca, d)) + pp * lerp(playa, tierra, vuelta(V.playa, d));
  }
  const d = -s, a = Math.min(1, d / 0.5);
  const hondo = pp * Math.min(22, 0.6 * a + 0.4 * d) + pr * Math.min(28, 1.5 * a + 1.3 * d)
    + pa * Math.min(40, 1.5 * a + 14 * suave((d - 1.5) / 1.5) + 2.5 * Math.max(0, d - 3));
  const objetivo = c.nivelMar - hondo / 1000;
  return objetivo + (Math.min(base, c.nivelMar - 0.001) - objetivo) * vuelta(V.mar, d);
}
