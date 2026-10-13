import type { Point } from './tipos';
import { createRng, derivarSeed } from './rng';

/** Unidades del generador por unidad estratégica del contrato (el factor de Unity). */
export const FACTOR = 12;

/** Doc 1.9b-1.9c, en unidades estratégicas. */
export const ESTRATEGICAS = { separacionCampamentos: 600, margenBorde: 60, focoMax: 150, fronteraMax: 250 } as const;

export type Anillo = 'foco' | 'frontera' | 'lejana';

export interface Campamento { id: string; posicion: Point }

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

/**
 * Campamentos de mercenarios: centros de los focos (Doc 1.9b). `puntuar` devuelve 0 si el sitio no es apto (agua, sin acceso,
 * sin fertilidad...) y cuanto mayor, mejor. Selección voraz por puntuación con azar de la semilla; separación fija entre
 * campamentos y margen de borde. Mismo `seed`, mismos campamentos.
 */
export function colocarCampamentos(
  seed: number,
  limites: { ancho: number; alto: number },
  puntuar: (p: Point) => number,
  paso = 30 * FACTOR,
): Campamento[] {
  const rng = createRng(derivarSeed(seed, 'campamentos-1'));
  const margen = ESTRATEGICAS.margenBorde * FACTOR, separacion = ESTRATEGICAS.separacionCampamentos * FACTOR;
  const candidatos: { p: Point; v: number }[] = [];
  for (let y = margen; y <= limites.alto - margen; y += paso) for (let x = margen; x <= limites.ancho - margen; x += paso) {
    const p = { x: x + (rng() - 0.5) * paso * 0.8, y: y + (rng() - 0.5) * paso * 0.8 };
    if (p.x < margen || p.y < margen || p.x > limites.ancho - margen || p.y > limites.alto - margen) continue;
    const s = puntuar(p);
    if (s > 0) candidatos.push({ p, v: s * (0.5 + rng() * 0.5) });
  }
  candidatos.sort((a, b) => b.v - a.v);
  const elegidos: Campamento[] = [];
  for (const c of candidatos) {
    if (elegidos.every(e => dist(e.posicion, c.p) >= separacion)) elegidos.push({ id: `campamento-${elegidos.length}`, posicion: c.p });
  }
  return elegidos;
}

/** Anillo de un punto por su distancia (estratégica) al campamento más cercano. */
export function anilloEn(campamentos: readonly Campamento[], p: Point): { anillo: Anillo; distancia: number; campamento: string } {
  let d = Infinity, id = '';
  for (const c of campamentos) { const x = dist(c.posicion, p) / FACTOR; if (x < d) { d = x; id = c.id; } }
  const anillo: Anillo = d <= ESTRATEGICAS.focoMax ? 'foco' : d <= ESTRATEGICAS.fronteraMax ? 'frontera' : 'lejana';
  return { anillo, distancia: d, campamento: id };
}
