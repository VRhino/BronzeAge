// Contrato v1 con Conquest/Unity (Docs/Coordinacion/01 §12-§17 y 02 §3-§4): la forma de lo que viaja por HTTP
// en las dos direcciones, NO la del dominio (`domain/types.ts` sigue siendo interno). La versión que consume C#
// es `contratos.schema.json`; esto es su espejo en TypeScript. Los fixtures de `fixtures.ts` están tipados con
// estos tipos (los comprueba `tsc`) y se validan contra el schema (los comprueba ajv en el test): así los dos
// lados no pueden separarse sin que algo falle.
import type { EdificioTipo } from '../../domain/types';

export const SCHEMA_VERSION = 1;

/** Tiempo de MUNDO, ms desde época (doc 01 §0): no avanza con el proceso caído. */
export type Instante = number;
/** Tiempo REAL UTC en ISO 8601 (doc 01 §0): credenciales y horas del servidor de batalla. */
export type IsoUtc = string;

export interface Punto {
  x: number;
  y: number;
}

export type LadoId = 'atacante' | 'defensor';
export type Controlador = 'humano' | 'bot';
export type Genero = 'masculino' | 'femenino';
export type SlotEquipo = 'arma' | 'casco' | 'torso' | 'guantes' | 'pantalones' | 'botas';

/** Ids de piezas del catálogo visual de Conquest (`AvatarParts`). Cosmético: BronzeAge no lo interpreta. */
export interface Avatar {
  cabezaId: string;
  peloId: string;
  barbaId: string;
  cejasId: string;
}

export interface Atributos {
  fuerza: number;
  destreza: number;
  armadura: number;
  vitalidad: number;
}

export interface Monedas {
  bronce: number;
  plata: number;
  oro: number;
}

export interface Estadistica {
  nombre: string;
  valor: number;
}

/** Objeto del inventario o del equipo (doc 01 §12.1), forma de `InventoryItem` de Conquest. */
export interface ItemInstancia {
  itemDefinitionId: string;
  tipo: 'arma' | 'armadura' | 'consumible' | 'visual';
  /** Lo no apilable lleva 1. */
  cantidad: number;
  /** Solo el equipo único; ausente = apilable. */
  itemInstanceId?: string;
  /** Solo el equipo único: se generan al crear el objeto, son datos suyos. */
  estadisticas?: Estadistica[];
  /** Por unidad si es apilable. */
  precio: number;
  /** -1 si no ocupa casilla. */
  casillaInventario: number;
}

export type Equipamiento = Record<SlotEquipo, ItemInstancia | null>;

export type ContenedorEscuadron =
  | { tipo: 'campamento' }
  | { tipo: 'ejercito'; ejercitoId: string }
  | { tipo: 'escolta'; caravanaId: string }
  /** Fuera del mundo con su héroe desconectado (Doc 1.10.6). */
  | { tipo: 'fuera' };

export interface EscuadronDto {
  id: string;
  heroeId: string;
  tropaId: string;
  nombre: string;
  cantidad: number;
  nivel: number;
  /** ACUMULADA, nunca baja (doc 01 §13). */
  experiencia: number;
  moral: number;
  /** Ids de habilidad de Conquest (`unlockedAbilities`). */
  habilidadesDesbloqueadas: string[];
  /** Índices en las formaciones de la definición de escuadra de Conquest (`permittedFormationIndexes`). */
  formacionesDesbloqueadas: number[];
  formacionSeleccionada: number;
  contenedor: ContenedorEscuadron;
  enGuarnicion: boolean;
  reservaBatalla?: { battleId: string };
  /** Tropa prestada por un campamento de mercenarios (Doc 1.9b, D80): no gana experiencia ni cuenta para «una escuadra por
   * tropa»; el campamento la retira si su héroe deja de residir en él. */
  prestada?: { campamentoId: string };
}

export interface LoadoutDto {
  id: string;
  heroeId: string;
  displayName: string;
  squadIds: string[];
  perksSeleccionados: number[];
  activo: boolean;
  /** DERIVADO al servir, nunca persistido (doc 01 §14). */
  liderazgoTotal: number;
}

