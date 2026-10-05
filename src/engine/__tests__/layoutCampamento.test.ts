import { describe, expect, it } from 'vitest';
import { MERCENARIOS } from '../../constants';
import type { EdificioCampamentoTipo } from '../../domain/types';
import { layoutCampamento } from '../layoutCampamento';
import type { RectanguloLocal } from '../trazado';

const campamento = (id: string, militar: EdificioCampamentoTipo) => ({ id, edificios: [...MERCENARIOS.edificiosFijos, militar] });
const solapan = (a: RectanguloLocal, b: RectanguloLocal) => a.x < b.x + b.ancho && b.x < a.x + a.ancho && a.y < b.y + b.alto && b.y < a.y + a.alto;

describe('layoutCampamento', () => {
  for (const id of ['mercenarios-0', 'mercenarios-1', 'mercenarios-2', 'mercenarios-3']) {
    for (const militar of MERCENARIOS.edificiosMilitares) {
      it(`${id} con ${militar}: todo colocado, con calles, sin solapes y dentro de la empalizada`, () => {
        const { elementos, calles, empalizada } = layoutCampamento(campamento(id, militar));
        const cuenta = (tipo: string) => elementos.filter((e) => e.tipo === tipo).length;
        expect(cuenta('taberna')).toBe(1);
        expect(cuenta('vivienda')).toBe(10);
        expect(cuenta('mercado')).toBe(1);
        expect(cuenta(militar)).toBe(1);
        expect(elementos.find((e) => e.tipo === 'taberna')!.huella).toMatchObject({ x: -6, y: -6 });
        expect(calles.length).toBeGreaterThan(0);
        for (const [i, a] of elementos.entries()) {
          for (const b of elementos.slice(i + 1)) expect(solapan(a.huella, b.huella)).toBe(false);
          for (const c of calles) expect(solapan(a.huella, c)).toBe(false);
          for (const m of [...empalizada.muro, ...empalizada.puerta]) expect(solapan(a.huella, m)).toBe(false);
        }
        expect(empalizada.puerta.length).toBeGreaterThan(0);
        const ejeX = (r: RectanguloLocal[]) => [Math.min(...r.map((m) => m.x)), Math.max(...r.map((m) => m.x + m.ancho))] as const;
        const [minX, maxX] = ejeX([...empalizada.muro, ...empalizada.puerta]);
        for (const e of elementos) expect(e.huella.x >= minX && e.huella.x + e.huella.ancho <= maxX).toBe(true);
      });
    }
  }

  it('es determinista y cada campamento tiene su propia planta', () => {
    const a = layoutCampamento(campamento('mercenarios-0', 'barracon'));
    expect(layoutCampamento(campamento('mercenarios-0', 'barracon'))).toEqual(a);
    expect(layoutCampamento(campamento('mercenarios-1', 'barracon'))).not.toEqual(a);
  });
});
