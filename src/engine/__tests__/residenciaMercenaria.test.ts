// Residencia en campamentos de mercenarios y almacén personal (Doc 2.5, 2026-10-02): se acaba el huérfano —se reubica en el
// momento de perder la casa—, y cada héroe tiene un almacén pequeño que viaja con él. Solo regla pura; los comandos van en `session/__tests__/comandosMercenarios.test.ts`.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Ejercito, Heroe } from '../../domain/types';
import { ALMACEN_PERSONAL } from '../../constants';
import { guardarEnAlmacenPersonal, sacarDelAlmacenPersonal, totalAlmacenPersonal } from '../almacenPersonal';
import { HeroeInvalidoError } from '../heroe';
import {
  acogerEnCampamentoMasCercano,
  campamentoDeResidente,
  campamentoMasCercano,
  MercenariosInvalidoError,
  posicionDeHeroe,
  reubicarResidentesDeRuina,
  residirEnCampamento,
} from '../mercenarios';
import { heroeDePrueba } from './fixtures';

const campamento = (id: string, x: number, y: number, residentesIds: string[] = []): CampamentoMercenarios =>
  ({ id, posicion: { x, y }, origen: 0, edificios: ['taberna'], residentesIds, creadoEn: 0 }) as unknown as CampamentoMercenarios;

const plaza = (id: string, faccionId: string, extra: Partial<Asentamiento> = {}): Asentamiento =>
  ({ id, faccionId, posicion: { x: 0, y: 0 }, heroesFundadoresIds: [], casasCompradas: [], cargos: {}, ...extra }) as unknown as Asentamiento;

const columna = (id: string, liderId: string, x: number, y: number, suministro: Record<string, number> = {}): Ejercito =>
  ({ id, liderId, posicionActual: { x, y }, suministro }) as unknown as Ejercito;

describe('residirEnCampamento', () => {
  const campamentos = [campamento('c1', 100, 100), campamento('c2', 900, 900)];

  it('cualquier héroe reside en el campamento, sin límite de plazas', () => {
    const r1 = residirEnCampamento(campamentos, [], 'h1', 'c1');
    const r2 = residirEnCampamento(r1.campamentos, [], 'h2', 'c1');
    expect(campamentoDeResidente(r2.campamentos, 'h1')?.id).toBe('c1');
    expect(r2.campamentos[0]!.residentesIds).toEqual(['h1', 'h2']);
  });

  it('deja la casa y sus cargos locales al pasar a un campamento, y se reside en un solo sitio', () => {
    const casa = plaza('a', 'f', { casasCompradas: ['h1'], cargos: { gobernadorId: 'h1' } as Asentamiento['cargos'] });
    const r = residirEnCampamento(campamentos, [casa], 'h1', 'c1');
    expect(r.asentamientos[0]!.casasCompradas).toEqual([]);
    expect(r.asentamientos[0]!.cargos.gobernadorId).toBeNull();

    // Y de un campamento a otro: sale del primero.
    const otro = residirEnCampamento(r.campamentos, r.asentamientos, 'h1', 'c2');
    expect(campamentoDeResidente(otro.campamentos, 'h1')?.id).toBe('c2');
    expect(otro.campamentos[0]!.residentesIds).toEqual([]);
  });

  it('rechaza un campamento que no existe y repetir el mismo', () => {
    expect(() => residirEnCampamento(campamentos, [], 'h1', 'nada')).toThrow(MercenariosInvalidoError);
    const ya = residirEnCampamento(campamentos, [], 'h1', 'c1').campamentos;
    expect(() => residirEnCampamento(ya, [], 'h1', 'c1')).toThrow(MercenariosInvalidoError);
  });
});

describe('se acaba el huérfano: se reubica en el momento de perder la casa', () => {
  const campamentos = [campamento('c1', 100, 100), campamento('c2', 1500, 1500)];

  it('acogerEnCampamentoMasCercano lleva a los héroes al campamento más cercano al sitio perdido', () => {
    const r = acogerEnCampamentoMasCercano(campamentos, ['h1', 'h2'], { x: 1400, y: 1400 });
    expect(campamentoDeResidente(r, 'h1')?.id).toBe('c2');
    expect(campamentoDeResidente(r, 'h2')?.id).toBe('c2');
  });

  it('quien ya residía en otro campamento se muda al más cercano; sin campamentos o sin héroes no cambia nada', () => {
    const yaEnC1 = [campamento('c1', 100, 100, ['h1']), campamentos[1]!];
    const r = acogerEnCampamentoMasCercano(yaEnC1, ['h1'], { x: 1400, y: 1400 });
    expect(r[0]!.residentesIds).toEqual([]);
    expect(r[1]!.residentesIds).toEqual(['h1']);
    expect(acogerEnCampamentoMasCercano([], ['h1'], { x: 0, y: 0 })).toEqual([]);
    expect(acogerEnCampamentoMasCercano(campamentos, [], { x: 0, y: 0 })).toEqual(campamentos);
  });

  it('campamentoMasCercano desempata por id', () => {
    const gemelos = [campamento('b', 100, 0), campamento('a', -100, 0)];
    expect(campamentoMasCercano(gemelos, { x: 0, y: 0 })?.id).toBe('a');
  });

  it('posicionDeHeroe: su columna o su plaza, y nada si no está en ninguna', () => {
    const casa = plaza('a', 'f', { posicion: { x: 7, y: 8 } });
    expect(posicionDeHeroe(heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'e1' }), [], [columna('e1', 'h1', 3, 4)])).toEqual({ x: 3, y: 4 });
    expect(posicionDeHeroe(heroeDePrueba('h1', { tipo: 'asentamiento', asentamientoId: 'a' }), [casa], [])).toEqual({ x: 7, y: 8 });
    expect(posicionDeHeroe(heroeDePrueba('h1', { tipo: 'ninguna' } as unknown as Heroe['ubicacion']), [casa], [])).toBeUndefined();
  });
});

