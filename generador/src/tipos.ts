// Tipos básicos del generador de mundo.

export interface Point {
  x: number;
  y: number;
}

/** Banda de relieve que resulta de umbralizar la elevación (ver `evaluarTerreno`). 'agua' es elevación baja
 * (mar o lago); 'cima' es la franja más alta, por encima de 'montana'. */
export type TerrenoTipo = 'agua' | 'costa' | 'llano' | 'colina' | 'montana' | 'cima';

/** Río como polilínea de puntos. */
export interface RioZona {
  id: string;
  puntos: Point[];
  fin: 'agua' | 'borde' | 'lago' | 'limite' | 'troncal';
  /** true si el descenso terminó en un mínimo local en vez de llegar a agua o al borde del mapa. */
  terminaEnLago: boolean;
  /** Siempre false hoy: no hay navegación fluvial. */
  navegable: boolean;
}

/** Depresión terminal de un río, tallada en el campo continuo de elevación. */
export interface LagoZona {
  id: string;
  centro: Point;
  radio: number;
  elevacionCentro: number;
  /** Orilla derivada del relieve, en lugar de un disco. */
  contorno?: Point[];
}
