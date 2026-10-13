import type { Point } from './tipos';
import type { Anillo, Campamento } from './campamentos';
import type { Veta } from './metales';
import type { MasaBosque, ParcelaBosque } from './parcelas';

/**
 * Contrato interno entre las piezas del generador y el escritor del artefacto (`world.proto`).
 * Todas las posiciones y distancias van en UNIDADES DEL GENERADOR (1 unidad estratégica = `FACTOR` = 12 unidades del generador);
 * el escritor del artefacto convierte a milésimas de unidad estratégica al serializar: `round(u * 1000 / 12)`.
 */

export type TipoRecurso = 'piedra' | 'cobre' | 'hierro' | 'estano' | 'oro' | 'ganaderia';

export interface RecursoMundo {
  id: string;
  tipo: TipoRecurso;
  posicion: Point;
  anillo: Anillo;
  /** Cantidad inicial del yacimiento, en unidades de juego (enteros). La ganadería es `livestock` en el motor. */
  cantidadInicial: number;
  vetaId?: string;
}

export interface BandidosMundo {
  id: string;
  posicion: Point;
  nivel: number;
  /** Veta a la que guarda, si es de distrito minero. */
  vetaId?: string;
  anillo: Anillo;
}

export interface AlijoMundo {
  id: string;
  posicion: Point;
  /** Oro que contiene. */
  oro: number;
  anillo: Anillo;
}

export interface NodoGrafo { id: number; x: number; y: number }
export interface AristaGrafo { a: number; b: number; costeMilesimas: number; vado: boolean }

/** Grafo de navegación: la ÚNICA autoridad de rutas. `espaciado` en unidades del generador (8 estratégicas = 96). */
export interface GrafoMundo { nodos: NodoGrafo[]; aristas: AristaGrafo[]; espaciado: number }

export type ClaseTerreno = 'agua' | 'costa' | 'llano' | 'colina' | 'montana' | 'cima';

/** Funciones de muestreo del relieve para rasterizar las capas. Los valores continuos van en 0..1. */
export interface MuestrasRelieve {
  altura(p: Point): number;
  terreno(p: Point): ClaseTerreno;
  fertilidad(p: Point): number;
  humedad(p: Point): number;
  densidadBosque(p: Point): number;
  transitable(p: Point): boolean;
}

export interface RioMundo { id: string; puntos: Point[]; terminaEnLago: boolean; navegable: boolean; vados: Point[] }

export interface MundoGenerado {
  seed: number;
  ancho: number;
  alto: number;
  generador: string;
  campamentos: Campamento[];
  recursos: RecursoMundo[];
  vetas: Veta[];
  masas: MasaBosque[];
  parcelas: ParcelaBosque[];
  bandidos: BandidosMundo[];
  alijos: AlijoMundo[];
  rios: RioMundo[];
  grafo: GrafoMundo;
  muestras: MuestrasRelieve;
}

export type { Anillo };
