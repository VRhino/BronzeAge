// Capa de aplicación: única puerta de entrada a la lógica de simulación (motor + dominio).
// Ninguna capa de interfaz debe importar nada de `../engine/*` ni de `../domain/types` para
// mutar estado o capturar errores — solo debe hablar con `gameStore` (acciones + getState +
// subscribe) y con tipos de datos para tipar lo que lee. Así, cambiar de interfaz (otra librería
// de UI, un cliente CLI, tests) no requiere tocar nada de esta carpeta ni de `engine/`.
import type {
  AcuerdoTrueque,
  Asentamiento,
  CaminoComercial,
  CampamentoBandido,
  CargoTipo,
  Caravana,
  EdificioTipo,
  Faccion,
  NodoRecurso,
  OrdenMercado,
  RecursoTipo,
  RegionId,
  RelacionPolitica,
  Titulo,
  WorldConfig,
  ZonaBosque,
  ZonaInfluencia,
} from '../domain/types';
import { CAMPAMENTOS_BANDIDOS, EDIFICIO_CATALOGO, FUNDACION, MANTENIMIENTO, NECESIDADES, NIVEL_FACCION, POLITICAS, POLITICA_CATALOGO, REJILLA_ASENTAMIENTO, SIMULACION_AUTO_COMERCIO, TROPAS_RECLUTABLES } from '../constants';
import { generarMapa, MAPA_DEFAULT, WORLDGEN_VERSION, type MapaGenerado } from '../worldgen';
import { crearEstadoMapa, crearMapa, type EstadoMapa, type Mapa } from '../world/mapa';
import { exportarParaUnityTerrain, UNITY_EXPORT_DEFAULT, type ExportUnityResultado, type OpcionesExportUnity } from '../world/exportUnity';

export { UNITY_EXPORT_DEFAULT };
import {
  produccionPorTick,
  manoObraInfo as calcularManoObraInfo,
  progresoNivelAsentamiento,
  capacidadViviendaPesants,
  capacidadViviendaArtesanos,
  edificiosPorTipoYEstado,
  cupoCaravanas as cupoCaravanasEngine,
  ticksCooldownCaravanaRestantes as ticksCooldownCaravanaRestantesEngine,
  tieneMercadoActivo as tieneMercadoActivoEngine,
  nivelActualDe,
  type ProduccionItem,
  type ManoObraInfo,
  type ProgresoNivelAsentamiento,
} from '../engine/asentamientoQuery';
export type { ProduccionItem, ManoObraInfo } from '../engine/asentamientoQuery';
import { encontrarCapital, calcularCostoMantenimiento, calcularNivelAsentamiento } from '../engine/mantenimiento';
import { consumoComidaPoblacion } from '../engine/population';
import { slotsDisponibles } from '../engine/politicas';
import { listarCamposBalance, actualizarCampoBalance, restaurarBalancePorDefecto, type CampoBalance } from './balanceConfig';
export type { CampoBalance } from './balanceConfig';
import {
  fundarAsentamiento as fundarAsentamientoEngine,
  evaluarViabilidadFundacion,
  FundacionInvalidaError,
  type ViabilidadFundacion,
} from '../engine/settlement';
export type { ViabilidadFundacion } from '../engine/settlement';
import { computeTodasLasZonas } from '../engine/zones';
import { avanzarSimulacion } from '../engine/simulation';
import { avanzarNpcGobernanza } from './npcGobernanza';
import { avanzarAutoComercioSimulado } from '../engine/simulacionAutoComercio';
import { proponerTrueque as proponerTruequeEngine, construirCaravanaComercial as construirCaravanaComercialEngine, CaravanaInvalidaError, TruequeInvalidoError } from '../engine/trade';
import { asegurarCaminoComercial } from '../engine/caminos';
import { controladorDeChokepoint } from '../engine/chokepoints';
import { colocarOrdenMercado as colocarOrdenMercadoEngine, calcularPrecioReferencia, OrdenInvalidaError } from '../engine/market';
import { crearFaccion as crearFaccionEngine, comprarCasa as comprarCasaEngine, calcularCapFundacion, calcularCupoNivel, capacidadCasas, FaccionInvalidaError } from '../engine/faccion';
import { asignarRey as asignarReyEngine, asignarEmbajador as asignarEmbajadorEngine, asignarCargoLocal as asignarCargoLocalEngine, CargoInvalidoError } from '../engine/cargos';
import { activarPolitica as activarPoliticaEngine, PoliticaInvalidaError } from '../engine/politicas';
import {
  proponerVasallaje as proponerVasallajeEngine,
  proponerAlianza as proponerAlianzaEngine,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
  DiplomaciaInvalidaError,
} from '../engine/diplomacia';
import { computeLigas, type LigaInfo } from '../engine/liga';
import { anexionar as anexionarEngine, fusionar as fusionarEngine, FusionInvalidaError } from '../engine/fusion';
import { reclutarTropa as reclutarTropaEngine, ReclutamientoInvalidoError, consumoRacionTropas } from '../engine/tropas';
import {
  anadirEdificioManualmente as anadirEdificioManualmenteEngine,
  quitarDeCola as quitarDeColaEngine,
  moverEnCola as moverEnColaEngine,
  reclamosDeFuentes as reclamosDeFuentesEngine,
  migrarEdificiosAEspacioLocal,
  ConstruccionManualInvalidaError,
} from '../engine/construction';
import {
  celdaMinimaDeEdificio,
  edificiosInternos,
  redDeCalles,
  segmentosDeRed,
  tamanoDeEdificio,
  type SegmentoTrazado,
} from '../engine/trazado';
import {
  iniciarAsedio as iniciarAsedioEngine,
  combateCampoAbierto as combateCampoAbiertoEngine,
  interceptarCaravana as interceptarCaravanaEngine,
  atacarCampamentoBandidos as atacarCampamentoBandidosEngine,
  CombateInvalidoError,
} from '../engine/combate';
import {
  lanzarCaravanaFundacion as lanzarCaravanaFundacionEngine,
  desarmarCaravanaFundacion as desarmarCaravanaFundacionEngine,
  ExpansionInvalidaError,
} from '../engine/expansion';

export interface EventoLog {
  tick: number;
  mensaje: string;
}

export interface GameState {
  /**
   * El mapa como DATOS PUROS (ver `src/worldgen/`): clonable y serializable, a diferencia de la fachada
   * `Mapa`, que es un objeto con índices. La fachada se deriva de aquí con `getMapa()` — el estado nunca
   * la guarda, para que una foto del historial siga siendo un objeto de datos y nada más.
   */
  mapa: MapaGenerado;
  /** Lo único del mapa que cambia jugando: cuánto se lleva extraído de cada yacimiento. */
  estadoMapa: EstadoMapa;
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  /** Caminos comerciales (Fase 0.3, Doc 1.6) — ver `engine/caminos.ts`. */
  caminos: CaminoComercial[];
  /** Campamentos de bandidos (Doc 1.9) — ver `engine/bandidos.ts`. */
  campamentosBandidos: CampamentoBandido[];
  bandidosProximoSpawnTick: number;
  /**
   * Facciones que juega el NPC de gobernanza (`app/npcGobernanza.ts`) en vez del jugador humano — se
   * enciende/apaga por Facción desde la pestaña Facción, en caliente y en ambos sentidos. Vive aquí, en la
   * capa de aplicación, y NO en `Faccion` (domain/types.ts) a propósito: es una decisión de quién maneja los
   * mandos, no un dato del mundo simulado. El `EstadoSimulacion` que recibe `avanzarSimulacion` se construye
   * campo a campo en `avanzarTick` y no incluye esto, así que el motor ni sabe que existe.
   */
  faccionesNpcIds: string[];
  tick: number;
  log: EventoLog[];
  historialJugadores: Record<string, EventoLog[]>;
}

/**
 * Nodo tal como viaja en el ARCHIVO: con `cantidad` = lo que le queda. En memoria el nodo es inmutable y
 * lleva `cantidadInicial`, y lo consumido vive aparte (`EstadoMapa`) — el archivo aplana las dos cosas en
 * un solo número porque es lo que ya guardaban las partidas existentes y no hay motivo para romperlas.
 */
export type NodoExportado = Omit<NodoRecurso, 'cantidadInicial'> & { cantidad: number };

