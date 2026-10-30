// Detalle del relieve a escala de pantalla: la receta de la mesa de escalas 3D (Consideraciones/Escalas_Mundo_3D_Definicion.md
// §3.1). Crestas y valles en lo que sube por encima del suelo, colinas en el piedemonte, lomas en el llano y valles en los
// ríos. Se pone encima del relieve regional cuando ya existen ríos, lagos y costas: no mueve la geografía, le da forma.
// Las alturas son las del relieve regional (cumbres hasta unos 500 m sobre el llano), que ya son las de la mesa con altura ×2.
import type { Point, RioZona } from './tipos';
import type { BarreraRegional } from './regional';
import { FACTOR } from './campamentos';
import { createRng, derivarSeed } from './rng';
import { evaluarRuido, generarCampoRuido, ruidoGradiente, type CampoRuido } from './ruido';

export const RELIEVE_DETALLE = {
  revision: 'detalle-1',
  /** «Tamaño del relieve» elegido en la mesa: multiplica todas las longitudes de onda. */
  escala: 2.2,
  /** Elevación (0-1, 1 = 1.000 m) desde la que se talla el macizo, y ancho de la transición desde el llano. Por debajo,
   * las tierras altas llevan colinas: son los corredores suaves entre regiones y con crestas se vuelven laberintos. */
  suelo: 0.55,
  transicion: 0.08,
  /** Fondo de los valles: el suelo más esta fracción de lo que el relieve regional sube por encima (crece siempre con
   * él, así que un río del macizo sigue bajando). */
  fondo: 0.35,
  /** Amplitud de las crestas sobre el fondo: un mínimo en todo el macizo más este múltiplo de lo que el relieve
   * regional sube por encima del suelo (los núcleos de las cordilleras, más altos). */
  amplitudMinima: 0.4,
  realce: 1.0,
  /** Rango útil del ruido de crestas (percentiles 15 y 90 medidos): se estira a 0-1 y se afila con la potencia. */
  crestaMin: 0.15,
  crestaMax: 0.9,
  afilado: 1.5,
  /** Desde aquí la elevación se comprime suavemente para no pasar de 1 (no hay mesetas recortadas). */
  techo: 0.95,
  /** Longitudes de onda, en unidades estratégicas (× escala). */
  ondaCrestas: 42,
  ondaModulacion: 70,
  ondaColinas: 14,
  ondaLomas: 6,
  /** Alturas, en elevación (79 m y 8 m: las de la mesa con relieve ×2,2 y altura ×2). */
  alturaColinas: 0.079,
  alturaLomas: 0.008,
  /** Por debajo de esta elevación (la orilla) no se añade nada: la costa no se mueve. */
  orilla: 0.4,
  /** Valles de los ríos: semiancho en u (× escala) en el llano y en el macizo, y cuánto se hunden. */
  valleLlano: 4,
  valleMacizo: 10,
  hondoValle: 1,
  /** Lado de la celda del índice de tramos de río, en unidades del generador. */
  celdaRios: 300,
  /** Los pasos de las barreras regionales se protegen con su mismo corredor: dentro no hay detalle y queda el collado
   * del relieve regional (si no, las crestas cierran el paso entre regiones); fuera se recupera en este múltiplo del
   * radio del paso (a lo largo) y del ancho de la barrera (a lo ancho). */
  bordePaso: 0.5,
} as const;

/** Campos de ruido e índice de ríos: datos puros, se consulta con `aplicarDetalle`. */
export interface DetalleRelieve {
  revision: string;
  crestas: CampoRuido;
  modulacion: CampoRuido;
  colinas: CampoRuido;
  lomas: CampoRuido;
  /** Tramos de río aplanados: ax, ay, bx, by. */
  tramos: number[];
  indice: Record<string, number[]>;
  /** Pasos de las barreras aplanados: cx, cy, tx, ty, radio, ancho de la barrera. */
  pasos: number[];
  /** Fuerza del detalle (1 = la receta): baja si con ella alguna región se queda sin conexión por tierra. */
  fuerza: number;
}

const suave = (t: number) => { const x = Math.min(1, Math.max(0, t)); return x * x * (3 - 2 * x); };

export function crearDetalleRelieve(seed: number, rios: readonly RioZona[], barreras: readonly BarreraRegional[] = [], fuerza = 1): DetalleRelieve {
  const C = RELIEVE_DETALLE, rng = createRng(derivarSeed(seed, 'relieve-detalle-1'));
  const onda = (u: number) => 1 / (u * C.escala * FACTOR);
  const crestas = generarCampoRuido(rng, { octavas: 4, frecuenciaBase: onda(C.ondaCrestas), lacunaridad: 2.1, persistencia: 0.5 });
  const modulacion = generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(C.ondaModulacion), lacunaridad: 2.03, persistencia: 0.5 });
  const colinas = generarCampoRuido(rng, { octavas: 3, frecuenciaBase: onda(C.ondaColinas), lacunaridad: 2.03, persistencia: 0.5 });
  // Dos octavas: con una tercera, las lomas del llano dan microladeras que se suman a las del relieve regional.
  const lomas = generarCampoRuido(rng, { octavas: 2, frecuenciaBase: onda(C.ondaLomas), lacunaridad: 2.03, persistencia: 0.5 });
  const tramos: number[] = [], indice: Record<string, number[]> = {};
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, k = tramos.length / 4;
    tramos.push(a.x, a.y, b.x, b.y);
    for (let y = Math.floor(Math.min(a.y, b.y) / C.celdaRios); y <= Math.floor(Math.max(a.y, b.y) / C.celdaRios); y++)
      for (let x = Math.floor(Math.min(a.x, b.x) / C.celdaRios); x <= Math.floor(Math.max(a.x, b.x) / C.celdaRios); x++) (indice[`${x},${y}`] ??= []).push(k);
  }
  const pasos = barreras.flatMap(b => b.pasos.flatMap(p => [p.centro.x, p.centro.y, p.tangente.x, p.tangente.y, p.radio, b.ancho]));
  return { revision: C.revision, crestas, modulacion, colinas, lomas, tramos, indice, pasos, fuerza };
}

