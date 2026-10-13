// Tipos del generador de mundo, extraídos de `domain/types.ts` del motor TypeScript (typescript-final).
// El generador es una herramienta fuera de línea (decisión D3) y no depende del motor.

export interface Point {
  x: number;
  y: number;
}

export type RecursoTipo = 'madera' | 'piedra' | 'trigo' | 'cobre' | 'estano' | 'oro' | 'livestock' | 'hierro';

export type Rareza = 'comun' | 'intermedio' | 'raro';

export interface NodoRecurso {
  id: string;
  tipo: RecursoTipo;
  rareza: Rareza;
  posicion: Point;
  /**
   * Unidades con las que el yacimiento NACE al generarse el mundo. No es lo que le queda: el agotamiento
   * es estado de partida y vive en `EstadoMapa.extraido` (ver `src/world/mapa.ts`) — para lo que queda de
   * verdad, `mapa.stock(id)`. Separados a propósito: mientras fueron el mismo campo, guardar una foto del
   * historial obligaba a clonar el mapa entero cada tick.
   */
  cantidadInicial: number;
}

/** Bosques: representados como zonas (polígono/círculo), no puntos — fuente de madera. */
export interface ZonaBosque {
  id: string;
  centro: Point;
  radio: number;
  densidad: number; // 0-1, afecta rendimiento de madera
}

/**
 * Relieve (Fase 0.1, Doc `Fase_0_1_Definicion.md`). Deriva de `CampoElevacion` por umbral — nunca se guarda
 * por punto, se consulta con `Mapa.terrenoEn`/`evaluarTerreno`. 'agua' aquí es "elevación baja" (lago/cauce),
 * no mar — comercio marítimo sigue fuera de alcance. 'cima' es la franja de elevación MÁS alta, por encima de
 * 'montana': terreno inhabitable — no se puede fundar ahí (`evaluarViabilidadFundacion`/`fundarAsentamiento`)
 * ni se generan recursos ahí (`RECURSO_BIOMA_PERMITIDO`/`BOSQUE_TERRENO_PERMITIDO` nunca la listan).
 */
export type TerrenoTipo = 'agua' | 'costa' | 'llano' | 'colina' | 'montana' | 'cima';

/**
 * Bioma (Fase 0.1): terreno + fertilidad + humedad (proxy = cercanía a río). Nunca se guarda por punto —
 * ver `evaluarBioma`. Mismo nivel de granularidad que `Rareza`.
 */
export type BiomaTipo = 'agua' | 'costa' | 'estepa' | 'llanuraFertil' | 'colina' | 'montana' | 'cima';

/**
 * Río (Fase 0.1): polilínea de puntos, no celdas — nace en un punto alto y desciende por gradiente de
 * máxima pendiente del campo de elevación hasta agua/borde del mapa, o queda atrapado en un mínimo local
 * (lago) — ver `generarRios`.
 */
export interface RioZona {
  id: string;
  puntos: Point[];
  fin: 'agua' | 'borde' | 'lago' | 'limite' | 'troncal';
  /** true si el descenso terminó en un mínimo local (gradiente ~0) en vez de llegar a 'agua' o al borde
   * del mapa — informativo, no cambia el trazo. */
  terminaEnLago: boolean;
  /** true si el río es lo bastante largo y de verdad desemboca (no `terminaEnLago`) como para ser
   * navegable — pensado para el comercio fluvial con barcos de fases futuras (ver `RIOS.proporcionNavegable`
   * en `worldgen/config.ts` y `generarRios`). No afecta la geometría del trazo, solo cómo se interpreta. */
  navegable: boolean;
}

/** Depresión terminal de un río, tallada en el campo continuo de elevación. */
export interface LagoZona {
  id: string;
  centro: Point;
  radio: number;
  elevacionCentro: number;
  /** Laboratorio regional: orilla derivada del relieve, en lugar de un disco. */
  contorno?: Point[];
}

/**
 * Región geográfica opcional (Fase 0.2, ver `worldgen/regiones.ts`): sesga la generación de elevación para
 * que se parezca al carácter conocido de una zona real del Egeo/Levante de la Edad de Bronce, en vez del
 * mundo libre de siempre. Vive aquí (no en `worldgen/`) porque `WorldConfig` es una entidad de dominio y
 * `worldgen/` depende de `domain/types`, nunca al revés.
 */
export type RegionId = 'greciaContinental' | 'anatolia' | 'egeo' | 'nilo' | 'mesopotamia';

export interface WorldConfig {
  ancho: number;
  alto: number;
  seed: number;
  /** `undefined` = generación libre de siempre (comportamiento sin cambios). */
  region?: RegionId;
}
