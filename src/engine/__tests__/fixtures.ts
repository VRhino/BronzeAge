// Fixtures compartidas para los tests de regresión del motor: construyen mapa/facción/asentamiento
// usando las funciones REALES del motor (generarMapa/crearFaccion/fundarAsentamiento), nunca objetos
// inventados a mano — así un test que pasa hoy sigue significando "el motor real produce esto".
import type { Asentamiento, Faccion } from '../../domain/types';
import { instante, type Instante } from '../../domain/tiempo';
import { generarMapa, MAPA_DEFAULT, type RandomFn } from '../../worldgen';
import { crearMapa, type Mapa } from '../../world/mapa';
import { SIMULACION } from '../../constants';
import { crearFaccion } from '../faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../settlement';
import type { ContextoSimulacion, EstadoSimulacion } from '../simulation';

export function crearMapaDeterminista(seed: number): Mapa {
  return crearMapa(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed }));
}

const EPOCA_MS = new Date(SIMULACION.epocaInicial).getTime();

/** Instante de mundo de un `tick`, misma fórmula que `instanteDeTick` (`session/estado.ts`, Fase D / doc 10)
 * — para que un test que razona en ticks pueda pasar el `Instante` que el motor ahora espera. */
export function instanteDeTest(tick = 0): Instante {
  return instante(EPOCA_MS + tick * SIMULACION.duracionTickMs);
}

/**
 * `ContextoSimulacion` para tests, con `instante`/`momento` DERIVADOS DEL TICK y nunca del reloj real: el
 * motor tiene que ser reproducible (ver `determinismo.test.ts`), así que un `Date.now()` aquí haría divergir
 * dos corridas idénticas. Pasar el MISMO `rng` en todos los ticks de una corrida — es una secuencia con
 * estado, no una fábrica.
 */
export function contextoDeTest(tick: number, rng: RandomFn): ContextoSimulacion {
  const i = instanteDeTest(tick);
  return { instante: i, momento: new Date(i).toISOString(), rng };
}

/**
 * Barre una grilla regular buscando una posición "recomendable" (fundable + bosque alcanzable, ver
 * `evaluarViabilidadFundacion`) — evita que los tests dependan de que el seed elegido a mano tenga
 * un bosque cerca del origen (0,0).
 */
export function posicionRecomendable(
  mapa: Mapa,
  asentamientosExistentes: Asentamiento[] = [],
  paso = 40
): { x: number; y: number } {
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      if (evaluarViabilidadFundacion(mapa, posicion, asentamientosExistentes).recomendable) return posicion;
    }
  }
  throw new Error('No se encontró posición recomendable en la grilla de test — revisa el seed/paso.');
}

/** Funda un asentamiento de un solo jugador en una posición recomendable (o la indicada), vía el motor real. */
export function fundarAsentamientoDeTest(
  mapa: Mapa,
  facciones: Faccion[],
  faccionId: string,
  asentamientosExistentes: Asentamiento[],
  tickActual = 0,
  posicion?: { x: number; y: number }
): { asentamiento: Asentamiento; facciones: Faccion[] } {
  const pos = posicion ?? posicionRecomendable(mapa, asentamientosExistentes);
  return fundarAsentamiento(mapa, facciones, faccionId, pos, [`jugador-${faccionId}-1`], asentamientosExistentes, instanteDeTest(tickActual));
}

export function crearFacciones(): Faccion[] {
  return [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
}

/**
 * `EstadoSimulacion` de test: `asentamientos`/`facciones` son lo único que varía de un test a otro en toda
 * la suite (revisión de duplicación 2026-08-25) — los otros 8 campos SIEMPRE arrancan vacíos/en cero, y ese
 * objeto literal de 10 campos estaba copiado tal cual en 13+ archivos. `overrides` cubre el caso — hoy
 * inexistente, pero no imposible — de un test que necesite arrancar con caravanas u órdenes ya puestas.
 */
export function crearEstadoDeTest(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  overrides: Partial<Omit<EstadoSimulacion, 'asentamientos' | 'facciones'>> = {}
): EstadoSimulacion {
  return {
    asentamientos,
    facciones,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnEn: instanteDeTest(0),
    ...overrides,
  };
}
