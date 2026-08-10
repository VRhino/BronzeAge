// Tipos de la pieza de generación de mundo. `src/worldgen/` no conoce el motor: solo depende de
// `domain/types` (entidades compartidas) y de su propia configuración — se puede ejecutar, testear y
// versionar por separado.

import type { NodoRecurso, RioZona, WorldConfig, ZonaBosque } from '../domain/types';
import type { CampoRuido } from './ruido';

// Los dos campos continuos del mundo son ruido fractal (ver `ruido.ts`): mismo tipo de dato, distintos
// parámetros (ver `FERTILIDAD`/`ELEVACION` en `config.ts`). Se mantienen como alias con nombre propio
// porque el resto del código habla de "el campo de fertilidad" y "el campo de elevación", no de "ruido" —
// y porque nada fuera de `fertilidad.ts`/`elevacion.ts` debe depender de su forma interna.

/** Campo continuo de fertilidad del suelo. Se consulta con `evaluarFertilidad`. */
export type CampoFertilidad = CampoRuido;

/** Campo continuo de relieve. Se consulta con `evaluarElevacion`/`evaluarTerreno`. */
export type CampoElevacion = CampoRuido;

/**
 * Resultado completo de la generación: DATOS PUROS, sin funciones ni estado de partida. Todo lo que el
 * motor necesita saber del mapa sale de aquí.
 *
 * El agotamiento de yacimientos (que hoy muta `NodoRecurso.cantidad` in-place) todavía vive dentro de
 * `nodos` — se separará como estado de partida en un paso posterior del plan.
 */
export interface MapaGenerado {
  /**
   * Versión del ALGORITMO de generación, no del formato de datos. La partida guardada solo almacena la
   * seed y regenera el mapa al cargar (decisión de diseño), así que un cambio en el pipeline produciría
   * un mundo distinto para la misma seed. Subir este número al tocar la generación permite DETECTAR ese
   * caso al importar en vez de cargar en silencio un mundo que no es el que se guardó.
   */
  version: number;
  config: WorldConfig;
  bosques: ZonaBosque[];
  nodos: NodoRecurso[];
  fertilidad: CampoFertilidad;
  /** Campo continuo de relieve (Fase 0.1). Consultar con `evaluarElevacion`/`evaluarTerreno` — nunca se
   * itera como rejilla, ver `Fase_0_1_Definicion.md`. */
  elevacion: CampoElevacion;
  /** Ríos como polilíneas (Fase 0.1). El bioma se deriva de esto + elevación + fertilidad bajo demanda
   * (`evaluarBioma`) — no existe un campo de bioma guardado. */
  rios: RioZona[];
}

/**
 * Se sube cuando cambia el pipeline de generación de forma que altere el mundo para una seed dada.
 * v2 (Fase 0.1): se inserta generación de elevación y ríos, se reordena fertilidad, y bosques/nodos pasan a
 * condicionar su colocación al terreno — cambia el orden de consumo del PRNG de arriba a abajo del
 * pipeline, así que TODAS las seeds/mundos guardados producen un mundo distinto.
 * v3: `MAPA_DEFAULT` pasa de 1000x1000 a 2000x2000 y las cantidades de bosques/nodos/livestock/ríos suben
 * ×4 (misma densidad por área) — aunque estos son valores de `config.ts` y no reordenan el PRNG, cambian
 * cuántos puntos se sortean en cada paso, así que también desplazan todo lo generado después para la misma
 * seed.
 * v4: nueva banda de elevación `cima` (por encima de `montana`, inhabitable — ver `ELEVACION.umbralCima`)
 * excluida de todo predicado de colocación (`RECURSO_BIOMA_PERMITIDO`/`BOSQUE_TERRENO_PERMITIDO` nunca la
 * listan) — el rejection-sampling rechaza algunos candidatos que antes aceptaba, así que necesita más
 * intentos en algunos puntos y el PRNG se desplaza otra vez.
 * v5: elevación y fertilidad pasan de suma de senos alineados a los ejes a RUIDO FRACTAL DE GRADIENTE (ver
 * `ruido.ts`) — el mundo entero cambia de forma, no solo de números: el relieve deja de ser un enrejado
 * regular. Umbrales de terreno/bioma recalibrados contra la nueva distribución. El import ya rechaza duro
 * por `worldgenVersion` en vez de migrar (contrato existente, ver `gameStore.importarSimulacion`).
 */
export const WORLDGEN_VERSION = 5;
