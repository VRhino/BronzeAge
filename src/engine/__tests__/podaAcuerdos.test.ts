// Los trueques terminados se podan pasada una semana (`TRUEQUE.retencionTerminadosMinutos`) salvo que una caravana los cite.
import { describe, expect, it } from 'vitest';
import type { AcuerdoTrueque, Caravana } from '../../domain/types';
import { TRUEQUE } from '../../constants';
import { instanteDeTick } from '../../session/estado';
import { podarAcuerdosTerminados } from '../trade';

const acuerdo = (id: string, estado: AcuerdoTrueque['estado'], expiraEnTick: number): AcuerdoTrueque =>
  ({ id, estado, expiraEn: instanteDeTick(expiraEnTick) }) as unknown as AcuerdoTrueque;
const caravanaCitando = (acuerdoId: string): Caravana => ({ id: 'c', origenAcuerdoId: acuerdoId }) as unknown as Caravana;

describe('podarAcuerdosTerminados', () => {
  const ahora = instanteDeTick(TRUEQUE.retencionTerminadosMinutos + 1000);

  it('quita los terminados viejos y conserva los recientes y los que siguen vivos', () => {
    const lista = [
      acuerdo('viejo-cumplido', 'cumplido', 0),
      acuerdo('viejo-expirado', 'expirado', 500),
      acuerdo('viejo-rechazado', 'rechazado', 10),
      acuerdo('reciente', 'expirado', 5000),
      acuerdo('activo', 'activo', 0),
      acuerdo('propuesto', 'propuesto', 0),
    ];
    expect(podarAcuerdosTerminados(lista, [], ahora).map((a) => a.id)).toEqual(['reciente', 'activo', 'propuesto']);
  });

  it('no quita el que una caravana todavía cita', () => {
    const lista = [acuerdo('citado', 'cumplido', 0), acuerdo('libre', 'cumplido', 0)];
    expect(podarAcuerdosTerminados(lista, [caravanaCitando('citado')], ahora).map((a) => a.id)).toEqual(['citado']);
  });

  it('sin nada que podar devuelve el mismo array', () => {
    const lista = [acuerdo('a', 'activo', 0)];
    expect(podarAcuerdosTerminados(lista, [], ahora)).toBe(lista);
  });
});