/** Formato de archivo para exportar/importar una simulación completa (ver `exportarSimulacion`/`importarSimulacion`). */
export interface SimulacionExportada {
  version: 1;
  exportadoEn: string;
  tick: number;
  /**
   * Versión del ALGORITMO de generación con el que se creó este mundo (ver `WORLDGEN_VERSION`). El archivo
   * no guarda el mapa completo, solo la seed y los datos ya modificados en partida: la fertilidad se
   * REGENERA al importar. Si el algoritmo cambiase, la misma seed daría otro campo de fertilidad y el save
   * se cargaría como un mundo distinto sin avisar — este campo permite detectarlo y rechazarlo.
   * Ausente en archivos exportados antes de que existiera el campo; se asumen de la versión 1.
   */
  worldgenVersion?: number;
  world: { config: WorldConfig; recursos: NodoExportado[]; bosques: ZonaBosque[] };
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  /** Ausente en archivos exportados antes de Fase 0.3 — se asume sin caminos todavía (`?? []` al importar). */
  caminos?: CaminoComercial[];
  /** Ausente en archivos exportados antes de esta mecánica — se asume sin campamentos todavía al importar. */
  campamentosBandidos?: CampamentoBandido[];
  bandidosProximoSpawnTick?: number;
  /** Ausente en archivos exportados antes de esta mecánica — se asume que ninguna Facción es NPC al importar. */
  faccionesNpcIds?: string[];
  log: EventoLog[];
  historialJugadores: Record<string, EventoLog[]>;
}

/** Catálogos de referencia (listas fijas, sin comportamiento) que la interfaz necesita para construir formularios. */
export const CATALOGOS = {
  cargos: ['gobernador', 'tesorero', 'general', 'maestroObras', 'sacerdote'] as CargoTipo[],
  origenesTropa: ['pesants', 'artesanos'] as const,
  tropasReclutables: TROPAS_RECLUTABLES,
  recursosTrueque: ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock'] as RecursoTipo[],
  recursosMercado: ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'livestock'] as RecursoTipo[],
  politicas: POLITICA_CATALOGO,
  duracionPoliticaTicks: POLITICAS.duracionTicksPorDefecto,
  slotsPorCargoBase: POLITICAS.slotsPorCargo,
  nivelFaccionPorSlotExtraGobernador: POLITICAS.nivelFaccionPorSlotExtraGobernador,
  maximoEdificiosEnCola: NECESIDADES.maximoEnCola,
  maximoEnConstruccionSimultanea: NECESIDADES.maximoEnConstruccionSimultanea,
  // Control manual de cola (Doc 4.2, a petición del usuario): catálogo completo salvo Centro Urbano, que
  // nunca pasa por cola (Doc 1.3) — usado por el selector de "añadir a la cola" de Gobernador/Maestro de
  // Obras Y por el segmento "Info:" que muestra costo/tiempo/gates antes de confirmar (mismo patrón que
  // `tropasReclutables` para el reclutamiento).
  // `puestoMercado` también fuera: no se construye, lo crea el motor al subir de nivel el Mercado (ver
  // `crearPuestosDeMercado`, engine/construction.ts).
  catalogoEdificios: (Object.keys(EDIFICIO_CATALOGO) as EdificioTipo[])
    .filter((tipo) => tipo !== 'centroUrbano' && tipo !== 'puestoMercado')
    .map((tipo) => {
      const def = EDIFICIO_CATALOGO[tipo] as {
        costo: Partial<Record<string, number>>;
        tiempoConstruccionTicks: number;
        requisitoNivelAsentamientoConstruccion?: number;
        nivelFaccionMinimo?: number;
      };
      return {
        tipo,
        costo: def.costo,
        tiempoConstruccionTicks: def.tiempoConstruccionTicks,
        requisitoNivelAsentamiento: def.requisitoNivelAsentamientoConstruccion ?? 0,
        requisitoNivelFaccion: def.nivelFaccionMinimo ?? 0,
      };
    }),
  /** Vista de Asentamiento (a petición del usuario): tamaño de celda de la rejilla local — solo dato, sin
   * comportamiento; `ui/canvas.ts` lo usa para dibujar (tamaño de edificio, paso de la cuadrícula), nunca para
   * decidir colocación, eso es del motor. */
  tamanoCeldaAsentamiento: REJILLA_ASENTAMIENTO.tamanoCelda,
  /** Radio ESTÁTICO del lienzo de la Vista de Asentamiento (a petición del usuario) — no depende de
   * `asentamiento.radioPotencial` (eso crece con nivel/construcción, ver `ZONA_INFLUENCIA`). Ver comentario en
   * `REJILLA_ASENTAMIENTO.radioMapa`, constants.ts. */
  radioMapaAsentamiento: REJILLA_ASENTAMIENTO.radioMapa,
};

type Listener = () => void;

