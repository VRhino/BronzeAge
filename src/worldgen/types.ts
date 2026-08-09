// Tipos de la pieza de generación de mundo. `src/worldgen/` no conoce el motor: solo depende de
// `domain/types` (entidades compartidas) y de su propia configuración — se puede ejecutar, testear y
// versionar por separado.

import type { NodoRecurso, WorldConfig, ZonaBosque } from '../domain/types';

/**
 * Una octava del campo de fertilidad: seno/coseno con frecuencia y fase propias. Se guarda como DATO
 * (no como la función ya cerrada sobre él) para que el campo sea serializable y comparable — ver
 * `evaluarFertilidad` en `fertilidad.ts`.
 */
export interface OctavaFertilidad {
  freq: number;
  faseX: number;
  faseY: number;
  peso: number;
}

export interface CampoFertilidad {
  octavas: OctavaFertilidad[];
  /** Suma de pesos, precalculada: normaliza el resultado a 0-1. */
  pesoTotal: number;
}

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
}

/** Se sube cuando cambia el pipeline de generación de forma que altere el mundo para una seed dada. */
export const WORLDGEN_VERSION = 1;
