// Formas del río de la fase 5 (experimento/rios): meandros que migran y dejan brazos muertos, y ríos trenzados.
import type { LagoZona, Point } from './tipos';
import { potencia } from './potencia';

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const suave = (t: number) => { const x = Math.min(1, Math.max(0, t)); return x * x * (3 - 2 * x); };

/** Lo andado hasta cada punto. */
function recorrido(ps: readonly Point[]): number[] {
  const s = [0];
  for (let i = 1; i < ps.length; i++) s.push(s[i - 1]! + Math.hypot(ps[i]!.x - ps[i - 1]!.x, ps[i]!.y - ps[i - 1]!.y));
  return s;
}

/** Una serie de valores por punto, leída a `d` de lo andado. */
function leer(s: readonly number[], v: readonly number[], d: number): number {
  let j = 0;
  while (j < s.length - 2 && s[j + 1]! < d) j++;
  return lerp(v[j]!, v[j + 1]!, Math.max(0, Math.min(1, (d - s[j]!) / (s[j + 1]! - s[j]! || 1))));
}

/** Remuestrea a pasos de `paso` (los extremos, los mismos objetos) con sus series de valores. */
function remuestrear(ps: readonly Point[], series: readonly (readonly number[])[], paso: number): { puntos: Point[]; series: number[][] } {
  const s = recorrido(ps), total = s[s.length - 1]!, n = Math.max(1, Math.round(total / paso));
  const puntos: Point[] = [ps[0]!], salida = series.map(v => [v[0]!]);
  // un solo recorrido: lo andado crece, el segmento también
  for (let k = 1, j = 0; k < n; k++) {
    const d = (total * k) / n;
    while (j < s.length - 2 && s[j + 1]! < d) j++;
    const t = Math.max(0, Math.min(1, (d - s[j]!) / (s[j + 1]! - s[j]! || 1)));
    puntos.push({ x: lerp(ps[j]!.x, ps[j + 1]!.x, t), y: lerp(ps[j]!.y, ps[j + 1]!.y, t) });
    series.forEach((v, i) => salida[i]!.push(lerp(v[j]!, v[j + 1]!, t)));
  }
  puntos.push(ps[ps.length - 1]!);
  series.forEach((v, i) => salida[i]!.push(v[v.length - 1]!));
  return { puntos, series: salida };
}

/** La migración (Howard y Knutson 1984, como meanderpy): cada punto del cauce se mueve hacia fuera de su curva, con la curvatura de aguas
 * arriba pesando con un retraso (`gamma`, a lo largo de `retraso` anchos). Cuando un cuello se cierra (dos puntos lejanos por el río a menos de
 * `cuello` anchos), el río lo corta y el lazo queda como brazo muerto. Solo en ríos de `anchoMinimo` m o más. El cinturón de meandros (`cinturon`, lo que el meandro se aparta del
 * trazado sin meandro) no se rebasa; los extremos del tramo no se mueven. La migración para cuando el río es `sinuosidad` veces más largo
 * que el trazado sin meandro (los ríos de llanura rondan 1,5-2,5).
 *
 * El paso de cada punto se alisa a lo largo del río con una campana de `alisado` anchos (autor, 2026-11-25: «desastre de meandros»): sin
 * ello crecían más deprisa las ondas cortas, de 2 a 5 anchos, que la rejilla de tallado (40 m) volvía un zigzag de celda en celda; un río de
 * 50 m salía con ondas de 5 anchos y cuellos de 1,8. Así se apagan las ondas de menos de unos 5 anchos y crecen las de un meandro de verdad
 * (10-14 anchos). Con `sinuosidad` 2 y cuellos que se cortan a 3 anchos, entre lazo y lazo queda tierra de al menos 2 anchos. */
export const MIGRACION = { anchoMinimo: 10, vueltas: 240, sinuosidad: 2, paso: 0.5, avance: 0.1, gamma: 2.5, retraso: 3, cuello: 3, separacionCuello: 2, holgura: 1.3,
  alisado: 1.5,
  /** Brazos muertos por tramo, como mucho (los últimos en cortarse); su ancho, en anchos del río; y su agua bajo la orilla más baja (normalizada). */
  brazosMuertos: 5, anchoBrazo: 0.8, lamina: 0.0003,
  /** Puntos de la orilla de un brazo muerto por lado, como poco. */
  puntosOrilla: 16 };

export interface BrazoMuerto { puntos: Point[]; ancho: number }

/** Mueve el trazado de un tramo con meandros. `base` es el trazado sin meandro (del que mide el cinturón), `cinturon` y `ancho` van por
 * punto del trazado. Devuelve el trazado nuevo con su caudal, valle y cinturón, y los brazos muertos que dejó. */
