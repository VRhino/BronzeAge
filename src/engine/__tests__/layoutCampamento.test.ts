import { describe, expect, it } from 'vitest';
import { MERCENARIOS } from '../../constants';
import type { EdificioCampamentoTipo } from '../../domain/types';
import { layoutCampamento } from '../layoutCampamento';

const edificios = (militar: EdificioCampamentoTipo): EdificioCampamentoTipo[] => [...MERCENARIOS.edificiosFijos, militar];

describe('layoutCampamento', () => {
  for (let origen = 0; origen < MERCENARIOS.origenes; origen++) {
    for (const militar of MERCENARIOS.edificiosMilitares) {
      it(`variante ${origen} con ${militar}: dentro de la empalizada, sin solapes y con la puerta libre`, () => {
        const { edificios: huellas, empalizada } = layoutCampamento({ origen, edificios: edificios(militar) });
        const { colMin, colMax, rowMin, rowMax } = MERCENARIOS.layout.interior;
        const ocupadas = new Set<string>();
        for (const h of huellas) {
          for (let c = h.col; c < h.col + h.ancho; c++) {
            for (let r = h.row; r < h.row + h.alto; r++) {
              expect(c >= colMin && c <= colMax && r >= rowMin && r <= rowMax).toBe(true);
              expect(ocupadas.has(`${c},${r}`)).toBe(false);
              ocupadas.add(`${c},${r}`);
            }
          }
        }
        expect(huellas).toHaveLength(5);
        const puertas = empalizada.filter((c) => c.clase === 'puerta');
        expect(puertas).toHaveLength(2);
        for (const p of puertas) expect(ocupadas.has(`${p.col},${p.row - 1}`)).toBe(false);
      });
    }
  }

  it('las variantes se distinguen y el resultado es determinista', () => {
    const a = layoutCampamento({ origen: 0, edificios: edificios('barracon') });
    expect(layoutCampamento({ origen: 0, edificios: edificios('barracon') })).toEqual(a);
    expect(layoutCampamento({ origen: 1, edificios: edificios('barracon') })).not.toEqual(a);
    expect(layoutCampamento({ origen: 2, edificios: edificios('barracon') })).not.toEqual(a);
  });
});
