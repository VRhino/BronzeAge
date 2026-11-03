import { describe, expect, it } from 'vitest';
import { FACTOR } from '../campamentos';
import { componentes, construirGrafo } from '../navegacion';
import { bloquearRios, colocarVados, PARAMETROS_VADOS as P, recorrerEn4 } from '../vados';

const L = { ancho: 12000, alto: 8000 }, lado = 4 * FACTOR;
// Un río recto de norte a sur que parte el mundo en dos, y otro diagonal.
const recto = { id: 'rio-recto', puntos: [{ x: 6000, y: 0 }, { x: 6000, y: 8000 }] };
const diagonal = { id: 'rio-diag', puntos: [{ x: 0, y: 0 }, { x: 12000, y: 8000 }] };

describe('ríos y vados', () => {
  it('siempre hay al menos un vado por río y van separados', () => {
    const v = colocarVados([recto, { id: 'corto', puntos: [{ x: 0, y: 0 }, { x: 300, y: 0 }] }]);
    expect(v.get('corto')!.length).toBe(1);
    const r = v.get('rio-recto')!;
    expect(r.length).toBeGreaterThanOrEqual(1);
    for (let i = 1; i < r.length; i++) expect(r[i]!.y - r[i - 1]!.y).toBeCloseTo(P.separacion * FACTOR);
  });
  it('sin vado el río parte el mundo y con vado se cruza por un solo sitio', () => {
    const transitable = (rios: typeof recto[], vados: { x: number; y: number }[]) => {
      const m = bloquearRios(L, rios, vados, lado), nx = Math.ceil(L.ancho / lado);
      return (p: { x: number; y: number }) => m[Math.floor(p.y / lado) * nx + Math.floor(p.x / lado)] === 0;
    };
    const sin = construirGrafo(L, transitable([recto], []), () => 1000);
    expect(componentes(sin).tamanos.length).toBe(2);
    const con = construirGrafo(L, transitable([recto], [{ x: 6000, y: 4000 }]), () => 1000);
    expect(componentes(con).tamanos.length).toBe(1);
  });
  it('un río diagonal no se cruza por las esquinas', () => {
    const m = bloquearRios(L, [diagonal], [], lado), nx = Math.ceil(L.ancho / lado);
    const g = construirGrafo(L, p => m[Math.floor(p.y / lado) * nx + Math.floor(p.x / lado)] === 0, () => 1000);
    expect(componentes(g).tamanos.length).toBe(2);
  });
  it('el cauce mide una celda: la orilla de al lado queda libre', () => {
    const m = bloquearRios(L, [recto], [], lado), nx = Math.ceil(L.ancho / lado);
    // El río va por x = 6000, dentro de la celda 125 (6000 / 48 = 125): bloquea esa y deja libres las dos de al lado.
    expect([124, 125, 126].map(x => m[40 * nx + x])).toEqual([0, 1, 0]);
  });
  it('un río que pasa justo por una esquina marca también una celda de al lado', () => {
    const marcas: string[] = [];
    recorrerEn4({ x: 10, y: 10 }, { x: 130, y: 130 }, 48, (x, y) => marcas.push(`${x},${y}`));
    expect(marcas).toEqual(['0,0', '1,0', '1,1', '2,1', '2,2']);
  });
});
