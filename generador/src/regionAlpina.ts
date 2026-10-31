// Región alpina: una región entera del mapa hecha montaña habitable (pedido del autor, 2026-10-31). Desde fuera se va
// subiendo; dentro hay una cadena de valles en altura con altiplanos a varias alturas, cerrados por montaña y unidos por
// pasos con su collado, y unos valles de entrada que suben desde fuera. Se aplica al final de `aplicarDetalle`, encima de
// todo lo demás: los ríos y lagos de la región siguen donde estaban, en el fondo de sus valles.
// Boceto de referencia: el terreno «Región alpina» de la mesa de escalas 3D (Consideraciones/artefactos/mesa_escalas_3d).
import type { LagoZona, Point, RioZona } from './tipos';
import type { RelieveRegional } from './regional';
import { evaluarRelieveRegional } from './regional';
import { FACTOR } from './campamentos';
import { createRng, derivarSeed, type RandomFn } from './rng';
import { evaluarRuido, generarCampoRuido, ruidoGradiente, type CampoRuido } from './ruido';

/** Longitudes en unidades estratégicas (u); alturas en elevación (0-1, 1 = 1.000 m). */
export const REGION_ALPINA = {
  revision: 'alpina-1',
  /** La región del relieve regional que se vuelve alpina (decisión del autor, 2026-10-31). */
  area: 'Tierras altas secas',
  /** Fracción del radio de la región que es alpina del todo, y fracción donde se acaba la subida desde fuera. */
  nucleo: 0.8,
  borde: 1.12,
  /** Bamboleo del borde, en fracción del radio, y su longitud de onda en u. */
  bambolear: 0.08,
  ondaBorde: 160,
  /** Suelo de la montaña: en el borde del núcleo y en el centro (sube hacia dentro, los puntos altos en medio). */
  sueloBorde: 0.6,
  sueloCentro: 0.65,
  /** Lo que suben las crestas sobre el suelo (afiladas con la potencia), y su longitud de onda en u. */
  alturaCrestas: 0.24,
  afilado: 1.6,
  ondaCrestas: 80,
  /** Altiplanos: separación mínima entre centros, radios y alturas del suelo (más altos hacia el centro). */
  separacion: 82,
  radioMin: 22,
  radioMax: 40,
  pisoMin: 0.58,
  pisoMax: 0.76,
  /** Lomas del suelo de los altiplanos: altura y longitud de onda en u. */
  lomas: 0.006,
  ondaLomas: 11,
  /** Pasos entre altiplanos: un pasillo con fondo de `fondoPaso` u a cada lado y laderas de `laderaPaso` u. Collado entre
   * `colladoMin` y `colladoMax` sobre la recta entre los dos suelos. Además del árbol que los une a todos, se abre un paso
   * más hacia el vecino más cercano sin paso si está a menos de `vecinoExtra` separaciones (así hay más de un camino). */
  fondoPaso: 2.5,
  laderaPaso: 11,
  colladoMin: 0.012,
  colladoMax: 0.035,
  vecinoExtra: 1.45,
  /** Valles de entrada: cuántos, cuánto salen fuera de la región (en fracción del radio) y su anchura. */
  entradas: 4,
  salida: 1.2,
  fondoEntrada: 4,
  laderaEntrada: 14,
  /** Los ríos y los lagos de la región se quedan en el fondo de su valle: semiancho del valle y margen del lago en u. */
  valleRio: 9,
  margenLago: 30,
  /** Semiancho en u del cauce que conserva su cota cuando el terreno alpino queda por debajo del río. */
  cauce: 1.5,
  /** Lado de la celda del índice de altiplanos y pasos, en u. */
  celda: 64,
} as const;

interface Altiplano { x: number; y: number; rx: number; ry: number; giro: number; piso: number }
/** Un paso o un valle de entrada: polilínea con las alturas de su fondo en cada vértice. */
interface Senda { puntos: number[]; cotas: number[]; fondo: number; ladera: number; entrada: boolean }

/** Datos puros (se copian a los hilos del relieve fino). Coordenadas en unidades del generador. */
export interface RegionAlpina {
  revision: string;
  centro: Point;
  radio: number;
  altiplanos: Altiplano[];
  sendas: Senda[];
  /** Índice de celdas: altiplanos y tramos de senda (senda, vértice) que tocan cada celda. */
  indice: Record<string, { a: number[]; s: number[] }>;
  rios: number[];
  indiceRios: Record<string, number[]>;
  lagos: { x: number; y: number; r: number }[];
  crestas: CampoRuido;
  borde: CampoRuido;
  lomas: CampoRuido;
}

