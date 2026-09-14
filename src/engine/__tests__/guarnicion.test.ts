// La guarnición (Doc 5.15.3): el cupo que da cada plaza a sus residentes, y quién defiende en un asedio que se
// resuelve con números (5.12.4): la guarnición y el loadout activo de los residentes que están dentro.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, Recinto } from '../../domain/types';
import { GUARNICION } from '../../constants';
import { cupoGuarnicion } from '../asentamientoQuery';
import { defensaDe, guarnicionDe } from '../tropa';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest, heroeDePrueba, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(42);
const plaza = () => fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
const RESIDENTE = 'jugador-faccion-1-1';

const edificio = (tipo: 'barracon' | 'galeriaDeTiro', nivelInterno: number): Edificio => ({
  id: `${tipo}-test`,
  tipo,
  posicion: { x: 18, y: 0 },
  estado: 'activo',
  ambito: 'asentamiento',
  nivelInterno,
});

const recinto = (levantado: boolean): Recinto => ({
  id: 'r1',
  nivel: 1,
  celdas: [{ col: 0, row: 0, clase: 'muro' }, { col: 1, row: 0, clase: 'puerta' }],
  avance: levantado ? 1 : -1,
  comprometidoEn: instanteDeTest(0),
});

describe('cupoGuarnicion', () => {
  it('sin Barracón ni Galería de tiro no hay guarnición, aunque haya muralla', () => {
    expect(cupoGuarnicion({ ...plaza(), recintos: [recinto(true)] })).toBe(0);
  });

  it('suma los dos edificios por nivel, el recinto completo y la política "Levas de guarnición"', () => {
    const base: Asentamiento = { ...plaza(), edificios: [...plaza().edificios, edificio('barracon', 2), edificio('galeriaDeTiro', 1)] };
    const [, nivel2] = GUARNICION.cupoPorNivelEdificio;
    const soloEdificios = nivel2! + GUARNICION.cupoPorNivelEdificio[0]!;
    expect(cupoGuarnicion(base)).toBe(soloEdificios);

    expect(cupoGuarnicion({ ...base, recintos: [recinto(false)] }), 'un recinto a medio levantar no suma').toBe(soloEdificios);
    const conMuralla = { ...base, recintos: [recinto(true)] };
    expect(cupoGuarnicion(conMuralla)).toBe(soloEdificios + GUARNICION.recintoCompleto);

    const levas = { id: 'p1', politicaId: 'levas_guarnicion', cargo: 'general' as const, activadaEn: instanteDeTest(0), expiraEn: instanteDeTest(150) };
    expect(cupoGuarnicion({ ...conMuralla, politicasActivas: [levas] })).toBe(soloEdificios + GUARNICION.recintoCompleto + 14);
  });
});

describe('defensaDe', () => {
  const escuadras = [
    escuadronDePrueba('en-guarnicion', RESIDENTE, 'milicia_lanceros', 10, { enGuarnicion: true }),
    escuadronDePrueba('en-loadout', RESIDENTE, 'lanceros_mimbre'),
    escuadronDePrueba('en-campamento', RESIDENTE, 'honderos'),
  ];
  const conLoadout = (ubicacion: Parameters<typeof heroeDePrueba>[1]) =>
    heroeDePrueba(RESIDENTE, ubicacion, {
      escuadrones: escuadras,
      loadouts: [{ id: 'l1', displayName: 'Defensa', squadIds: ['en-loadout'], perksSeleccionados: [], activo: true }],
    });

  it('dentro de la plaza, el residente defiende con su loadout activo además de la guarnición', () => {
    const a = plaza();
    const heroes = [conLoadout({ tipo: 'asentamiento', asentamientoId: a.id })];
    expect(defensaDe(a, heroes).map((e) => e.id)).toEqual(['en-guarnicion', 'en-loadout']);
  });

  it('fuera de ella solo queda la guarnición; el resto del campamento nunca defiende', () => {
    const a = plaza();
    const heroes = [conLoadout({ tipo: 'desconectado', punto: { x: 0, y: 0 } })];
    expect(defensaDe(a, heroes).map((e) => e.id)).toEqual(['en-guarnicion']);
    expect(guarnicionDe(a, heroes).map((e) => e.id)).toEqual(['en-guarnicion']);
  });
});
