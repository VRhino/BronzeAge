import { describe, expect, it } from 'vitest';
import { anilloEn, colocarCampamentos, ESTRATEGICAS, FACTOR } from '../campamentos';
import { cuotasPorAnillo, PARAMETROS_METALES as P, repartirMetales, type Sitio } from '../metales';

const ANCHO = 32004, ALTO = 24000;
const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);

// Mundo llano sintético: todo apto. Comprueba las reglas, no la geografía.
const campamentos = colocarCampamentos(7, { ancho: ANCHO, alto: ALTO }, () => 1);
const sitios: Sitio[] = [];
for (let y = 60; y < ALTO; y += 120) for (let x = 60; x < ANCHO; x += 120) sitios.push({ posicion: { x, y }, calidad: { estano: 0.7, oro: 0.7 } });

describe('campamentos', () => {
  it('separados al menos 600 estratégicas y con margen de borde', () => {
    expect(campamentos.length).toBeGreaterThanOrEqual(8);
    for (const [i, a] of campamentos.entries()) {
      expect(a.posicion.x).toBeGreaterThanOrEqual(ESTRATEGICAS.margenBorde * FACTOR);
      for (const b of campamentos.slice(i + 1)) expect(dist(a.posicion, b.posicion)).toBeGreaterThanOrEqual(ESTRATEGICAS.separacionCampamentos * FACTOR);
    }
  });
  it('misma semilla, mismos campamentos', () => {
    expect(colocarCampamentos(7, { ancho: ANCHO, alto: ALTO }, () => 1)).toEqual(campamentos);
  });
});

describe('estaño y oro por gradiente', () => {
  const { nodos, vetas } = repartirMetales(7, campamentos, sitios);
  const de = (tipo: string, anillo?: string) => nodos.filter(n => n.tipo === tipo && (!anillo || n.anillo === anillo));

  it('las cuotas por anillo suman exactamente el total', () => {
    expect(cuotasPorAnillo(56, P.reparto)).toEqual({ foco: 3, frontera: 14, lejana: 39 });
    expect(cuotasPorAnillo(48, P.reparto)).toEqual({ foco: 2, frontera: 12, lejana: 34 });
    expect(de('estano')).toHaveLength(56);
    expect(de('oro')).toHaveLength(48);
    expect(de('estano', 'lejana')).toHaveLength(39);
  });
  it('cada nodo está en el anillo que declara', () => {
    for (const n of nodos) expect(anilloEn(campamentos, n.posicion).anillo).toBe(n.anillo);
  });
  it('el foco es una muestra en su borde exterior y con un tercio de cantidad', () => {
    for (const n of nodos.filter(n => n.anillo === 'foco')) {
      const d = anilloEn(campamentos, n.posicion).distancia;
      expect(d).toBeGreaterThanOrEqual(P.bordeFoco.min);
      expect(d).toBeLessThanOrEqual(P.bordeFoco.max);
      expect(n.cantidadFactor).toBeCloseTo(1 / 3);
    }
  });
  it('vetas de 3 a 6 estaños con 0 a 2 oros, dentro del radio', () => {
    expect(vetas.length).toBe(9);
    for (const v of vetas) {
      const propios = nodos.filter(n => n.vetaId === v.id);
      const estano = propios.filter(n => n.tipo === 'estano').length, oro = propios.length - estano;
      expect(estano).toBeGreaterThanOrEqual(P.tamanoVeta.min);
      expect(estano).toBeLessThanOrEqual(P.tamanoVeta.max);
      expect(oro).toBeLessThanOrEqual(2);
      for (const n of propios) expect(dist(n.posicion, v.centro)).toBeLessThanOrEqual(P.radioVeta.max * FACTOR);
      expect(v.nodosIds.sort()).toEqual(propios.map(n => n.id).sort());
    }
  });
  it('ids únicos y determinista', () => {
    expect(new Set(nodos.map(n => n.id)).size).toBe(nodos.length);
    expect(repartirMetales(7, campamentos, sitios).nodos).toEqual(nodos);
  });
});