const suave = (t: number) => { const x = Math.min(1, Math.max(0, t)); return x * x * (3 - 2 * x); };

function bezier(a: Point, m: Point, b: Point, rng: RandomFn, amplitud: number, n = 40): Point[] {
  const o: Point[] = [];
  for (let k = 0; k <= n; k++) { const t = k / n, u = 1 - t; o.push({ x: u * u * a.x + 2 * u * t * m.x + t * t * b.x, y: u * u * a.y + 2 * u * t * m.y + t * t * b.y }); }
  // Meandros: un vaivén lateral suave que se apaga en los extremos.
  const f1 = rng() * 6.283, f2 = rng() * 6.283, c1 = 1.5 + rng() * 1.5, c2 = 3.5 + rng() * 2;
  return o.map((p, k) => {
    const t = k / n, q = o[Math.min(n, k + 1)]!, r = o[Math.max(0, k - 1)]!, dx = q.x - r.x, dy = q.y - r.y, l = Math.hypot(dx, dy) || 1;
    const w = amplitud * Math.sin(Math.PI * t) * (0.7 * Math.sin(c1 * Math.PI * t + f1) + 0.3 * Math.sin(c2 * Math.PI * t + f2));
    return { x: p.x - dy / l * w, y: p.y + dx / l * w };
  });
}