export function migrar(puntos: readonly Point[], series: { caudal: readonly number[]; valle: readonly number[]; cinturon: readonly number[]; ancho: readonly number[] },
  base: readonly Point[]): { puntos: Point[]; caudal: number[]; valle: number[]; cinturon: number[]; brazos: BrazoMuerto[] } {
  const sinCambio = { puntos: [...puntos], caudal: [...series.caudal], valle: [...series.valle], cinturon: [...series.cinturon], brazos: [] as BrazoMuerto[] };
  if (puntos.length < 8 || Math.max(...series.cinturon) <= 0) return sinCambio;
  const M = MIGRACION, anchoMedio = series.ancho.reduce((a, b) => a + b, 0) / series.ancho.length;
  // los ríos pequeños no migran: sus meandros (unos 12 anchos) no se ven a la escala del mundo
  if (anchoMedio < M.anchoMinimo) return sinCambio;
  let r = remuestrear(puntos, [series.caudal, series.valle, series.cinturon, series.ancho], anchoMedio * M.paso);
  let ps = r.puntos, [caudal, valle, cinturon, ancho] = r.series as [number[], number[], number[], number[]];
  const cortes: BrazoMuerto[] = [];
  // la distancia al trazado sin meandro (para no salirse del cinturón) y el punto de él más cercano
  const sBase = recorrido(base), totalBase = sBase[sBase.length - 1]!;
  // el punto del trazado sin meandro más cercano, buscado cerca de donde cae a esa altura del tramo (`f`, 0-1)
  const alBase = (p: Point, f: number): { d: number; q: Point } => {
    let d = Infinity, q = base[0]!, k = 0;
    while (k < base.length - 2 && sBase[k + 1]! < f * totalBase) k++;
    for (let i = Math.max(0, k - 12); i + 1 < base.length && i <= k + 12; i++) {
      const a = base[i]!, b = base[i + 1]!, dx = b.x - a.x, dy = b.y - a.y;
      const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
      const e = Math.hypot(a.x + dx * t - p.x, a.y + dy * t - p.y);
      if (e < d) { d = e; q = { x: a.x + dx * t, y: a.y + dy * t }; }
    }
    return { d, q };
  };
  const largoBase = recorrido(base).at(-1)!;
  for (let vuelta = 0; vuelta < M.vueltas; vuelta++) {
    const n = ps.length;
    if (recorrido(ps).at(-1)! >= M.sinuosidad * largoBase) break;
    // curvatura con signo en cada punto
    const C = new Float64Array(n);
    for (let i = 1; i < n - 1; i++) {
      const a = ps[i - 1]!, b = ps[i]!, c = ps[i + 1]!;
      const ux = b.x - a.x, uy = b.y - a.y, vx = c.x - b.x, vy = c.y - b.y, lu = Math.hypot(ux, uy) || 1, lv = Math.hypot(vx, vy) || 1;
      C[i] = (2 * (ux * vy - uy * vx)) / (lu * lv * (lu + lv));
    }
    const nuevos = ps.map(p => ({ ...p }));
    // el paso de cada punto, hacia fuera de la curva, acotado y apagado en los extremos del cinturón
    const paso = new Float64Array(n);
    for (let i = 1; i < n - 1; i++) {
      if (cinturon[i]! <= 0) continue;
      const w = ancho[i]!, ds = w * M.paso, alfa = 1 / (M.retraso * w);
      let suma = 0, pesos = 0;
      for (let k = 1; k < 40 && i - k >= 0; k++) { const g = Math.exp(-alfa * k * ds); suma += C[i - k]! * g; pesos += g; }
      const R = -C[i]! + M.gamma * (pesos ? suma / pesos : 0);
      paso[i] = Math.max(-1, Math.min(1, R * w * 4)) * M.avance * w * suave(cinturon[i]! / w);
    }
    // alisado a lo largo del río (las muestras van cada `paso` anchos): apaga las ondas cortas
    const sigma = M.alisado / M.paso, radio = Math.ceil(3 * sigma), campana = Array.from({ length: radio + 1 }, (_, k) => Math.exp(-(k * k) / (2 * sigma * sigma)));
    for (let i = 1; i < n - 1; i++) {
      if (cinturon[i]! <= 0) continue;
      let suma = 0, pesos = 0;
      for (let k = -radio; k <= radio; k++) { const j = i + k; if (j < 1 || j > n - 2) continue; suma += paso[j]! * campana[Math.abs(k)]!; pesos += campana[Math.abs(k)]!; }
      const mover = suma / pesos, w = ancho[i]!;
      const a = ps[i - 1]!, b = ps[i + 1]!, tx = b.x - a.x, ty = b.y - a.y, l = Math.hypot(tx, ty) || 1;
      nuevos[i] = { x: ps[i]!.x - (ty / l) * mover, y: ps[i]!.y + (tx / l) * mover };
      const { d, q } = alBase(nuevos[i]!, i / (n - 1)), tope = cinturon[i]! * M.holgura + w;
      if (d > tope) nuevos[i] = { x: q.x + (nuevos[i]!.x - q.x) * (tope / d), y: q.y + (nuevos[i]!.y - q.y) * (tope / d) };
    }
    nuevos[0] = ps[0]!; nuevos[n - 1] = ps[n - 1]!;
    // los cuellos (se miran en cada vuelta): el primero que se cierra se corta
    const s = recorrido(nuevos);
    // en una rejilla de celdas del tamaño del cuello más ancho: cada punto solo mira los de su celda y las vecinas
    const lado = M.cuello * Math.max(...ancho), celdas = new Map<string, number[]>();
    nuevos.forEach((q, k) => { const c = `${Math.floor(q.x / lado)},${Math.floor(q.y / lado)}`; (celdas.get(c) ?? celdas.set(c, []).get(c)!).push(k); });
    cuellos: for (let i = 1; i < n - 1; i++) {
      if (cinturon[i]! <= 0) continue;
      const cx = Math.floor(nuevos[i]!.x / lado), cy = Math.floor(nuevos[i]!.y / lado), cerca: number[] = [];
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) cerca.push(...(celdas.get(`${cx + dx},${cy + dy}`) ?? []));
      for (const j of cerca.sort((x, y) => y - x)) {
        if (j <= i || j >= n - 1 || s[j]! - s[i]! < M.separacionCuello * ancho[i]! * Math.PI) continue;
        if (Math.hypot(nuevos[j]!.x - nuevos[i]!.x, nuevos[j]!.y - nuevos[i]!.y) < M.cuello * ancho[i]!) {
          cortes.push({ puntos: nuevos.slice(i, j + 1), ancho: ancho[i]! * M.anchoBrazo });
          const quita = (v: number[]) => [...v.slice(0, i + 1), ...v.slice(j)];
          nuevos.splice(i + 1, j - i - 1);
          caudal = quita(caudal); valle = quita(valle); cinturon = quita(cinturon); ancho = quita(ancho);
          break cuellos;
        }
      }
    }
    r = remuestrear(nuevos, [caudal, valle, cinturon, ancho], anchoMedio * M.paso);
    ps = r.puntos; [caudal, valle, cinturon, ancho] = r.series as [number[], number[], number[], number[]];
  }
  if (!cortes.length && ps.length === puntos.length) return sinCambio;
  const fin = remuestrear(ps, [caudal, valle, cinturon], Math.max(10, Math.min(45, anchoMedio * 1.2)));
  // el cinturón que puede ocupar ya el río (lo que se aparta del trazado sin meandro como mucho): la vega lo cubre entero
  return { puntos: fin.puntos, caudal: fin.series[0]!, valle: fin.series[1]!, cinturon: fin.series[2]!.map(c => (c > 0 ? c * M.holgura + anchoMedio : 0)),
    brazos: cortes.slice(-M.brazosMuertos) };
}

