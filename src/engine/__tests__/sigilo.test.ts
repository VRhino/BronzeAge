import { describe, expect, it } from 'vitest';
import type { Sigilo } from '../../domain/types';
import { CATALOGO_SIGILO } from '../../constants';
import { motivoSigiloRechazado, sigiloLibre } from '../sigilo';

const VALIDO: Sigilo = {
  formaId: 'clasico',
  campoId: 'liso',
  emblemaId: 'toro',
  colorPrimarioId: 'rojo',
  colorSecundarioId: 'oro',
  colorEmblemaId: 'blanco',
  orlaId: 'ninguna',
  colorOrlaId: 'negro',
};

describe('motivoSigiloRechazado', () => {
  it('acepta uno del catálogo que nadie lleva', () => {
    expect(motivoSigiloRechazado(VALIDO, [])).toBeNull();
  });

  it('rechaza ids fuera del catálogo, colores repetidos y los emblemas reservados', () => {
    expect(motivoSigiloRechazado({ ...VALIDO, campoId: 'inventado' }, [])).toBe('invalido');
    expect(motivoSigiloRechazado({ ...VALIDO, formaId: 'inventada' }, [])).toBe('invalido');
    expect(motivoSigiloRechazado({ ...VALIDO, orlaId: 'inventada' }, [])).toBe('invalido');
    expect(motivoSigiloRechazado({ ...VALIDO, colorEmblemaId: 'inventado' }, [])).toBe('invalido');
    expect(motivoSigiloRechazado({ ...VALIDO, colorSecundarioId: 'rojo' }, [])).toBe('invalido');
    expect(motivoSigiloRechazado(CATALOGO_SIGILO.reservados.bandidos, [])).toBe('invalido');
  });

  it('rechaza solo el duplicado exacto: cambiar un color basta', () => {
    expect(motivoSigiloRechazado(VALIDO, [{ ...VALIDO }])).toBe('duplicado');
    expect(motivoSigiloRechazado(VALIDO, [{ ...VALIDO, colorSecundarioId: 'blanco' }])).toBeNull();
    expect(motivoSigiloRechazado(VALIDO, [{ ...VALIDO, formaId: 'aspis' }])).toBeNull();
    expect(motivoSigiloRechazado(VALIDO, [{ ...VALIDO, colorEmblemaId: 'negro' }])).toBeNull();
  });

  it('sin orla el color de la orla no distingue; con orla sí', () => {
    expect(motivoSigiloRechazado(VALIDO, [{ ...VALIDO, colorOrlaId: 'verde' }])).toBe('duplicado');
    const conOrla = { ...VALIDO, orlaId: 'greca' };
    expect(motivoSigiloRechazado(conOrla, [{ ...conOrla, colorOrlaId: 'verde' }])).toBeNull();
    expect(motivoSigiloRechazado(conOrla, [{ ...conOrla }])).toBe('duplicado');
  });
});

describe('sigiloLibre', () => {
  it('es determinista, válido y salta los ocupados', () => {
    const primero = sigiloLibre('faccion-1', []);
    expect(sigiloLibre('faccion-1', [])).toEqual(primero);
    expect(motivoSigiloRechazado(primero, [])).toBeNull();
    const segundo = sigiloLibre('faccion-1', [primero]);
    expect(segundo).not.toEqual(primero);
    expect(motivoSigiloRechazado(segundo, [primero])).toBeNull();
  });

  it('300 ids dan 300 sigilos válidos y distintos', () => {
    const ocupados: Sigilo[] = [];
    for (let i = 0; i < 300; i++) ocupados.push(sigiloLibre(`faccion-${i}`, ocupados));
    expect(new Set(ocupados.map((s) => JSON.stringify(s))).size).toBe(300);
    for (const s of ocupados) expect(motivoSigiloRechazado(s, [])).toBeNull();
  });
});
