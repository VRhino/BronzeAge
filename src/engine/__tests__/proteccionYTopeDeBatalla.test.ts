// Decisiones del usuario del 2026-09-27: una plaza recién fundada nace protegida un día (Doc 5.12.9), la última plaza
// que colapsa deja a su Facción derrotada sin ganador, y en una batalla con números entran como mucho
// `BATALLA.capacidad` héroes por bando: los de más nivel y mejores escuadras (Doc 5.15.1).
import { describe, expect, it } from 'vitest';
import type { Heroe } from '../../domain/types';
import { BATALLA, OCUPACION } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import { registrarDerrota } from '../faccion';
import { fundarAsentamiento } from '../settlement';
import { defensaDe, heroesQueEntranADefender } from '../tropa';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest, heroeDePrueba, instanteDeTest } from './fixtures';

describe('protección al fundar', () => {
  it('una plaza nace protegida un día', () => {
    const mapa = crearMapaDeterminista(7);
    const pos = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento.posicion;
    const { asentamiento } = fundarAsentamiento(mapa, crearFacciones(), 'faccion-1', pos, ['h1'], [], instanteDeTest(50));
    expect(asentamiento.protegidaHasta).toEqual(sumar(instanteDeTest(50), minutos(OCUPACION.proteccionMinutos)));
  });
});

describe('derrota sin ganador', () => {
  it('perder la última plaza por colapso deja `derrotadaPor: null`', () => {
    expect(registrarDerrota(crearFacciones(), [], 'faccion-2', null).find((f) => f.id === 'faccion-2')!.derrotadaPor).toBeNull();
  });
});

describe('tope de héroes por bando en un asedio con números', () => {
  const mapa = crearMapaDeterminista(7);
  const tope = BATALLA.capacidad.asedio;
  const ids = Array.from({ length: tope + 2 }, (_, i) => `h${i + 1}`);
  const plaza = { ...fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento, heroesFundadoresIds: [], casasCompradas: ids };
  // h1 es de nivel 3 pero con la escuadra más pequeña; del resto, a más número, más hombres.
  const heroes: Heroe[] = ids.map((id, i) => {
    const escuadra = escuadronDePrueba(`e-${id}`, id, 'milicia_lanceros', i === 0 ? 1 : 10 + i);
    return heroeDePrueba(id, { tipo: 'asentamiento', asentamientoId: plaza.id }, {
      controlador: 'bot',
      escuadrones: [escuadra],
      nivel: i === 0 ? 3 : 1,
      loadouts: [{ id: `${id}-l`, displayName: 'Default', squadIds: [escuadra.id], perksSeleccionados: [], activo: true }],
    });
  });

  it('entran los de más nivel y, a igual nivel, los de escuadras más fuertes', () => {
    const entran = heroesQueEntranADefender(plaza, heroes, new Set()).map((h) => h.id);
    expect(entran).toHaveLength(tope);
    expect(entran[0]).toBe('h1');
    expect(entran).not.toContain('h2');
    expect(entran).not.toContain('h3');
  });

  it('las escuadras de los que no entran no defienden', () => {
    expect(defensaDe(plaza, heroes, new Set())).toHaveLength(tope);
  });
});
