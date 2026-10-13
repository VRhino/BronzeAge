import { describe, expect, it } from 'vitest';
import type { Point } from '../tipos';
import { componentes, construirGrafo, estadisticasGrafo, nodoMasCercano } from '../navegacion';

const E = 96;
const lim = { ancho: 10 * E, alto: 8 * E };
const llano = () => 1;
const todo = () => true;

describe('construirGrafo', () => {
  it('retícula completa: nodos, ids fila-mayor y 8 vecinos', () => {
    const g = construirGrafo(lim, todo, llano);
    expect(g.espaciado).toBe(E);
    expect(g.nodos).toHaveLength(80);
    expect(g.nodos[0]).toEqual({ id: 0, x: E / 2, y: E / 2 });
    expect(g.nodos[10]).toEqual({ id: 10, x: E / 2, y: E * 1.5 });
    g.nodos.forEach((n, i) => expect(n.id).toBe(i));
    // 10x8: horizontales 9*8, verticales 10*7, diagonales 2*9*7
    expect(g.aristas).toHaveLength(72 + 70 + 126);
    expect(g.aristas.every(a => a.a < a.b)).toBe(true);
  });

  it('el coste en llano es 1000 en ortogonales y 1414 en diagonales; el coste del terreno se promedia', () => {
    const g = construirGrafo(lim, todo, llano);
    const de = (a: number, b: number) => g.aristas.find(e => e.a === a && e.b === b)!;
    expect(de(0, 1).costeMilesimas).toBe(1000);
    expect(de(0, 10).costeMilesimas).toBe(1000);
    expect(de(0, 11).costeMilesimas).toBe(1414);
    expect(de(1, 10).costeMilesimas).toBe(1414); // SO
    // coste lineal en x: 1 + x/96; media sobre la arista (0,1), de x=48 a x=144: 2
    const g2 = construirGrafo(lim, todo, p => 1 + p.x / E);
    expect(g2.aristas.find(e => e.a === 0 && e.b === 1)!.costeMilesimas).toBe(2000);
  });

  it('un obstáculo que corta una arista la quita aunque sus extremos sean transitables', () => {
    // pozo en medio de la arista 0-1 (entre x=48 y x=144), lejos de los centros de los nodos
    const pozo = (p: Point) => !(Math.abs(p.x - E) < 10 && p.y < E);
    const g = construirGrafo(lim, pozo, llano);
    expect(g.nodos).toHaveLength(80);
    expect(g.aristas.some(e => e.a === 0 && e.b === 1)).toBe(false);
    expect(g.aristas.some(e => e.a === 0 && e.b === 10)).toBe(true);
  });

  it('islas: el mar parte el grafo y la componente mayor es la tierra grande', () => {
    // oeste: columnas 0..3; este: columnas 6..9 menos un mar en 7..9 x 3..5, con una isla de una celda en (8,4)
    const tierra = (p: Point) => {
      const c = Math.floor(p.x / E), f = Math.floor(p.y / E);
      if (c <= 3) return true;
      if (c < 6) return false;
      if (c >= 7 && f >= 3 && f <= 5) return c === 8 && f === 4;
      return true;
    };
    const g = construirGrafo(lim, tierra, llano);
    const comp = componentes(g);
    expect([...comp.tamanos].sort((a, b) => b - a)).toEqual([32, 23, 1]);
    const est = estadisticasGrafo(g, comp);
    expect(est.componentes).toBe(3);
    expect(est.nodosMayor).toBe(32);
    expect(est.fraccionMayor).toBeCloseTo(32 / 56);
  });

  it('río sin vado corta el grafo; con un paso transitable y su vado se reconecta y las aristas del paso son vado', () => {
    const rio = (paso: boolean) => (p: Point) => !(p.x > 4.2 * E && p.x < 5.8 * E && !(paso && p.y > 3 * E && p.y < 5 * E));
    const sin = construirGrafo(lim, rio(false), llano);
    expect(estadisticasGrafo(sin).componentes).toBe(2);
    const vado = { x: 5 * E, y: 4 * E };
    const con = construirGrafo(lim, rio(true), llano, [vado]);
    expect(estadisticasGrafo(con).componentes).toBe(1);
    const vados = con.aristas.filter(e => e.vado);
    expect(vados.length).toBeGreaterThan(0);
    // toda arista de vado está cerca del punto; ninguna lejos
    for (const e of vados) {
      const a = con.nodos[e.a]!, b = con.nodos[e.b]!;
      expect(Math.min(Math.hypot(a.x - vado.x, a.y - vado.y), Math.hypot(b.x - vado.x, b.y - vado.y))).toBeLessThanOrEqual(E * 1.6);
    }
    // sin vados declarados nada es vado
    expect(construirGrafo(lim, rio(true), llano).aristas.some(e => e.vado)).toBe(false);
  });

  it('es determinista', () => {
    const t = (p: Point) => Math.sin(p.x * 0.013) + Math.cos(p.y * 0.021) > -0.6;
    const c = (p: Point) => 1 + Math.abs(Math.sin(p.x * 0.01 + p.y * 0.02));
    const v = [{ x: 300, y: 300 }, { x: 600, y: 200 }];
    expect(construirGrafo(lim, t, c, v)).toEqual(construirGrafo(lim, t, c, v));
  });

  it('rendimiento: un mundo de ~83.000 nodos con transitabilidad sencilla tarda pocos segundos', () => {
    const mundo = { ancho: 2667 * 12, alto: 2000 * 12 };
    const t0 = Date.now();
    const g = construirGrafo(mundo, p => (p.x * 0.0007) % 1 > 0.02, llano, []);
    const ms = Date.now() - t0;
    expect(g.nodos.length).toBeGreaterThan(80000);
    expect(g.aristas.length).toBeGreaterThan(300000);
    expect(ms).toBeLessThan(10000);
  });

  it('nodoMasCercano devuelve el nodo del punto o nada si está lejos', () => {
    const g = construirGrafo(lim, p => p.x < 3 * E, llano);
    expect(nodoMasCercano(g, { x: E / 2 + 5, y: E / 2 })?.id).toBe(0);
    expect(nodoMasCercano(g, { x: 9 * E, y: 4 * E })).toBeUndefined();
  });
});
