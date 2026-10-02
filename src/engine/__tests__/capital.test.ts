// La capital de la Facción (Doc 2.2): quién es, y el traslado con Palacio y cooldown.
import { describe, expect, it } from 'vitest';
import { CAPITAL } from '../../constants';
import { dias, sumar } from '../../domain/tiempo';
import type { Asentamiento, Faccion } from '../../domain/types';
import { designarCapital, CapitalInvalidaError } from '../capital';
import { encontrarCapital } from '../mantenimiento';
import { instanteDeTest } from './fixtures';

const palacio = { id: 'p', tipo: 'palacio', estado: 'activo' };
/** Plazas mínimas: solo los campos que lee la regla. */
const plaza = (id: string, fundadoEn: number, conPalacio: boolean, extra: Partial<Asentamiento> = {}) =>
  ({ id, faccionId: 'f', fundadoEn, edificios: conPalacio ? [palacio] : [], ...extra }) as unknown as Asentamiento;
const faccion = (extra: Partial<Faccion> = {}) => ({ id: 'f', nombre: 'F', ...extra }) as Faccion;

const vieja = () => plaza('vieja', 1, false);
const nueva = () => plaza('nueva', 2, true);
const T0 = instanteDeTest(0);

describe('encontrarCapital', () => {
  it('sin designar, el asentamiento vivo más antiguo', () => {
    expect(encontrarCapital('f', [nueva(), vieja()])?.id).toBe('vieja');
  });

  it('la designada gana mientras conserve el Palacio y sea de la Facción', () => {
    expect(encontrarCapital('f', [vieja(), { ...nueva(), capitalDeFaccionId: 'f' }])?.id).toBe('nueva');
    // Perdió el Palacio: vuelve al más antiguo.
    expect(encontrarCapital('f', [vieja(), { ...nueva(), capitalDeFaccionId: 'f', edificios: [] }])?.id).toBe('vieja');
    // Cambió de manos: la marca era de la otra Facción.
    expect(encontrarCapital('g', [{ ...nueva(), faccionId: 'g', capitalDeFaccionId: 'f' }, plaza('g1', 1, false, { faccionId: 'g' })])?.id).toBe('g1');
  });
});

describe('designarCapital', () => {
  it('exige un asentamiento propio con Palacio activo', () => {
    expect(() => designarCapital(faccion(), [vieja(), nueva()], 'vieja', T0)).toThrow(CapitalInvalidaError);
    expect(() => designarCapital(faccion(), [vieja(), nueva()], 'no-existe', T0)).toThrow(CapitalInvalidaError);
    expect(() => designarCapital(faccion(), [{ ...nueva(), faccionId: 'otra' }], 'nueva', T0)).toThrow(CapitalInvalidaError);
  });

  it('designa, marca solo esa plaza y anota cuándo', () => {
    const r = designarCapital(faccion(), [vieja(), nueva()], 'nueva', T0);
    expect(r.asentamientos.find((a) => a.id === 'nueva')?.capitalDeFaccionId).toBe('f');
    expect(r.faccion.capitalDesignadaEn).toBe(T0);
    expect(encontrarCapital('f', r.asentamientos)?.id).toBe('nueva');
  });

  it('rechaza designar la que ya es capital', () => {
    const una = plaza('vieja', 1, true);
    expect(() => designarCapital(faccion(), [una], 'vieja', T0)).toThrow(CapitalInvalidaError);
  });

  it('el traslado tiene cooldown, y se levanta al pasar el plazo', () => {
    const otra = plaza('otra', 3, true);
    const primera = designarCapital(faccion(), [vieja(), nueva(), otra], 'nueva', T0);
    const pronto = sumar(T0, dias(CAPITAL.cooldownDias - 1));
    expect(() => designarCapital(primera.faccion, primera.asentamientos, 'otra', pronto)).toThrow(CapitalInvalidaError);

    const despues = designarCapital(primera.faccion, primera.asentamientos, 'otra', sumar(T0, dias(CAPITAL.cooldownDias)));
    expect(despues.asentamientos.filter((a) => a.capitalDeFaccionId === 'f').map((a) => a.id)).toEqual(['otra']);
  });

  it('si la capital designada se perdió, no hay cooldown', () => {
    const otra = plaza('otra', 3, true);
    const primera = designarCapital(faccion(), [vieja(), nueva(), otra], 'nueva', T0);
    const sinPalacio = primera.asentamientos.map((a) => (a.id === 'nueva' ? { ...a, edificios: [] } : a));
    expect(() => designarCapital(primera.faccion, sinPalacio, 'otra', sumar(T0, dias(1)))).not.toThrow();
  });
});
