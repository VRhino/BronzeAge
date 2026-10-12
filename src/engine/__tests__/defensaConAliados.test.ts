// Quién defiende una plaza con números (2026-10-12): sus residentes y, en el cupo que dejan, los héroes de la propia Facción o de una aliada
// (alianza o vasallaje, en cualquier sentido) que están DENTRO en ese momento, con la columna que dejaron a la puerta.
import { describe, expect, it } from 'vitest';
import { BATALLA } from '../../constants';
import type { Asentamiento, Faccion, Heroe, RelacionPolitica, UbicacionHeroe } from '../../domain/types';
import { defensoresDeLaPlaza } from '../tropa';
import { escuadronDePrueba, heroeDePrueba, instanteDeTest } from './fixtures';

const PLAZA = 'plaza-vasalla';
const plaza = { id: PLAZA, faccionId: 'vasalla', heroesFundadoresIds: [], casasCompradas: [] } as unknown as Asentamiento;
const faccion = (id: string, ciudadanos: string[]): Faccion => ({ id, nombre: id, reyId: ciudadanos[0] ?? null, embajadorId: null, ciudadanosIds: ciudadanos, reputacion: 0, experiencia: 0, nivel: 1 }) as unknown as Faccion;
const relacion = (tipo: RelacionPolitica['tipo'], a: string, b: string): RelacionPolitica => ({ id: `${tipo}-${a}-${b}`, tipo, faccionAId: a, faccionBId: b, creadoEn: instanteDeTest(0), estado: 'activa' }) as RelacionPolitica;

const dentro: UbicacionHeroe = { tipo: 'asentamiento', asentamientoId: PLAZA };
/** Un visitante con una escuadra en la columna que dejó aparcada a la puerta. */
const visitante = (id: string, nivel = 1, extra: Partial<Heroe> = {}): Heroe =>
  heroeDePrueba(id, dentro, { nivel, escuadrones: [escuadronDePrueba(`esc-${id}`, id, 'milicia_lanceros', 10, { contenedor: { tipo: 'ejercito', ejercitoId: `col-${id}` } })], ...extra });
const residente = (id: string, nivel = 1): Heroe =>
  heroeDePrueba(id, dentro, {
    nivel,
    escuadrones: [escuadronDePrueba(`esc-${id}`, id, 'milicia_lanceros', 10)],
    loadouts: [{ id: `l-${id}`, nombre: 'l', activo: true, squadIds: [`esc-${id}`] }] as never,
  });
const NINGUNO = new Set<string>();

describe('defensoresDeLaPlaza: los amigos que están dentro', () => {
  const facciones = [faccion('vasalla', []), faccion('senora', ['sen']), faccion('aliada', ['ali']), faccion('neutral', ['neu']), faccion('enemiga', ['ene'])];
  const relaciones = [relacion('vasallaje', 'senora', 'vasalla'), relacion('alianza', 'vasalla', 'aliada'), relacion('guerra', 'enemiga', 'vasalla')];

  it('el señor y el aliado que están dentro defienden con su columna aparcada; el neutral y el enemigo no', () => {
    const heroes = [visitante('sen'), visitante('ali'), visitante('neu'), visitante('ene')];
    const d = defensoresDeLaPlaza(plaza, heroes, NINGUNO, facciones, relaciones);
    expect(d.heroes.map((h) => h.id).sort()).toEqual(['ali', 'sen']);
    expect(d.escuadras.map((e) => e.id).sort()).toEqual(['esc-ali', 'esc-sen']);
  });

  it('el vasallo defiende a su señor, y quien está fuera, herido o desconectado no defiende', () => {
    const senora = { ...plaza, faccionId: 'senora' } as Asentamiento;
    const heroes = [
      visitante('van'),
      visitante('lejos', 1, { ubicacion: { tipo: 'columna', ejercitoId: 'x' } }),
      visitante('herido'),
      visitante('off', 1, { fuera: { carro: {} } }),
    ];
    const fac = [faccion('senora', []), faccion('vasalla', ['van', 'lejos', 'herido', 'off'])];
    const d = defensoresDeLaPlaza(senora, heroes, new Set(['herido']), fac, [relacion('vasallaje', 'senora', 'vasalla')]);
    expect(d.heroes.map((h) => h.id)).toEqual(['van']);
  });

  it('los residentes ocupan el cupo primero; los amigos entran en lo que queda, por nivel', () => {
    const casa = Array.from({ length: BATALLA.capacidad.asedio - 1 }, (_, i) => residente(`r${i}`));
    const plazaLlena = { ...plaza, heroesFundadoresIds: casa.map((h) => h.id) } as Asentamiento;
    const heroes = [...casa, visitante('sen', 3), visitante('ali', 9)];
    const d = defensoresDeLaPlaza(plazaLlena, heroes, NINGUNO, facciones, relaciones);
    expect(d.heroes).toHaveLength(BATALLA.capacidad.asedio);
    expect(d.heroes.map((h) => h.id)).toContain('ali');
    expect(d.heroes.map((h) => h.id), 'solo cabe uno: el de más nivel').not.toContain('sen');
  });

  it('con la casa llena no entra ningún visitante', () => {
    const casa = Array.from({ length: BATALLA.capacidad.asedio }, (_, i) => residente(`r${i}`));
    const plazaLlena = { ...plaza, heroesFundadoresIds: casa.map((h) => h.id) } as Asentamiento;
    const d = defensoresDeLaPlaza(plazaLlena, [...casa, visitante('sen', 9)], NINGUNO, facciones, relaciones);
    expect(d.heroes.map((h) => h.id)).not.toContain('sen');
  });
});