export type UbicacionHeroe =
  | { tipo: 'asentamiento'; asentamientoId: string }
  | { tipo: 'columna'; ejercitoId: string }
  /** Dentro de un campamento de mercenarios: su vista de asentamiento (D73). */
  | { tipo: 'mercenarios'; campamentoId: string }
  | { tipo: 'desconectado'; punto: Punto };

/**
 * El héroe propio, completo (doc 02 §4.1). `plazasRecordadas` y `exploracionPersonal` no viajan aquí: la
 * proyección ya las sirve fundidas en `asentamientosConocidos` y en la niebla del mapa.
 */
export interface HeroeDto {
  schemaVersion: typeof SCHEMA_VERSION;
  id: string;
  /** null en héroes bot. */
  jugadorId: string | null;
  controlador: Controlador;
  displayName: string;
  classDefinitionId: string;
  genero: Genero;
  avatar: Avatar;
  nivel: number;
  experienciaHaciaSiguienteNivel: number;
  puntosDeAtributoSinGastar: number;
  puntosDePerkSinGastar: number;
  atributosBase: Atributos;
  perksDesbloqueados: number[];
  liderazgoBase: number;
  ubicacion: UbicacionHeroe;
  /** Lo que guarda en su campamento de residencia (Doc 2.5). */
  almacenPersonal?: Record<string, number>;
  /** Oro de bandidos y alijos (Doc 1.9, D27): solo se gasta en el mercado de un campamento o en el fondo de refundación. */
  oroDeBotin?: number;
  /** Lo comprado hoy en el mercado de su campamento, contra el cupo diario (Doc 3.3b). */
  cupoCampamento?: { dia: number; comprado: Record<string, number> };
  /** Cuándo recogió la última ración gratis de su campamento (Doc 1.9b). */
  racionEn?: Instante;
  /** Víveres: el trigo que come su columna, siempre con él, hasta `LOGISTICA.capacidadViveresPorHeroe` (Doc 5.13). Ausente = 0. */
  viveres?: number;
  /** Cuándo destruyó cada campamento de bandidos de las últimas 24 h: el botín decrece con ellas (Doc 1.9, D26). */
  bandidosDestruidosEn?: Instante[];
  /** Los alijos de exploración que ya abrió: cada uno, una vez por héroe (Doc 1.9b, D60). */
  alijosAbiertos?: string[];
  heridoHasta?: Instante;
  /** Desconexión pedida: sale del mundo en este instante (Doc 1.10.6). */
  desconectaEn?: Instante;
  /** Desconectado (Doc 1.10.6): no da órdenes. Dentro de una plaza o campamento no se mueve; en el mapa sale del mundo con su
   * columna y `carro` es lo que llevaba. */
  fuera?: { carro: Record<string, number> };
  escuadrones: EscuadronDto[];
  loadouts: LoadoutDto[];
  inventario: ItemInstancia[];
  equipamiento: Equipamiento;
  monedasHeroe: Monedas;
  /** DERIVADO: cupo de guarnición en su residencia, 0 si es huérfano (Doc 5.15.3). */
  cupoGuarnicion: number;
}

/** Lo que un jugador ve de un héroe ajeno (doc 02 §4.1, Doc 5.16.7). */
export interface HeroePublicoDto {
  heroeId: string;
  displayName: string;
  classDefinitionId: string;
  nivel: number;
  heridoHasta?: Instante;
  escuadrasQueLleva: { tropaId: string; cantidad: number; nivel: number }[];
  /** Por hueco, el `itemDefinitionId` de lo que lleva puesto. */
  equipamiento: Record<SlotEquipo, string | null>;
}

/** Qué se disputa (doc 01 §15). `campo_abierto` es una batalla campal entre ejércitos o una persecución entre columnas personales (Doc 5.15.1b). */
export type ContextoEstrategico =
  | { tipo: 'asedio'; asentamientoId: string }
  | { tipo: 'campo_abierto'; punto: Punto; columnas: 'ejercitos' | 'solitarios' }
  | { tipo: 'caravana'; caravanaId: string; punto: Punto }
  | { tipo: 'campamento_bandidos'; campamentoId: string; punto: Punto };