function idsNoVacios(csv: string): string[] {
  return csv
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

/**
 * Único dueño del estado de simulación y única puerta hacia el motor. Expone acciones de alto
 * nivel (una por intención de usuario) que ya resuelven validación/errores de dominio y dejan
 * constancia en el log — la interfaz nunca necesita conocer las clases de error del motor.
 */
export class GameStore {
  private state: GameState;
  private listeners = new Set<Listener>();
  private contadorAcciones = 0;
  /** Una foto completa del estado al final de cada tick (índice = número de tick) — alimenta el slider de línea de tiempo. */
  private historial: GameState[] = [];
  /** Tick más antiguo con foto disponible. 0 en una partida normal; el tick importado tras un `importarSimulacion` (no hay fotos de ticks previos a ese punto). */
  private historialDesde = 0;
  /**
   * Fachadas `Mapa` ya construidas, indexadas por el ESTADO del que salen (ver `getMapa`). La clave es el
   * estado y no el mundo generado porque todas las fotos del historial comparten el mismo `MapaGenerado`
   * por referencia — indexar por él devolvería la fachada de la partida en curso al pedir la de una foto.
   */
  private mapasPorEstado = new WeakMap<EstadoMapa, Mapa>();

  constructor() {
    this.state = {
      estadoMapa: crearEstadoMapa(),
      mapa: generarMapa({ ...MAPA_DEFAULT, seed: 1 }),
      asentamientos: [],
      facciones: [],
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
      campamentosBandidos: [],
      bandidosProximoSpawnTick: 0,
      faccionesNpcIds: [],
      tick: 0,
      log: [],
      historialJugadores: {},
    };
    this.registrar('Mundo generado. Selecciona una facción y haz clic en el mapa para fundar.');
    this.notify();
  }

  getState(): Readonly<GameState> {
    return this.state;
  }

  /**
   * Fachada de consulta del mapa (índices + consultas espaciales) para un estado dado; por defecto, el
   * estado en vivo. Se cachea por objeto `MapaGenerado`, así que la partida en curso reutiliza siempre la
   * misma instancia y cada foto del historial construye la suya solo si alguien llega a dibujarla.
   */
  getMapa(estado: Readonly<GameState> = this.state): Mapa {
    const cacheado = this.mapasPorEstado.get(estado.estadoMapa);
    if (cacheado) return cacheado;
    const mapa = crearMapa(estado.mapa, estado.estadoMapa);
    this.mapasPorEstado.set(estado.estadoMapa, mapa);
    return mapa;
  }

  /** Foto de solo lectura del estado tal como estaba al final del tick `tick`, o `undefined` si no existe. */
  getSnapshot(tick: number): Readonly<GameState> | undefined {
    return this.historial[tick];
  }

  /** Rango con fotos disponibles para el slider de línea de tiempo. */
  getTickRange(): { min: number; max: number } {
    return { min: this.historialDesde, max: this.state.tick };
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Clon profundo del estado actual para guardar en el historial. El MUNDO (`mapa`) se comparte por
   * referencia sin clonar nada: es inmutable desde que se genera. Lo único que hay que fotografiar es
   * `estadoMapa`, un registro de números — antes esto obligaba a clonar los ~117 objetos-nodo cada tick.
   */
  private clonarEstadoActual(): GameState {
    return {
      mapa: this.state.mapa,
      estadoMapa: {
        extraido: { ...this.state.estadoMapa.extraido },
        regeneraEnTick: { ...this.state.estadoMapa.regeneraEnTick },
      },
      asentamientos: structuredClone(this.state.asentamientos),
      facciones: structuredClone(this.state.facciones),
      caravanas: structuredClone(this.state.caravanas),
      acuerdos: structuredClone(this.state.acuerdos),
      ordenes: structuredClone(this.state.ordenes),
      relaciones: structuredClone(this.state.relaciones),
      titulos: structuredClone(this.state.titulos),
      caminos: structuredClone(this.state.caminos),
      campamentosBandidos: structuredClone(this.state.campamentosBandidos),
      bandidosProximoSpawnTick: this.state.bandidosProximoSpawnTick,
      faccionesNpcIds: [...this.state.faccionesNpcIds],
      tick: this.state.tick,
      log: structuredClone(this.state.log),
      historialJugadores: structuredClone(this.state.historialJugadores),
    };
  }

  private notify(): void {
    this.historial[this.state.tick] = this.clonarEstadoActual();
    for (const listener of this.listeners) listener();
  }

  private registrar(mensaje: string): void {
    this.state.log = [{ tick: this.state.tick, mensaje }, ...this.state.log];
  }

  /**
   * Descarta ids de `faccionesNpcIds` cuya Facción ya no existe — una anexión o fusión (`engine/fusion.ts`)
   * hace desaparecer una de las dos Facciones, y la fusión además crea una TERCERA nueva, que nace bajo
   * control manual: si el jugador quiere que también la juegue el NPC, la marca a mano.
   */
  private sincronizarFaccionesNpc(): void {
    this.state.faccionesNpcIds = this.state.faccionesNpcIds.filter((id) => this.state.facciones.some((f) => f.id === id));
  }

  private registrarJugador(jugadorId: string, mensaje: string): void {
    if (!jugadorId) return;
    const lista = this.state.historialJugadores[jugadorId] ?? [];
    this.state.historialJugadores[jugadorId] = [{ tick: this.state.tick, mensaje }, ...lista];
  }

  // --- Derivados de solo lectura (evitan que la interfaz importe funciones del motor) ---
  // Todos aceptan datos opcionales para poder calcularse tanto sobre el estado en vivo como
  // sobre una foto del historial (vista de línea de tiempo) — por defecto usan el estado en vivo.

  getZonas(asentamientos: Asentamiento[] = this.state.asentamientos): ZonaInfluencia[] {
    return computeTodasLasZonas(asentamientos);
  }

  /**
   * Trazado urbano de UN asentamiento, ya resuelto a coordenadas locales para dibujar: los tramos de calle y
   * de camino, y el rectángulo que ocupa cada edificio (por id). Todo derivado en el motor
   * (`engine/trazado.ts`), nada persistido — ver `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
   *
   * Acoplamiento 0: `ui/canvas.ts` recibe esto ya masticado y no sabe nada de celdas, aristas, barrios ni
   * manzanas. La interfaz nunca decide un trazado, solo lo pinta.
   */
  getTrazadoAsentamiento(asentamiento: Asentamiento): {
    calles: SegmentoTrazado[];
    caminos: SegmentoTrazado[];
    huellas: Record<string, { x: number; y: number; ancho: number; alto: number }>;
  } {
    const { calles, caminos } = segmentosDeRed(redDeCalles(asentamiento.id, asentamiento.edificios));
    const huellas: Record<string, { x: number; y: number; ancho: number; alto: number }> = {};
    for (const edificio of edificiosInternos(asentamiento.edificios)) {
      const min = celdaMinimaDeEdificio(edificio);
      const tamano = tamanoDeEdificio(edificio);
      huellas[edificio.id] = {
        x: min.col * CATALOGOS.tamanoCeldaAsentamiento,
        y: min.row * CATALOGOS.tamanoCeldaAsentamiento,
        ancho: tamano.ancho * CATALOGOS.tamanoCeldaAsentamiento,
        alto: tamano.alto * CATALOGOS.tamanoCeldaAsentamiento,
      };
    }
    return { calles, caminos, huellas };
  }

  /**
   * Chokepoint id -> asentamiento que lo controla (Fase 0.3, Doc 1.5): mismo criterio que las fronteras,
   * derivado de las zonas de influencia (ver `engine/chokepoints.ts` `controladorDeChokepoint`). Solo
   * lectura, para pintar el mapa (`ui/canvas.ts`) — la interfaz nunca debe recalcular esta regla por su
   * cuenta, solo leerla de aquí (acoplamiento 0 entre interfaz y motor).
   */
  chokepointsControl(zonas: ZonaInfluencia[] = this.getZonas()): Map<string, string> {
    const resultado = new Map<string, string>();
    for (const chokepoint of this.getMapa().listarChokepoints()) {
      const controladorId = controladorDeChokepoint(chokepoint, zonas);
      if (controladorId) resultado.set(chokepoint.id, controladorId);
    }
    return resultado;
  }

  getLigas(relaciones: RelacionPolitica[] = this.state.relaciones, facciones: Faccion[] = this.state.facciones): LigaInfo[] {
    return computeLigas(relaciones, facciones);
  }

  capFundacion(nivel: number): number {
    return calcularCapFundacion(nivel);
  }

  /**
   * Progreso de nivel de FACCIÓN (Doc 1.7/Fase_0_5 §8): experiencia acumulada contra el umbral que falta
   * para el siguiente nivel — mismo dato que decide `cupoAsentamientosFaccion`/`capFundacion` de esta Facción.
   */
  nivelFaccionInfo(faccion: Faccion): { nivel: number; esMaximo: boolean; experiencia: number; umbralActual: number; umbralSiguiente: number | null } {
    const esMaximo = faccion.nivel >= NIVEL_FACCION.nivelMaximo;
    return {
      nivel: faccion.nivel,
      esMaximo,
      experiencia: faccion.experiencia,
      umbralActual: faccion.nivel > 1 ? NIVEL_FACCION.xpParaNivel[faccion.nivel - 2]! : 0,
      umbralSiguiente: esMaximo ? null : NIVEL_FACCION.xpParaNivel[faccion.nivel - 1]!,
    };
  }

  /**
   * Cupo de asentamientos en nivel 2/3 que le corresponde a esta Facción por su nivel actual (Doc Fase_0_5
   * §5, ver `CUPO_NIVEL_ASENTAMIENTO`) contra cuántos de sus asentamientos YA ocupan cada uno (por
   * `nivelActual` operativo, no `nivel`/nivelAlcanzado — igual criterio que `engine/simulation.ts`).
   */
  cupoAsentamientosFaccion(faccion: Faccion): { nivel2: { ocupados: number; total: number }; nivel3: { ocupados: number; total: number } } {
    const propios = this.state.asentamientos.filter((a) => a.faccionId === faccion.id);
    return {
      nivel2: { ocupados: propios.filter((a) => nivelActualDe(a) === 2).length, total: calcularCupoNivel(faccion.nivel, 2) },
      nivel3: { ocupados: propios.filter((a) => nivelActualDe(a) === 3).length, total: calcularCupoNivel(faccion.nivel, 3) },
    };
  }

  cupoVivienda(asentamiento: Asentamiento): number {
    return capacidadCasas(asentamiento);
  }

  precioReferencia(recurso: string, asentamientos: Asentamiento[] = this.state.asentamientos): number {
    return calcularPrecioReferencia(recurso, asentamientos);
  }

  /**
   * Evalúa un emplazamiento antes de fundar (solo lectura): si es legal y, sobre todo, si tiene madera al
   * alcance — sin bosque en el radio inicial el asentamiento casi siempre acaba en ruinas (ver
   * `evaluarViabilidadFundacion`). NO bloquea nada: alimenta el aviso previo de la interfaz.
   */
  viabilidadFundacion(posicion: { x: number; y: number }): ViabilidadFundacion {
    return evaluarViabilidadFundacion(this.getMapa(), posicion, this.state.asentamientos);
  }

  /**
   * Progreso de nivel de asentamiento (modelo de gates, Doc 4.5, rediseño de progreso Fase 0): población
   * actual vs. requerida y qué edificios de la lista todavía faltan por tener activos, para el siguiente nivel.
   */
  nivelAsentamientoInfo(asentamiento: Asentamiento): ProgresoNivelAsentamiento {
    return progresoNivelAsentamiento(asentamiento);
  }

  /**
   * Por qué un asentamiento que YA cumple los gates de nivel (población + edificios) no sube: cupo de nivel
   * ocupado por su Facción (Doc Fase_0_5 §5) — `avanzarNivelAsentamiento` (engine/mantenimiento.ts) exige gates
   * Y cupo libre a la vez, y se queda "elegible, esperando cupo" indefinidamente si no lo hay (nunca se
   * bloquea ni se le baja nada). Devuelve `null` cuando el cupo NO es el motivo: nivel máximo, gates todavía
   * sin cumplir, o nivel objetivo fuera de la curva de cupo definida hoy (solo 2 y 3, ver `engine/simulation.ts`
   * — 1, 4 y 5 no tienen tope).
   */
  cupoNivelInfo(asentamiento: Asentamiento): { nivelObjetivo: number; ocupados: number; cupoTotal: number } | null {
    const nivelElegible = calcularNivelAsentamiento(asentamiento);
    if (nivelElegible <= asentamiento.nivel) return null;
    const nivelObjetivo = asentamiento.nivel + 1;
    if (nivelObjetivo < 2 || nivelObjetivo > 3) return null;
    const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId);
    if (!faccion) return null;
    const cupoTotal = calcularCupoNivel(faccion.nivel, nivelObjetivo as 2 | 3);
    const ocupados = this.state.asentamientos.filter((a) => a.faccionId === asentamiento.faccionId && nivelActualDe(a) === nivelObjetivo).length;
    if (ocupados < cupoTotal) return null;
    return { nivelObjetivo, ocupados, cupoTotal };
  }

  /** Coste de mantenimiento del tick actual, recurso por recurso, con lo disponible y si alcanza a cubrirlo. */
  mantenimientoInfo(asentamiento: Asentamiento): {
    enGracia: boolean;
    ticksParaFinGracia: number;
    items: { recurso: string; costoPorTick: number; disponible: number; cubierto: boolean }[];
  } {
    const ticksDesdeFundacion = this.state.tick - asentamiento.fundadoEnTick;
    const enGracia = ticksDesdeFundacion < MANTENIMIENTO.graciaTicks;
    const capital = encontrarCapital(asentamiento.faccionId, this.state.asentamientos);
    const costo = calcularCostoMantenimiento(asentamiento, capital);
    const items = Object.entries(costo).map(([recurso, cantidad]) => {
      const disponible = asentamiento.almacen[recurso]?.cantidad ?? 0;
      return { recurso, costoPorTick: cantidad ?? 0, disponible, cubierto: disponible >= (cantidad ?? 0) };
    });
    // El trigo NO viene de `calcularCostoMantenimiento` (ya no lo cobra Mantenimiento directamente, ver
    // engine/mantenimiento.ts) — el "apartado de trigo" que se muestra aquí es la suma real de consumo de
    // comida de la población + raciones de tropas, que se descuenta en `avanzarNutricionPoblacion`/`avanzarMantenimientoTropas`.
    const costoTrigo = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento);
    const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
    items.push({ recurso: 'trigo', costoPorTick: costoTrigo, disponible: trigoDisponible, cubierto: trigoDisponible >= costoTrigo });
    return { enGracia, ticksParaFinGracia: Math.max(0, MANTENIMIENTO.graciaTicks - ticksDesdeFundacion), items };
  }

  /** Slots de política disponibles para `cargo` según el nivel de Facción (el Gobernador escala con el nivel). */
  slotsPoliticaDisponibles(cargo: CargoTipo, nivelFaccion: number): number {
    return slotsDisponibles(cargo, nivelFaccion);
  }

  /** Producción por tick de cada edificio activo de extracción/producción primaria, agrupada por tipo. */
  produccionInfo(asentamiento: Asentamiento): ProduccionItem[] {
    const zona = this.getZonas().find((z) => z.asentamientoId === asentamiento.id);
    return produccionPorTick(asentamiento, this.getMapa(), zona?.poligono ?? []);
  }

  /** Demanda de mano de obra agregada (pesants) frente a lo que piden los edificios productores activos. */
  manoObraInfo(asentamiento: Asentamiento): ManoObraInfo {
    return calcularManoObraInfo(asentamiento);
  }

  /** Población actual vs. límite por clase (Doc 4.1): Pesants/Artesanos limitados por cupos de Vivienda
   * (separados por clase, ver `capacidadViviendaPesants`/`capacidadViviendaArtesanos`), Nobleza por la
   * `capacidadNobles` del Palacio — solo lectura, no altera el motor. */
  poblacionInfo(asentamiento: Asentamiento): {
    pesants: { actual: number; limite: number };
    artesanos: { actual: number; limite: number };
    nobleza: { actual: number; limite: number };
  } {
    const palaciosActivos = edificiosPorTipoYEstado(asentamiento, 'palacio').length;
    return {
      pesants: { actual: asentamiento.poblacion.pesants, limite: capacidadViviendaPesants(asentamiento) },
      artesanos: { actual: asentamiento.poblacion.artesanos, limite: capacidadViviendaArtesanos(asentamiento) },
      nobleza: { actual: asentamiento.poblacion.nobleza, limite: palaciosActivos * EDIFICIO_CATALOGO.palacio.capacidadNobles },
    };
  }

  // --- Acciones (una por intención de usuario) ---

  fundarAsentamiento(faccionId: string, posicion: { x: number; y: number }, numJugadores: number): void {
    const n = Math.min(FUNDACION.maxJugadoresFundacionGrupal, Math.max(1, numJugadores || 1));
    const jugadoresIds = Array.from({ length: n }, (_, i) => `jugador-${faccionId}-${i + 1}`);
    try {
      const resultado = fundarAsentamientoEngine(
        this.getMapa(),
        this.state.facciones,
        faccionId,
        posicion,
        jugadoresIds,
        this.state.asentamientos,
        this.state.tick
      );
      this.state.asentamientos = [...this.state.asentamientos, resultado.asentamiento];
      this.state.facciones = resultado.facciones;
      const nombreFaccion = this.state.facciones.find((f) => f.id === faccionId)?.nombre ?? faccionId;
      this.registrar(`${nombreFaccion} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`);
      for (const jugadorId of jugadoresIds) {
        this.registrarJugador(jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
      }
    } catch (err) {
      if (err instanceof FundacionInvalidaError) this.registrar(`Fundación rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Caravana de Fundación (Doc 1.8): expande una Facción más allá de su primer asentamiento. Lleva consigo
   * a ciudadanos ya existentes de la Facción (no jugadores nuevos) y reserva de inmediato un cupo del Cap
   * de Fundación (Doc 1.7) mientras esté en tránsito. */
  lanzarCaravanaFundacion(origenAsentamientoId: string, destino: { x: number; y: number }, numJugadores: number): void {
    try {
      const origen = this.state.asentamientos.find((a) => a.id === origenAsentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === origen.faccionId)!;
      const resultado = lanzarCaravanaFundacionEngine(
        this.getMapa(),
        origen,
        faccion,
        destino,
        this.state.asentamientos,
        this.state.caravanas,
        numJugadores,
        this.state.tick,
        this.contadorAcciones++
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === origen.id ? resultado.origenActualizado : a));
      this.state.caravanas = [...this.state.caravanas, resultado.caravana];
      this.registrar(`${origen.id}: lanza una Caravana de Fundación hacia (${Math.round(destino.x)}, ${Math.round(destino.y)}).`);
    } catch (err) {
      if (err instanceof ExpansionInvalidaError) this.registrar(`Caravana de Fundación rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Desarma una Caravana de Fundación en tránsito y reembolsa su contenido íntegro al asentamiento de origen. */
  desarmarCaravanaFundacion(caravanaId: string): void {
    try {
      const caravana = this.state.caravanas.find((c) => c.id === caravanaId)!;
      const origen = this.state.asentamientos.find((a) => a.id === caravana.origenAsentamientoId)!;
      const actualizado = desarmarCaravanaFundacionEngine(origen, caravana);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === origen.id ? actualizado : a));
      this.state.caravanas = this.state.caravanas.filter((c) => c.id !== caravanaId);
      this.registrar(`${origen.id}: desarma la Caravana de Fundación ${caravanaId} y recupera su contenido.`);
    } catch (err) {
      if (err instanceof ExpansionInvalidaError) this.registrar(`No se pudo desarmar la caravana: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Creación libre de Facción (Doc 0): cualquier nombre no vacío y no repetido, sin límite de cantidad. */
  crearFaccion(nombre: string): void {
    try {
      const nombreLimpio = nombre.trim();
      const yaExiste = this.state.facciones.some((f) => f.nombre.toLowerCase() === nombreLimpio.toLowerCase());
      if (yaExiste) throw new FaccionInvalidaError(`Ya existe una Facción llamada "${nombreLimpio}".`);
      const nueva = crearFaccionEngine(`faccion-custom-${this.contadorAcciones++}`, nombreLimpio);
      this.state.facciones = [...this.state.facciones, nueva];
      this.registrar(`Nueva Facción fundada: ${nueva.nombre}.`);
    } catch (err) {
      if (err instanceof FaccionInvalidaError) this.registrar(`Creación de Facción rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /**
   * Cede al NPC de gobernanza (`app/npcGobernanza.ts`) el control de una Facción, o lo retoma. Es solo un id
   * dentro o fuera de una lista que se lee al principio de cada `avanzarTick`, así que funciona en caliente y
   * en ambos sentidos a mitad de partida: el NPC no deja nada que impida volver a jugarla a mano (cargos,
   * reservas, Mercado y tropas son estado normal del juego, creado con las mismas funciones del motor que usa
   * el jugador humano).
   */
  alternarFaccionNpc(faccionId: string, activo: boolean): void {
    const faccion = this.state.facciones.find((f) => f.id === faccionId);
    if (!faccion) return;
    const yaEsNpc = this.state.faccionesNpcIds.includes(faccionId);
    if (activo === yaEsNpc) return;

    this.state.faccionesNpcIds = activo
      ? [...this.state.faccionesNpcIds, faccionId]
      : this.state.faccionesNpcIds.filter((id) => id !== faccionId);
    this.registrar(
      activo
        ? `${faccion.nombre}: pasa a estar controlada por el NPC de gobernanza (juega sola).`
        : `${faccion.nombre}: vuelve a control manual del jugador.`
    );
    this.notify();
  }

  /** ¿Esta Facción la juega el NPC? (`GameState.faccionesNpcIds`, para la pestaña Facción). */
  esFaccionNpc(faccionId: string): boolean {
    return this.state.faccionesNpcIds.includes(faccionId);
  }

  asignarRey(faccionId: string, jugadorId: string): void {
    try {
      const faccion = this.state.facciones.find((f) => f.id === faccionId)!;
      this.state.facciones = this.state.facciones.map((f) => (f.id === faccion.id ? asignarReyEngine(f, jugadorId) : f));
      this.registrar(`${faccion.nombre}: ${jugadorId} es el nuevo Rey.`);
      this.registrarJugador(jugadorId, `Nombrado Rey de ${faccion.nombre}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Rey rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  asignarEmbajador(faccionId: string, jugadorId: string): void {
    try {
      const faccion = this.state.facciones.find((f) => f.id === faccionId)!;
      this.state.facciones = this.state.facciones.map((f) => (f.id === faccion.id ? asignarEmbajadorEngine(f, jugadorId) : f));
      this.registrar(`${faccion.nombre}: ${jugadorId} es el nuevo Embajador.`);
      this.registrarJugador(jugadorId, `Nombrado Embajador de ${faccion.nombre}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Embajador rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  asignarCargoLocal(asentamientoId: string, cargo: CargoTipo, jugadorId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const actualizado = asignarCargoLocalEngine(asentamiento, faccion, cargo, jugadorId);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: ${jugadorId} asignado como ${cargo}.`);
      this.registrarJugador(jugadorId, `Asignado como ${cargo} en ${asentamiento.id}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Cargo rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  comprarCasa(asentamientoId: string, jugadorId: string): void {
    try {
      const resultado = comprarCasaEngine(this.state.facciones, this.state.asentamientos, asentamientoId, jugadorId);
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === resultado.asentamiento.id ? resultado.asentamiento : a));
      this.registrar(`${jugadorId} compra casa en ${asentamientoId} y obtiene ciudadanía.`);
      this.registrarJugador(jugadorId, `Compra casa en ${asentamientoId} y obtiene ciudadanía.`);
    } catch (err) {
      if (err instanceof FaccionInvalidaError) this.registrar(`Compra de casa rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  activarPolitica(asentamientoId: string, cargo: CargoTipo, politicaId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const actualizado = activarPoliticaEngine(asentamiento, faccion, cargo, politicaId, this.state.tick, this.contadorAcciones++);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: política "${politicaId}" activada por ${cargo}.`);
    } catch (err) {
      if (err instanceof PoliticaInvalidaError) this.registrar(`Política rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  proponerRelacion(
    tipo: 'vasallaje' | 'alianza',
    faccionAId: string,
    faccionBId: string,
    tributoRecurso: string,
    tributoCantidad: number
  ): void {
    try {
      const nueva =
        tipo === 'vasallaje'
          ? proponerVasallajeEngine(
              this.state.facciones,
              this.state.relaciones,
              faccionAId,
              faccionBId,
              tributoRecurso,
              tributoCantidad,
              this.state.tick,
              this.contadorAcciones++
            )
          : proponerAlianzaEngine(this.state.facciones, this.state.relaciones, faccionAId, faccionBId, this.state.tick, this.contadorAcciones++);
      this.state.relaciones = [...this.state.relaciones, nueva];
      this.registrar(`Relación propuesta: ${nueva.id}.`);
    } catch (err) {
      if (err instanceof DiplomaciaInvalidaError) this.registrar(`Relación rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  romperRelacion(relacionId: string, iniciadorFaccionId: string): void {
    if (!relacionId) return;
    const resultado = romperRelacionEngine(this.state.facciones, this.state.relaciones, relacionId, iniciadorFaccionId);
    this.state.facciones = resultado.facciones;
    this.state.relaciones = resultado.relaciones;
    this.registrar(`Relación ${relacionId} rota voluntariamente.`);
    this.notify();
  }

  rebelionVasallo(relacionId: string): void {
    if (!relacionId) return;
    try {
      const resultado = rebelionVasalloEngine(this.state.facciones, this.state.relaciones, this.state.acuerdos, this.state.asentamientos, relacionId);
      this.state.facciones = resultado.facciones;
      this.state.relaciones = resultado.relaciones;
      this.state.acuerdos = resultado.acuerdos;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof DiplomaciaInvalidaError) this.registrar(`Rebelión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  anexionar(faccionAId: string, faccionBId: string): void {
    try {
      const resultado = anexionarEngine(this.state.facciones, this.state.asentamientos, faccionAId, faccionBId);
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = resultado.asentamientos;
      this.sincronizarFaccionesNpc();
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof FusionInvalidaError) this.registrar(`Anexión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  fusionar(faccionAId: string, faccionBId: string, nuevoNombre: string, nuevoReyId: string): void {
    try {
      const resultado = fusionarEngine(
        this.state.facciones,
        this.state.asentamientos,
        faccionAId,
        faccionBId,
        nuevoNombre || 'Facción Fusionada',
        nuevoReyId,
        this.state.tick
      );
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = resultado.asentamientos;
      this.sincronizarFaccionesNpc();
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof FusionInvalidaError) this.registrar(`Fusión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  proponerTrueque(
    asentamientoAId: string,
    recursoA: string,
    cantidadA: number,
    asentamientoBId: string,
    recursoB: string,
    cantidadB: number
  ): void {
    try {
      const nuevo = proponerTruequeEngine(
        this.state.asentamientos,
        asentamientoAId,
        asentamientoBId,
        recursoA,
        recursoB,
        cantidadA,
        cantidadB,
        this.state.tick,
        this.contadorAcciones++
      );
      this.state.acuerdos = [...this.state.acuerdos, nuevo];
      this.registrar(`Trueque propuesto: ${nuevo.id}.`);

      // Camino Comercial (Doc 1.6, Fase 0.3): se genera al establecer la relación comercial, no cada vez
      // que se propone un trueque nuevo — `asegurarCaminoComercial` no hace nada si el par ya tiene uno.
      const asentamientoA = this.state.asentamientos.find((a) => a.id === asentamientoAId);
      const asentamientoB = this.state.asentamientos.find((a) => a.id === asentamientoBId);
      if (asentamientoA && asentamientoB) {
        const caminosPrevios = this.state.caminos.length;
        this.state.caminos = asegurarCaminoComercial(this.state.caminos, this.getMapa(), asentamientoA, asentamientoB);
        if (this.state.caminos.length > caminosPrevios) {
          this.registrar(`Nuevo camino comercial entre ${asentamientoA.id} y ${asentamientoB.id}.`);
        }
      }
    } catch (err) {
      if (err instanceof TruequeInvalidoError) this.registrar(`Trueque rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  colocarOrdenMercado(asentamientoId: string, tipo: 'compra' | 'venta', recurso: string, cantidad: number, precio: number | undefined): void {
    try {
      const nueva = colocarOrdenMercadoEngine(this.state.asentamientos, asentamientoId, tipo, recurso, cantidad, this.state.tick, precio, this.contadorAcciones++);
      this.state.ordenes = [...this.state.ordenes, nueva];
      this.registrar(`Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`);
    } catch (err) {
      if (err instanceof OrdenInvalidaError) this.registrar(`Orden rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Ampliación de comercio (a petición del usuario): construye una caravana comercial propia — cuesta
   * madera, exige Mercado activo y respeta el cupo de flota del asentamiento (Doc 3.3). */
  crearCaravana(asentamientoId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const { asentamiento: actualizado, caravana } = construirCaravanaComercialEngine(
        asentamiento,
        this.state.caravanas,
        this.state.tick,
        this.contadorAcciones++
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.state.caravanas = [...this.state.caravanas, caravana];
      this.registrar(`${asentamientoId}: construye una caravana comercial (${caravana.id}).`);
    } catch (err) {
      if (err instanceof CaravanaInvalidaError) this.registrar(`Caravana rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Solo lectura, para la pestaña Guerra/Acciones: cupo de flota, cuántas caravanas propias tiene el
   * asentamiento y en qué estado (Doc 3.3, ampliación de comercio), y el cooldown de creación (compartido con
   * la Caravana de Fundación, `CARAVANA_COOLDOWN.ticksCooldown` — a petición del usuario). */
  caravanasInfo(asentamiento: Asentamiento): {
    mercadoActivo: boolean;
    cupo: number;
    disponibles: number;
    enTransito: number;
    retornando: number;
    ticksCooldownRestantes: number;
  } {
    const propias = this.state.caravanas.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === asentamiento.id);
    return {
      mercadoActivo: tieneMercadoActivoEngine(asentamiento),
      cupo: cupoCaravanasEngine(asentamiento),
      disponibles: propias.filter((c) => c.estado === 'disponible').length,
      enTransito: propias.filter((c) => c.estado === 'en_transito').length,
      retornando: propias.filter((c) => c.estado === 'retornando').length,
      ticksCooldownRestantes: ticksCooldownCaravanaRestantesEngine(asentamiento, this.state.tick),
    };
  }

  /** Reclutamiento por equipo (Doc 5.7/5.8): recluta una tropa específica vía Barracón/Galería de tiro, de
   * origen Pesants o Artesanos. Nobleza ya no recluta tropas (sigue existiendo como clase de población, Doc 4.1).
   * Escuadrón de UN jugador (Doc 2.5, a petición del usuario): `jugadorId` debe ser residente de `asentamientoId`.
   * La cantidad de soldados reclutada es el faltante hasta `TROPAS_RECLUTABLES[].unidadesPorDefecto` (repone bajas
   * si el jugador ya tenía el escuadrón por debajo del tope), no la elige el jugador. */
  reclutarTropa(asentamientoId: string, jugadorId: string, tropaId: string, origen: 'pesants' | 'artesanos'): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const antes = asentamiento.escuadrones.find((e) => e.jugadorId === jugadorId && e.tropaId === tropaId)?.cantidad ?? 0;
      const actualizado = reclutarTropaEngine(asentamiento, jugadorId, tropaId, origen, this.state.tick, this.contadorAcciones++);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      const despues = actualizado.escuadrones.find((e) => e.jugadorId === jugadorId && e.tropaId === tropaId)?.cantidad ?? 0;
      this.registrar(`${asentamiento.id}: ${jugadorId} recluta ${despues - antes} de la tropa "${tropaId}" (${origen}).`);
    } catch (err) {
      if (err instanceof ReclutamientoInvalidoError) this.registrar(`Reclutamiento rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Residentes de un asentamiento (Doc 2.5): fundadores + casas compradas, deduplicado — cualquiera de ellos
   * puede reclutar o reponer SU escuadrón ahí (ver `reclutarTropa`). Usado por la UI para el selector de Jugador
   * en Reclutamiento y para agrupar los chips de Combate por jugador. */
  jugadoresDeAsentamiento(asentamientoId: string): string[] {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId);
    if (!asentamiento) return [];
    return [...new Set([...asentamiento.jugadoresFundadoresIds, ...asentamiento.casasCompradas])];
  }

  /**
   * Control manual de cola (Doc 4.2, a petición del usuario — reemplaza el mecanismo de política de
   * desbloqueo que tenían Barracón/Galería de tiro/Palacio/Mercado): Gobernador o Maestro de Obras añaden
   * CUALQUIER edificio del catálogo a la cola, siempre que el asentamiento pueda pagarlo — la ubicación la
   * sigue decidiendo siempre el algoritmo de colocación, nunca el jugador.
   */
  anadirEdificioManualmente(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', tipo: EdificioTipo): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const zona = this.getZonas().find((z) => z.asentamientoId === asentamiento.id);
      const capital = encontrarCapital(asentamiento.faccionId, this.state.asentamientos);
      const reclamos = reclamosDeFuentesEngine(this.state.asentamientos);
      const actualizado = anadirEdificioManualmenteEngine(
        asentamiento,
        faccion,
        cargo,
        tipo,
        zona?.poligono ?? [],
        this.getMapa(),
        capital,
        reclamos,
        this.contadorAcciones++
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: ${cargo} añade ${tipo} a la cola (pagado).`);
    } catch (err) {
      if (err instanceof ConstruccionManualInvalidaError) this.registrar(`Añadir a la cola rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Quita un proyecto `en_cola` (solo si aún no empezó a construirse) y devuelve el costo completo pagado. */
  quitarDeCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const actualizado = quitarDeColaEngine(asentamiento, cargo, edificioId);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: ${cargo} quita un proyecto de la cola (recursos devueltos).`);
    } catch (err) {
      if (err instanceof ConstruccionManualInvalidaError) this.registrar(`Quitar de la cola rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Mueve un proyecto `en_cola` una posición arriba/abajo en el orden de arranque. */
  moverEnCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string, direccion: 'arriba' | 'abajo'): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const actualizado = moverEnColaEngine(asentamiento, cargo, edificioId, direccion);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: ${cargo} reordena la cola de construcción.`);
    } catch (err) {
      if (err instanceof ConstruccionManualInvalidaError) this.registrar(`Reordenar cola rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Overhaul de auto-construcción: pausa/reanuda la detección de NUEVAS necesidades en un asentamiento — lo
   * ya pagado (`en_cola`/`en_construccion`) sigue avanzando normal (ver `Asentamiento.autoConstruccionPausada`,
   * `engine/construction.ts`). */
  pausarAutoConstruccion(asentamientoId: string): void {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
    this.state.asentamientos = this.state.asentamientos.map((a) =>
      a.id === asentamientoId ? { ...a, autoConstruccionPausada: true } : a
    );
    this.registrar(`${asentamiento.id}: auto-construcción pausada.`);
    this.notify();
  }

  reanudarAutoConstruccion(asentamientoId: string): void {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
    this.state.asentamientos = this.state.asentamientos.map((a) =>
      a.id === asentamientoId ? { ...a, autoConstruccionPausada: false } : a
    );
    this.registrar(`${asentamiento.id}: auto-construcción reanudada.`);
    this.notify();
  }

  /** Calibra la reserva manual de un recurso (0-999, a petición del usuario, ver `Asentamiento.reservaManual`)
   * — tope que el camino AUTOMÁTICO de construcción no puede tocar (`engine/construction.ts`); la
   * construcción manual queda exenta a propósito. Requiere Tesorero asignado (mismo criterio que exigen las
   * políticas por cargo, ver `engine/politicas.ts`). */
  calibrarReservaManual(asentamientoId: string, recurso: RecursoTipo, valor: number): void {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
    if (!asentamiento.cargos.tesoreroId) {
      this.registrar(`Calibrar reserva rechazado: ${asentamiento.id} necesita un Tesorero asignado.`);
      this.notify();
      return;
    }
    const limpio = Math.max(0, Math.min(999, Math.round(valor)));
    this.state.asentamientos = this.state.asentamientos.map((a) =>
      a.id === asentamientoId ? { ...a, reservaManual: { ...a.reservaManual, [recurso]: limpio } } : a
    );
    this.notify();
  }

  /** Renombra un asentamiento (a petición del usuario) — solo `nombre` (presentación), `id` nunca cambia:
   * sigue siendo la llave interna estable que usan caravanas/acuerdos/caminos/zonas. Vacío = vuelve a
   * mostrar `id` (ver `etiquetaAsentamiento`, main.ts). */
  renombrarAsentamiento(asentamientoId: string, nombre: string): void {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
    const nombreLimpio = nombre.trim();
    this.state.asentamientos = this.state.asentamientos.map((a) =>
      a.id === asentamientoId ? { ...a, nombre: nombreLimpio || undefined } : a
    );
    this.registrar(`${asentamiento.id}: renombrado a "${nombreLimpio || asentamiento.id}".`);
    this.notify();
  }

  iniciarAsedio(atacanteId: string, defensorId: string, escuadronesCsv: string): void {
    try {
      const atacante = this.state.asentamientos.find((a) => a.id === atacanteId)!;
      const defensor = this.state.asentamientos.find((a) => a.id === defensorId)!;
      const resultado = iniciarAsedioEngine(atacante, defensor, idsNoVacios(escuadronesCsv), this.state.facciones, this.state.relaciones, this.state.tick);
      this.state.asentamientos = this.state.asentamientos.map((a) => {
        if (a.id === resultado.atacante.id) return resultado.atacante;
        if (a.id === resultado.defensor.id) return resultado.defensor;
        return a;
      });
      this.state.facciones = resultado.facciones;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Asedio rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  combateCampoAbierto(asentamientoAId: string, escuadronesACsv: string, asentamientoBId: string, escuadronesBCsv: string): void {
    try {
      const asentamientoA = this.state.asentamientos.find((a) => a.id === asentamientoAId)!;
      const asentamientoB = this.state.asentamientos.find((a) => a.id === asentamientoBId)!;
      const resultado = combateCampoAbiertoEngine(
        asentamientoA,
        idsNoVacios(escuadronesACsv),
        asentamientoB,
        idsNoVacios(escuadronesBCsv),
        this.state.facciones,
        this.state.relaciones,
        this.state.tick
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => {
        if (a.id === resultado.asentamientoA.id) return resultado.asentamientoA;
        if (a.id === resultado.asentamientoB.id) return resultado.asentamientoB;
        return a;
      });
      this.state.facciones = resultado.facciones;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Combate rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  interceptarCaravana(atacanteId: string, escuadronesCsv: string, caravanaId: string): void {
    try {
      const atacante = this.state.asentamientos.find((a) => a.id === atacanteId)!;
      const caravana = this.state.caravanas.find((c) => c.id === caravanaId)!;
      const resultado = interceptarCaravanaEngine(
        atacante,
        idsNoVacios(escuadronesCsv),
        caravana,
        this.state.tick,
        this.state.facciones,
        this.state.asentamientos
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
      this.state.facciones = resultado.facciones;
      if (resultado.caravanaCapturada) this.state.caravanas = this.state.caravanas.filter((c) => c.id !== caravana.id);
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Intercepción rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  /** Ataque de un jugador a un campamento de bandidos (Doc 1.9): si gana, se quita del mundo y se agenda el
   * plazo de reaparición (`CAMPAMENTOS_BANDIDOS.ticksRespawn`) — el spawn en sí lo evalúa `avanzarTick`. */
  atacarCampamentoBandidos(atacanteId: string, escuadronesCsv: string, campamentoId: string): void {
    try {
      const atacante = this.state.asentamientos.find((a) => a.id === atacanteId)!;
      const campamento = this.state.campamentosBandidos.find((c) => c.id === campamentoId)!;
      const resultado = atacarCampamentoBandidosEngine(atacante, idsNoVacios(escuadronesCsv), campamento, this.state.tick, this.state.facciones);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
      this.state.facciones = resultado.facciones;
      if (resultado.campamentoDestruido) {
        this.state.campamentosBandidos = this.state.campamentosBandidos.filter((c) => c.id !== campamento.id);
        this.state.bandidosProximoSpawnTick = this.state.tick + CAMPAMENTOS_BANDIDOS.ticksRespawn;
      }
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Ataque a campamento rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  avanzarTick(): void {
    this.state.tick += 1;
    const resultado = avanzarSimulacion(
      {
        asentamientos: this.state.asentamientos,
        facciones: this.state.facciones,
        caravanas: this.state.caravanas,
        acuerdos: this.state.acuerdos,
        ordenes: this.state.ordenes,
        relaciones: this.state.relaciones,
        titulos: this.state.titulos,
        caminos: this.state.caminos,
        campamentosBandidos: this.state.campamentosBandidos,
        bandidosProximoSpawnTick: this.state.bandidosProximoSpawnTick,
      },
      this.getMapa(),
      this.state.tick
    );
    this.state.asentamientos = resultado.asentamientos;
    this.state.facciones = resultado.facciones;
    this.state.caravanas = resultado.caravanas;
    this.state.acuerdos = resultado.acuerdos;
    this.state.ordenes = resultado.ordenes;
    this.state.relaciones = resultado.relaciones;
    this.state.titulos = resultado.titulos;
    this.state.caminos = resultado.caminos;
    this.state.campamentosBandidos = resultado.campamentosBandidos;
    this.state.bandidosProximoSpawnTick = resultado.bandidosProximoSpawnTick;
    for (const evento of resultado.eventos) this.registrar(evento);

    // SIMULACION_AUTO_COMERCIO (ver constants.ts): NPC de trueque solo-para-simulación, apagado por defecto.
    // No forma parte de `avanzarSimulacion` a propósito — el juego real sigue siendo 100% manual (Doc 3.2).
    if (SIMULACION_AUTO_COMERCIO.activo) {
      const trasAutoComercio = avanzarAutoComercioSimulado(
        {
          asentamientos: this.state.asentamientos,
          facciones: this.state.facciones,
          caravanas: this.state.caravanas,
          acuerdos: this.state.acuerdos,
          ordenes: this.state.ordenes,
          relaciones: this.state.relaciones,
          titulos: this.state.titulos,
          caminos: this.state.caminos,
          campamentosBandidos: this.state.campamentosBandidos,
          bandidosProximoSpawnTick: this.state.bandidosProximoSpawnTick,
        },
        this.getMapa(),
        this.state.tick
      );
      this.state.asentamientos = trasAutoComercio.asentamientos;
      this.state.caravanas = trasAutoComercio.caravanas;
      this.state.acuerdos = trasAutoComercio.acuerdos;
    }

    this.avanzarFaccionesNpc();

    this.notify();
  }

  /**
   * Turno del NPC de gobernanza (`app/npcGobernanza.ts`) para las Facciones cedidas, DESPUÉS del tick del
   * motor — mismo orden que usan los scripts de batch. El NPC no es parte de `avanzarSimulacion` a propósito:
   * decide con las funciones públicas del motor exactamente igual que este store cuando el jugador pulsa un
   * botón, y solo sobre `faccionesNpcIds`. Sin Facciones cedidas no se llama a nada.
   */
  private avanzarFaccionesNpc(): void {
    if (this.state.faccionesNpcIds.length === 0) return;

    const resultado = avanzarNpcGobernanza(
      {
        asentamientos: this.state.asentamientos,
        facciones: this.state.facciones,
        caravanas: this.state.caravanas,
        acuerdos: this.state.acuerdos,
        ordenes: this.state.ordenes,
        relaciones: this.state.relaciones,
        titulos: this.state.titulos,
        caminos: this.state.caminos,
        campamentosBandidos: this.state.campamentosBandidos,
        bandidosProximoSpawnTick: this.state.bandidosProximoSpawnTick,
      },
      this.getMapa(),
      this.state.tick,
      { faccionesIds: this.state.faccionesNpcIds, contadorInicial: this.contadorAcciones }
    );

    this.state.asentamientos = resultado.estado.asentamientos;
    this.state.facciones = resultado.estado.facciones;
    this.state.caravanas = resultado.estado.caravanas;
    this.state.acuerdos = resultado.estado.acuerdos;
    this.state.campamentosBandidos = resultado.estado.campamentosBandidos;
    this.state.bandidosProximoSpawnTick = resultado.estado.bandidosProximoSpawnTick;
    // Los ids que el motor generó dentro del NPC salieron de este mismo contador: se adelanta para que la
    // próxima acción manual del jugador no reutilice uno (ver `ConfigNpcGobernanza.contadorInicial`).
    this.contadorAcciones = resultado.contadorFinal;

    for (const evento of resultado.eventos) this.registrar(`[NPC] ${evento}`);
  }

  regenerarMundo(seed: number, region?: RegionId): void {
    this.historial = [];
    this.historialDesde = 0;
    this.state = {
      estadoMapa: crearEstadoMapa(),
      mapa: generarMapa({ ...MAPA_DEFAULT, seed, region }),
      asentamientos: [],
      facciones: [],
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
      campamentosBandidos: [],
      bandidosProximoSpawnTick: 0,
      faccionesNpcIds: [],
      tick: 0,
      log: [],
      historialJugadores: {},
    };
    this.registrar(`Mundo regenerado con seed ${seed}.`);
    this.notify();
  }

  /** Serializa la simulación completa (mundo, asentamientos, facciones, log, historial de jugadores...) a JSON. */
  exportarSimulacion(): string {
    const payload: SimulacionExportada = {
      version: 1,
      exportadoEn: new Date().toISOString(),
      tick: this.state.tick,
      worldgenVersion: this.state.mapa.version,
      // El archivo guarda de cada nodo la cantidad RESTANTE (no la inicial): es el único dato del mapa que
      // la partida modifica y que no se puede recuperar regenerando desde la seed.
      world: { config: this.state.mapa.config, recursos: this.getMapa().nodosConStock(), bosques: this.state.mapa.bosques },
      asentamientos: this.state.asentamientos,
      facciones: this.state.facciones,
      caravanas: this.state.caravanas,
      acuerdos: this.state.acuerdos,
      ordenes: this.state.ordenes,
      relaciones: this.state.relaciones,
      titulos: this.state.titulos,
      caminos: this.state.caminos,
      campamentosBandidos: this.state.campamentosBandidos,
      bandidosProximoSpawnTick: this.state.bandidosProximoSpawnTick,
      faccionesNpcIds: this.state.faccionesNpcIds,
      log: this.state.log,
      historialJugadores: this.state.historialJugadores,
    };
    return JSON.stringify(payload, null, 2);
  }

  /**
   * Heightmap (RAW 16-bit) + metadata (posiciones de nodos/bosques/ríos/chokepoints/asentamientos en metros)
   * del mapa actual, listos para Unity Terrain — ver `world/exportUnity.ts`. Puro respecto al estado: no
   * muta nada, solo deriva del `mapa` y los `asentamientos` en vivo.
   */
  exportarMapaUnity(opciones?: OpcionesExportUnity): ExportUnityResultado {
    return exportarParaUnityTerrain(this.state.mapa, this.state.asentamientos, opciones);
  }

  /**
   * Reemplaza la simulación completa por la contenida en `json` (formato de `exportarSimulacion`).
   * El archivo no lleva el mapa entero: se REGENERA desde `world.config` (determinista por seed, Doc 1.1)
   * y luego se le superponen los `recursos`/`bosques` exportados, que sí pueden venir ya modificados
   * (p. ej. yacimientos agotados). Lo único que no se puede superponer es el campo de fertilidad, que se
   * recrea — de ahí la comprobación de `worldgenVersion`.
   * Cualquier problema de formato se reporta en el log como una acción rechazada más — nunca se
   * propaga a la interfaz — porque a diferencia del resto de acciones, el origen del dato es un
   * archivo externo no confiable, no el propio estado ya validado de la simulación.
   */
  importarSimulacion(json: string): void {
    try {
      const payload = JSON.parse(json) as Partial<SimulacionExportada>;
      if (
        payload?.version !== 1 ||
        !payload.world?.config ||
        !Array.isArray(payload.world.recursos) ||
        !Array.isArray(payload.world.bosques) ||
        !Array.isArray(payload.asentamientos) ||
        !Array.isArray(payload.facciones)
      ) {
        throw new Error('el archivo no tiene el formato esperado de una simulación exportada.');
      }

      // Se rechaza en vez de cargarlo a medias: con otro algoritmo de generación, la misma seed produce un
      // campo de fertilidad distinto y las granjas del save rendirían otra cosa, sin ninguna señal visible.
      const versionMapa = payload.worldgenVersion ?? 1;
      if (versionMapa !== WORLDGEN_VERSION) {
        throw new Error(
          `el mundo se generó con la versión ${versionMapa} del generador y esta build usa la ${WORLDGEN_VERSION}; ` +
            'el mapa no se puede reconstruir igual a partir de la seed.'
        );
      }

      const mapaRegenerado = generarMapa(payload.world.config);
      // El mundo se recupera entero de la seed; del archivo solo se lee cuánto se había extraído ya de cada
      // yacimiento (diferencia entre lo que tenía al generarse y lo que el archivo dice que le quedaba).
      const extraido: Record<string, number> = {};
      for (const nodo of payload.world.recursos) {
        const original = mapaRegenerado.nodos.find((n) => n.id === nodo.id);
        if (!original) continue;
        const gastado = original.cantidadInicial - nodo.cantidad;
        if (gastado > 0) extraido[nodo.id] = gastado;
      }

      this.historial = [];
      this.historialDesde = payload.tick ?? 0;
      this.state = {
        mapa: mapaRegenerado,
        estadoMapa: { extraido, regeneraEnTick: {} },
        // Vista de Asentamiento: los saves anteriores traían los edificios en coords del mapa general y sin
        // `ambito` — se migran a coords locales del espacio plano (idempotente, ver `migrarEdificiosAEspacioLocal`).
        asentamientos: migrarEdificiosAEspacioLocal(payload.asentamientos),
        facciones: payload.facciones,
        caravanas: payload.caravanas ?? [],
        acuerdos: payload.acuerdos ?? [],
        ordenes: payload.ordenes ?? [],
        relaciones: payload.relaciones ?? [],
        titulos: payload.titulos ?? [],
        caminos: payload.caminos ?? [],
        campamentosBandidos: payload.campamentosBandidos ?? [],
        bandidosProximoSpawnTick: payload.bandidosProximoSpawnTick ?? 0,
        // Se filtra contra las Facciones que el archivo trae de verdad: un id huérfano dejaría una entrada
        // muerta que volvería a activarse sola si alguien creara después una Facción con ese mismo id.
        faccionesNpcIds: (payload.faccionesNpcIds ?? []).filter((id) => payload.facciones!.some((f) => f.id === id)),
        tick: payload.tick ?? 0,
        log: payload.log ?? [],
        historialJugadores: payload.historialJugadores ?? {},
      };
      this.registrar(`Simulación importada (tick ${this.state.tick}).`);
    } catch (err) {
      const razon = err instanceof Error ? err.message : 'formato desconocido';
      this.registrar(`Importación rechazada: ${razon}`);
    }
    this.notify();
  }

  /** Todos los valores de balance editables (ver `balanceConfig.ts`), agrupados como aparecen en constants.ts. */
  getBalance(): CampoBalance[] {
    return listarCamposBalance();
  }

  /** Cambia un valor de balance en caliente. Afecta de inmediato a toda acción/tick posterior del motor. */
  actualizarBalance(path: string, valor: number): void {
    if (actualizarCampoBalance(path, valor)) {
      this.registrar(`Balance actualizado: ${path} = ${valor}.`);
    } else {
      this.registrar(`Valor de balance rechazado: "${path}" no es un campo válido.`);
    }
    this.notify();
  }

  /** Vuelve todos los valores de balance a los de fábrica (los que tenían al cargar la página). */
  restaurarBalance(): void {
    restaurarBalancePorDefecto();
    this.registrar('Valores de balance restaurados a los de fábrica.');
    this.notify();
  }
}

/** Instancia única que usa la interfaz. */
export const gameStore = new GameStore();
