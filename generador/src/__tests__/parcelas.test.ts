import { describe, expect, it } from 'vitest';
import { FACTOR } from '../campamentos';
import { generarParcelas, PARAMETROS_PARCELAS as P } from '../parcelas';

const L = { ancho: 12000, alto: 8000 };
const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
// Un bosque grande (radio 60 estratégicas), uno pequeño (radio 6) y un charco ralo que no cuenta.
const grande = { x: 3000, y: 4000, r: 60 * FACTOR }, pequeno = { x: 9000, y: 4000, r: 6 * FACTOR }, ralo = { x: 9000, y: 1000, r: 20 * FACTOR };
const densidad = (p: { x: number; y: number }) =>
  dist(p, grande) < grande.r ? 0.8 : dist(p, pequeno) < pequeno.r ? 0.7 : dist(p, ralo) < ralo.r ? 0.05 : 0;
const campamentos = [{ id: 'c', posicion: { x: 3000, y: 4000 } }];
const { masas, parcelas } = generarParcelas(L, densidad, campamentos);

describe('parcelas de bosque', () => {
  it('una masa por bosque; lo ralo no cuenta', () => {
    expect(masas).toHaveLength(2);
    const g = masas[0]!;
    expect(g.radio / FACTOR).toBeGreaterThan(55);
    expect(g.radio / FACTOR).toBeLessThan(65);
  });
  it('parcelas separadas al menos 2·rp y dentro de su masa', () => {
    for (const [i, a] of parcelas.entries()) for (const b of parcelas.slice(i + 1)) expect(dist(a.centro, b.centro)).toBeGreaterThanOrEqual(2 * P.rp * FACTOR);
    for (const q of parcelas) expect(densidad(q.centro)).toBeGreaterThanOrEqual(P.densidadMinima);
  });
  it('el bosque grande admite muchas y el pequeño al menos una', () => {
    const g = masas.find(m => m.radio / FACTOR > 20)!, p = masas.find(m => m.radio / FACTOR < 20)!;
    expect(g.parcelasIds.length).toBeGreaterThan(15);
    expect(p.parcelasIds.length).toBeGreaterThanOrEqual(1);
    expect(p.parcelasIds.length).toBeLessThanOrEqual(4);
  });
  it('un bosque continental se parte en comarcas de tamaño acotado', () => {
    const inmenso = (p: { x: number; y: number }) => (dist(p, { x: 6000, y: 4000 }) < 150 * FACTOR ? 0.8 : 0);
    const r = generarParcelas({ ancho: 12000, alto: 8000 }, inmenso, []);
    expect(r.masas.length).toBeGreaterThanOrEqual(4);
    for (const m of r.masas) expect(m.area / (P.celda * FACTOR) ** 2).toBeLessThanOrEqual(P.celdasMaximas);
    expect(r.parcelas.length).toBeGreaterThan(200);
  });
  it('cada parcela está listada en su masa y es determinista', () => {
    for (const m of masas) for (const id of m.parcelasIds) expect(parcelas.find(q => q.id === id)!.masaId).toBe(m.id);
    expect(generarParcelas(L, densidad, campamentos)).toEqual({ masas, parcelas });
  });
});