/** El héroe al congelar el ticket. Los atributos van ya con el equipo aplicado: los calcula BronzeAge. */
export type HeroSnapshot = Pick<
  HeroeDto,
  'displayName' | 'classDefinitionId' | 'nivel' | 'genero' | 'avatar' | 'perksDesbloqueados' | 'equipamiento'
> & {
  atributosEfectivos: Atributos;
  /** Cuánto botín le cabe (doc 01 §15). */
  casillasInventarioLibres: number;
};

/** Lo que ha progresado una escuadra: igual en el héroe que en el ticket. */
export type ProgresionEscuadra = Pick<
  EscuadronDto,
  'nivel' | 'experiencia' | 'moral' | 'habilidadesDesbloqueadas' | 'formacionesDesbloqueadas' | 'formacionSeleccionada'
>;

export type SquadSnapshot = ProgresionEscuadra & {
  squadId: string;
  /** Dueño. null en tropas sin dueño (bandidos, carreteros), que no persisten entre batallas. */
  heroeId: string | null;
  tropaId: string;
  /** Fuerza RESERVADA: el resultado cierra `supervivientesAlCierre + muertos` sobre esto. */
  efectivosAutorizados: number;
};

export interface BattleParticipantSnapshot {
  heroeId: string;
  /** Su Facción: un bando puede mezclar Facciones (Doc 5.15.1b). null si no tiene. */
  faccionId: string | null;
  controlador: Controlador;
  heroe: HeroSnapshot;
  /** Solo las que lleva consigo, dentro de su Liderazgo. */
  escuadras: SquadSnapshot[];
}

export interface BattleSide {
  /** null si el bando no es de nadie (campamento de bandidos). */
  faccionId: string | null;
  /** En HÉROES (Doc 5.15.1). Las escuadras sin héroe no ocupan plaza. */
  capacidadMaxima: number;
  participantes: BattleParticipantSnapshot[];
  /** Guarnición, escolta, tropas de bandidos o carreteros: las maneja la IA de juego. */
  escuadrasSinHeroe: SquadSnapshot[];
}

/** Batalla en el mapa general: Unity monta el terreno de `GET /jugador/partidas/:gameId/mapa/:mapaId`. */
export interface BattleMapReference {
  tipo: 'mapa';
  mapaId: string;
  centro: Punto;
}

/**
 * El asentamiento congelado al abrir la batalla (doc 01 §17), en coordenadas LOCALES: origen en el centro del
 * Centro Urbano, `y` hacia abajo, una celda = `unidadesPorCelda` unidades. Son las convenciones del fixture de
 * BA-005.
 */
export interface SettlementBattleSnapshot {
  tipo: 'asentamiento';
  settlementId: string;
  layoutVersion: number;
  unidadesPorCelda: number;
  /** Solo los edificios internos. */
  edificios: {
    edificioId: string;
    tipo: EdificioTipo;
    /** Centro de la huella, en unidades. */
    posicion: Punto;
    /** Huella en celdas, con la rotación ya aplicada. */
    ancho: number;
    alto: number;
    nivelInterno?: number;
    estado: 'en_cola' | 'en_construccion' | 'activo';
    danado?: boolean;
  }[];
  /** La celda (col, row) ocupa de (col, row) a (col + 1, row + 1), por `unidadesPorCelda`. */
  recintos: {
    recintoId: string;
    /** Nivel FÍSICO en pie: 1 empalizada, 2 muro de piedra, 3 muralla con adarve. */
    nivel: number;
    /** Solo las celdas EN PIE, en orden de recorrido desde la puerta principal. */
    celdas: { col: number; row: number; clase: 'muro' | 'puerta' | 'torre' }[];
  }[];
}

