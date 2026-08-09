// Fixtures compartidas para los tests de regresión del motor: construyen mundo/facción/asentamiento
// usando las funciones REALES del motor (generateWorld/crearFaccion/fundarAsentamiento), nunca objetos
// inventados a mano — así un test que pasa hoy sigue significando "el motor real produce esto".
import { WORLD_DEFAULT } from '../../constants';
import type { Asentamiento, Faccion, World } from '../../domain/types';
import { crearFaccion } from '../faccion';
import { createRng } from '../rng';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../settlement';
import { generateWorld } from '../world';

export function crearMundoDeterminista(seed: number): World {
  return generateWorld({ ancho: WORLD_DEFAULT.ancho, alto: WORLD_DEFAULT.alto, seed });
}

/**
 * Barre una grilla regular buscando una posición "recomendable" (fundable + bosque alcanzable, ver
 * `evaluarViabilidadFundacion`) — evita que los tests dependan de que el seed elegido a mano tenga
 * un bosque cerca del origen (0,0).
 */
export function posicionRecomendable(
  world: World,
  asentamientosExistentes: Asentamiento[] = [],
  paso = 40
): { x: number; y: number } {
  for (let x = paso; x < world.config.ancho; x += paso) {
    for (let y = paso; y < world.config.alto; y += paso) {
      const posicion = { x, y };
      if (evaluarViabilidadFundacion(world, posicion, asentamientosExistentes).recomendable) return posicion;
    }
  }
  throw new Error('No se encontró posición recomendable en la grilla de test — revisa el seed/paso.');
}

/** Funda un asentamiento de un solo jugador en una posición recomendable (o la indicada), vía el motor real. */
export function fundarAsentamientoDeTest(
  world: World,
  facciones: Faccion[],
  faccionId: string,
  asentamientosExistentes: Asentamiento[],
  tickActual = 0,
  posicion?: { x: number; y: number }
): { asentamiento: Asentamiento; facciones: Faccion[] } {
  const pos = posicion ?? posicionRecomendable(world, asentamientosExistentes);
  return fundarAsentamiento(world, facciones, faccionId, pos, [`jugador-${faccionId}-1`], asentamientosExistentes, tickActual);
}

export function crearFacciones(): Faccion[] {
  return [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
}

/**
 * `population.ts` (crecimiento estocástico) y `combate.ts` (jitter de combate) llaman a `Math.random()`
 * global en vez de pasar por el PRNG con seed de `rng.ts` — el motor NO es 100% determinista de punta a
 * punta pese a que la generación de mundo sí lo es. Para poder escribir tests de regresión exactos
 * (determinismo/snapshot) sustituimos `Math.random` global por el mismo PRNG con seed que usa el resto
 * del motor mientras dura el test. Llamar a la función devuelta al terminar para restaurar el original.
 */
export function mockMathRandomDeterminista(seed: number): () => void {
  const original = Math.random;
  Math.random = createRng(seed);
  return () => {
    Math.random = original;
  };
}