/** Cuánto de `p` es corredor de paso (0-1): 1 en el collado y el valle de aproximación del relieve regional, y una
 * transición de `bordePaso` alrededor. */
function enPaso(d: DetalleRelieve, p: Point): number {
  const b = RELIEVE_DETALLE.bordePaso;
  let v = 0;
  for (let i = 0; i < d.pasos.length; i += 6) {
    const dx = p.x - d.pasos[i]!, dy = p.y - d.pasos[i + 1]!, tx = d.pasos[i + 2]!, ty = d.pasos[i + 3]!, radio = d.pasos[i + 4]!, ancho = d.pasos[i + 5]!;
    const transversal = Math.abs(-dx * ty + dy * tx), longitudinal = Math.abs(dx * tx + dy * ty);
    const largo = 1 - suave((longitudinal - (radio * 0.42 + transversal * 0.24)) / (b * radio));
    const anchura = 1 - suave((transversal - ancho * 2.8) / (b * ancho * 2.8));
    v = Math.max(v, largo * anchura);
  }
  return v;
}

/** Crestas (ruido «ridged»): 1 en la línea de cumbre, cerca de 0 en las vaguadas; cada octava pesa según la anterior. */
function crestas(r: CampoRuido, p: Point): number {
  let a = 1, f = r.frecuenciaBase, s = 0, t = 0, w = 1;
  for (let i = 0; i < r.octavas; i++) {
    const o = r.desplazamientos[i]!;
    let v = 1 - Math.abs(ruidoGradiente(r.permutacion, p.x * f + o.dx, p.y * f + o.dy) / Math.SQRT1_2);
    v *= v * w;
    w = Math.min(1, v * 1.8);
    s += a * v; t += a; a *= r.persistencia; f *= r.lacunaridad;
  }
  return s / t;
}

/** Cuánto se hunde un valle de río en `p` (0-1): campana de semiancho que crece del llano al macizo. */
function valle(d: DetalleRelieve, p: Point, macizo: number): number {
  const C = RELIEVE_DETALLE, semiancho = (C.valleLlano + (C.valleMacizo - C.valleLlano) * macizo) * C.escala * FACTOR;
  const alcance = 2.5 * semiancho, r = Math.ceil(alcance / C.celdaRios), cx = Math.floor(p.x / C.celdaRios), cy = Math.floor(p.y / C.celdaRios);
  let d2 = alcance * alcance;
  for (let y = cy - r; y <= cy + r; y++) for (let x = cx - r; x <= cx + r; x++) for (const k of d.indice[`${x},${y}`] ?? []) {
    const ax = d.tramos[4 * k]!, ay = d.tramos[4 * k + 1]!, bx = d.tramos[4 * k + 2]!, by = d.tramos[4 * k + 3]!, dx = bx - ax, dy = by - ay;
    const l = dx * dx + dy * dy, t = l ? Math.max(0, Math.min(1, ((p.x - ax) * dx + (p.y - ay) * dy) / l)) : 0;
    const ex = p.x - ax - t * dx, ey = p.y - ay - t * dy, e2 = ex * ex + ey * ey;
    if (e2 < d2) d2 = e2;
  }
  return Math.exp(-d2 / (semiancho * semiancho));
}

/** Elevación con el detalle, a partir de la del relieve regional (`base`, 0-1) en `p`. Pura.
 * Fondo (sigue al relieve regional y crece con él) + crestas, colinas y lomas, que se apagan sobre los cauces y en los
 * pasos: en un río queda solo el fondo, así que el río sigue bajando como bajaba. */
export function aplicarDetalle(d: DetalleRelieve, p: Point, base: number): number {
  const C = RELIEVE_DETALLE;
  if (base < C.orilla) return base;
  const abierto = 1 - enPaso(d, p);
  const macizo = suave((base - C.suelo) / C.transicion) * abierto, pie = suave((base - 0.47) / 0.04) * (1 - macizo) * abierto;
  const llano = suave((base - C.orilla) / 0.03) * (1 - macizo) * abierto;
  const seco = macizo > 0 || pie > 0 || llano > 0 ? 1 - C.hondoValle * valle(d, p, macizo) : 1;
  let e = base > C.suelo ? C.suelo + (base - C.suelo) * C.fondo : base;
  if (macizo > 0) {
    // Crestas y vaguadas con relieve propio en todo el macizo; la modulación hace unas sierras más altas que otras.
    const k = 0.6 + 0.4 * suave((evaluarRuido(d.modulacion, p) - 0.3) / 0.4);
    const q = Math.min(1, Math.max(0, (crestas(d.crestas, p) - C.crestaMin) / (C.crestaMax - C.crestaMin)));
    e += d.fuerza * macizo * (C.amplitudMinima + C.realce * (base - C.suelo)) * q ** C.afilado * k * seco;
  }
  if (pie > 0) e += d.fuerza * C.alturaColinas * pie * Math.max(0, (evaluarRuido(d.colinas, p) - 0.5) * 1.8 + 0.35) * seco;
  if (llano > 0) e += d.fuerza * C.alturaLomas * (evaluarRuido(d.lomas, p) - 0.5) * 2 * llano * seco;
  return e > C.techo ? C.techo + (1 - C.techo) * Math.tanh((e - C.techo) / (1 - C.techo)) : e;
}
