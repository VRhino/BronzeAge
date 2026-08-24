// Fixtures compartidas para los tests de regresión del motor: construyen mapa/facción/asentamiento
// usando las funciones REALES del motor (generarMapa/crearFaccion/fundarAsentamiento), nunca objetos
// inventados a mano — así un test que pasa hoy sigue significando "el motor real produce esto".
import type { Asentamiento, Faccion } from '../../domain/types';
import { generarMapa, MAPA_DEFAULT, type RandomFn } from '../../worldgen';
import { crearMapa, type Mapa } from '../../world/mapa';
import { crearFaccion } from '../faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../settlement';
import type { ContextoSimulacion } from '../simulation';

export function crearMapaDeterminista(seed: number): Mapa {
  return crearMapa(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed }));
}

/** Fecha arbitraria y FIJA de la que arrancan los tests — ver `contextoDeTest`. */
const INICIO_PARTIDA_DE_TEST = Date.UTC(2026, 0, 1, 0, 0, 0);
/** Duración de simulación que se atribuye a cada tick en los tests. Arbitraria: nada del motor la usa
 * todavía (el tick no tiene duración real hasta la Fase D), solo sirve para que `momento` avance de forma
 * monótona y reproducible. */
const MS_POR_TICK_DE_TEST = 60_000;

/**
 * `ContextoSimulacion` para tests, con `momento` DERIVADO DEL TICK y nunca del reloj real: el motor tiene que
 * ser reproducible (ver `determinismo.test.ts`, que compara dos corridas completas), así que un `Date.now()`
 * aquí haría divergir dos corridas idénticas por los timestamps. Pasar el MISMO `rng` en todos los ticks de
 * una corrida — es una secuencia con estado, no una fábrica.
 */
export function contextoDeTest(tick: number, rng: RandomFn): ContextoSimulacion {
  return { tick, momento: new Date(INICIO_PARTIDA_DE_TEST + tick * MS_POR_TICK_DE_TEST).toISOString(), rng };
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
  return fundarAsentamiento(mapa, facciones, faccionId, pos, [`jugador-${faccionId}-1`], asentamientosExistentes, tickActual);
}

export function crearFacciones(): Faccion[] {
  return [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
}
