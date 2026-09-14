// La guarnición (Doc 5.15.3): el cupo que da cada plaza a sus residentes, y quién defiende en un asedio que se
// resuelve con números (5.12.4): la guarnición y el loadout activo de los residentes que están dentro.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, Ejercito, Recinto } from '../../domain/types';
import { GUARNICION } from '../../constants';
import { cupoGuarnicion } from '../asentamientoQuery';
import { desalojarResidentes } from '../combate';
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
    expect(defensaDe(a, heroes, new Set()).map((e) => e.id)).toEqual(['en-guarnicion', 'en-loadout']);
  });

  it('fuera de ella solo queda la guarnición; el resto del campamento nunca defiende', () => {
    const a = plaza();
    const heroes = [conLoadout({ tipo: 'desconectado', punto: { x: 0, y: 0 } })];
    expect(defensaDe(a, heroes, new Set()).map((e) => e.id)).toEqual(['en-guarnicion']);
    expect(guarnicionDe(a, heroes).map((e) => e.id)).toEqual(['en-guarnicion']);
  });

  it('herido, su loadout no defiende aunque esté dentro; la guarnición sí, no tiene héroe (Doc 5.16.4)', () => {
    const a = plaza();
    const heroes = [conLoadout({ tipo: 'asentamiento', asentamientoId: a.id })];
    expect(defensaDe(a, heroes, new Set([RESIDENTE])).map((e) => e.id)).toEqual(['en-guarnicion']);
  });

  describe('cuando cae la plaza (Doc 5.15.5)', () => {
    it('el defensor sale junto a ella, con las escuadras con las que defendió y el carro vacío; lo demás, a 0', () => {
      const a = plaza();
      const r = desalojarResidentes(a, [a], [conLoadout({ tipo: 'asentamiento', asentamientoId: a.id })], [], new Set(['en-loadout']), instanteDeTest(5));

      const columna = r.columnas[0]!;
      expect(r.columnas).toHaveLength(1);
      expect(columna.posicionActual).toEqual(a.posicion);
      expect(columna.suministro, 'carro vacío').toEqual({});
      expect(columna.escuadrones.map((e) => e.id)).toEqual(['en-loadout']);
      const tras = r.heroes[0]!;
      expect(tras.ubicacion).toEqual({ tipo: 'columna', ejercitoId: columna.id });
      expect(tras.escuadrones.find((e) => e.id === 'en-loadout')!.contenedor).toEqual({ tipo: 'ejercito', ejercitoId: columna.id });
      expect(
        tras.escuadrones.filter((e) => e.id !== 'en-loadout').every((e) => e.cantidad === 0 && !e.enGuarnicion),
        'guarnición y resto del campamento a 0'
      ).toBe(true);
    });

    it('un visitante vuelve a la columna que dejó aparcada, sin columna nueva', () => {
      const a = plaza();
      const visitante = heroeDePrueba('visitante', { tipo: 'asentamiento', asentamientoId: a.id });
      const aparcada: Ejercito = {
        id: 'aparcada',
        faccionId: 'faccion-2',
        origenAsentamientoId: 'otra',
        participantes: [{ heroeId: 'visitante', unidoEn: instanteDeTest(0) }],
        tipo: 'personal',
        liderId: 'visitante',
        politicaDeUnion: 'rechazar',
        escuadronIds: [],
        suministro: {},
        caravanasAdjuntasIds: [],
        objetivo: { tipo: 'punto', punto: a.posicion },
        ruta: [],
        progreso: 0,
        posicionActual: a.posicion,
        estado: 'estacionado',
      };

      const r = desalojarResidentes(a, [a], [visitante], [aparcada], new Set(), instanteDeTest(5));

      expect(r.columnas).toEqual([]);
      expect(r.heroes[0]!.ubicacion).toEqual({ tipo: 'columna', ejercitoId: 'aparcada' });
    });
  });
});
