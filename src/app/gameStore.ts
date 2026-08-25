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
  Edificio,
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
  ZonaFaccion,
  ZonaInfluencia,
} from '../domain/types';
import { EDIFICIO_CATALOGO, MANTENIMIENTO, NECESIDADES, NIVEL_FACCION, POLITICAS, POLITICA_CATALOGO, REJILLA_ASENTAMIENTO, TROPAS_RECLUTABLES } from '../constants';
import { generarMapa, WORLDGEN_VERSION, type MapaGenerado } from '../worldgen';
import { crearMapa, type EstadoMapa, type Mapa } from '../world/mapa';
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
  ratioManoObraArtesanos,
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
// --- Motor: SOLO consultas derivadas ---
// Los COMANDOS ya no se importan aquí: viven en `session/comandos/` y este store los invoca a través de
// `GameSession` (Docs/Arquitectura/7_Diseno_GameSession.md §6). Lo que queda son las funciones que alimentan
// las consultas de solo lectura de la interfaz — pendientes de triaje en el doc 8.
import { evaluarViabilidadFundacion, type ViabilidadFundacion } from '../engine/settlement';
export type { ViabilidadFundacion } from '../engine/settlement';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../engine/zones';
import { controladorDeChokepoint } from '../engine/chokepoints';
import { calcularPrecioReferencia } from '../engine/market';
import { calcularCapFundacion, calcularCupoNivel, capacidadCasas } from '../engine/faccion';
import { computeLigas, type LigaInfo } from '../engine/liga';
import { consumoRacionTropas } from '../engine/tropas';
import {
  estadoMejoraEdificio as estadoMejoraEdificioEngine,
  factorLineaProduccion,
  type EstadoMejoraEdificio,
} from '../engine/construction';
export type { EstadoMejoraEdificio } from '../engine/construction';
import {
  celdaMinimaDeEdificio,
  edificiosInternos,
  redDeCalles,
  segmentosDeRed,
  tamanoDeEdificio,
  type SegmentoTrazado,
} from '../engine/trazado';
import { poderEscuadron } from '../engine/combate';

// --- Capa de partida: la lógica de cada comando vive aquí, no en este archivo ---
import { GameSession, type GameSessionState } from '../session/gameSession';
import { ACTOR_LOCAL, type ManejadorComando, type ResultadoComando } from '../session/comandos/tipos';
import { fundarAsentamiento as fundarAsentamientoCmd } from '../session/comandos/fundarAsentamiento';
import { crearFaccion as crearFaccionCmd } from '../session/comandos/crearFaccion';
import { alternarFaccionNpc as alternarFaccionNpcCmd } from '../session/comandos/alternarFaccionNpc';
import {
  activarPolitica as activarPoliticaCmd,
  asignarCargoLocal as asignarCargoLocalCmd,
  asignarEmbajador as asignarEmbajadorCmd,
  asignarRey as asignarReyCmd,
  comprarCasa as comprarCasaCmd,
} from '../session/comandos/cargos';
import {
  anexionar as anexionarCmd,
  fusionar as fusionarCmd,
  proponerRelacion as proponerRelacionCmd,
  rebelionVasallo as rebelionVasalloCmd,
  romperRelacion as romperRelacionCmd,
} from '../session/comandos/diplomacia';
import {
  colocarOrdenMercado as colocarOrdenMercadoCmd,
  crearCaravana as crearCaravanaCmd,
  proponerTrueque as proponerTruequeCmd,
} from '../session/comandos/comercio';
import {
  atacarCampamentoBandidos as atacarCampamentoBandidosCmd,
  combateCampoAbierto as combateCampoAbiertoCmd,
  interceptarCaravana as interceptarCaravanaCmd,
  iniciarAsedio as iniciarAsedioCmd,
  reclutarTropa as reclutarTropaCmd,
} from '../session/comandos/militar';
import {
  alternarAutoConstruccion as alternarAutoConstruccionCmd,
  anadirEdificioManualmente as anadirEdificioManualmenteCmd,
  calibrarReservaManual as calibrarReservaManualCmd,
  mejorarEdificioAhora as mejorarEdificioAhoraCmd,
  moverEnCola as moverEnColaCmd,
  quitarDeCola as quitarDeColaCmd,
  renombrarAsentamiento as renombrarAsentamientoCmd,
} from '../session/comandos/construccion';
import {
  desarmarCaravanaFundacion as desarmarCaravanaFundacionCmd,
  lanzarCaravanaFundacion as lanzarCaravanaFundacionCmd,
} from '../session/comandos/expansion';

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
   * Facciones que juega el NPC de gobernanza (`session/npcGobernanza.ts`) en vez del jugador humano — se
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

