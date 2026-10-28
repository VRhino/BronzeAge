import type { Point } from './tipos';
import { createRng, derivarSeed } from './rng';
import { anilloEn, FACTOR, type Campamento } from './campamentos';
import type { Veta } from './metales';
import type { AlijoMundo, BandidosMundo } from './mundoTipos';

/**
 * Campamentos de bandidos y alijos iniciales (Doc 1.9, 1.9b, 1.9c). Todas las cifras de distancia van en UNIDADES
 * ESTRATÉGICAS aquí y se convierten con `FACTOR` al usarlas; las posiciones de salida son unidades del generador.
 *
 * - Cada veta: un campamento de bandidos de nivel 2-3 cerca de su centro y un alijo lejano cerca de ella.
 * - Cada campamento de mercenarios: alijos cercanos (3 a 20-40 y 2 a 40-90), con ese campamento como el más
 *   cercano y fuera de su protección. Sus bandidos no salen de aquí: los crea el motor en su anillo (12-25, según la demanda;
 *   decisión del autor del 2026-10-28, `CAMPAMENTOS_BANDIDOS.anillo`).
 * - Cada alijo trae una sola cosa, al azar y con la misma probabilidad: oro o uno de los recursos de fundar (madera, piedra,
 *   trigo). Las cantidades van por banda, cercano / medio / lejano: oro 15 / 25 / 55 y recurso 5 / 10 / 15 (decisión del
 *   autor del 2026-10-28; antes era solo oro 3 / 5 / 11).
 */
export const PARAMETROS_CONTENIDO = {
  /** Nivel al azar de un campamento de bandidos (pesos de `CAMPAMENTOS_BANDIDOS.niveles`). */
  pesosNivel: { 1: 0.5, 2: 0.3, 3: 0.2 } as Record<number, number>,
  veta: {
    niveles: [2, 3] as const,
    /** Del centro de la veta; si no cabe, se ensancha por pasos hasta el radio de la veta. */
    bandidos: [{ min: 3, max: 8 }, { min: 0, max: 15 }, { min: 0, max: 30 }],
    alijo: [{ min: 10, max: 25 }, { min: 8, max: 30 }],
    contenido: { oro: 55, recurso: 15 },
  },
  alijosCercanos: [
    { desde: 20, hasta: 40, cuantos: 3, contenido: { oro: 15, recurso: 5 } },
    { desde: 40, hasta: 90, cuantos: 2, contenido: { oro: 25, recurso: 10 } },
  ],
  /** Lo que puede traer un alijo, todo con la misma probabilidad. */
  recursosAlijo: ['oro', 'madera', 'piedra', 'trigo'] as const,
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
  // El contenido sale de su propio generador: no mueve la tirada de las posiciones.
  const rngContenido = createRng(derivarSeed(seed, 'contenido-alijos-1'));
  const contenido = (c: { oro: number; recurso: number }): Pick<AlijoMundo, 'recurso' | 'cantidad'> => {
    const recurso = P.recursosAlijo[Math.floor(rngContenido() * P.recursosAlijo.length)]!;
    return { recurso, cantidad: recurso === 'oro' ? c.oro : c.recurso };
  };
  const bandidos: BandidosMundo[] = [];
  const alijos: AlijoMundo[] = [];
  const cerca = (a: Point, b: Point, estrategicas: number) => dist(a, b) < estrategicas * FACTOR;

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
    alijos.push({ id: `alijo-${v.id}`, posicion: q, ...contenido(P.veta.contenido), anillo: 'lejana' });
  }

  // 2. Cada campamento de mercenarios: sus alijos cercanos (los bandidos de su anillo los crea el motor).
  for (const m of campamentos) {
    let k = 0;
    for (const banda of P.alijosCercanos) for (let i = 0; i < banda.cuantos; i++) {
      const p = primero(m.posicion, [{ min: banda.desde, max: banda.hasta }],
        c => alijoLibre(c) && anilloEn(campamentos, c).campamento === m.id, `un alijo de ${m.id}`);
      alijos.push({ id: `alijo-${m.id}-${k++}`, posicion: p, ...contenido(banda.contenido), anillo: anilloEn(campamentos, p).anillo });
    }
  }
  return { bandidos, alijos };
}
