import type { LagoZona, Point, RioZona } from './tipos';
import { evaluarElevacion, type CampoElevacion, type CauceTroncal } from './elevacion';
import { generarRiosPorDrenaje, type MallaDrenaje } from './drenaje';
import { createRng, derivarSeed } from './rng';
import { puntoEnLago } from './aguaRegional';
import { enPaso } from './relieveDetalle';
import { ELEVACION } from './config';
import { brazosTrenzados, lagoDeBrazo, migrar, MIGRACION, TRENZAS, zonasTrenzadas, type BrazoMuerto } from './fluvial';
import { alturaFluvial, aplanarFranja, aplicarCauces, celdasDeLinea, crearCaucesFinos, desaguar, erosionarCorriente, EROSION_CORRIENTE, type RejillaFluvial } from './rejillaFluvial';

export interface RioRegional extends RioZona { orden: number; caudal: number[]; anchos: number[] }

/** Geometría hidráulica (Leopold y Maddock): el ancho del cauce crece con la raíz del caudal. Unidades del generador
 * (metros) y área drenada en hectáreas como caudal relativo. `k` deja en 0,4 u (5 m) las cabeceras, en unas 2 u (25 m) un río de 2700 ha y en 5 u (60 m) el troncal en su boca (unas 15000 ha). */
export const CAUCE = { k: 0.5, minimo: 4.8, maximo: 60 };
/** El cauce fino de cada río publicado. */
export const CAUCE_TALLADO = {
  /** Lo hondo del cauce fino bajo la rejilla (`rejillaFluvial.ts`, `CaucesFinos`), en m: `ancho · [2]`, entre [0] y [1]. */
  hondo: [0.5, 3, 0.05] as const };

export const anchoDeCaudal = (hectareas: number): number => Math.min(CAUCE.maximo, Math.max(CAUCE.minimo, CAUCE.k * Math.sqrt(hectareas)));

/** El trazado de cada cauce y su tallado en la rejilla. Metros salvo donde se dice.
 * - `simplificar`: tolerancia de Douglas-Peucker que borra la escalera de la rejilla; `chaikin`, vueltas de recorte de esquinas;
 *   `paso`, la separación de los puntos del trazado (los publicados son los centros de las celdas por las que pasa).
 * - `bajada`: lo mínimo que baja el lecho por metro (elevación normalizada), para que cada celda del cauce quede por debajo de la anterior.
 * - `publicar`: la tolerancia (en celdas) con que se simplifican los centros de celda publicados (quita la escalera y los quiebros de celda). */
export const TALLADO = {
  simplificar: 30, chaikin: 3, paso: 45, bajada: 1e-7, publicar: 0.75,
};
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/** Douglas-Peucker: los puntos que quedan para no apartarse más de `tolerancia` del trazado (siempre los extremos). */
function simplificar(puntos: readonly Point[], tolerancia: number): number[] {
  const quedan = new Uint8Array(puntos.length);
  quedan[0] = quedan[puntos.length - 1] = 1;
  const pila: [number, number][] = [[0, puntos.length - 1]];
  while (pila.length) {
    const [a, b] = pila.pop()!, pa = puntos[a]!, pb = puntos[b]!, dx = pb.x - pa.x, dy = pb.y - pa.y, l = Math.hypot(dx, dy) || 1;
    let peor = -1, lejos = tolerancia;
    for (let i = a + 1; i < b; i++) {
      const d = Math.abs((puntos[i]!.x - pa.x) * dy - (puntos[i]!.y - pa.y) * dx) / l;
      if (d > lejos) { lejos = d; peor = i; }
    }
    if (peor >= 0) { quedan[peor] = 1; pila.push([a, peor], [peor, b]); }
  }
  return [...quedan.keys()].filter(i => quedan[i]);
}

/** El trazado de un tramo: sin la escalera de la rejilla (Douglas-Peucker), con las esquinas redondeadas (Chaikin) y remuestreado cada
 * `paso`. Los extremos (nacimientos, confluencias, desembocaduras) no se mueven: así los tramos siguen unidos por su punto común.
 * El caudal va con los puntos, interpolado. */