/** Formato de archivo para exportar/importar una simulación completa (ver `exportarSimulacion`/`importarSimulacion`).
 * `version` se sube cada vez que la forma de estos datos cambia de forma incompatible — sin migración interna
 * para digerir versiones viejas, un archivo con otra `version` se rechaza de entrada (a petición del usuario:
 * el proyecto está en desarrollo continuo, las partidas viejas se abandonan, no se migran). */
export interface SimulacionExportada {
  version: 2;
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
  // `crearPuestosDeMercado`, engine/construction.ts). Mismo trato para las anclas de Etapa 3
  // (`plaza`/`plazaDeArmas`/`patioDeGremios`, marcadores gratis que nacen por la regla de semilla de grupo,
  // engine/trazado.ts) y `tallerCarpinteria` (nace al completarse la Carpintería, `crearTalleresDeCarpinteria`).
  // `pozo`/`parque` (Etapa 4, punto 4): mismo trato, son las otras dos opciones del sorteo de ancla residencial.
  catalogoEdificios: (Object.keys(EDIFICIO_CATALOGO) as EdificioTipo[])
    .filter(
      (tipo) =>
        tipo !== 'centroUrbano' &&
        tipo !== 'puestoMercado' &&
        tipo !== 'plaza' &&
        tipo !== 'plazaDeArmas' &&
        tipo !== 'patioDeGremios' &&
        tipo !== 'tallerCarpinteria' &&
        tipo !== 'pozo' &&
        tipo !== 'parque'
    )
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
  /**
   * La partida vive aquí, no en este objeto (Docs/Arquitectura/7_Diseno_GameSession.md §6, paso 3).
   * `GameStore` ya no posee estado de simulación: es el ADAPTADOR de navegador sobre `GameSession` —
   * suscripciones, historial de depuración y traducción de `ResultadoComando` a entradas de log. Es también
   * la pieza que en la Fase B5 se sustituye por un cliente de API remota sin tocar la interfaz.
   */
  private session: GameSession;
  private listeners = new Set<Listener>();
  /** Una foto completa del estado al final de cada tick (índice = número de tick) — alimenta el slider de línea de tiempo. */
  private historial: GameSessionState[] = [];
  /** Tick más antiguo con foto disponible. 0 en una partida normal; el tick importado tras un `importarSimulacion` (no hay fotos de ticks previos a ese punto). */
  private historialDesde = 0;
  /**
   * Fachadas `Mapa` de las FOTOS del historial, indexadas por el estado del que salen (ver `getMapa`). La
   * clave es el estado y no el mundo generado porque todas las fotos comparten el mismo `MapaGenerado` por
   * referencia — indexar por él devolvería la fachada de una foto cualquiera al pedir la de otra.
   */
  private mapasPorEstado = new WeakMap<EstadoMapa, Mapa>();
  /** Última fusión de zonas por facción calculada, con la firma de los asentamientos de los que salió — ver
   * `getZonasFusionadas`. Artefacto de render, no estado de partida: se puede tirar en cualquier momento. */
  private zonasFusionadasCache: { clave: string; valor: ZonaFaccion[] } | null = null;

  /**
   * Estado de la partida, leído siempre de `GameSession`. Es un getter y no un campo a propósito: así las
   * consultas derivadas de esta clase (`getZonas`, `produccionInfo`, `manoObraInfo`…) siguen leyendo
   * `this.state.X` exactamente igual que antes, sin que ninguna tenga que cambiar. `GameSessionState` es un
   * superconjunto de lo que era `GameState` (añade `gameId`, `version` y `eventosDominio`), así que la
   * interfaz tampoco nota la diferencia.
   */
  private get state(): Readonly<GameSessionState> {
    return this.session.getState();
  }

