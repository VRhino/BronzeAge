// Revamp de caravanas (Doc 3.13, `Consideraciones/Revamp_Caravanas_Definicion.md`) — Paso 1: derivaciones
// puras de una caravana compuesta. Vive en su propio módulo porque lo consumen tanto `trade.ts` como
// `ejercitos.ts` y esos dos no se importan entre sí.
//
// El motor NO sabe de jugadores ni de sesiones: aquí solo se hace aritmética sobre la caravana y los
// catálogos. La validación de residencia / propiedad de escuadrones para la escolta vive en `session/`.

import { ANIMAL_CATALOGO, CARAVANA_CATALOGO, CARRO_CATALOGO } from '../constants';
import type { Caravana } from '../domain/types';

/** Los carros que llevan animal — los únicos que viajan y cuentan capacidad (Doc 3.13.1). */
function carrosConTraccion(caravana: Caravana) {
  return (caravana.carros ?? []).filter((c) => c.animal !== undefined);
}

/**
 * Capacidad de carga de una caravana (Doc 3.13.1): suma de `capacidadBase × factorCarga` sobre los carros con
 * animal. Sin `carros` (pre-revamp o categoría sin revamp) cae al valor fijo de `CARAVANA_CATALOGO`.
 *
 * NO aplica los multiplicadores de política (`carga_ampliada`): eso lo hace el llamador, que es quien tiene
 * el asentamiento de origen.
 */
export function capacidadCaravana(caravana: Caravana): number {
  if (caravana.carros === undefined) return CARAVANA_CATALOGO[caravana.tipo].capacidad;
  return carrosConTraccion(caravana).reduce(
    (suma, c) => suma + CARRO_CATALOGO[c.tipoCarro].capacidadBase * ANIMAL_CATALOGO[c.animal!].factorCarga,
    0
  );
}

/**
 * Velocidad de una caravana (Doc 3.13.1): la del animal más lento. Los carros no capean velocidad todavía
 * (todos los tipos manejan igual). Sin `carros` cae al valor fijo de `CARAVANA_CATALOGO`. Con carros pero sin
 * ningún animal → 0: una caravana así no puede salir.
 *
 * NO aplica los multiplicadores de política (`rutas_rapidas`): eso lo hace el llamador.
 */
export function velocidadCaravana(caravana: Caravana): number {
  if (caravana.carros === undefined) return CARAVANA_CATALOGO[caravana.tipo].velocidad;
  const conTraccion = carrosConTraccion(caravana);
  if (conTraccion.length === 0) return 0;
  return Math.min(...conTraccion.map((c) => ANIMAL_CATALOGO[c.animal!].velocidad));
}