export function trazarCauce(puntos: readonly Point[], caudal: readonly number[], o = TALLADO): { puntos: Point[]; caudal: number[] } {
  if (puntos.length < 3) return { puntos: [...puntos], caudal: [...caudal] };
  const indices = simplificar(puntos, o.simplificar);
  let ps = indices.map(i => puntos[i]!), qs = indices.map(i => caudal[i]!);
  for (let vuelta = 0; vuelta < o.chaikin && ps.length > 2; vuelta++) {
    const np: Point[] = [ps[0]!], nq: number[] = [qs[0]!];
    for (let i = 0; i + 1 < ps.length; i++) {
      const a = ps[i]!, b = ps[i + 1]!;
      for (const t of [0.25, 0.75]) {
        if ((t === 0.25 && i === 0) || (t === 0.75 && i === ps.length - 2)) continue;
        np.push({ x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) }); nq.push(lerp(qs[i]!, qs[i + 1]!, t));
      }
    }
    np.push(ps[ps.length - 1]!); nq.push(qs[qs.length - 1]!);
    ps = np; qs = nq;
  }
  const s = [0];
  for (let i = 1; i < ps.length; i++) s.push(s[i - 1]! + Math.hypot(ps[i]!.x - ps[i - 1]!.x, ps[i]!.y - ps[i - 1]!.y));
  const total = s[s.length - 1]!, n = Math.max(1, Math.round(total / o.paso));
  const salida: Point[] = [puntos[0]!], q: number[] = [caudal[0]!];
  for (let k = 1, j = 0; k < n; k++) {
    const d = (total * k) / n;
    while (s[j + 1]! < d) j++;
    const t = (d - s[j]!) / (s[j + 1]! - s[j]! || 1);
    salida.push({ x: lerp(ps[j]!.x, ps[j + 1]!.x, t), y: lerp(ps[j]!.y, ps[j + 1]!.y, t) }); q.push(lerp(qs[j]!, qs[j + 1]!, t));
  }
  salida.push(puntos[puntos.length - 1]!); q.push(caudal[caudal.length - 1]!);
  return { puntos: salida, caudal: q };
}

/** Suavizado local con retroceso: una esquina inválida no cancela el resto.
 * Conserva extremos/confluencias y comprueba cada segmento modificado cada 25 m. */
export function suavizarCauceRegional(puntos: Point[], altura: (p: Point) => number): Point[] {
  const salida = puntos.map(p => ({ ...p }));
  const baja = (a: Point, b: Point) => {
    const pasos = Math.max(4, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / 25));
    let anterior = altura(a);
    if (altura(b) >= anterior) return false;
    for (let j = 1; j <= pasos; j++) {
      const h = altura({ x: a.x + (b.x - a.x) * j / pasos, y: a.y + (b.y - a.y) * j / pasos });
      if (h > anterior + 1e-7) return false;
      anterior = h;
    }
    return true;
  };
  for (let pasada = 0; pasada < 3; pasada++) for (let k = 1; k < salida.length - 1; k++) {
    const i = pasada % 2 ? salida.length - 1 - k : k;
    const a = salida[i - 1]!, p = salida[i]!, b = salida[i + 1]!;
    for (const peso of [0.5, 0.25, 0.125]) {
      const q = { x: p.x + ((a.x + b.x) / 2 - p.x) * peso, y: p.y + ((a.y + b.y) / 2 - p.y) * peso };
      if (baja(a, q) && baja(q, b)) { salida[i] = q; break; }
    }
  }
  return salida;
}

function vallesPrincipales(campo: CampoElevacion): CauceTroncal[] {
  const { ancho, alto } = campo.regional.limites;
  const fase = campo.regional.fase;
  const rutas = [
    [[0.54, 0.995], [0.535, 0.94], [0.56, 0.88], [0.54, 0.82], [0.55, 0.76], [0.53, 0.68]],
    [[0.955, 0.36], [0.90, 0.46], [0.93, 0.55], [0.91, 0.67], [0.94, 0.78], [0.90, 0.87], [0.92, 1]],
  ];
  return rutas.map((ruta, id) => {
    const puntos: Point[] = [];
    for (let i = 0; i < ruta.length - 1; i++) {
      const a = ruta[Math.max(0, i - 1)]!, b = ruta[i]!, c = ruta[i + 1]!, d = ruta[Math.min(ruta.length - 1, i + 2)]!;
      for (let j = 0; j < 8; j++) {
        const t = j / 8, eje = (k: number) => 0.5 * (2 * b[k]! + (-a[k]! + c[k]!) * t
          + (2 * a[k]! - 5 * b[k]! + 4 * c[k]! - d[k]!) * t * t + (-a[k]! + 3 * b[k]! - 3 * c[k]! + d[k]!) * t * t * t);
        puntos.push({ x: (eje(0) + Math.sin((i + t) * 2 + fase) * 0.003 * Math.sin(Math.PI * (i + t) / (ruta.length - 1))) * ancho, y: eje(1) * alto });
      }
    }
    const ultimo = ruta[ruta.length - 1]!; puntos.push({ x: ultimo[0]! * ancho, y: ultimo[1]! * alto });
    return { puntos, cotas: puntos.map((_, i) => (id === 0 ? 0.447 : 0.493) - i / (puntos.length - 1) * (id === 0 ? 0.08 : 0.066)),
      radio: (id === 0 ? 750 : 900) * Math.min(ancho, alto) / 24000,
      caja: { minX: Math.min(...puntos.map(p => p.x)), maxX: Math.max(...puntos.map(p => p.x)), minY: Math.min(...puntos.map(p => p.y)), maxY: Math.max(...puntos.map(p => p.y)) } };
  });
}


/** Meandros del llano (experimento/rios, fase 4): curva de Kinoshita (Parker 1983), `θ(φ) = θ0·sen φ + θ0³·(Js·cos 3φ − Jf·sen 3φ)`,
 * llevada como desvío lateral sobre el trazado. La longitud de onda es `onda` anchos y la amplitud, `amplitud` anchos; los dos se
 * apagan con la pendiente (por encima de `pendiente` no hay meandro), con la altura (de `alturaPlena` a `alturaNula`) y en los extremos
 * del tramo, que no se mueven. Sin meandro si alguno caería en el agua. */
