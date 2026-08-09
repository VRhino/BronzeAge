import type { NodoRecurso, Point, Rareza, ZonaBosque } from '../domain/types';
import { LIVESTOCK, RECURSO_CANTIDAD_NODO, RECURSO_RAREZA, RECURSO_TIPOS_POR_RAREZA } from './config';
import { colocarConEspaciado, type Limites } from './colocacion';
import { randInt, randRange, type RandomFn } from './rng';

/**
 * Nodos minerales de una rareza (Doc 1.1). `colocadosGlobal` se comparte entre llamadas y se MUTA: cada
 * rareza respeta el espaciado frente a lo ya colocado por las anteriores, aplicando su propio
 * `espacioMinimo` (los raros se separan mucho más que los comunes).
 *
 * Las posiciones de todos los tipos se sortean antes que las cantidades, no intercaladas — el orden de
 * consumo del PRNG es parte del contrato: alterarlo cambia todos los mundos ya guardados para una seed
 * dada (ver `WORLDGEN_VERSION`).
 */
export function generarNodosDeRareza(
  rng: RandomFn,
  limites: Limites,
  rareza: Rareza,
  colocadosGlobal: Point[],
  bosques: ZonaBosque[]
): NodoRecurso[] {
  const { cantidadBase, espacioMinimo } = RECURSO_RAREZA[rareza];
  const tipos = RECURSO_TIPOS_POR_RAREZA[rareza];
  const nodos: NodoRecurso[] = [];
  for (const tipo of tipos) {
    const posiciones = colocarConEspaciado(rng, limites, cantidadBase, espacioMinimo, colocadosGlobal, bosques);
    const rango = RECURSO_CANTIDAD_NODO[tipo as keyof typeof RECURSO_CANTIDAD_NODO];
    for (const posicion of posiciones) {
      colocadosGlobal.push(posicion);
      nodos.push({
        id: `${tipo}-${nodos.length}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
        tipo: tipo as NodoRecurso['tipo'],
        rareza,
        posicion,
        cantidadInicial: Math.round(randRange(rng, rango.min, rango.max)),
      });
    }
  }
  return nodos;
}

/** Livestock (Doc 1.4): fauna libre, fuera del esquema de rareza de los minerales. */
export function generarLivestock(
  rng: RandomFn,
  limites: Limites,
  colocadosGlobal: Point[],
  bosques: ZonaBosque[]
): NodoRecurso[] {
  const posiciones = colocarConEspaciado(rng, limites, LIVESTOCK.cantidadBase, LIVESTOCK.espacioMinimo, colocadosGlobal, bosques);
  return posiciones.map((posicion, i) => {
    colocadosGlobal.push(posicion);
    return {
      id: `livestock-${i}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
      tipo: 'livestock' as const,
      rareza: 'comun' as const,
      posicion,
      cantidadInicial: randInt(rng, LIVESTOCK.cantidadPorManada.min, LIVESTOCK.cantidadPorManada.max),
    };
  });
}