/** El lago de un brazo muerto: la media luna del lazo cortado, con su ancho, a la cota `nivel`. Su centro, en mitad del lazo. */
export function lagoDeBrazo(id: string, brazo: BrazoMuerto, nivel: number): LagoZona | undefined {
  const crudos = brazo.puntos.slice(1, -1);
  if (crudos.length < 4) return undefined;
  // la orilla, con al menos `puntosOrilla` puntos por lado (un brazo corto quedaría en un polígono basto)
  const ps = crudos.length >= MIGRACION.puntosOrilla ? crudos : remuestrear(crudos, [], recorrido(crudos).at(-1)! / (MIGRACION.puntosOrilla - 1)).puntos;
  const lado = (i: number, signo: number) => {
    const a = ps[Math.max(0, i - 1)]!, b = ps[Math.min(ps.length - 1, i + 1)]!, l = Math.hypot(b.x - a.x, b.y - a.y) || 1;
    // las puntas se afinan: el brazo muerto se ciega por los extremos
    const w = (brazo.ancho / 2) * Math.sqrt(Math.sin((Math.PI * (i + 0.5)) / ps.length));
    return { x: ps[i]!.x - ((b.y - a.y) / l) * w * signo, y: ps[i]!.y + ((b.x - a.x) / l) * w * signo };
  };
  const contorno = [...ps.map((_, i) => lado(i, 1)), ...ps.map((_, i) => lado(ps.length - 1 - i, -1))];
  const centro = ps[Math.floor(ps.length / 2)]!;
  const radio = Math.max(...contorno.map(p => Math.hypot(p.x - centro.x, p.y - centro.y))) + 1;
  return { id, centro: { ...centro }, radio, contorno, elevacionCentro: nivel };
}