export interface BattleRules {
  /** Doc 5.15.1: 1800 en un asedio, 900 en el resto. Si se agota, gana el defensor, en cualquier batalla. */
  duracionMaximaSegundos: number;
  versionBalance: number;
  versionCatalogoTropas: number;
  versionCatalogoHeroe: string;
  versionCatalogoObjetos: string;
}

export interface BattleTicket {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  gameId: string;
  ticketRevision: number;
  contextoEstrategico: ContextoEstrategico;
  bandos: Record<LadoId, BattleSide>;
  mapa: BattleMapReference | SettlementBattleSnapshot;
  reglas: BattleRules;
}

/** Un héroe que se une a un bando después del ticket (Doc 5.15.1). El ticket no cambia; la lista solo crece. */
export interface IncorporacionBatalla {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  /** 1, 2, 3… en el orden en que se unieron. */
  secuencia: number;
  lado: LadoId;
  participante: BattleParticipantSnapshot;
}

export interface TokenParticipante {
  heroeId: string;
  token: string;
  expiraEn: IsoUtc;
}

/** `POST /v1/batallas/:battleId/asignacion` (Conquest → BronzeAge). */
export interface BattleServerAssignment {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  ticketRevision: number;
  intentoAsignacionId: string;
  instancia: { host: string; puerto: number; protocolo: string };
  /** Solo héroes humanos: los bot no se conectan. */
  tokensParticipante: TokenParticipante[];
}

/** `POST /v1/batallas/:battleId/tokens` (Conquest → BronzeAge): los humanos que se unen después de la asignación. */
export interface TokensBatalla {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  intentoAsignacionId: string;
  tokensParticipante: TokenParticipante[];
}

/** `POST /v1/batallas/:battleId/inicio` (Conquest → BronzeAge). */
export interface InicioBatalla {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  ticketRevision: number;
  intentoAsignacionId: string;
}

export interface Botin {
  /** Forma de `ItemInstancia` sin tipo, precio ni casilla: los pone BronzeAge. */
  objetos: { itemDefinitionId: string; cantidad: number; itemInstanceId?: string; estadisticas?: Estadistica[] }[];
  monedas: Monedas;
}

/** `POST /v1/batallas/:battleId/salidas` (Conquest → BronzeAge): un héroe sale de la batalla (Doc 5.15.1, CQ-011). */
export interface SalidaBatalla {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  ticketRevision: number;
  intentoAsignacionId: string;
  heroeId: string;
  /** `no_conectado` solo antes de empezar (`asignada`): sale sin castigo y sube la revisión del ticket. Los demás, con la partida en curso. */
  motivo: 'no_conectado' | 'eliminado' | 'abandono' | 'desconexion';
  /** Queda herido (Doc 5.16.4). */
  derrotado: boolean;
  /** Cómo quedaron sus escuadras al salir. Vacío si no llegó a desplegarlas. */
  escuadras: { squadId: string; supervivientes: number; muertos: number }[];
}

/** `POST /v1/batallas/:battleId/resultado` (Conquest → BronzeAge). */
export interface BattleResult {
  schemaVersion: typeof SCHEMA_VERSION;
  battleId: string;
  resultId: string;
  ticketRevision: number;
  intentoAsignacionId: string;
  inicio: IsoUtc;
  fin: IsoUtc;
  ganador: LadoId;
  razon: 'objetivos_capturados' | 'aniquilacion' | 'tiempo_agotado';
  /** Informativo: BronzeAge no aplica nada a partir de los objetivos. */
  objetivos: { objetivoId: string; capturadoPor: LadoId | null }[];
  porEscuadra: { squadId: string; desplegados: number; supervivientesAlCierre: number; muertos: number; xpGanada: number }[];
  /** `derrotado` dice quién queda herido (Doc 5.16.4); sin él, quien está en el bando perdedor. No repite a quien salió antes (`SalidaBatalla`). */
  porHeroe: { heroeId: string; participo: boolean; sobrevivioAlCierre: boolean; xpGanada: number; derrotado?: boolean; botin?: Botin }[];
  versionServidor: string;
}