export function crearRegionAlpina(seed: number, regional: RelieveRegional, rios: readonly RioZona[], lagos: readonly LagoZona[]): RegionAlpina | undefined {
  const C = REGION_ALPINA, U = FACTOR, area = regional.areas.find(a => a.nombre === C.area);
  if (!area) return undefined;
  const rng = createRng(derivarSeed(seed, 'region-alpina-1'));
  const centro = area.centro, R = area.radio, { ancho, alto } = regional.limites;
  const dentro = (p: Point, margen: number) => p.x > margen && p.y > margen && p.x < ancho - margen && p.y < alto - margen;
  const lagosR = lagos.map(l => ({ x: l.centro.x, y: l.centro.y, r: l.radio }));
  const cercaDeLago = (p: Point, margen: number) => lagosR.some(l => Math.hypot(p.x - l.x, p.y - l.y) < l.r + margen);

  // Altiplanos: el primero en el centro (allí busca la conexión el acceso de la región); el resto, por muestreo de disco.
  const altiplanos: Altiplano[] = [];
  const piso = (p: Point) => {
    const t = 1 - Math.min(1, Math.hypot(p.x - centro.x, p.y - centro.y) / (R * C.nucleo));
    return C.pisoMin + (C.pisoMax - C.pisoMin) * Math.min(1, Math.max(0, 0.15 + 0.6 * t + 0.4 * (rng() - 0.5)));
  };
  const nuevo = (p: Point): Altiplano => {
    const r = (C.radioMin + rng() * (C.radioMax - C.radioMin)) * U;
    return { x: p.x, y: p.y, rx: r, ry: r * (0.55 + rng() * 0.3), giro: rng() * Math.PI, piso: piso(p) };
  };
  if (!cercaDeLago(centro, 40 * U)) altiplanos.push(nuevo(centro));
  for (let intento = 0; intento < 4000; intento++) {
    const a = rng() * 2 * Math.PI, d = Math.sqrt(rng()) * R * (C.nucleo - 0.06), p = { x: centro.x + Math.cos(a) * d, y: centro.y + Math.sin(a) * d };
    if (!dentro(p, 40 * U) || cercaDeLago(p, 25 * U)) continue;
    if (altiplanos.some(q => Math.hypot(q.x - p.x, q.y - p.y) < C.separacion * U)) continue;
    altiplanos.push(nuevo(p));
  }

  // Pasos: árbol de longitud mínima entre altiplanos (todos quedan unidos) y algún paso más para que haya rodeos.
  const sendas: Senda[] = [];
  const unidos = new Set<string>(), clave = (i: number, j: number) => (i < j ? `${i},${j}` : `${j},${i}`);
  const paso = (i: number, j: number) => {
    unidos.add(clave(i, j));
    const a = altiplanos[i]!, b = altiplanos[j]!, l = Math.hypot(b.x - a.x, b.y - a.y), desvio = (rng() - 0.5) * 0.35 * l;
    const m = { x: (a.x + b.x) / 2 - (b.y - a.y) / l * desvio, y: (a.y + b.y) / 2 + (b.x - a.x) / l * desvio };
    const puntos = bezier(a, m, b, rng, Math.min(6 * U, l * 0.05));
    const collado = C.colladoMin + rng() * (C.colladoMax - C.colladoMin);
    const cotas = puntos.map((_, k) => { const t = k / (puntos.length - 1); return a.piso + (b.piso - a.piso) * t + collado * Math.sin(Math.PI * t) ** 2; });
    sendas.push({ puntos: puntos.flatMap(p => [p.x, p.y]), cotas, fondo: C.fondoPaso * U, ladera: C.laderaPaso * U, entrada: false });
  };
  const enArbol = new Set<number>([0]);
  while (enArbol.size < altiplanos.length) {
    let mejor: [number, number, number] = [Infinity, -1, -1];
    for (const i of enArbol) for (let j = 0; j < altiplanos.length; j++) if (!enArbol.has(j)) {
      const d = Math.hypot(altiplanos[i]!.x - altiplanos[j]!.x, altiplanos[i]!.y - altiplanos[j]!.y);
      if (d < mejor[0]) mejor = [d, i, j];
    }
    paso(mejor[1], mejor[2]); enArbol.add(mejor[2]);
  }
  for (let i = 0; i < altiplanos.length; i++) {
    const vecinos = altiplanos.map((q, j) => [Math.hypot(q.x - altiplanos[i]!.x, q.y - altiplanos[i]!.y), j] as const)
      .filter(([, j]) => j !== i && !unidos.has(clave(i, j))).sort((a, b) => a[0] - b[0]);
    if (vecinos[0] && vecinos[0][0] < C.vecinoExtra * C.separacion * U && rng() < 0.6) paso(i, vecinos[0][1]);
  }

  // Valles de entrada: desde los altiplanos de fuera hacia fuera de la región, repartidos en ángulo y acabando en tierra.
  const exteriores = altiplanos.map((a, i) => ({ i, d: Math.hypot(a.x - centro.x, a.y - centro.y), ang: Math.atan2(a.y - centro.y, a.x - centro.x) }))
    .filter(o => o.d > R * 0.45).sort((a, b) => b.d - a.d);
  const elegidas: number[] = [];
  for (const o of exteriores) {
    if (elegidas.length >= C.entradas) break;
    const a = altiplanos[o.i]!, fin = { x: centro.x + Math.cos(o.ang) * R * C.salida, y: centro.y + Math.sin(o.ang) * R * C.salida };
    if (!dentro(fin, 20 * U) || evaluarRelieveRegional(regional, fin) < 0.45 || cercaDeLago(fin, 20 * U)) continue;
    if (elegidas.some(k => Math.abs(Math.atan2(Math.sin(exteriores.find(e => e.i === k)!.ang - o.ang), Math.cos(exteriores.find(e => e.i === k)!.ang - o.ang))) < 1.1)) continue;
    elegidas.push(o.i);
    const m = { x: (a.x + fin.x) / 2, y: (a.y + fin.y) / 2 };
    const puntos = bezier(a, m, fin, rng, Math.min(8 * U, Math.hypot(fin.x - a.x, fin.y - a.y) * 0.05));
    // La cota de fuera no se sabe aquí: el fondo es el suelo del altiplano y `aplicarAlpina` lo baja hacia el terreno de
    // fuera a la vez que la región se apaga.
    const cotas = puntos.map(() => a.piso);
    sendas.push({ puntos: puntos.flatMap(p => [p.x, p.y]), cotas, fondo: C.fondoEntrada * U, ladera: C.laderaEntrada * U, entrada: true });
  }

  const celda = C.celda * U, indice: RegionAlpina['indice'] = {};
  const meter = (minX: number, minY: number, maxX: number, maxY: number, f: (e: { a: number[]; s: number[] }) => void) => {
    for (let y = Math.floor(minY / celda); y <= Math.floor(maxY / celda); y++)
      for (let x = Math.floor(minX / celda); x <= Math.floor(maxX / celda); x++) f(indice[`${x},${y}`] ??= { a: [], s: [] });
  };
  altiplanos.forEach((a, k) => { const r = a.rx * 1.35; meter(a.x - r, a.y - r, a.x + r, a.y + r, e => e.a.push(k)); });
  sendas.forEach((s, k) => {
    const alcance = s.fondo + s.ladera;
    for (let v = 1; v < s.puntos.length / 2; v++) {
      const ax = s.puntos[2 * v - 2]!, ay = s.puntos[2 * v - 1]!, bx = s.puntos[2 * v]!, by = s.puntos[2 * v + 1]!;
      meter(Math.min(ax, bx) - alcance, Math.min(ay, by) - alcance, Math.max(ax, bx) + alcance, Math.max(ay, by) + alcance, e => e.s.push(k, v));
    }
  });
  // Ríos que cruzan la región, en el mismo índice de celdas.
  const tramosRios: number[] = [], indiceRios: Record<string, number[]> = {}, alcanceRio = C.valleRio * U * 2.5;
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!;
    if (Math.min(Math.hypot(a.x - centro.x, a.y - centro.y), Math.hypot(b.x - centro.x, b.y - centro.y)) > R * C.borde * 1.1) continue;
    const k = tramosRios.length / 4; tramosRios.push(a.x, a.y, b.x, b.y);
    for (let y = Math.floor((Math.min(a.y, b.y) - alcanceRio) / celda); y <= Math.floor((Math.max(a.y, b.y) + alcanceRio) / celda); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - alcanceRio) / celda); x <= Math.floor((Math.max(a.x, b.x) + alcanceRio) / celda); x++) (indiceRios[`${x},${y}`] ??= []).push(k);
  }
  const onda = (u: number) => 1 / (u * U);
  return {
    revision: C.revision, centro, radio: R, altiplanos, sendas, indice, rios: tramosRios, indiceRios,
    lagos: lagosR.filter(l => Math.hypot(l.x - centro.x, l.y - centro.y) < R * C.borde + l.r + C.margenLago * U),
    crestas: generarCampoRuido(rng, { octavas: 4, frecuenciaBase: onda(C.ondaCrestas), lacunaridad: 2.1, persistencia: 0.5 }),
    borde: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(C.ondaBorde), lacunaridad: 2.03, persistencia: 0.5 }),
    lomas: generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(C.ondaLomas), lacunaridad: 2.03, persistencia: 0.5 }),
  };
}