/** Ríos trenzados: donde el río baja con pendiente media (entre `pendienteMinima` y `pendienteMaxima`), es ancho (`anchoMinimo` m) y no
 * hace meandros, a lo largo de al menos `largoMinimo` m, su lecho se abre en una llanura de grava de `lecho` anchos y el agua se reparte
 * en brazos cortos alrededor de barras de grava (`brazosTrenzados`). Cada brazo lleva `parte` del caudal y se aparta hasta `aparte` anchos. */
export const TRENZAS = { pendienteMinima: 0.004, pendienteMaxima: 0.03, anchoMinimo: 12, largoMinimo: 600, lecho: 2.6, parte: 0.25,
  aparte: 1.2, paso: 30, barra: [8, 16] as const, doble: 0.25,
  /** Lo que se aparta un brazo, como poco, en celdas de la rejilla donde se tallan. */
  aparteCeldas: 1.5 };

/** Los tramos trenzados de un río (índices de inicio y fin en `puntos`) y su peso por punto (0-1, para abrir el lecho). */
export function zonasTrenzadas(puntos: readonly Point[], valle: readonly number[], ancho: readonly number[], cinturon: readonly number[]): { zonas: [number, number][]; peso: number[] } {
  const s = recorrido(puntos), n = puntos.length, T = TRENZAS;
  const pendiente = puntos.map((_, i) => {
    let a = i, b = i;
    while (a > 0 && s[i]! - s[a]! < 150) a--;
    while (b < n - 1 && s[b]! - s[i]! < 150) b++;
    return Math.max(0, (valle[a]! - valle[b]!) * 1000 / (s[b]! - s[a]! || 1));
  });
  const vale = puntos.map((_, i) => pendiente[i]! > T.pendienteMinima && pendiente[i]! < T.pendienteMaxima && ancho[i]! >= T.anchoMinimo && cinturon[i]! <= 0);
  const zonas: [number, number][] = [], peso = new Array<number>(n).fill(0);
  for (let i = 1; i < n - 1; i++) {
    if (!vale[i]) continue;
    let j = i;
    while (j + 1 < n - 1 && vale[j + 1]) j++;
    if (s[j]! - s[i]! >= T.largoMinimo) {
      zonas.push([i, j]);
      for (let k = i; k <= j; k++) peso[k] = suave(Math.min(s[k]! - s[i]!, s[j]! - s[k]!) / 150);
    }
    i = j;
  }
  return { zonas, peso };
}

/** Los brazos de una zona trenzada: barras de grava cortas, una tras otra a lo largo de la zona, de `barra` anchos (de [0] a [1], al azar),
 * con un brazo que sale del río y vuelve a él rodeándola, a un lado y al otro; de vez en cuando (`doble`) dos a la vez, uno a cada lado.
 * Cada brazo empieza y acaba en un punto del río y se aparta como poco `aparteMinimo` (la rejilla donde se tallan no ve menos: más cerca,
 * el brazo acabaría al otro lado del río). */
export function brazosTrenzados(puntos: readonly Point[], ancho: readonly number[], i: number, j: number, semilla: number, aparteMinimo = 0): Point[][] {
  const T = TRENZAS, s = recorrido(puntos), xs = puntos.map(p => p.x), ys = puntos.map(p => p.y), brazos: Point[][] = [];
  let azar = Math.abs(Math.sin(semilla * 12.9898) * 43758.5453) % 1;
  const siguiente = () => (azar = (azar * 9301 + 49297) % 233280 / 233280);
  let k = i, lado = siguiente() < 0.5 ? 1 : -1;
  while (k < j) {
    const w = ancho[k]!, largo = w * (T.barra[0] + (T.barra[1] - T.barra[0]) * siguiente());
    // el final de la barra: el primer punto del río a `largo` de su principio
    let fin = k;
    while (fin < j && s[fin]! - s[k]! < largo) fin++;
    if (fin - k < 2 || s[fin]! - s[k]! < largo * 0.6) break;
    for (const signo of siguiente() < T.doble ? [lado, -lado] : [lado]) {
      const brazo: Point[] = [puntos[k]!], d0 = s[k]!, total = s[fin]! - d0;
      for (let d = T.paso; d < total - T.paso / 2; d += T.paso) {
        const at = d0 + d, a = leer(s, xs, at - 15), c = leer(s, xs, at + 15), ay = leer(s, ys, at - 15), cy = leer(s, ys, at + 15);
        const l = Math.hypot(c - a, cy - ay) || 1, aparte = signo * Math.max(T.aparte * leer(s, ancho, at), aparteMinimo) * potencia(Math.sin(Math.PI * d / total), 0.7);
        brazo.push({ x: leer(s, xs, at) - ((cy - ay) / l) * aparte, y: leer(s, ys, at) + ((c - a) / l) * aparte });
      }
      brazo.push(puntos[fin]!);
      if (brazo.length >= 4) brazos.push(brazo);
    }
    lado = -lado;
    k = fin;
  }
  return brazos;
}