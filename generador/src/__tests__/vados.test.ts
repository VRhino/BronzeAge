import { describe, expect, it } from 'vitest';
import { FACTOR } from '../campamentos';
import { componentes, construirGrafo } from '../navegacion';
import { bloquearRios, colocarVados, PARAMETROS_VADOS as P } from '../vados';

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
});
