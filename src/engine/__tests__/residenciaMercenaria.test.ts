// Residencia en campamentos de mercenarios y almacén personal (Doc 2.5, 2026-10-02): se acaba el huérfano, y cada héroe tiene
// un almacén pequeño que viaja con él. Solo regla pura; los comandos van en `session/__tests__/comandosMercenarios.test.ts`.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Ejercito, Faccion, Heroe } from '../../domain/types';
import { ALMACEN_PERSONAL } from '../../constants';
import { guardarEnAlmacenPersonal, sacarDelAlmacenPersonal, totalAlmacenPersonal } from '../almacenPersonal';
import { HeroeInvalidoError } from '../heroe';
import { acogerHeroesSinCasa, campamentoDeResidente, campamentoMasCercano, MercenariosInvalidoError, residirEnCampamento } from '../mercenarios';
import { heroeDePrueba } from './fixtures';

const campamento = (id: string, x: number, y: number, residentesIds: string[] = []): CampamentoMercenarios =>
  ({ id, posicion: { x, y }, origen: 0, edificios: ['taberna'], residentesIds, creadoEn: 0 }) as unknown as CampamentoMercenarios;

const plaza = (id: string, faccionId: string, extra: Partial<Asentamiento> = {}): Asentamiento =>
  ({ id, faccionId, posicion: { x: 0, y: 0 }, heroesFundadoresIds: [], casasCompradas: [], cargos: {}, ...extra }) as unknown as Asentamiento;

const faccion = (id: string, ciudadanosIds: string[]): Faccion => ({ id, ciudadanosIds }) as unknown as Faccion;
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

describe('acogerHeroesSinCasa: se acaba el huérfano', () => {
  const campamentos = [campamento('c1', 100, 100), campamento('c2', 1500, 1500)];
  const fuera = (id: string, ejercitoId: string, extra: Partial<Heroe> = {}) => heroeDePrueba(id, { tipo: 'columna', ejercitoId }, extra);

  it('el ciudadano sin casa pasa al campamento más cercano a donde está', () => {
    const heroes = [fuera('h1', 'e1'), fuera('h2', 'e2')];
    const ejercitos = [columna('e1', 'h1', 150, 150), columna('e2', 'h2', 1400, 1400)];
    const r = acogerHeroesSinCasa(campamentos, [], heroes, [faccion('f', ['h1', 'h2'])], ejercitos);

    expect(campamentoDeResidente(r, 'h1')?.id).toBe('c1');
    expect(campamentoDeResidente(r, 'h2')?.id).toBe('c2');
  });

  it('no toca a quien ya tiene casa o campamento, a quien no tiene Facción, ni a los bots', () => {
    const conCasa = plaza('a', 'f', { casasCompradas: ['h1'] });
    const heroes = [fuera('h1', 'e1'), fuera('h2', 'e1'), fuera('h3', 'e1'), fuera('bot', 'e1', { controlador: 'bot' })];
    const yaResidente = [campamento('c1', 100, 100, ['h3']), campamentos[1]!];
    const r = acogerHeroesSinCasa(yaResidente, [conCasa], heroes, [faccion('f', ['h1', 'h3', 'bot'])], [columna('e1', 'h1', 0, 0)]);

    expect(r.flatMap((c) => c.residentesIds)).toEqual(['h3']); // h1 tiene casa, h2 no es ciudadano, h3 ya reside, el bot se queda como está
  });

  it('sin posición conocida va al primero, y sin campamentos no hace nada', () => {
    const sinUbicar = heroeDePrueba('h1', { tipo: 'ninguna' } as unknown as Heroe['ubicacion']);
    expect(acogerHeroesSinCasa(campamentos, [], [sinUbicar], [faccion('f', ['h1'])], [])[0]!.residentesIds).toEqual(['h1']);
    expect(acogerHeroesSinCasa([], [], [sinUbicar], [faccion('f', ['h1'])], [])).toEqual([]);
  });

  it('campamentoMasCercano desempata por id', () => {
    const gemelos = [campamento('b', 100, 0), campamento('a', -100, 0)];
    expect(campamentoMasCercano(gemelos, { x: 0, y: 0 })?.id).toBe('a');
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
