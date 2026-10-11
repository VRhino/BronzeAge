// Decisiones del usuario sobre diplomacia y Facciones (2026-10-11, `Docs/Mechanics/Diplomacia_Facciones.md`): el tributo lo pagan todas las plazas
// de la vasalla a la capital de la señora, la fusión revisa el nombre como `crearFaccion`, y la anexión no deja la marca de capital de la absorbida.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion, RelacionPolitica } from '../../domain/types';
import { avanzarTributos } from '../diplomacia';
import { exigirFusionPosible, FusionInvalidaError } from '../fusion';
import { trasladarFaccion, type MundoDeFacciones } from '../trasladoDeFaccion';
import { instanteDeTest } from './fixtures';
import { almacenSintetico } from './tradeFixtures';

const plaza = (id: string, faccionId: string, recursos: Record<string, number>, extra: Partial<Asentamiento> = {}): Asentamiento =>
  ({ id, faccionId, fundadoEn: instanteDeTest(0), edificios: [], almacen: almacenSintetico(recursos), ...extra }) as unknown as Asentamiento;
const faccion = (id: string, nombre: string, reyId: string): Faccion => ({ id, nombre, reyId, embajadorId: null, ciudadanosIds: [reyId], reputacion: 0, experiencia: 0, nivel: 1 }) as unknown as Faccion;

describe('el tributo (2026-10-11)', () => {
  const vasallaje: RelacionPolitica = {
    id: 'v1',
    tipo: 'vasallaje',
    faccionAId: 'senora',
    faccionBId: 'vasalla',
    tributo: { recurso: 'madera', cantidadPorMinuto: 10 },
    creadoEn: instanteDeTest(0),
    estado: 'activa',
  };
  const palacio = { id: 'pal', tipo: 'palacio', estado: 'activo' };

  it('pagan todas las plazas de la vasalla, a partes iguales, y todo va a la capital de la señora', () => {
    const vieja = plaza('s-vieja', 'senora', { madera: 0 }, { fundadoEn: instanteDeTest(0) });
    const capital = plaza('s-capital', 'senora', { madera: 0 }, { fundadoEn: instanteDeTest(5), capitalDeFaccionId: 'senora', edificios: [palacio] as never });
    const v1 = plaza('v1', 'vasalla', { madera: 100 });
    const v2 = plaza('v2', 'vasalla', { madera: 100 });
    const r = avanzarTributos([vasallaje], [vieja, capital, v1, v2]);
    const madera = (id: string) => r.asentamientos.find((a) => a.id === id)!.almacen['madera']!.cantidad;
    expect(madera('v1')).toBe(95);
    expect(madera('v2')).toBe(95);
    expect(madera('s-capital'), 'a la capital designada, no a la primera plaza').toBe(10);
    expect(madera('s-vieja')).toBe(0);
    expect(r.eventos).toHaveLength(2);
  });

  it('cada plaza paga hasta su parte con lo que tenga; la que no tiene nada no paga', () => {
    const capital = plaza('s', 'senora', { madera: 0 });
    const r = avanzarTributos([vasallaje], [capital, plaza('v1', 'vasalla', { madera: 2 }), plaza('v2', 'vasalla', { madera: 0 })]);
    expect(r.asentamientos.find((a) => a.id === 's')!.almacen['madera']!.cantidad).toBe(2);
    expect(r.eventos).toHaveLength(1);
  });
});

describe('la fusión revisa el nombre como crearFaccion', () => {
  const a = faccion('a', 'Alfa', 'ra');
  const b = faccion('b', 'Beta', 'rb');
  const otra = faccion('c', 'Gamma', 'rc');

  it('rechaza el nombre de una tercera Facción (sin mirar mayúsculas), y uno vacío', () => {
    expect(() => exigirFusionPosible([a, b, otra], [], 'a', 'b', ' gamma ', 'ra')).toThrow(FusionInvalidaError);
    expect(() => exigirFusionPosible([a, b, otra], [], 'a', 'b', '   ', 'ra')).toThrow(FusionInvalidaError);
  });

  it('acepta un nombre libre y el de una de las dos que se disuelven', () => {
    expect(exigirFusionPosible([a, b, otra], [], 'a', 'b', 'Delta', 'ra').nombre).toBe('Delta');
    expect(exigirFusionPosible([a, b, otra], [], 'a', 'b', 'alfa', 'ra').nombre).toBe('alfa');
  });
});

describe('la anexión no deja la marca de capital de la absorbida', () => {
  it('las plazas de B pasan a A sin la marca `capitalDeFaccionId` de B; la de A se conserva', () => {
    const mundo = {
      facciones: [faccion('a', 'A', 'ra'), faccion('b', 'B', 'rb')],
      asentamientos: [plaza('pa', 'a', {}, { capitalDeFaccionId: 'a' }), plaza('pb', 'b', {}, { capitalDeFaccionId: 'b' })],
      relaciones: [],
      ejercitos: [],
      caravanas: [],
      memoriaPorFaccion: {},
      tecnologia: { porFaccion: {}, primeros: {} },
    } as unknown as MundoDeFacciones;
    const tras = trasladarFaccion(mundo, 'b', 'a');
    expect(tras.asentamientos.find((s) => s.id === 'pb')).toMatchObject({ faccionId: 'a', capitalDeFaccionId: undefined });
    expect(tras.asentamientos.find((s) => s.id === 'pa')!.capitalDeFaccionId).toBe('a');
  });
});
