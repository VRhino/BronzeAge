import type { Point } from './tipos';
import { createRng, derivarSeed, type RandomFn } from './rng';
import { anilloEn, FACTOR, type Campamento } from './campamentos';
import type { Veta } from './metales';
import type { AlijoMundo, BandidosMundo } from './mundoTipos';

/**
 * Campamentos de bandidos y alijos iniciales (Doc 1.9, 1.9b, 1.9c). Todas las cifras de distancia van en UNIDADES
 * ESTRATÉGICAS aquí y se convierten con `FACTOR` al usarlas; las posiciones de salida son unidades del generador.
 *
 * - Cada veta: un campamento de bandidos de nivel 2-3 cerca de su centro y un alijo lejano (11 de oro) cerca de ella.
 * - Cada campamento de mercenarios: un anillo de bandidos a 30-50 (contenido para los recién llegados, D42) y alijos
 *   cercanos (3 a 20-40 con 3 de oro, 2 a 40-90 con 5), con ese campamento como el más cercano y fuera de su protección.
 * - El oro de las bandas es el del motor antiguo (3 / 5 / 11), sin reescalar.
 */
export const PARAMETROS_CONTENIDO = {
  /** Nivel al azar de un campamento de bandidos (pesos de `CAMPAMENTOS_BANDIDOS.niveles`). */
  pesosNivel: { 1: 0.5, 2: 0.3, 3: 0.2 } as Record<number, number>,
  veta: {
    niveles: [2, 3] as const,
    /** Del centro de la veta; si no cabe, se ensancha por pasos hasta el radio de la veta. */
    bandidos: [{ min: 3, max: 8 }, { min: 0, max: 15 }, { min: 0, max: 30 }],
    alijo: [{ min: 10, max: 25 }, { min: 8, max: 30 }],
    oroAlijo: 11,
  },
  /** SUPOSICIÓN: el canon fija el rango (30-50) y la demanda 1-6, no la cantidad inicial. */
  anillo: { radioMin: 30, radioMax: 50, inicialesPorCampamento: 3 },
  alijosCercanos: [
    { desde: 20, hasta: 40, cuantos: 3, oro: 3 },
    { desde: 40, hasta: 90, cuantos: 2, oro: 5 },
  ],
  /** Protección de un campamento de mercenarios (`MERCENARIOS.radioProteccion`). */
  radioProteccion: 10,
  /** SUPOSICIONES de separación mínima entre elementos (estratégicas). */
  separacion: { bandidos: 12, alijos: 8, alijoBandidos: 6 },
  intentos: 60,
};

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

export function colocarBandidosYAlijos(
  seed: number,
  limites: { ancho: number; alto: number },
  campamentos: readonly Campamento[],
  vetas: readonly Veta[],
  apto: (p: Point) => boolean,
  P = PARAMETROS_CONTENIDO,
): { bandidos: BandidosMundo[]; alijos: AlijoMundo[] } {
  const rng = createRng(derivarSeed(seed, 'contenido-vivo-1'));
  const bandidos: BandidosMundo[] = [];
  const alijos: AlijoMundo[] = [];
  const cerca = (a: Point, b: Point, estrategicas: number) => dist(a, b) < estrategicas * FACTOR;

  const nivelAlAzar = (r: RandomFn) => {
    let t = r();
    for (const n of [1, 2, 3]) { t -= P.pesosNivel[n]!; if (t < 0) return n; }
    return 1;
  };

  /** Punto en el anillo `[min,max]` (estratégicas) de `c`, o undefined tras `P.intentos` tiradas. */
  const muestrear = (c: Point, min: number, max: number, vale: (p: Point) => boolean): Point | undefined => {
    for (let i = 0; i < P.intentos; i++) {
      const a = rng() * 2 * Math.PI, r = (min + rng() * (max - min)) * FACTOR;
      const p = { x: c.x + r * Math.cos(a), y: c.y + r * Math.sin(a) };
      if (p.x < 0 || p.y < 0 || p.x > limites.ancho || p.y > limites.alto || !apto(p)) continue;
      if (campamentos.some(m => cerca(m.posicion, p, P.radioProteccion))) continue;
      if (vale(p)) return p;
    }
    return undefined;
  };
  const primero = (c: Point, anillos: readonly { min: number; max: number }[], vale: (p: Point) => boolean, que: string) => {
    for (const a of anillos) { const p = muestrear(c, a.min, a.max, vale); if (p) return p; }
    throw new Error(`Contenido: sin sitio para ${que}`);
  };

  const libreDeBandidos = (p: Point) => bandidos.every(b => !cerca(b.posicion, p, P.separacion.bandidos));
  const libreDeAlijos = (p: Point) => alijos.every(a => !cerca(a.posicion, p, P.separacion.alijos));
  const alijoLibre = (p: Point) => libreDeAlijos(p) && bandidos.every(b => !cerca(b.posicion, p, P.separacion.alijoBandidos));
  const bandidoLibre = (p: Point) => libreDeBandidos(p) && alijos.every(a => !cerca(a.posicion, p, P.separacion.alijoBandidos));

  // 1. Distritos mineros: su guardián (nivel 2-3) y su alijo lejano.
  for (const v of vetas) {
    const p = primero(v.centro, P.veta.bandidos, bandidoLibre, `los bandidos de ${v.id}`);
    const niveles = P.veta.niveles, peso = niveles.map(n => P.pesosNivel[n]!);
    let t = rng() * peso.reduce((s, n) => s + n, 0), nivel: number = niveles[niveles.length - 1]!;
    for (const [i, n] of niveles.entries()) { t -= peso[i]!; if (t < 0) { nivel = n; break; } }
    bandidos.push({ id: `bandidos-${v.id}`, posicion: p, nivel, vetaId: v.id, anillo: anilloEn(campamentos, p).anillo });
    const q = primero(v.centro, P.veta.alijo, c => alijoLibre(c) && anilloEn(campamentos, c).anillo === 'lejana', `el alijo de ${v.id}`);
    alijos.push({ id: `alijo-${v.id}`, posicion: q, oro: P.veta.oroAlijo, anillo: 'lejana' });
  }

  // 2. Cada campamento de mercenarios: su anillo de bandidos y sus alijos cercanos.
  for (const m of campamentos) {
    for (let i = 0; i < P.anillo.inicialesPorCampamento; i++) {
      const p = primero(m.posicion, [{ min: P.anillo.radioMin, max: P.anillo.radioMax }], bandidoLibre, `el anillo de ${m.id}`);
      bandidos.push({ id: `bandidos-${m.id}-${i}`, posicion: p, nivel: nivelAlAzar(rng), anillo: anilloEn(campamentos, p).anillo });
    }
  }
  for (const m of campamentos) {
    let k = 0;
    for (const banda of P.alijosCercanos) for (let i = 0; i < banda.cuantos; i++) {
      const p = primero(m.posicion, [{ min: banda.desde, max: banda.hasta }],
        c => alijoLibre(c) && anilloEn(campamentos, c).campamento === m.id, `un alijo de ${m.id}`);
      alijos.push({ id: `alijo-${m.id}-${k++}`, posicion: p, oro: banda.oro, anillo: anilloEn(campamentos, p).anillo });
    }
  }
  return { bandidos, alijos };
}
