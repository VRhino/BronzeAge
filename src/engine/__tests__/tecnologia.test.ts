import { describe, expect, it } from 'vitest';
import { ERAS, TARIFA_ADOPCION, TECNOLOGIAS } from '../../constants';
import type { TecnologiaId } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import { estadoTecnologiaInicial, tecnologiasDe, tieneTecnologia, TECNOLOGIAS_DE_ARRANQUE } from '../tecnologia';

const ids = Object.keys(TECNOLOGIAS) as TecnologiaId[];

describe('catálogo de tecnologías (Doc 6.6)', () => {
  it('solo las de arranque van sin logro, y todas las demás llevan logro con umbral positivo', () => {
    for (const id of ids) {
      const t = TECNOLOGIAS[id];
      if (t.deArranque) expect(t.logro, id).toBeUndefined();
      else expect(t.logro!.umbral, id).toBeGreaterThan(0);
    }
    expect([...TECNOLOGIAS_DE_ARRANQUE].sort()).toEqual(['hostigamiento_tribal', 'leva_comunal']);
  });

  it('un hito solo pide tecnologías de su Era o anteriores', () => {
    for (const id of ids) {
      for (const c of TECNOLOGIAS[id].hito) {
        if (c.tipo !== 'tecnologia') continue;
        expect(ERAS[TECNOLOGIAS[c.id].era].orden, `${id} pide ${c.id}`).toBeLessThanOrEqual(ERAS[TECNOLOGIAS[id].era].orden);
      }
    }
  });

  it('cada Era tiene tarifa de adopción', () => {
    for (const era of Object.keys(ERAS) as (keyof typeof ERAS)[]) expect(TARIFA_ADOPCION[era].oro).toBeGreaterThan(0);
  });
});

describe('estado de tecnología', () => {
  it('el servidor arranca en la Era I y una Facción sin entrada solo tiene las de arranque', () => {
    const estado = estadoTecnologiaInicial(instante(0));
    expect(estado.era).toBe('reinos_palaciales');
    expect(tecnologiasDe(estado, 'f1').adoptadas.sort()).toEqual(['hostigamiento_tribal', 'leva_comunal']);
    expect(tieneTecnologia(estado, 'f1', 'leva_comunal')).toBe(true);
    expect(tieneTecnologia(estado, 'f1', 'metalurgia_cobre')).toBe(false);
  });
});
