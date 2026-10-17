import { describe, expect, it } from 'vitest';
import { anilloEn, colocarCampamentos, FACTOR } from '../campamentos';
import { PARAMETROS_CONTENIDO as P, colocarBandidosYAlijos } from '../contenidoVivo';

const ANCHO = 32004, ALTO = 24000;
const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
const est = (a: { x: number; y: number }, b: { x: number; y: number }) => dist(a, b) / FACTOR;

// Mundo llano sintético: todo apto. Comprueba las reglas, no la geografía.
const limites = { ancho: ANCHO, alto: ALTO };
const campamentos = colocarCampamentos(7, limites, () => 1);
const todo = () => true;
const { bandidos, alijos } = colocarBandidosYAlijos(7, limites, campamentos, todo);

describe('bandidos y alijos', () => {
  it('anillo de bandidos de 30 a 50 de cada campamento, fuera de su protección', () => {
    for (const m of campamentos) {
      const anillo = bandidos.filter(b => anilloEn(campamentos, b.posicion).campamento === m.id);
      expect(anillo).toHaveLength(P.anillo.inicialesPorCampamento);
      for (const b of anillo) {
        expect(est(b.posicion, m.posicion)).toBeGreaterThanOrEqual(P.anillo.radioMin);
        expect(est(b.posicion, m.posicion)).toBeLessThanOrEqual(P.anillo.radioMax);
        expect(b.anillo).toBe('foco');
      }
    }
  });
  it('alijos cercanos por banda, con su campamento como el más cercano', () => {
    for (const m of campamentos) {
      for (const banda of P.alijosCercanos) {
        const de = alijos.filter(a => a.oro === banda.oro && anilloEn(campamentos, a.posicion).campamento === m.id && a.id.startsWith(`alijo-${m.id}-`));
        expect(de).toHaveLength(banda.cuantos);
        for (const a of de) { const d = est(a.posicion, m.posicion); expect(d).toBeGreaterThanOrEqual(banda.desde); expect(d).toBeLessThanOrEqual(banda.hasta); }
      }
    }
  });
  it('separaciones mínimas, protección de campamento, límites y ids únicos', () => {
    const todos = [...bandidos.map(b => b.posicion), ...alijos.map(a => a.posicion)];
    for (const p of todos) {
      expect(p.x).toBeGreaterThanOrEqual(0); expect(p.x).toBeLessThanOrEqual(ANCHO);
      expect(p.y).toBeGreaterThanOrEqual(0); expect(p.y).toBeLessThanOrEqual(ALTO);
      for (const m of campamentos) expect(est(p, m.posicion)).toBeGreaterThanOrEqual(P.radioProteccion);
    }
    for (const [i, a] of bandidos.entries()) for (const b of bandidos.slice(i + 1)) expect(est(a.posicion, b.posicion)).toBeGreaterThanOrEqual(P.separacion.bandidos);
    for (const [i, a] of alijos.entries()) for (const b of alijos.slice(i + 1)) expect(est(a.posicion, b.posicion)).toBeGreaterThanOrEqual(P.separacion.alijos);
    for (const a of alijos) for (const b of bandidos) expect(est(a.posicion, b.posicion)).toBeGreaterThanOrEqual(P.separacion.alijoBandidos);
    for (const l of [bandidos, alijos]) expect(new Set(l.map(x => x.id)).size).toBe(l.length);
  });
  it('el anillo declarado coincide con la distancia al campamento más cercano', () => {
    for (const x of [...bandidos, ...alijos]) expect(x.anillo).toBe(anilloEn(campamentos, x.posicion).anillo);
  });
  it('oro total coherente con las bandas', () => {
    const porCampamento = P.alijosCercanos.reduce((s, b) => s + b.cuantos * b.oro, 0);
    expect(alijos.reduce((s, a) => s + a.oro, 0)).toBe(campamentos.length * porCampamento);
  });
  it('respeta el terreno apto', () => {
    const sinFranja = (p: { x: number; y: number }) => !(p.x > 14000 && p.x < 15000);
    const r = colocarBandidosYAlijos(7, limites, campamentos, sinFranja);
    for (const x of [...r.bandidos, ...r.alijos]) expect(sinFranja(x.posicion)).toBe(true);
  });
  it('sin terreno apto, falla con un error claro', () => {
    expect(() => colocarBandidosYAlijos(7, limites, campamentos, () => false)).toThrow(/Contenido/);
  });
  it('misma semilla, mismo resultado; otra semilla, otro', () => {
    expect(colocarBandidosYAlijos(7, limites, campamentos, todo)).toEqual({ bandidos, alijos });
    expect(colocarBandidosYAlijos(8, limites, campamentos, todo).bandidos).not.toEqual(bandidos);
  });
});
