// Gate de nivel 2 en la construcción BASE de Barracón y Galería de tiro (ver
// Consideraciones/Vista_Asentamiento_Trazado_Urbano.md §5.7.1 y Docs/4 §4.2.1).
//
// POR QUÉ EXISTE EL GATE: son los dos únicos tipos capaces de abrir el grupo militar en el trazado urbano, y
// el primero que se construye arrastra consigo la Plaza de Armas. Al fundar, el disco urbano mide 5 celdas y
// no hay ningún hueco que respete la separación mínima entre anclas — el núcleo militar nacía pegado al
// Centro Urbano y se quedaba ahí el resto de la partida, porque ningún ancla se muda nunca. Sin gate era
// alcanzable en el TICK 1: el Barracón cuesta 30 de madera y la caravana de fundación entrega 50.
//
// Y POR QUÉ VA EN LA CONSTRUCCIÓN Y NO EN EL RECLUTAMIENTO: el último test protege una AUSENCIA deliberada.
// `reclutarTropa` no comprueba el nivel del asentamiento a propósito — un asentamiento que sube a nivel 2,
// construye Armería y Barracón y luego se degrada a nivel 1 sigue pudiendo reclutar con lo que ya tiene. Los
// edificios y el equipo están físicamente ahí; perder nivel no borra lo que ya levantaste. Sin este test, esa
// ausencia se lee como un olvido y alguien la "arregla".
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, EdificioTipo } from '../../domain/types';
import { EDIFICIO_CATALOGO } from '../../constants';
import { anadirEdificioManualmente, ConstruccionManualInvalidaError } from '../construction';
import { reclutarTropa } from '../tropas';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, SIN_MUNDO } from './fixtures';

const SEED = 42;
const RECLAMOS_VACIOS = { nodos: new Set<string>(), lenerasPorBosque: new Map<string, number>() };
const TIPOS_MILITARES: EdificioTipo[] = ['barracon', 'galeriaDeTiro'];

/** Asentamiento fundado, con Gobernador asignado y madera de sobra: así lo único que puede hacer fallar a
 * `anadirEdificioManualmente` es el gate de nivel, no la falta de cargo ni de fondos. */
function base(nivel: number) {
  const mapa = crearMapaDeterminista(SEED);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const preparado: Asentamiento = {
    ...asentamiento,
    // Los dos: el gate lee `nivelActualDe`, que devuelve `nivelActual ?? nivel` — y un asentamiento recién
    // fundado ya trae `nivelActual` explícito, así que tocar solo `nivel` no cambiaría nada.
    nivel,
    nivelActual: nivel,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    almacen: { ...asentamiento.almacen, madera: { ...asentamiento.almacen.madera!, cantidad: 500 } },
  };
  return { asentamiento: preparado, faccion: facciones[0]!, mapa };
}

describe('Barracón / Galería de tiro — gate de construcción a nivel 2', () => {
  it('el catálogo declara el gate en los dos tipos', () => {
    for (const tipo of TIPOS_MILITARES) {
      const catalogo = EDIFICIO_CATALOGO[tipo] as { requisitoNivelAsentamientoConstruccion?: number };
      expect(catalogo.requisitoNivelAsentamientoConstruccion).toBe(2);
    }
  });

  it('no se pueden añadir a mano en un asentamiento de nivel 1', () => {
    for (const tipo of TIPOS_MILITARES) {
      const { asentamiento, faccion, mapa } = base(1);
      expect(() =>
        anadirEdificioManualmente(asentamiento, faccion, 'gobernador', tipo, [], mapa, undefined, RECLAMOS_VACIOS)
      ).toThrow(ConstruccionManualInvalidaError);
    }
  });

  it('en nivel 2 sí entran en la cola', () => {
    for (const tipo of TIPOS_MILITARES) {
      const { asentamiento, faccion, mapa } = base(2);
      const resultado = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', tipo, [], mapa, undefined, RECLAMOS_VACIOS);
      expect(resultado.edificios.some((e) => e.tipo === tipo && e.estado === 'en_cola')).toBe(true);
    }
  });

  it('un asentamiento DEGRADADO a nivel 1 sigue reclutando con el Barracón que ya construyó', () => {
    const { asentamiento } = base(2);
    const barracon: Edificio = {
      id: `barracon-${asentamiento.id}`,
      tipo: 'barracon',
      posicion: { x: 18, y: 0 },
      estado: 'activo',
      ambito: 'asentamiento',
    };
    // `nivelActual` 1 sobre `nivel` 2 es exactamente la forma de un asentamiento degradado (Doc Fase_0_5 §6.2):
    // ya alcanzó el nivel que permitió construir el Barracón, pero ahora opera por debajo.
    const degradado: Asentamiento = {
      ...asentamiento,
      nivelActual: 1,
      edificios: [...asentamiento.edificios, barracon],
      poblacion: { ...asentamiento.poblacion, pesants: 100 },
      almacen: {
        ...asentamiento.almacen,
        armaMadera: { ...asentamiento.almacen.armaMadera!, cantidad: 100 },
        // Trigo de sobra (no es el foco de este test): desde que `reclutarTropa` exige reserva de trigo
        // proyectada (ver `engine/tropas.ts`), reclutar necesita más que los 100 con que funda por defecto.
        trigo: { ...asentamiento.almacen.trigo!, cantidad: 1000 },
      },
    };

    // `lanceros_mimbre`: Barracón, nivelRequerido 1, cuesta armaMadera — la tropa más barata de la vía militar.
    const trasReclutar = reclutarTropa(degradado, SIN_MUNDO, 'jugador-faccion-1-1', 'faccion-1', 'lanceros_mimbre', 'pesants', 0);
    expect(trasReclutar.escuadrones.some((e) => e.tropaId === 'lanceros_mimbre')).toBe(true);
  });
});