export const MEANDROS = { onda: 12, amplitud: 3, theta: 1.25, Js: 0.05, Jf: 0.03, pendiente: 0.012, alturaPlena: 0.43, alturaNula: 0.5 };

/** Un periodo de la curva lateral de Kinoshita, normalizado a ±1 (desvío lateral por fase). */
const KINOSHITA = (() => {
  const n = 256, { theta: t0, Js, Jf } = MEANDROS, y: number[] = [0];
  for (let k = 1; k <= n; k++) {
    const f = ((k - 0.5) / n) * 2 * Math.PI;
    y.push(y[k - 1]! + Math.sin(t0 * Math.sin(f) + t0 ** 3 * (Js * Math.cos(3 * f) - Jf * Math.sin(3 * f))));
  }
  const media = y.reduce((s, v) => s + v, 0) / y.length, max = Math.max(...y.map(v => Math.abs(v - media)));
  return y.map(v => (v - media) / max);
})();
const kinoshita = (fase: number) => {
  const f = (((fase / (2 * Math.PI)) % 1) + 1) % 1 * (KINOSHITA.length - 1), i = Math.floor(f);
  return lerp(KINOSHITA[i]!, KINOSHITA[Math.min(i + 1, KINOSHITA.length - 1)]!, f - i);
};

/** Con cada punto, `valle`: la altura del fondo del valle a esa altura del tramo (la del trazado sin meandro, que va por el fondo) y
 * `cinturon`: lo que se aparta ahí el meandro como mucho (su amplitud). El tallado hace con ellos la vega plana de todo el cinturón: el
 * meandro no se sube a las laderas del valle. */
export function meandrear(puntos: readonly Point[], caudal: readonly number[], altura: (p: Point) => number): { puntos: Point[]; caudal: number[]; valle: number[]; cinturon: number[] } {
  const n = puntos.length, quieto = () => ({ puntos: [...puntos], caudal: [...caudal], valle: puntos.map(altura), cinturon: puntos.map(() => 0) });
  if (n < 4) return quieto();
  const s = [0];
  for (let i = 1; i < n; i++) s.push(s[i - 1]! + Math.hypot(puntos[i]!.x - puntos[i - 1]!.x, puntos[i]!.y - puntos[i - 1]!.y));
  const total = s[n - 1]!, h = puntos.map(altura);
  const en = (xs: readonly number[], d: number) => {
    let j = 0;
    while (j < n - 2 && s[j + 1]! < d) j++;
    return lerp(xs[j]!, xs[j + 1]!, Math.max(0, Math.min(1, (d - s[j]!) / (s[j + 1]! - s[j]! || 1))));
  };
  const xs = puntos.map(p => p.x), ys = puntos.map(p => p.y);
  // la pendiente, en una ventana de 300 m
  const pendienteEn = (d: number) => Math.max(0, (en(h, Math.max(0, d - 150)) - en(h, Math.min(total, d + 150))) * 1000 / (Math.min(total, d + 150) - Math.max(0, d - 150) || 1));
  const anchoEn = (d: number) => anchoDeCaudal(en(caudal, d));
  const fuerza = (d: number) => {
    const llano = Math.max(0, Math.min(1, 1 - pendienteEn(d) / MEANDROS.pendiente));
    const bajo = Math.max(0, Math.min(1, (MEANDROS.alturaNula - en(h, d)) / (MEANDROS.alturaNula - MEANDROS.alturaPlena)));
    const extremos = Math.min(1, Math.min(d, total - d) / (MEANDROS.onda * anchoEn(d)));
    return llano * bajo * extremos;
  };
  // fase inicial fija por tramo (de su primer punto): el mismo mundo da los mismos meandros
  let fase = ((Math.sin(puntos[0]!.x * 12.9898 + puntos[0]!.y * 78.233) * 43758.5453) % 1 + 1) % 1 * 2 * Math.PI;
  const salida: Point[] = [puntos[0]!], q: number[] = [caudal[0]!], valle: number[] = [h[0]!], cinturon: number[] = [0];
  let hubo = false;
  for (let d = 0; d < total;) {
    const ancho = anchoEn(d), onda = MEANDROS.onda * ancho, paso = Math.max(10, Math.min(TALLADO.paso, onda / 10));
    d = Math.min(total, d + paso);
    if (d >= total) break;
    fase += (2 * Math.PI * paso) / onda;
    // la normal del trazado, de un cuarto de onda antes a uno después (no la del segmento, que se quiebra en cada punto)
    const a = Math.max(0, d - onda / 4), b = Math.min(total, d + onda / 4), tx = en(xs, b) - en(xs, a), ty = en(ys, b) - en(ys, a), l = Math.hypot(tx, ty) || 1;
    const amplitud = MEANDROS.amplitud * ancho * fuerza(d), desvio = amplitud * kinoshita(fase);
    if (Math.abs(desvio) > 1e-3) hubo = true;
    const p = { x: en(xs, d) - (ty / l) * desvio, y: en(ys, d) + (tx / l) * desvio };
    if (Math.abs(desvio) > 1e-3 && altura(p) < 0.375) return quieto();
    salida.push(p); q.push(en(caudal, d)); valle.push(en(h, d)); cinturon.push(amplitud);
  }
  salida.push(puntos[n - 1]!); q.push(caudal[n - 1]!); valle.push(h[n - 1]!); cinturon.push(0);
  return hubo ? { puntos: salida, caudal: q, valle, cinturon } : quieto();
}

