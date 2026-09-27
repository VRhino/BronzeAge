// Solo el combate DIGNO da experiencia de Facción (`esCombateDigno`, engine/combate.ts; decisión del usuario
// 2026-09-27). En la Era I medida, los asedios suicidas y los campamentos de bandidos aplastados daban experiencia
// igual que un combate reñido, y una Facción llegaba a nivel 10 en 1,2 días.
import { describe, expect, it } from 'vitest';
import type { CampamentoBandido } from '../../domain/types';
import { NIVEL_FACCION } from '../../constants';
import { atacarCampamentoBandidos, esCombateDigno, poderTotal } from '../combate';
import { createRng } from '../../worldgen';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest } from './fixtures';

function atacarCampamentoDePoder(poderRelativoDelCampamento: number): number {
  const mapa = crearMapaDeterminista(7);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const tropa = [escuadronDePrueba('e1', 'heroe-1')];
  const campamento: CampamentoBandido = {
    id: 'bandidos-1',
    posicion: asentamiento.posicion,
    bosqueId: 'bosque-1',
    asentamientoId: asentamiento.id,
    poder: poderTotal(tropa, false) * poderRelativoDelCampamento,
  } as CampamentoBandido;
  const r = atacarCampamentoBandidos(asentamiento, tropa, ['e1'], campamento, facciones, createRng(1));
  const antes = facciones.find((f) => f.id === 'faccion-1')!.experiencia;
  return r.facciones.find((f) => f.id === 'faccion-1')!.experiencia - antes;
}

describe('combate digno', () => {
  it('lo es si el bando débil tiene al menos `ratioCombateDigno` del poder del fuerte', () => {
    expect(esCombateDigno(100, 100 * NIVEL_FACCION.ratioCombateDigno)).toBe(true);
    expect(esCombateDigno(100 * NIVEL_FACCION.ratioCombateDigno - 1, 100)).toBe(false);
  });

  it('aplastar un campamento de bandidos muy inferior no da experiencia', () => {
    expect(atacarCampamentoDePoder(0.1)).toBe(0);
  });

  it('un choque reñido sí la da', () => {
    expect(atacarCampamentoDePoder(0.9)).toBe(NIVEL_FACCION.xp.combate);
  });
});