/** Sigilo de una Facción y piezas con las que se compone (Doc 2.8.1), generado desde `engine/sigilo.ts`. */
export interface Sigilo {
  formaId: string;
  campoId: string;
  emblemaId: string;
  colorPrimarioId: string;
  colorSecundarioId: string;
  colorEmblemaId: string;
  orlaId: string;
  colorOrlaId: string;
}
export interface CatalogoSigilos {
  schemaVersion: typeof SCHEMA_VERSION;
  version: number;
  formas: string[];
  campos: string[];
  orlas: string[];
  emblemas: string[];
  colores: { id: string; nombre: string; hex: string }[];
  reservados: { neutro: Sigilo; bandidos: Sigilo; mercenarios: Sigilo };
}

/** Catálogo puente de tropas (doc 01 §13, CQ-003), generado desde `constants.ts`. */
export interface CatalogoTropas {
  schemaVersion: typeof SCHEMA_VERSION;
  version: number;
  tropas: {
    tropaId: string;
    nombre: string;
    escalon: number;
    unidades: number;
    costeLiderazgo: number;
    tipo: 'cuerpo_a_cuerpo' | 'a_distancia';
  }[];
}

/** Los edificios que tiene un campamento de mercenarios (Doc 1.9b). */
export type EdificioCampamentoTipo = 'taberna' | 'vivienda' | 'mercado' | 'barracon' | 'galeriaDeTiro' | 'caballerizas';

/** Lo que se ve en su planta: sus edificios y los de relleno del trazado (puestos del mercado, plazas que sirven de ancla). */
export type ElementoCampamentoTipo = EdificioCampamentoTipo | 'puestoMercado' | 'plazaDeArmas' | 'plaza' | 'pozo' | 'parque';

/**
 * El campamento de mercenarios tal como viaja en `campamentosMercenarios` de la proyección (doc 01 §10): enclave neutral, sin Facción ni zona.
 * `edificios` son los tipos que tiene (diez viviendas, taberna, mercado y uno militar); su planta es `EscenaCampamento`.
 */
export interface CampamentoMercenariosDto {
  id: string;
  posicion: Punto;
  /** Variante de aspecto de los modelos, 0..2: solo cambia cómo se ve. */
  origen: number;
  edificios: EdificioCampamentoTipo[];
  residentesIds: string[];
  eligieronComoInicial: number;
  bandidosEn?: Instante;
  poblacion: number;
  poblacionEn: Instante;
  mercado: Record<string, number>;
  fondos: Record<string, Record<string, number>>;
  creadoEn: Instante;
}

/**
 * La planta de un campamento de mercenarios, para montar su escena (D73). Misma convención que `SettlementBattleSnapshot`: coordenadas
 * locales con el origen en el centro de la taberna —el ancla del trazado—, `y` hacia abajo, y la celda (col, row) ocupa de (col, row) a
 * (col + 1, row + 1), por `unidadesPorCelda`. Viaja en la proyección solo estando dentro (`escenaCampamento`), y es DERIVADA del `id` y los
 * `edificios`: dos llamadas con los mismos datos dan lo mismo mientras no cambie `layoutVersion`.
 */
export interface EscenaCampamento {
  campamentoId: string;
  /** El aspecto de los modelos (`CampamentoMercenariosDto.origen`); la planta no depende de él. */
  origen: number;
  layoutVersion: number;
  unidadesPorCelda: number;
  edificios: {
    edificioId: string;
    tipo: ElementoCampamentoTipo;
    /** Centro de la huella, en unidades. */
    posicion: Punto;
    /** Huella en celdas. */
    ancho: number;
    alto: number;
  }[];
  /** Calles, en celdas (esquina superior izquierda y tamaño). Es suelo transitable. */
  calles: { col: number; row: number; ancho: number; alto: number }[];
  /** La empalizada, decorativa y sin efecto de juego (dentro no hay combate): sus celdas, y por cuál se entra y se sale (`puerta`). */
  empalizada: { col: number; row: number; clase: 'muro' | 'puerta' }[];
}
