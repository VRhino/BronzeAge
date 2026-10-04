// Colocación de los campamentos de mercenarios al crear el mundo (Doc 1.9b, D1/D32/D35). Mapa sintético: lo que se mide es
// la regla de dónde, no el terreno real.
import { describe, expect, it } from 'vitest';
import type { Point, ZonaBosque } from '../../domain/types';
import { MERCENARIOS } from '../../constants';
import type { Mapa } from '../../world/mapa';
import { colocarCampamentosIniciales } from '../mercenarios';
import { instanteDeTest } from './fixtures';

function mapaDe(opciones: { seed?: number; agua?: (p: Point) => boolean; bosques?: ZonaBosque[] } = {}): Mapa {
  const limites = { ancho: 2000, alto: 2000 };
  return {
    limites,
    seed: opciones.seed ?? 1,
    dentroDelMapa: (p: Point) => p.x >= 0 && p.y >= 0 && p.x <= limites.ancho && p.y <= limites.alto,
    esTransitable: () => true,
    terrenoEn: (p: Point) => (opciones.agua?.(p) ? 'agua' : 'llano'),
    listarBosques: () => opciones.bosques ?? [],
  } as unknown as Mapa;
}

const T0 = instanteDeTest(0);
const distancia = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

describe('los campamentos del mundo', () => {
  it('se separan al menos la distancia mínima, lejos del borde, y caben varios', () => {
    const cs = colocarCampamentosIniciales(mapaDe(), T0);
    expect(cs.length).toBeGreaterThanOrEqual(3);
    cs.forEach((c, i) => {
      expect(c.id).toBe(`mercenarios-${i}`);
      expect(c.posicion.x).toBeGreaterThanOrEqual(MERCENARIOS.margenBorde);
      expect(c.posicion.y).toBeLessThanOrEqual(2000 - MERCENARIOS.margenBorde);
      for (const otro of cs.slice(i + 1)) expect(distancia(c.posicion, otro.posicion)).toBeGreaterThanOrEqual(MERCENARIOS.separacion);
    });
  });

  it('salen de la seed del mapa: misma seed, mismos campamentos; otra seed, otros', () => {
    expect(colocarCampamentosIniciales(mapaDe({ seed: 7 }), T0)).toEqual(colocarCampamentosIniciales(mapaDe({ seed: 7 }), T0));
    expect(colocarCampamentosIniciales(mapaDe({ seed: 8 }), T0)).not.toEqual(colocarCampamentosIniciales(mapaDe({ seed: 7 }), T0));
  });

  it('nunca sobre agua', () => {
    const agua = (p: Point) => p.x < 1000;
    for (const c of colocarCampamentosIniciales(mapaDe({ agua }), T0)) expect(c.posicion.x).toBeGreaterThanOrEqual(1000);
  });

  it('pegados al borde de un bosque si lo hay cerca', () => {
    const bosque = { centro: { x: 1000, y: 1000 }, radio: 900 } as ZonaBosque;
    for (const c of colocarCampamentosIniciales(mapaDe({ bosques: [bosque] }), T0)) {
      const d = distancia(c.posicion, bosque.centro);
      expect(d <= bosque.radio || Math.abs(d - (bosque.radio + MERCENARIOS.pegadoAlBorde)) < 1e-6 || d - bosque.radio > MERCENARIOS.margenBosque).toBe(true);
    }
  });
});