describe('ruina de una plaza: los residentes no se quedan sin casa', () => {
  const ruina = plaza('ruina', 'f', { heroesFundadoresIds: ['h1'], casasCompradas: ['h2'], posicion: { x: 50, y: 50 } });
  const campamentos = [campamento('c1', 100, 100)];

  it('van a la plaza más cercana de su Facción si le queda alguna', () => {
    const lejos = plaza('lejos', 'f', { posicion: { x: 900, y: 900 } });
    const cerca = plaza('cerca', 'f', { posicion: { x: 60, y: 60 } });
    const ajena = plaza('ajena', 'g', { posicion: { x: 51, y: 51 } });
    const r = reubicarResidentesDeRuina(ruina, [lejos, cerca, ajena], campamentos);

    expect(r.asentamientos.find((a) => a.id === 'cerca')!.casasCompradas).toEqual(['h1', 'h2']);
    expect(r.asentamientos.find((a) => a.id === 'lejos')!.casasCompradas).toEqual([]);
    expect(r.campamentos[0]!.residentesIds).toEqual([]);
  });

  it('sin plazas de su Facción, al campamento de mercenarios más cercano', () => {
    const r = reubicarResidentesDeRuina(ruina, [plaza('ajena', 'g')], campamentos);
    expect(r.campamentos[0]!.residentesIds).toEqual(['h1', 'h2']);
  });
});

describe('almacén personal', () => {
  const heroe = heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'e1' });

  it('guarda del carro lo que cabe hasta el tope, y lo saca de vuelta hasta lo que cabe en el carro', () => {
    const e = columna('e1', 'h1', 0, 0, { madera: 300, oro: 50 });
    const g = guardarEnAlmacenPersonal(heroe, e, 'madera', 200);
    expect(g.movido).toBe(200);
    expect(g.heroe.almacenPersonal).toEqual({ madera: 200 });
    expect(g.ejercito.suministro).toEqual({ madera: 100, oro: 50 });

    const s = sacarDelAlmacenPersonal(g.heroe, g.ejercito, 'madera', 500, 170);
    expect(s.movido, 'el carro de 170 lleva ya 150: solo caben 20').toBe(20);
    expect(s.heroe.almacenPersonal).toEqual({ madera: 180 });
    expect(s.ejercito.suministro['madera']).toBe(120);
  });

  it('el tope es en total, de cualquier recurso', () => {
    const lleno = { ...heroe, almacenPersonal: { piedra: ALMACEN_PERSONAL.capacidad - 10 } };
    const g = guardarEnAlmacenPersonal(lleno, columna('e1', 'h1', 0, 0, { oro: 100 }), 'oro', 100);
    expect(g.movido).toBe(10);
    expect(totalAlmacenPersonal(g.heroe)).toBe(ALMACEN_PERSONAL.capacidad);
    expect(() => guardarEnAlmacenPersonal(g.heroe, columna('e1', 'h1', 0, 0, { oro: 100 }), 'oro', 1)).toThrow(HeroeInvalidoError);
  });

  it('solo el Líder, con cantidades positivas y recursos que haya', () => {
    const e = columna('e1', 'h1', 0, 0, { madera: 10 });
    expect(() => guardarEnAlmacenPersonal({ ...heroe, id: 'h2' }, e, 'madera', 5)).toThrow(HeroeInvalidoError);
    expect(() => guardarEnAlmacenPersonal(heroe, e, 'madera', 0)).toThrow(HeroeInvalidoError);
    expect(() => guardarEnAlmacenPersonal(heroe, e, 'piedra', 5)).toThrow(HeroeInvalidoError);
    expect(() => sacarDelAlmacenPersonal(heroe, e, 'madera', 5, 100)).toThrow(HeroeInvalidoError); // no hay nada guardado
  });

  it('un recurso que queda en cero desaparece del registro', () => {
    const e = columna('e1', 'h1', 0, 0, { madera: 10 });
    const g = guardarEnAlmacenPersonal(heroe, e, 'madera', 10);
    expect(g.ejercito.suministro).toEqual({});
    expect(sacarDelAlmacenPersonal(g.heroe, g.ejercito, 'madera', 10, 100).heroe.almacenPersonal).toEqual({});
  });
});