  /** Momento de simulación para una operación lanzada desde la interfaz. El reloj lo pone esta capa —
   * `GameSession` no lo lee nunca (ver `ContextoComando`). */
  private ahora(): string {
    return new Date().toISOString();
  }

  constructor() {
    this.session = GameSession.crear('local', { seed: 1 });
    this.session.registrarEventoAdministrativo(this.ahora(), 'Mundo generado. Selecciona una facción y haz clic en el mapa para fundar.');
    this.notify();
  }

  getState(): Readonly<GameState> {
    return this.state;
  }

  /**
   * Fachada de consulta del mapa (índices + consultas espaciales) para un estado dado; por defecto, el
   * estado en vivo, que resuelve la propia `GameSession` — es la dueña de la partida y ya lleva su fachada.
   * Aquí solo se construyen las de las FOTOS del historial, cacheadas por el estado del que salen para que
   * cada una se calcule como mucho una vez, y solo si alguien llega a dibujarla.
   */
  getMapa(estado: Readonly<GameState> = this.state): Mapa {
    if (estado === this.state) return this.session.getMapa();
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
  private clonarEstadoActual(): GameSessionState {
    return {
      gameId: this.state.gameId,
      version: this.state.version,
      eventosDominio: structuredClone(this.state.eventosDominio),
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

  /**
   * Ejecuta un comando de `GameSession` y refresca la interfaz. Los comandos ya registran sus propios
   * eventos en el log de la partida (ver `exito()` en `session/comandos/tipos.ts`), así que aquí solo queda
   * traducir un RECHAZO a la entrada de log que la consola mostraba antes — es la única parte de la
   * traducción `ResultadoComando` → texto que sigue viviendo en el adaptador.
   */
  private despachar<P, R>(
    manejador: ManejadorComando<P, R>,
    params: P,
    etiquetaRechazo: string
  ): ResultadoComando<R> {
    const resultado = this.session.ejecutar(manejador, params, { momento: this.ahora(), actor: ACTOR_LOCAL });
    if (!resultado.ok) {
      this.session.registrarEventoAdministrativo(this.ahora(), `${etiquetaRechazo}: ${resultado.codigoError ?? 'desconocido'}`);
    }
    this.notify();
    return resultado;
  }

  // --- Derivados de solo lectura (evitan que la interfaz importe funciones del motor) ---
  // Todos aceptan datos opcionales para poder calcularse tanto sobre el estado en vivo como
  // sobre una foto del historial (vista de línea de tiempo) — por defecto usan el estado en vivo.

  getZonas(asentamientos: Asentamiento[] = this.state.asentamientos): ZonaInfluencia[] {
    return computeTodasLasZonas(asentamientos);
  }

  /**
   * Las zonas de influencia agrupadas y FUSIONADAS por facción, para pintar el territorio como una sola
   * silueta en vez de un disco por asentamiento (ver `computeZonasFusionadasPorFaccion`). Solo lectura, solo
   * para dibujar: la interfaz nunca debe fusionar polígonos por su cuenta (acoplamiento 0 con el motor).
   *
   * Cacheado con una ranura: `render()` se dispara en cada `mousemove` sobre el lienzo, y la fusión (rejilla
   * de muestreo + marching squares, ver `unirFormas`) es cara de sobra para no repetirla mientras nada haya
   * cambiado. La clave es la única entrada de la que depende el resultado — posición, radio y facción de cada
   * asentamiento — así que basta con que crezca un radio para que se recalcule, y alternar entre el presente
   * y una foto del historial también la invalida (una ranura, no un mapa: se alterna poco y así no se
   * acumulan fotos antiguas).
   */
  getZonasFusionadas(
    zonas: ZonaInfluencia[] = this.getZonas(),
    asentamientos: Asentamiento[] = this.state.asentamientos
  ): ZonaFaccion[] {
    const clave = asentamientos
      .map((a) => `${a.id}|${a.faccionId}|${a.posicion.x},${a.posicion.y}|${a.radioPotencial}`)
      .join(';');
    if (this.zonasFusionadasCache?.clave === clave) return this.zonasFusionadasCache.valor;
    const valor = computeZonasFusionadasPorFaccion(zonas, asentamientos);
    this.zonasFusionadasCache = { clave, valor };
    return valor;
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

  /** Producción y consumo estimados del edificio individual para el tooltip de la vista urbana. */
  edificioEconomiaInfo(asentamiento: Asentamiento, edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'estado' | 'posicion' | 'ambito'>): {
    produccion: { recurso: string; cantidadPorTick: number }[];
    consumo: { recurso: string; cantidadPorTick: number }[];
    consumoTotal: { recurso: string; cantidadPorTick: number }[];
  } {
    const activosDelTipo = asentamiento.edificios.filter((e) => e.tipo === edificio.tipo && e.estado === 'activo').length;
    if ((edificio.estado !== undefined && edificio.estado !== 'activo') || activosDelTipo === 0 || edificio.tipo === 'centroUrbano') return { produccion: [], consumo: [], consumoTotal: [] };

    const produccionAgregada = this.produccionInfo(asentamiento).filter((item) => item.tipo === edificio.tipo);
    const produccion = produccionAgregada.map((item) => ({ recurso: item.recurso, cantidadPorTick: item.cantidadPorTick / activosDelTipo }));
    const definicion = EDIFICIO_CATALOGO[edificio.tipo] as { niveles?: Record<number, { recetas?: { produce: string; produccionBase: number; consumePorUnidad: Record<string, number> }[] }> };
    const recetas = definicion.niveles?.[edificio.nivelInterno ?? 1]?.recetas ?? [];
    const consumo = recetas.flatMap((receta) => {
      const salida = produccion.find((item) => item.recurso === receta.produce)?.cantidadPorTick ?? 0;
      return Object.entries(receta.consumePorUnidad).map(([recurso, cantidad]) => ({ recurso, cantidadPorTick: cantidad * salida }));
    });
    const ratioArtesano = ratioManoObraArtesanos(asentamiento);
    const consumoTotal = recetas.flatMap((receta) => {
      const salidaTotal = receta.produccionBase * ratioArtesano * factorLineaProduccion(edificio as Edificio, receta, asentamiento);
      return Object.entries(receta.consumePorUnidad).map(([recurso, cantidad]) => ({ recurso, cantidadPorTick: cantidad * salidaTotal }));
    });
    return { produccion, consumo, consumoTotal };
  }

  /** Resumen militar de solo lectura para la interfaz del asentamiento. */
  poderMilitarInfo(asentamiento: Asentamiento): { soldados: number; poder: number } {
    return {
      soldados: asentamiento.escuadrones.reduce((total, escuadron) => total + escuadron.cantidad, 0),
      poder: asentamiento.escuadrones.reduce((total, escuadron) => total + poderEscuadron(escuadron, this.state.tick), 0),
    };
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
    this.despachar(fundarAsentamientoCmd, { faccionId, posicion, numJugadores }, 'Fundación rechazada');
  }

  lanzarCaravanaFundacion(origenAsentamientoId: string, destino: { x: number; y: number }, numJugadores: number): void {
    this.despachar(lanzarCaravanaFundacionCmd, { origenAsentamientoId, destino, numJugadores }, 'Caravana de Fundación rechazada');
  }

  desarmarCaravanaFundacion(caravanaId: string): void {
    this.despachar(desarmarCaravanaFundacionCmd, { caravanaId }, 'No se pudo desarmar la caravana');
  }

  crearFaccion(nombre: string): void {
    this.despachar(crearFaccionCmd, { nombre }, 'Facción rechazada');
  }

  alternarFaccionNpc(faccionId: string, activo: boolean): void {
    this.despachar(alternarFaccionNpcCmd, { faccionId, activo }, 'Cesión al NPC rechazada');
  }

  /** ¿Esta Facción la juega el NPC? (`GameSessionState.faccionesNpcIds`, para la pestaña Facción). */
  esFaccionNpc(faccionId: string): boolean {
    return this.state.faccionesNpcIds.includes(faccionId);
  }

  asignarRey(faccionId: string, jugadorId: string): void {
    this.despachar(asignarReyCmd, { faccionId, jugadorId }, 'Rey rechazado');
  }

  asignarEmbajador(faccionId: string, jugadorId: string): void {
    this.despachar(asignarEmbajadorCmd, { faccionId, jugadorId }, 'Embajador rechazado');
  }

  asignarCargoLocal(asentamientoId: string, cargo: CargoTipo, jugadorId: string): void {
    this.despachar(asignarCargoLocalCmd, { asentamientoId, cargo, jugadorId }, 'Cargo rechazado');
  }

  comprarCasa(asentamientoId: string, jugadorId: string): void {
    this.despachar(comprarCasaCmd, { asentamientoId, jugadorId }, 'Compra de casa rechazada');
  }

  activarPolitica(asentamientoId: string, cargo: CargoTipo, politicaId: string): void {
    this.despachar(activarPoliticaCmd, { asentamientoId, cargo, politicaId }, 'Política rechazada');
  }

  proponerRelacion(
    tipo: 'vasallaje' | 'alianza',
    faccionAId: string,
    faccionBId: string,
    tributoRecurso: string,
    tributoCantidad: number
  ): void {
    this.despachar(
      proponerRelacionCmd,
      { tipo, faccionAId, faccionBId, tributoRecurso: tributoRecurso as RecursoTipo, tributoCantidad },
      'Relación rechazada'
    );
  }

  romperRelacion(relacionId: string, iniciadorFaccionId: string): void {
    this.despachar(romperRelacionCmd, { relacionId, iniciadorFaccionId }, 'Ruptura rechazada');
  }

  rebelionVasallo(relacionId: string): void {
    this.despachar(rebelionVasalloCmd, { relacionId }, 'Rebelión rechazada');
  }

  anexionar(faccionAId: string, faccionBId: string): void {
    this.despachar(anexionarCmd, { faccionAId, faccionBId }, 'Anexión rechazada');
  }

  fusionar(faccionAId: string, faccionBId: string, nuevoNombre: string, nuevoReyId: string): void {
    this.despachar(fusionarCmd, { faccionAId, faccionBId, nuevoNombre, nuevoReyId }, 'Fusión rechazada');
  }

  proponerTrueque(
    asentamientoAId: string,
    recursoA: string,
    cantidadA: number,
    asentamientoBId: string,
    recursoB: string,
    cantidadB: number
  ): void {
    this.despachar(
      proponerTruequeCmd,
      {
        asentamientoAId,
        recursoA: recursoA as RecursoTipo,
        cantidadA,
        asentamientoBId,
        recursoB: recursoB as RecursoTipo,
        cantidadB,
      },
      'Trueque rechazado'
    );
  }

  colocarOrdenMercado(asentamientoId: string, tipo: 'compra' | 'venta', recurso: string, cantidad: number, precio: number | undefined): void {
    this.despachar(
      colocarOrdenMercadoCmd,
      { asentamientoId, tipo, recurso: recurso as RecursoTipo, cantidad, precio },
      'Orden rechazada'
    );
  }

  crearCaravana(asentamientoId: string): void {
    this.despachar(crearCaravanaCmd, { asentamientoId }, 'Caravana rechazada');
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

  reclutarTropa(asentamientoId: string, jugadorId: string, tropaId: string, origen: 'pesants' | 'artesanos'): void {
    this.despachar(reclutarTropaCmd, { asentamientoId, jugadorId, tropaId, origen }, 'Reclutamiento rechazado');
  }

  /** Residentes de un asentamiento (Doc 2.5): fundadores + casas compradas, deduplicado — cualquiera de ellos
   * puede reclutar o reponer SU escuadrón ahí (ver `reclutarTropa`). Usado por la UI para el selector de Jugador
   * en Reclutamiento y para agrupar los chips de Combate por jugador. */
  jugadoresDeAsentamiento(asentamientoId: string): string[] {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId);
    if (!asentamiento) return [];
    return [...new Set([...asentamiento.jugadoresFundadoresIds, ...asentamiento.casasCompradas])];
  }

  anadirEdificioManualmente(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', tipo: EdificioTipo): void {
    this.despachar(anadirEdificioManualmenteCmd, { asentamientoId, cargo, tipo }, 'Añadir a la cola rechazado');
  }

  quitarDeCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string): void {
    this.despachar(quitarDeColaCmd, { asentamientoId, cargo, edificioId }, 'Quitar de la cola rechazado');
  }

  moverEnCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string, direccion: 'arriba' | 'abajo'): void {
    this.despachar(moverEnColaCmd, { asentamientoId, cargo, edificioId, direccion }, 'Reordenar cola rechazado');
  }

  mejorarEdificioAhora(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string): void {
    this.despachar(mejorarEdificioAhoraCmd, { asentamientoId, cargo, edificioId }, 'Mejorar edificio rechazado');
  }

  /** Estado de la próxima mejora de un edificio concreto (nivel, costo, si hay fondos hoy) — `null` si el
   * edificio no existe o no tiene mejora posible. Usado por la UI para mostrar el botón "Mejorar ahora". */
  infoMejoraEdificio(asentamiento: Asentamiento, edificioId: string): EstadoMejoraEdificio | null {
    const edificio = asentamiento.edificios.find((e) => e.id === edificioId);
    if (!edificio) return null;
    const capital = encontrarCapital(asentamiento.faccionId, this.state.asentamientos);
    return estadoMejoraEdificioEngine(asentamiento, edificio, capital);
  }

  pausarAutoConstruccion(asentamientoId: string): void {
    this.despachar(alternarAutoConstruccionCmd, { asentamientoId, pausada: true }, 'Pausar auto-construcción rechazado');
  }

  reanudarAutoConstruccion(asentamientoId: string): void {
    this.despachar(alternarAutoConstruccionCmd, { asentamientoId, pausada: false }, 'Reanudar auto-construcción rechazado');
  }

  calibrarReservaManual(asentamientoId: string, recurso: RecursoTipo, valor: number): void {
    this.despachar(calibrarReservaManualCmd, { asentamientoId, recurso, valor }, 'Calibrar reserva rechazado');
  }

  renombrarAsentamiento(asentamientoId: string, nombre: string): void {
    this.despachar(renombrarAsentamientoCmd, { asentamientoId, nombre }, 'Renombrar rechazado');
  }

  iniciarAsedio(atacanteId: string, defensorId: string, escuadronesCsv: string): void {
    this.despachar(iniciarAsedioCmd, { atacanteId, defensorId, escuadronIds: idsNoVacios(escuadronesCsv) }, 'Asedio rechazado');
  }

  combateCampoAbierto(asentamientoAId: string, escuadronesACsv: string, asentamientoBId: string, escuadronesBCsv: string): void {
    this.despachar(
      combateCampoAbiertoCmd,
      {
        asentamientoAId,
        escuadronIdsA: idsNoVacios(escuadronesACsv),
        asentamientoBId,
        escuadronIdsB: idsNoVacios(escuadronesBCsv),
      },
      'Combate rechazado'
    );
  }

  interceptarCaravana(atacanteId: string, escuadronesCsv: string, caravanaId: string): void {
    this.despachar(interceptarCaravanaCmd, { atacanteId, escuadronIds: idsNoVacios(escuadronesCsv), caravanaId }, 'Intercepción rechazada');
  }

  atacarCampamentoBandidos(atacanteId: string, escuadronesCsv: string, campamentoId: string): void {
    this.despachar(
      atacarCampamentoBandidosCmd,
      { atacanteId, escuadronIds: idsNoVacios(escuadronesCsv), campamentoId },
      'Ataque a campamento rechazado'
    );
  }

  /**
   * Un tick completo tal como lo pide la interfaz: el tick del motor, después el trueque automático de
   * simulación (apagado por defecto) y por último el turno del NPC de gobernanza. El ORDEN es el mismo que
   * tenía este store antes de delegar; ahora las tres son operaciones de sistema de `GameSession`.
   */
  avanzarTick(): void {
    const momento = this.ahora();
    this.session.avanzarTick(momento);
    this.session.avanzarAutoComercio(momento);
    this.session.avanzarFaccionesNpc(momento);
    this.notify();
  }

  /**
   * Descarta la partida actual y crea otra desde cero con la seed indicada.
   *
   * NO es un comando de partida (Docs/Arquitectura/7_Diseno_GameSession.md §2.ter): se lanza ANTES de
   * empezar a jugar, y dispararlo con una partida en curso pierde todo lo que hubiera en ella. Por eso
   * sustituye la `GameSession` entera en vez de mutar nada. En el backend será una operación destructiva de
   * administración —descartar una partida y crear otra—, sujeta a rol técnico y confirmación explícita, no
   * un botón más de la consola.
   */
  regenerarMundo(seed: number, region?: RegionId): void {
    this.historial = [];
    this.historialDesde = 0;
    this.mapasPorEstado = new WeakMap();
    this.zonasFusionadasCache = null;
    this.session = GameSession.crear('local', { seed, region });
    this.session.registrarEventoAdministrativo(this.ahora(), `Mundo regenerado con seed ${seed}.`);
    this.notify();
  }

  /** Serializa la simulación completa (mundo, asentamientos, facciones, log, historial de jugadores...) a JSON. */
  exportarSimulacion(): string {
    const payload: SimulacionExportada = {
      version: 2,
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
      // Rechazo explícito por versión, separado del resto de la validación de formato (a petición del
      // usuario: desarrollo continuo, sin migración de partidas guardadas — un archivo de otra versión se
      // rechaza con un mensaje propio, no se intenta adaptar).
      if (payload?.version !== 2) {
        throw new Error(
          `esta partida es de una versión de guardado anterior (${payload?.version ?? 'desconocida'}) y ya no es compatible — hay que empezar una partida nueva.`
        );
      }
      if (
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
      this.mapasPorEstado = new WeakMap();
      this.zonasFusionadasCache = null;
      // Reconstrucción de partida, no comando: se sustituye la `GameSession` entera (doc 7 §2.ter).
      // `version` arranca en 0 y `eventosDominio` vacío — el formato de archivo v2 es anterior a ambos y no
      // los guarda; recuperarlos es parte de la persistencia real de Fase B3.
      this.session = GameSession.importar({
        siguienteId: 0,
        worldgenVersion: WORLDGEN_VERSION,
        state: {
          gameId: 'local',
          version: 0,
          eventosDominio: [],
          mapa: mapaRegenerado,
          estadoMapa: { extraido, regeneraEnTick: {} },
          asentamientos: payload.asentamientos,
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
        },
      });
      this.session.registrarEventoAdministrativo(this.ahora(), `Simulación importada (tick ${this.state.tick}).`);
    } catch (err) {
      const razon = err instanceof Error ? err.message : 'formato desconocido';
      this.session.registrarEventoAdministrativo(this.ahora(), `Importación rechazada: ${razon}`);
    }
    this.notify();
  }

  /** Todos los valores de balance editables (ver `balanceConfig.ts`), agrupados como aparecen en constants.ts. */
  getBalance(): CampoBalance[] {
    return listarCamposBalance();
  }

  /**
   * Cambia un valor de balance en caliente. Afecta de inmediato a toda acción/tick posterior del motor.
   *
   * NO es un comando de partida (doc 7 §2.ter): `balanceConfig.ts` muta los objetos de `constants.ts` EN EL
   * SITIO, así que esto toca configuración del PROCESO, no el estado de esta partida. Por eso no pasa por
   * `GameSession.ejecutar` y solo deja rastro en el log administrativo. Fase C lo convierte en configuración
   * versionada por partida con auditoría real (doc 2, punto 8).
   */
  actualizarBalance(path: string, valor: number): void {
    const mensaje = actualizarCampoBalance(path, valor)
      ? `Balance actualizado: ${path} = ${valor}.`
      : `Valor de balance rechazado: "${path}" no es un campo válido.`;
    this.session.registrarEventoAdministrativo(this.ahora(), mensaje);
    this.notify();
  }

  /** Vuelve todos los valores de balance a los de fábrica. Misma salvedad que `actualizarBalance`: es
   * configuración del proceso, no de la partida. */
  restaurarBalance(): void {
    restaurarBalancePorDefecto();
    this.session.registrarEventoAdministrativo(this.ahora(), 'Valores de balance restaurados a los de fábrica.');
    this.notify();
  }
}

/** Instancia única que usa la interfaz. */
export const gameStore = new GameStore();