/** Orilla de un lago a una cota dada: 64 rayos desde el fondo hasta donde el terreno supera el nivel. Sin orilla cerrada (un rayo se
 * escapa, toca el borde o un puerto), no hay lago. */
function orillaANivel(campo: CampoElevacion, id: string, centro: Point, nivel: number, escala: number, aceptar: (p: Point) => boolean): LagoZona | undefined {
  const puntos: Point[] = [];
  let radio = 0;
  const { ancho, alto } = campo.regional.limites;
  for (let k = 0; k < 64; k++) {
    const angulo = k * Math.PI * 2 / 64;
    const p = (r: number) => ({ x: centro.x + Math.cos(angulo) * r, y: centro.y + Math.sin(angulo) * r });
    let r = 25 * escala;
    while (r < 1400 * escala && evaluarElevacion(campo, p(r)) < nivel) r += 25 * escala;
    if (r >= 1400 * escala) return undefined;
    let a = Math.max(0, r - 25 * escala), b = r;
    for (let j = 0; j < 10; j++) { const m = (a + b) / 2; if (evaluarElevacion(campo, p(m)) < nivel) a = m; else b = m; }
    const borde = p((a + b) / 2);
    if (!aceptar(borde) || borde.x <= 0 || borde.y <= 0 || borde.x >= ancho || borde.y >= alto) return undefined;
    puntos.push(borde); radio = Math.max(radio, b);
  }
  return radio > 30 * escala ? { id, centro, radio: radio + 1, contorno: puntos, elevacionCentro: nivel } : undefined;
}


/** La rejilla erosionada de cada relieve regional: no depende de los lagos, así que las repeticiones (con `factorLagos`) la reutilizan. */
const erosionadas = new WeakMap<object, { columnas: number; filas: number; celda: number; mar: Uint8Array; fijo: Uint8Array; erosionada: Float64Array }>();

/** Un tramo de la red, de confluencia a confluencia: sus celdas (las del drenaje), su caudal por celda, sus extremos y su forma. */
interface Tramo {
  celdas: number[]; caudal: number[]; desde: number; hasta: number; orden: number; fin: RioZona['fin']; terminaEnLago: boolean;
  /** El trazado sin meandro (por el fondo del valle) y el definitivo, con su caudal, cinturón de meandros, trenzas y brazos muertos. */
  base: Point[]; trazado: Point[]; caudalTrazado: number[]; cinturon: number[]; trenza: number[];
  trenzas: { brazo: Point[]; caudal: number }[]; muertos: BrazoMuerto[];
}

/** La hidrografía sobre una sola rejilla de alturas (`rejillaFluvial.ts`), fuente de verdad del relieve de los ríos:
 * 1. el relieve regional (con los valles troncales) se muestrea en la rejilla y se erosiona (potencia de la corriente y difusión);
 * 2. se drena: los ríos nacen en la montaña y bajan por los receptores; los lagos son las hoyas hondas que cruzan, con desagüe;
 * 3. la red se parte en tramos de confluencia a confluencia, con su caudal y su orden de Strahler;
 * 4. cada tramo toma su forma (suavizado, meandros en el llano que migran y dejan brazos muertos, trenzas donde baja con pendiente media);
 * 5. en la rejilla se tallan las vegas de los cinturones de meandros y los lechos de grava, y luego cada cauce, de arriba abajo por la red:
 *    cada celda de un cauce queda por debajo de la anterior. Los ríos publicados son los centros de esas celdas, así que bajan por
 *    construcción en el relieve continuo, que interpola la rejilla pasando por sus nodos (`campo.fluvial`). */