/** Crestas (ruido «ridged»): 1 en la línea de cumbre, cerca de 0 en las vaguadas. */
function crestas(r: CampoRuido, p: Point): number {
  let a = 1, f = r.frecuenciaBase, s = 0, t = 0, w = 1;
  for (let i = 0; i < r.octavas; i++) {
    const o = r.desplazamientos[i]!;
    let v = 1 - Math.abs(ruidoGradiente(r.permutacion, p.x * f + o.dx, p.y * f + o.dy) / Math.SQRT1_2);
    v *= v * w; w = Math.min(1, v * 1.8);
    s += a * v; t += a; a *= r.persistencia; f *= r.lacunaridad;
  }
  return s / t;
}

/** Cuánto pesa la región en `p` (0-1): 1 en el núcleo, 0 fuera del borde. */
export function pesoAlpino(r: RegionAlpina, p: Point): number {
  const C = REGION_ALPINA, d = Math.hypot(p.x - r.centro.x, p.y - r.centro.y) / r.radio;
  if (d > C.borde + C.bambolear) return 0;
  return 1 - suave((d + C.bambolear * (evaluarRuido(r.borde, p) - 0.5) * 2 - C.nucleo) / (C.borde - C.nucleo));
}

/** Elevación con la región alpina, a partir de la que ya tiene `p` (`e`, con el detalle). Pura. */
export function aplicarAlpina(r: RegionAlpina, p: Point, e: number): number {
  const C = REGION_ALPINA, peso = pesoAlpino(r, p);
  if (peso <= 0) return e;
  const U = FACTOR, celda = C.celda * U, en = r.indice[`${Math.floor(p.x / celda)},${Math.floor(p.y / celda)}`];
  // Montaña: un suelo que sube hacia el centro y crestas encima, con la subida desde fuera en el borde.
  const hacia = 1 - Math.min(1, Math.hypot(p.x - r.centro.x, p.y - r.centro.y) / (r.radio * C.nucleo));
  const q = Math.min(1, Math.max(0, (crestas(r.crestas, p) - 0.15) / 0.75));
  let h = C.sueloBorde + (C.sueloCentro - C.sueloBorde) * hacia + C.alturaCrestas * q ** C.afilado;
  // Pasos y valles de entrada: fondo casi plano con su perfil y laderas hasta la montaña.
  let entrada = 0;
  for (let k = 0; en && k < en.s.length; k += 2) {
    const s = r.sendas[en.s[k]!]!, v = en.s[k + 1]!;
    const ax = s.puntos[2 * v - 2]!, ay = s.puntos[2 * v - 1]!, bx = s.puntos[2 * v]!, by = s.puntos[2 * v + 1]!, dx = bx - ax, dy = by - ay;
    const l2 = dx * dx + dy * dy, t = l2 ? Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / l2)) : 0;
    const d = Math.hypot(p.x - ax - t * dx, p.y - ay - t * dy);
    if (d >= s.fondo + s.ladera) continue;
    const g = (1 - suave((d - s.fondo) / s.ladera)) ** 1.6;
    let fondo = s.cotas[v - 1]! * (1 - t) + s.cotas[v]! * t;
    // Valle de entrada: el fondo baja del altiplano al terreno de fuera a la vez que la región se apaga, y corta lo que
    // haya en la subida (también una cordillera vieja).
    if (s.entrada) { fondo = fondo * peso + Math.min(e, fondo) * (1 - peso); entrada = Math.max(entrada, g); }
    if (h > fondo || s.entrada) h += (fondo - h) * g;
  }
  // Altiplanos: suelo casi llano con lomas suaves; el borde es corto y deja fuera la ladera de la montaña.
  for (let k = 0; en && k < en.a.length; k++) {
    const a = r.altiplanos[en.a[k]!]!, ca = Math.cos(a.giro), sa = Math.sin(a.giro), dx = p.x - a.x, dy = p.y - a.y;
    const u = (dx * ca + dy * sa) / a.rx, w = (-dx * sa + dy * ca) / a.ry;
    const ex = Math.hypot(u, w) * (1 + 0.22 * (evaluarRuido(r.borde, { x: p.x * 9 + a.x, y: p.y * 9 }) - 0.5) * 2);
    if (ex > 1.05) continue;
    const g = 1 - suave((ex - 0.78) / 0.26);
    h += (a.piso + C.lomas * (evaluarRuido(r.lomas, p) - 0.5) * 2 + 0.0025 * ex * ex - h) * g;
  }
  // Ríos y lagos: el valle que ya tenían (`e`) manda donde quede más bajo.
  let valle = 0, cauce = Infinity;
  const semiancho = C.valleRio * U, cx = Math.floor(p.x / celda), cy = Math.floor(p.y / celda);
  for (const k of r.indiceRios[`${cx},${cy}`] ?? []) {
    const ax = r.rios[4 * k]!, ay = r.rios[4 * k + 1]!, bx = r.rios[4 * k + 2]!, by = r.rios[4 * k + 3]!, dx = bx - ax, dy = by - ay;
    const l = dx * dx + dy * dy, t = l ? Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / l)) : 0;
    const d2 = (p.x - ax - t * dx) ** 2 + (p.y - ay - t * dy) ** 2;
    valle = Math.max(valle, Math.exp(-d2 / (semiancho * semiancho)));
    cauce = Math.min(cauce, d2);
  }
  for (const l of r.lagos) {
    const d = Math.hypot(p.x - l.x, p.y - l.y);
    if (d < l.r + C.margenLago * U) valle = Math.max(valle, 1 - suave((d - l.r) / (C.margenLago * U)));
  }
  if (valle > 0 && e < h) h += (e - h) * valle;
  // Si un altiplano o un paso queda por debajo de un río, el cauce conserva su cota (el río sigue bajando): un dique estrecho.
  else if (cauce < Infinity && e > h) h += (e - h) * Math.exp(-cauce / (REGION_ALPINA.cauce * U) ** 2);
  return e + (h - e) * Math.max(peso, entrada * Math.min(1, 3 * peso));
}
