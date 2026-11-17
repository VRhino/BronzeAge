import { describe, expect, it } from 'vitest';
import { anilloEn, colocarCampamentos, FACTOR } from '../campamentos';
import { PARAMETROS_CONTENIDO as P, colocarBandidosYAlijos } from '../contenidoVivo';
import { repartirMetales, type Sitio } from '../metales';

const ANCHO = 32004, ALTO = 24000;
const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
const est = (a: { x: number; y: number }, b: { x: number; y: number }) => dist(a, b) / FACTOR;

// Mundo llano sintético: todo apto. Comprueba las reglas, no la geografía.
const limites = { ancho: ANCHO, alto: ALTO };
const campamentos = colocarCampamentos(7, limites, () => 1);
const sitios: Sitio[] = [];
for (let y = 60; y < ALTO; y += 120) for (let x = 60; x < ANCHO; x += 120) sitios.push({ posicion: { x, y }, calidad: { estano: 0.7, oro: 0.7 } });
const { vetas } = repartirMetales(7, campamentos, sitios);
const todo = () => true;
const { bandidos, alijos } = colocarBandidosYAlijos(7, limites, campamentos, vetas, todo);

describe('bandidos y alijos', () => {
  it('un bandido de nivel 2-3 y un alijo lejano por veta, cerca de su centro', () => {
    for (const v of vetas) {
      const bs = bandidos.filter(b => b.vetaId === v.id), a = alijos.filter(x => x.id === `alijo-${v.id}`);
      expect(bs).toHaveLength(1);
      expect(a).toHaveLength(1);
      expect([2, 3]).toContain(bs[0]!.nivel);
      expect(est(bs[0]!.posicion, v.centro)).toBeLessThanOrEqual(8);
      expect(est(a[0]!.posicion, v.centro)).toBeLessThanOrEqual(25);
      expect(a[0]!.anillo).toBe('lejana');
      expect(a[0]!.cantidad).toBe(a[0]!.recurso === 'oro' ? P.veta.contenido.oro : P.veta.contenido.recurso);
    }
  });
  it('solo hay bandidos de veta: los del anillo de cada campamento los crea el motor', () => {
    expect(bandidos.every(b => b.vetaId)).toBe(true);
    expect(bandidos).toHaveLength(vetas.length);
  });
  it('alijos cercanos por banda, con su campamento como el más cercano', () => {
    for (const m of campamentos) {
      // Los ids siguen el orden de las bandas: `alijo-<campamento>-<n>`.
      const suyos = alijos.filter(a => a.id.startsWith(`alijo-${m.id}-`)), n = (a: { id: string }) => Number(a.id.slice(`alijo-${m.id}-`.length));
      let k = 0;
      for (const banda of P.alijosCercanos) {
        const de = suyos.filter(a => n(a) >= k && n(a) < k + banda.cuantos);
        expect(de).toHaveLength(banda.cuantos);
        k += banda.cuantos;
        for (const a of de) {
          expect(anilloEn(campamentos, a.posicion).campamento).toBe(m.id);
          const d = est(a.posicion, m.posicion); expect(d).toBeGreaterThanOrEqual(banda.desde); expect(d).toBeLessThanOrEqual(banda.hasta);
          expect(a.cantidad).toBe(a.recurso === 'oro' ? banda.contenido.oro : banda.contenido.recurso);
        }
      }
      expect(suyos).toHaveLength(k);
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
  it('cada alijo trae una sola cosa: oro, madera, piedra o trigo, y salen las cuatro', () => {
    for (const a of alijos) expect(P.recursosAlijo).toContain(a.recurso);
    expect(new Set(alijos.map(a => a.recurso)).size).toBe(P.recursosAlijo.length);
  });
  it('el contenido no mueve las posiciones', () => {
    const otro = colocarBandidosYAlijos(7, limites, campamentos, vetas, todo, { ...P, recursosAlijo: ['oro'] as unknown as typeof P.recursosAlijo });
    expect(otro.alijos.map(a => a.posicion)).toEqual(alijos.map(a => a.posicion));
    expect(otro.alijos.every(a => a.recurso === 'oro')).toBe(true);
  });
  it('respeta el terreno apto', () => {
    const sinFranja = (p: { x: number; y: number }) => !(p.x > 14000 && p.x < 15000);
    const r = colocarBandidosYAlijos(7, limites, campamentos, vetas, sinFranja);
    for (const x of [...r.bandidos, ...r.alijos]) expect(sinFranja(x.posicion)).toBe(true);
  });
  it('sin terreno apto, falla con un error claro', () => {
    expect(() => colocarBandidosYAlijos(7, limites, campamentos, vetas, () => false)).toThrow(/Contenido/);
  });
  it('misma semilla, mismo resultado; otra semilla, otro', () => {
    expect(colocarBandidosYAlijos(7, limites, campamentos, vetas, todo)).toEqual({ bandidos, alijos });
    expect(colocarBandidosYAlijos(8, limites, campamentos, vetas, todo).bandidos).not.toEqual(bandidos);
  });
});