export function generarHidrografiaRegional(seed: number, campo: CampoElevacion, factorLagos = 1) {
  const limites = campo.regional.limites, escala = Math.min(limites.ancho, limites.alto) / 24000;
  campo.caucesTroncales = vallesPrincipales(campo);
  campo.fluvial = undefined;
  campo.cauces = undefined;
  // 1. la rejilla, erosionada (una vez por relieve regional)
  let cache = erosionadas.get(campo.regional);
  if (!cache) {
    const celda = EROSION_CORRIENTE.celda * escala, columnas = Math.ceil(limites.ancho / celda), filas = Math.ceil(limites.alto / celda), n = columnas * filas;
    const base = new Float64Array(n), mar = new Uint8Array(n), fijo = new Uint8Array(n), sinLagos = { ...campo, lagos: [] };
    for (let k = 0; k < n; k++) {
      const x = k % columnas, y = (k - x) / columnas;
      base[k] = evaluarElevacion(sinLagos, { x: (x + 0.5) * celda, y: (y + 0.5) * celda });
      mar[k] = Number(base[k]! < ELEVACION.umbralAgua);
      fijo[k] = Number(mar[k] || x === 0 || y === 0 || x === columnas - 1 || y === filas - 1);
    }
    const erosionada = Float64Array.from(base);
    erosionarCorriente(erosionada, columnas, filas, celda, fijo);
    cache = { columnas, filas, celda, mar, fijo, erosionada };
    erosionadas.set(campo.regional, cache);
  }
  const { columnas, filas, celda, mar, fijo } = cache, n = columnas * filas, H = Float64Array.from(cache.erosionada);
  const rejilla: RejillaFluvial = { celda, columnas, filas, altura: H as unknown as Float32Array };
  // desde aquí, el relieve que ven los lagos y los meandros es el de la rejilla (que se va tallando)
  campo.fluvial = rejilla;
  const altura = (p: Point) => evaluarElevacion(campo, p);
  const centro = (k: number): Point => ({ x: ((k % columnas) + 0.5) * celda, y: (Math.floor(k / columnas) + 0.5) * celda });
  const indice = (p: Point) => Math.min(filas - 1, Math.max(0, Math.floor(p.y / celda))) * columnas + Math.min(columnas - 1, Math.max(0, Math.floor(p.x / celda)));
  // 2. el drenaje
  const desague = desaguar(H, columnas, filas, fijo);
  const malla: MallaDrenaje = { columnas, filas, celda, altura: H, agua: mar, montana: Uint8Array.from(H, h => Number(h >= ELEVACION.umbralMontana)), desague };
  const aceptar = (p: Point) => campo.regional.barreras.every(b => b.pasos.every(paso => {
    const dx = p.x - paso.centro.x, dy = p.y - paso.centro.y;
    return Math.abs(dx * paso.tangente.x + dy * paso.tangente.y) > 600 * escala
      || Math.abs(-dx * paso.tangente.y + dy * paso.tangente.x) > b.ancho * 2.8 + 300 * escala;
  }));
  // Una hoya pegada al valle troncal es parte de ese valle, no un lago que corte su lámina.
  const junto = Math.min(limites.ancho, limites.alto) * 0.014;
  const cercaDeTroncal = (p: Point) => campo.caucesTroncales!.some(cauce => cauce.puntos.slice(1).some((q, k) => {
    const a = cauce.puntos[k]!, dx = q.x - a.x, dy = q.y - a.y, t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
    return Math.hypot(a.x + dx * t - p.x, a.y + dy * t - p.y) < cauce.radio + junto;
  }));
  const aceptados: LagoZona[] = [];
  // los troncales nacen siempre, en su primera celda de tierra
  const forzados = campo.caucesTroncales.map(c => c.puntos.map(indice).find(k => !fijo[k])).filter((k): k is number => k !== undefined);
  const { trazas, lagos } = generarRiosPorDrenaje(createRng(derivarSeed(seed, 'regional-drenaje')), malla, {
    cantidad: 65, largoMinimo: 1200 * escala, espacioNacimientos: 1100 * escala, forzados,
    // El lago llena la hoya hasta casi su desagüe (menos con `factorLagos`, si cortaba el paso por tierra).
    lago: (c, nivel) => {
      if (!aceptar(c) || cercaDeTroncal(c)) return undefined;
      const fondo = altura(c), lago = orillaANivel(campo, `lago-${aceptados.length}`, c, fondo + (nivel - fondo) * 0.85 * factorLagos, escala, aceptar);
      if (!lago || aceptados.some(l => Math.hypot(l.centro.x - lago.centro.x, l.centro.y - lago.centro.y) < l.radio + lago.radio)) return undefined;
      aceptados.push(lago);
      return lago;
    },
  });
  // 3. la red: vértices por celda; el caudal, el área drenada (ha); Strahler de arriba abajo
  const ha = (celda * celda) / 10000;
  const vertices = new Map<number, { siguiente?: number; entradas: number[]; orden: number; fin: RioZona['fin']; troncal: boolean; caudal: number }>();
  for (const t of trazas) t.celdas.forEach((k, i) => {
    if (!vertices.has(k)) vertices.set(k, { entradas: [], orden: 1, fin: t.fin, troncal: forzados.includes(k), caudal: 0 });
    if (i + 1 < t.celdas.length) vertices.get(k)!.siguiente = t.celdas[i + 1]!;
  });
  for (const [k, v] of vertices) if (v.siguiente !== undefined) vertices.get(v.siguiente)!.entradas.push(k);
  const porAltura = [...vertices.keys()].sort((a, b) => desague.lleno[b]! - desague.lleno[a]!);
  for (const k of porAltura) {
    const v = vertices.get(k)!, ordenes = v.entradas.map(e => vertices.get(e)!.orden).sort((a, b) => b - a);
    v.orden = Math.max(v.troncal ? 2 : 1, !ordenes.length ? 1 : ordenes[0]! + Number(ordenes.length > 1 && ordenes[0] === ordenes[1]));
    // el caudal: el área drenada de la celda y, como poco, lo que le llega (un río que se une a otro paralelo trae su propia cuenca)
    v.caudal = Math.max(desague.acumulacion[k]! * ha, v.entradas.reduce((suma, e) => suma + vertices.get(e)!.caudal, 0));
  }
  const caudalDe = (k: number) => vertices.get(k)!.caudal;
  // 4. los tramos y su forma
  const tramos: Tramo[] = [];
  for (const [k, v] of vertices) {
    if (v.siguiente === undefined || v.entradas.length === 1) continue;
    const celdas = [k];
    let actual = k;
    while (vertices.get(actual)!.siguiente !== undefined) {
      actual = vertices.get(actual)!.siguiente!; celdas.push(actual);
      if (vertices.get(actual)!.entradas.length !== 1) break;
    }
    const caudal = celdas.map(caudalDe);
    // el caudal no baja aguas abajo; en la confluencia, el afluente lleva lo suyo (la suma empieza en el tramo que sale de ella)
    for (let i = 1; i < caudal.length; i++) caudal[i] = Math.max(caudal[i]!, caudal[i - 1]!);
    const ultimo = vertices.get(actual)!;
    if (ultimo.entradas.length > 1 && caudal.length > 1) caudal[caudal.length - 1] = caudal[caudal.length - 2]!;
    const suave = trazarCauce(celdas.map(centro), caudal), conMeandros = meandrear(suave.puntos, suave.caudal, altura);
    const forma = migrar(conMeandros.puntos, { ...conMeandros, ancho: conMeandros.caudal.map(anchoDeCaudal) }, suave.puntos);
    const anchos = forma.caudal.map(anchoDeCaudal), trenzado = zonasTrenzadas(forma.puntos, forma.valle, anchos, forma.cinturon);
    tramos.push({ celdas, caudal, desde: k, hasta: actual, orden: v.orden, fin: ultimo.siguiente !== undefined ? 'limite' : ultimo.fin,
      terminaEnLago: ultimo.siguiente === undefined && ultimo.fin === 'lago', base: suave.puntos, trazado: forma.puntos, caudalTrazado: forma.caudal,
      cinturon: forma.cinturon, trenza: trenzado.peso, muertos: forma.brazos,
      trenzas: trenzado.zonas.flatMap(([i, j], z) => brazosTrenzados(forma.puntos, anchos, i, j, tramos.length * 7 + z, celda * TRENZAS.aparteCeldas).map(brazo => ({ brazo, caudal: forma.caudal[i]! * TRENZAS.parte }))) });
  }
  // 5. el tallado en la rejilla, de arriba abajo por la red: primero los llanos (vegas y lechos de grava), luego los cauces
  const enRejilla = (p: Point) => alturaFluvial(rejilla, p);
  const nivelBajando = (ps: readonly Point[]) => {
    const nivel: number[] = [];
    ps.forEach((p, i) => nivel.push(Math.min(enRejilla(p), i ? nivel[i - 1]! - TALLADO.bajada * Math.hypot(p.x - ps[i - 1]!.x, p.y - ps[i - 1]!.y) : Infinity)));
    return nivel;
  };
  const leer = (xs: readonly number[], f: number) => xs[Math.min(xs.length - 1, Math.round(f * (xs.length - 1)))]!;
  for (const t of tramos) {
    if (t.cinturon.some(c => c > 0)) {
      // la vega del cinturón de meandros: llana, por el trazado sin meandro y tan ancha como el cinturón
      const medio = t.base.map((_, i) => { const f = i / (t.base.length - 1); return leer(t.cinturon, f) + anchoDeCaudal(leer(t.caudalTrazado, f)); });
      aplanarFranja(H, columnas, filas, celda, t.base, medio, nivelBajando(t.base));
    }
    if (t.trenza.some(w => w > 0)) {
      // el lecho de grava de las trenzas
      const medio = t.trazado.map((_, i) => { const w = anchoDeCaudal(t.caudalTrazado[i]!); return Math.max(w * TRENZAS.lecho / 2, celda * TRENZAS.aparteCeldas + w / 2) * t.trenza[i]!; });
      aplanarFranja(H, columnas, filas, celda, t.trazado, medio, nivelBajando(t.trazado));
    }
  }
  const dueno = new Int32Array(n).fill(-1), vertice: number[][] = [];
  const pendientes = new Map<number, number>();
  for (const t of tramos) pendientes.set(t.hasta, (pendientes.get(t.hasta) ?? 0) + 1);
  const cola = tramos.filter(t => !pendientes.get(t.desde)), hechos = new Set<Tramo>(), tallados: Tramo[] = [];
  const porDesde = new Map<number, Tramo[]>();
  for (const t of tramos) porDesde.set(t.desde, [...(porDesde.get(t.desde) ?? []), t]);
  /** Talla un cauce por sus celdas: cada una queda por debajo de la anterior (y `hondo` bajo su suelo); las de otro cauce se saltan (salvo
   * el final, si `conFin`: una confluencia, que se baja). Devuelve las celdas que son suyas, que serán los puntos publicados. */
  const tallarCauce = (id: number, celdas: readonly number[], hondo: (k: number, f: number) => number, conFin = true): number[] => {
    const suyas = [celdas[0]!];
    let antes = H[celdas[0]!]!;
    for (let q = 1; q < celdas.length; q++) {
      const k = celdas[q]!, fin = conFin && q === celdas.length - 1;
      if (!fin && dueno[k]! >= 0 && dueno[k] !== id) continue;
      const a = centro(suyas[suyas.length - 1]!), b = centro(k);
      const v = Math.min(H[k]!, H[k]! - hondo(k, q / (celdas.length - 1)), antes - TALLADO.bajada * Math.hypot(b.x - a.x, b.y - a.y));
      H[k] = Math.min(H[k]!, v); antes = H[k]!;
      if (dueno[k]! < 0) dueno[k] = id;
      suyas.push(k);
    }
    return suyas;
  };
  while (cola.length) {
    const t = cola.shift()!;
    if (hechos.has(t)) continue;
    hechos.add(t); tallados.push(t);
    const id = tramos.indexOf(t);
    const celdas = celdasDeLinea(t.trazado, celda, columnas, filas);
    celdas[0] = t.desde; celdas[celdas.length - 1] = t.hasta;
    // la línea de celdas solo asegura que la red baja (el desagüe, los lagos); el lecho del río, con su ancho y su hondo, es el cauce fino
    // de su trazado (abajo): un surco aquí sería otro cauce al lado, por donde ya no va el río
    vertice[id] = tallarCauce(id, celdas, () => 0);
    const quedan = (pendientes.get(t.hasta) ?? 0) - 1;
    pendientes.set(t.hasta, quedan);
    if (quedan <= 0) cola.push(...(porDesde.get(t.hasta) ?? []));
  }
  if (hechos.size < tramos.length) throw new Error(`tallar: ${tramos.length - hechos.size} tramos sin orden aguas abajo`);
  // las trenzas: salen de su río y vuelven a él por el lecho de grava; se quedan si llegan bajando
  const trenzas: RioRegional[] = [];
  // lo que se publica de cada tramo: su trazado suave, de la celda de su confluencia de arriba a la de abajo (los tramos que se juntan
  // comparten ese punto), con el caudal de cada punto
  const publicados = tramos.map(t => [centro(t.desde), ...t.trazado.slice(1, -1), centro(t.hasta)]);
  const caudalPublicado = tramos.map(t => { const q = [...t.caudalTrazado]; for (let i = 1; i < q.length; i++) q[i] = Math.max(q[i]!, q[i - 1]!); return q; });
  const cercaEnTrazado = (id: number, q: Point) => {
    const ps = publicados[id]!;
    let mejor = { d: Infinity, i: 0, p: ps[0]! };
    for (let i = 0; i + 1 < ps.length; i++) {
      const a = ps[i]!, b = ps[i + 1]!, dx = b.x - a.x, dy = b.y - a.y, u = Math.max(0, Math.min(1, ((q.x - a.x) * dx + (q.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
      const p = { x: a.x + dx * u, y: a.y + dy * u }, d = Math.hypot(p.x - q.x, p.y - q.y);
      if (d < mejor.d) mejor = { d, i, p };
    }
    return mejor;
  };
  const enTrazado = (id: number, q: Point) => cercaEnTrazado(id, q).p;
  const insertarEnTrazado = (id: number, q: Point) => {
    const ps = publicados[id]!;
    if (ps.some(p => p.x === q.x && p.y === q.y)) return;
    const { i } = cercaEnTrazado(id, q);
    ps.splice(i + 1, 0, q); caudalPublicado[id]!.splice(i + 1, 0, caudalPublicado[id]![i]!);
  };
  tramos.forEach((t, id) => t.trenzas.forEach(({ brazo, caudal }, b) => {
    const celdas = celdasDeLinea(brazo, celda, columnas, filas), propio = 1000 + id * 10 + b;
    // sale y vuelve por celdas de su río (las que publica): si no, no se apoya en él
    const propias = new Set(vertice[id]!);
    if (celdas.length < 3 || !propias.has(celdas[0]!) || !propias.has(celdas[celdas.length - 1]!)) return;
    const ultima = celdas[celdas.length - 1]!, sinFin = celdas.slice(0, -1);
    const suyas = tallarCauce(propio, sinFin, () => 0, false);
    if (suyas.length < 2 || !(H[ultima]! < H[suyas[suyas.length - 1]!]!)) return;
    const todas = [...suyas, ultima].map(centro), puntos = simplificar(todas, celda * TALLADO.publicar).map(i => todas[i]!);
    // sale y vuelve por el trazado de su río (el que se publica): los extremos, en el punto de él más cercano
    puntos[0] = enTrazado(id, puntos[0]!); puntos[puntos.length - 1] = enTrazado(id, puntos[puntos.length - 1]!);
    // fuera la que corta a su propio río (pasaría de un lado al otro)
    const madre = publicados[id]!;
    const lado = (p: Point, q: Point, r: Point) => Math.sign((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x));
    const corta = (a: Point, b: Point, c: Point, d: Point) => lado(a, b, c) * lado(a, b, d) < 0 && lado(c, d, a) * lado(c, d, b) < 0;
    if (puntos.some((b, i) => i > 0 && madre.some((d, k) => k > 0 && corta(puntos[i - 1]!, b, madre[k - 1]!, d)))) return;
    // el río principal lleva los puntos donde la trenza sale y vuelve
    for (const q of [puntos[0]!, puntos[puntos.length - 1]!]) insertarEnTrazado(id, q);
    trenzas.push({ id: `tramo-${id}-trenza-${b}`, puntos, fin: 'limite', terminaEnLago: false, navegable: false, orden: t.orden,
      caudal: puntos.map(() => caudal), anchos: puntos.map(() => anchoDeCaudal(caudal)) });
  }));
  // los brazos muertos, lagos en media luna sobre la vega (el agua apenas bajo su orilla más baja), sin tocar el río, otro lago ni un puerto
  const pasos = campo.regional.barreras.flatMap(b => b.pasos.flatMap(q => [q.centro.x, q.centro.y, q.tangente.x, q.tangente.y, q.radio, b.ancho]));
  const muertos: LagoZona[] = [];
  tramos.forEach((t, id) => {
    const ps = vertice[id]!.map(centro);
    for (const brazo of t.muertos) {
      const cerca = (q: Point) => ps.reduce((m, p, i) => { const d = Math.hypot(p.x - q.x, p.y - q.y); return d < m.d ? { d, i } : m; }, { d: Infinity, i: 0 });
      const libre = brazo.puntos.filter(q => cerca(q).d > Math.max(celda * 0.75, anchoDeCaudal(t.caudal[0]!) * 0.6) + brazo.ancho * 0.5);
      if (libre.length < 4) continue;
      const lago = lagoDeBrazo(`lago-brazo-${lagos.length + muertos.length}`, { puntos: libre, ancho: brazo.ancho }, 0);
      // muy pequeño (menos de media celda de radio) no se ve como brazo muerto
      if (!lago || lago.radio < celda / 2) continue;
      lago.elevacionCentro = Math.min(...lago.contorno!.map(enRejilla)) - MIGRACION.lamina;
      if (!puntoEnLago(lago, lago.centro) || enPaso(pasos, lago.centro) > 0) continue;
      if (![...lagos, ...muertos].some(l => Math.hypot(l.centro.x - lago.centro.x, l.centro.y - lago.centro.y) < l.radio + lago.radio)) muertos.push(lago);
    }
  });
  // el relieve de los ríos queda en la rejilla (en Float32, para los hilos del paquete de relieve)
  campo.fluvial = { celda, columnas, filas, altura: Float32Array.from(H) };
  // y, encima, el cauce fino de cada río publicado: su cota por punto, lo que da la rejilla menos `hondo`, siempre bajando; de arriba
  // abajo por la red, cada tramo empieza a la cota más baja de los que llegan a su confluencia (así la junta no sube)
  const hondo = (w: number) => (Math.min(CAUCE_TALLADO.hondo[1], Math.max(CAUCE_TALLADO.hondo[0], w * CAUCE_TALLADO.hondo[2])) * escala) / 1000;
  const cotas = (ps: readonly Point[], anchos: readonly number[], desde = Infinity) => {
    const nivel: number[] = [];
    ps.forEach((p, i) => nivel.push(Math.min(alturaFluvial(campo.fluvial!, p) - hondo(anchos[i]!),
      i ? nivel[i - 1]! - TALLADO.bajada * Math.hypot(p.x - ps[i - 1]!.x, p.y - ps[i - 1]!.y) : desde)));
    return nivel;
  };
  const enJunta = new Map<number, number>(), finos: { puntos: Point[]; nivel: number[]; medio: number[] }[] = [];
  for (const t of tallados) {
    const id = tramos.indexOf(t), puntos = publicados[id]!, anchos = caudalPublicado[id]!.map(anchoDeCaudal), nivel = cotas(puntos, anchos, enJunta.get(t.desde));
    enJunta.set(t.hasta, Math.min(enJunta.get(t.hasta) ?? Infinity, nivel[nivel.length - 1]!));
    finos.push({ puntos, nivel, medio: anchos.map(w => w / 2) });
  }
  // las trenzas salen del lecho de su río: empiezan bajo el cauce de este donde salen
  const madres = crearCaucesFinos(finos);
  for (const b of trenzas) {
    const p = b.puntos[0]!, salida = aplicarCauces(madres, p, alturaFluvial(campo.fluvial, p));
    finos.push({ puntos: b.puntos, nivel: cotas(b.puntos, b.anchos, salida), medio: b.anchos.map(w => w / 2) });
  }
  campo.cauces = crearCaucesFinos(finos);
  // los ríos: los centros de las celdas de cada cauce, sin la escalera de la rejilla (Douglas-Peucker: los que quedan siguen bajando, son
  // parte de una serie que baja), con su caudal (el del trazado, a lo largo) y su ancho
  const rios: RioRegional[] = tramos.map((t, id) => {
    const caudal = caudalPublicado[id]!;
    for (let i = 1; i < caudal.length; i++) caudal[i] = Math.max(caudal[i]!, caudal[i - 1]!);
    return { id: `tramo-${id}`, puntos: publicados[id]!, fin: t.fin, terminaEnLago: t.terminaEnLago, navegable: false, orden: t.orden,
      caudal, anchos: caudal.map(anchoDeCaudal) };
  });
  return { rios: [...rios, ...trenzas], lagos: [...lagos, ...muertos] };
}
