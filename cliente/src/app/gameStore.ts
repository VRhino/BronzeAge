// Capa de aplicación: única puerta de entrada a la lógica de simulación (motor + dominio) para el cliente de
// JUGADOR. Ninguna capa de interfaz debe importar nada de `../engine/*` ni de `../domain/types` para mutar
// estado o capturar errores — solo debe hablar con `gameStore` (acciones + getState + subscribe) y con tipos
// de datos para tipar lo que lee. Así, cambiar de interfaz (otra librería de UI, un cliente CLI, tests) no
// requiere tocar nada de esta carpeta ni de `engine/`.
//
// Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3 — migración de `main.ts`: `GameStore` deja de poseer
// una `GameSession` en memoria del navegador y pasa a ser un ADAPTADOR DE RED sobre `apiCliente.ts`. El estado
// vive en el proceso del SERVIDOR; aquí solo se cachea la última foto conocida (`estadoCache`), refrescada
// tras cada acción aceptada. Las ~30 acciones de esta clase son ahora `async` por eso — cada una implica al
// menos una ida y vuelta HTTP.
//
// Lo que se retira en esta migración (confirmado con el usuario: son operaciones de ADMINISTRACIÓN, no de
// jugador — `importarSimulacion`, el panel de Balance y el slider de línea de tiempo. No tienen dueño
// todavía: no hay separación real todavía entre "cliente de jugador" y "herramienta de administración" (una
// sola interfaz sirve a los dos propósitos por ahora, a propósito, según el usuario — esa separación es
// trabajo futuro), así que `regenerarMundo` SÍ se queda aquí en vez de en un panel aparte.
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
} from '@motor/domain/types';
import { EDIFICIO_CATALOGO, MANTENIMIENTO, NECESIDADES, NIVEL_FACCION, POLITICAS, POLITICA_CATALOGO, REJILLA_ASENTAMIENTO, SIMULACION, TROPAS_RECLUTABLES } from '@motor/constants';
import { crearMapa, type EstadoMapa, type Mapa } from '@motor/world/mapa';
import { exportarParaUnityTerrain, UNITY_EXPORT_DEFAULT, type ExportUnityResultado, type OpcionesExportUnity } from '@motor/world/exportUnity';

export { UNITY_EXPORT_DEFAULT };
import {
  produccionPorMinuto,
  manoObraInfo as calcularManoObraInfo,
  progresoNivelAsentamiento,
  capacidadViviendaPesants,
  capacidadViviendaArtesanos,
  edificiosPorTipoYEstado,
  cupoCaravanas as cupoCaravanasEngine,
  cooldownCaravanaRestante as cooldownCaravanaRestanteEngine,
  tieneMercadoActivo as tieneMercadoActivoEngine,
  nivelActualDe,
  ratioManoObraArtesanos,
  type ProduccionItem,
  type ManoObraInfo,
  type ProgresoNivelAsentamiento,
} from '@motor/engine/asentamientoQuery';
export type { ProduccionItem, ManoObraInfo } from '@motor/engine/asentamientoQuery';
import { encontrarCapital, calcularCostoMantenimiento, calcularNivelAsentamiento } from '@motor/engine/mantenimiento';
import { consumoComidaPoblacion } from '@motor/engine/population';
import { slotsDisponibles } from '@motor/engine/politicas';
// --- Motor: SOLO consultas derivadas ---
// Los COMANDOS ya no se importan aquí: viven en `session/comandos/` y se ejecutan en el SERVIDOR, a través de
// `apiCliente.ejecutarComando` (por nombre, ver `session/comandos/registro.ts`). Lo que queda son las
// funciones que alimentan las consultas de solo lectura de la interfaz.
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '@motor/engine/zones';
import { calcularCapFundacion, calcularCupoNivel, capacidadCasas } from '@motor/engine/faccion';
import { computeLigas, type LigaInfo } from '@motor/engine/liga';
import { consumoRacionTropas } from '@motor/engine/tropas';
import {
  estadoMejoraEdificio as estadoMejoraEdificioEngine,
  factorLineaProduccion,
  type EstadoMejoraEdificio,
} from '@motor/engine/construction';
export type { EstadoMejoraEdificio } from '@motor/engine/construction';
import {
  celdaMinimaDeEdificio,
  edificiosInternos,
  redDeCalles,
  segmentosDeRed,
  tamanoDeEdificio,
  type SegmentoTrazado,
} from '@motor/engine/trazado';
import { poderEscuadron } from '@motor/engine/combate';

// --- Capa de partida: vive en el servidor, se habla por HTTP ---
import type { EstadoAdmin, EventoLogAdmin } from '@motor/session/estado';
import type { EventoDominio } from '@motor/domain/eventos';
import { isoDeInstante, proyectarLog } from '@motor/session/estado';
import type { ParamsDe, TipoComando } from '@motor/session/comandos/registro';
import type { MapaGenerado } from '@motor/worldgen';
import { ApiError, consultarEstado, crearOResumirPartida, ejecutarComando, obtenerMapa } from './apiCliente';

/** Entrada de log en texto — Fase D: `momento` (ISO de mundo) en vez de `tick`. Mismo shape que
 * `EventoLogAdmin` del motor (`proyectarLog` la produce). */
export type EventoLog = EventoLogAdmin;

const EPOCA_MUNDO_MS = new Date(SIMULACION.epocaInicial).getTime();

/** Tiempo de mundo legible ("día D · HH:MM") desde un `Instante` (ms) o su forma ISO — para la interfaz.
 * Fase D: los campos temporales del estado son `Instante`/`Duracion` de mundo, no ordinales de tick. */
export function fmtTiempoMundo(t: number | string): string {
  const ms = typeof t === 'string' ? Date.parse(t) : t;
  const min = Math.max(0, Math.round((ms - EPOCA_MUNDO_MS) / 60_000));
  const dia = Math.floor(min / 1440);
  return `día ${dia} · ${String(Math.floor((min % 1440) / 60)).padStart(2, '0')}:${String(min % 60).padStart(2, '0')}`;
}

/**
 * Lo que la interfaz consume: el estado que devuelve el servidor MÁS el log en texto, que ya no es estado
 * persistido sino una proyección de `eventosDominio` (ver `proyectarLog`, `session/estado.ts`). Se deriva
 * aquí, en el cliente, a partir de los eventos que el servidor ya manda — así el mismo hecho no viaja dos
 * veces por la red ni se guarda dos veces en el snapshot.
 *
 * Sin `mapa` (Fase C11): el servidor solo manda `mapaId` en el estado; el `MapaGenerado` real se pide UNA vez
 * por `mapaId` (nunca cambia durante la partida) y se cachea aparte — ver `mapaGeneradoCache` más abajo.
 */
export type GameState = EstadoAdmin & { log: EventoLog[] };

/**
 * Nodo tal como viaja en el ARCHIVO: con `cantidad` = lo que le queda. En memoria el nodo es inmutable y
 * lleva `cantidadInicial`, y lo consumido vive aparte (`EstadoMapa`) — el archivo aplana las dos cosas en
 * un solo número porque es lo que ya guardaban las partidas existentes y no hay motivo para romperlas.
 */
export type NodoExportado = Omit<NodoRecurso, 'cantidadInicial'> & { cantidad: number };

/** Formato de archivo para exportar una simulación completa (ver `exportarSimulacion`). `version` se sube
 * cada vez que la forma de estos datos cambia de forma incompatible. */
export interface SimulacionExportada {
  version: 2;
  exportadoEn: string;
  tick: number;
  worldgenVersion?: number;
  world: { config: WorldConfig; recursos: NodoExportado[]; bosques: ZonaBosque[] };
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  caminos?: CaminoComercial[];
  campamentosBandidos?: CampamentoBandido[];
  bandidosProximoSpawnEn?: number;
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
  duracionPoliticaMinutos: POLITICAS.duracionMinutosPorDefecto,
  slotsPorCargoBase: POLITICAS.slotsPorCargo,
  nivelFaccionPorSlotExtraGobernador: POLITICAS.nivelFaccionPorSlotExtraGobernador,
  maximoEdificiosEnCola: NECESIDADES.maximoEnCola,
  maximoEnConstruccionSimultanea: NECESIDADES.maximoEnConstruccionSimultanea,
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
        tiempoConstruccionMinutos: number;
        requisitoNivelAsentamientoConstruccion?: number;
        nivelFaccionMinimo?: number;
      };
      return {
        tipo,
        costo: def.costo,
        tiempoConstruccionMinutos: def.tiempoConstruccionMinutos,
        requisitoNivelAsentamiento: def.requisitoNivelAsentamientoConstruccion ?? 0,
        requisitoNivelFaccion: def.nivelFaccionMinimo ?? 0,
      };
    }),
  tamanoCeldaAsentamiento: REJILLA_ASENTAMIENTO.tamanoCelda,
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
 * Único punto de contacto de la interfaz de JUGADOR con la partida. Expone acciones de alto nivel (una por
 * intención de usuario) — la interfaz nunca necesita conocer los tipos de comando ni hablar HTTP por su
 * cuenta.
 */
export class GameStore {
  private gameId: string;
  private estadoCache: EstadoAdmin;
  private listeners = new Set<Listener>();
  /** Mensajes de rechazo/error SOLO EN MEMORIA del cliente (nunca persistidos): a diferencia de la versión
   * local anterior, el servidor no tiene manera de que el cliente le pida "anota este rechazo en el log" sin
   * fingir que fue un comando — se muestran igual en la consola de la interfaz, pero no sobreviven a un
   * refresco de página ni cuentan para la versión de concurrencia. Diferencia de comportamiento menor y
   * deliberada (ver Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3). Con tope para no crecer sin límite
   * en una sesión de navegador muy larga. */
  private logEfimero: EventoLog[] = [];
  /** Log en texto derivado de `eventosDominio`, cacheado contra el array del que sale (ver `getState`). */
  private logCache: { sobre: EventoDominio[]; log: EventoLog[] } | null = null;
  /** Fachada `Mapa` del estado en vivo, cacheada por referencia — se invalida sola en cuanto `estadoCache`
   * cambia (cada acción/tick trae un `estadoMapa` distinto). Un solo hueco, no un mapa: sin historial de fotos
   * que cachear ya no hace falta más que eso (ver `GameSession.getMapa`, mismo patrón). */
  private mapaCache: { sobre: EstadoMapa; mapa: Mapa } | null = null;
  /**
   * El `MapaGenerado` real, cacheado aparte del estado (Fase C11): el servidor solo manda `mapaId` en cada
   * respuesta —el mapa en sí no cambia NUNCA durante la partida (125 KB medidos, idénticos byte a byte en
   * todo el tick 0-200, Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md §6.4)—, así que se pide
   * UNA vez por `mapaId` y se guarda aquí. `sincronizarMapa` es lo único que lo toca tras el constructor: solo
   * vuelve a pedirlo si `mapaId` cambia (`regenerarMundo`, la única operación que reemplaza la seed).
   * Nunca `null` tras el constructor — invariante que sostiene `getMapa()` sin comprobarlo en cada llamada.
   */
  private mapaGeneradoCache: { id: string; mapa: MapaGenerado };
  /** Última fusión de zonas por facción calculada, con la firma de los asentamientos de los que salió — ver
   * `getZonasFusionadas`. Artefacto de render, no estado de partida: se puede tirar en cualquier momento. */
  private zonasFusionadasCache: { clave: string; valor: ZonaFaccion[] } | null = null;

  private constructor(gameId: string, estadoInicial: EstadoAdmin, mapaInicial: MapaGenerado) {
    this.gameId = gameId;
    this.estadoCache = estadoInicial;
    this.mapaGeneradoCache = { id: estadoInicial.mapaId, mapa: mapaInicial };
  }

  /** Pide el mapa real si `mapaId` cambió desde la última sincronización (Fase C11) — en la inmensa mayoría
   * de las llamadas es un no-op síncrono, porque el mapa no cambia entre comandos. Solo dispara una petición
   * de verdad tras `regenerarMundo`, que es la única operación que reemplaza la seed de la partida. */
  private async sincronizarMapa(mapaId: string): Promise<void> {
    if (this.mapaGeneradoCache.id === mapaId) return;
    this.mapaGeneradoCache = { id: mapaId, mapa: await obtenerMapa(this.gameId, mapaId) };
  }

  /** Conecta con una partida ya existente, o la crea si el `gameId` no tiene ninguna todavía — NO destructivo
   * (ver `apiCliente.crearOResumirPartida`). Descartar una partida en curso y crear otra de cero SÍ lo es —
   * ver `regenerarMundo`, más abajo.
   *
   * Un 409 aquí NO es un fallo de conexión: `POST /partidas` lo devuelve cuando el `gameId` ya está abierto en
   * este proceso (`server/api.ts`) — exactamente lo normal al recargar la página con una partida ya en curso
   * (otra pestaña, u otra carga anterior, ya la abrió). El jugador no necesita "crearla" de nuevo, solo
   * conectarse a la que ya hay — se ignora el 409 y se sigue directo a pedir su estado (y su mapa, Fase C11:
   * primera y única vez que se pide sin pasar por `sincronizarMapa`, porque el constructor aún no existe). */
  static async crear(gameId: string, seed: number): Promise<GameStore> {
    try {
      await crearOResumirPartida(gameId, seed);
    } catch (err) {
      if (!(err instanceof ApiError && err.status === 409)) throw err;
    }
    const estado = await consultarEstado(gameId);
    const mapa = await obtenerMapa(gameId, estado.mapaId);
    return new GameStore(gameId, estado, mapa);
  }

  private get state(): Readonly<EstadoAdmin> {
    return this.estadoCache;
  }

  /**
   * Estado en vivo con el log ya derivado. La proyección se cachea contra el array de eventos del que sale
   * (misma estrategia que `mapaCache`): la interfaz llama a `getState()` en cada notificación, y `main.ts`
   * repinta el log entero, así que rehacer el `map` sobre todo el historial en cada render sería gratuito
   * solo al principio de la partida.
   */
  getState(): Readonly<GameState> {
    if (this.logCache?.sobre !== this.estadoCache.eventosDominio) {
      this.logCache = { sobre: this.estadoCache.eventosDominio, log: proyectarLog(this.estadoCache.eventosDominio) };
    }
    const log = this.logEfimero.length === 0 ? this.logCache.log : [...this.logEfimero, ...this.logCache.log];
    return { ...this.estadoCache, log };
  }

  /** Fachada de consulta del mapa (índices + consultas espaciales) del estado en vivo. El `MapaGenerado` sale
   * de `mapaGeneradoCache` (Fase C11), no de `estado` — ya no viaja dentro de `EstadoAdmin`. */
  getMapa(estado: Readonly<EstadoAdmin> = this.state): Mapa {
    if (this.mapaCache?.sobre === estado.estadoMapa) return this.mapaCache.mapa;
    const mapa = crearMapa(this.mapaGeneradoCache.mapa, estado.estadoMapa);
    this.mapaCache = { sobre: estado.estadoMapa, mapa };
    return mapa;
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) listener();
  }

  private registrarRechazoEfimero(mensaje: string): void {
    this.logEfimero = [{ momento: isoDeInstante(this.estadoCache.instante), mensaje }, ...this.logEfimero].slice(0, 50);
  }

  private mensajeDeError(err: unknown): string {
    return err instanceof ApiError ? err.message : 'Error desconocido al hablar con el servidor.';
  }

  /**
   * Manda un comando por nombre al servidor y refresca la interfaz. Aceptado: se pide el estado completo de
   * vuelta (el servidor no lo manda solo — ver `apiCliente.ts` — para no cargar cada respuesta con ~2 MB de
   * estado cuando la mayoría de comandos solo cambian un puñado de campos). Rechazado, o si la petición HTTP
   * en sí falla: se anota en el log efímero, igual que hacía antes el rechazo local — la interfaz nunca
   * necesita distinguir un rechazo de dominio de un fallo de red.
   */
  private async despachar<T extends TipoComando>(tipo: T, params: ParamsDe<T>, etiquetaRechazo: string): Promise<void> {
    try {
      const respuesta = await ejecutarComando(this.gameId, tipo, params);
      if (respuesta.resultado.ok) {
        this.estadoCache = await consultarEstado(this.gameId);
        await this.sincronizarMapa(this.estadoCache.mapaId);
      } else {
        this.registrarRechazoEfimero(`${etiquetaRechazo}: ${respuesta.resultado.codigoError ?? 'desconocido'}`);
      }
    } catch (err) {
      this.registrarRechazoEfimero(this.mensajeDeError(err));
    }
    this.notify();
  }

  // --- Derivados de solo lectura (evitan que la interfaz importe funciones del motor) ---
  // Todos aceptan datos opcionales para poder calcularse tanto sobre el estado en vivo como sobre uno pasado
  // explícitamente — por defecto usan el estado en vivo.

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
   * asentamiento.
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
   * (`engine/trazado.ts`), nada persistido.
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
   * §5, ver `CUPO_NIVEL_ASENTAMIENTO`) contra cuántos de sus asentamientos YA ocupan cada uno.
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

  /** Precio de referencia calculado en el SERVIDOR (doc 9, 2026-08-26): necesita el almacén de todos los
   * asentamientos del mundo, no solo los propios — entrada privilegiada que este cliente no debe recalcular
   * aunque hoy tenga los datos para hacerlo (ve la partida entera por ser administración). Cacheado en el
   * servidor con TTL de un minuto real; aquí solo se lee. */
  precioReferencia(recurso: string): number {
    return this.state.preciosReferencia[recurso] ?? 0;
  }

  /** Progreso de nivel de asentamiento (modelo de gates, Doc 4.5): población actual vs. requerida y qué
   * edificios de la lista todavía faltan por tener activos, para el siguiente nivel. */
  nivelAsentamientoInfo(asentamiento: Asentamiento): ProgresoNivelAsentamiento {
    return progresoNivelAsentamiento(asentamiento);
  }

  /**
   * Por qué un asentamiento que YA cumple los gates de nivel (población + edificios) no sube: cupo de nivel
   * ocupado por su Facción. Devuelve `null` cuando el cupo NO es el motivo.
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
    minutosParaFinGracia: number;
    items: { recurso: string; costoPorMinuto: number; disponible: number; cubierto: boolean }[];
  } {
    const minutosDesdeFundacion = (this.state.instante - asentamiento.fundadoEn) / 60_000;
    const enGracia = minutosDesdeFundacion < MANTENIMIENTO.graciaMinutos;
    const capital = encontrarCapital(asentamiento.faccionId, this.state.asentamientos);
    const costo = calcularCostoMantenimiento(asentamiento, capital);
    const items = Object.entries(costo).map(([recurso, cantidad]) => {
      const disponible = asentamiento.almacen[recurso]?.cantidad ?? 0;
      return { recurso, costoPorMinuto: cantidad ?? 0, disponible, cubierto: disponible >= (cantidad ?? 0) };
    });
    const costoTrigo = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento);
    const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
    items.push({ recurso: 'trigo', costoPorMinuto: costoTrigo, disponible: trigoDisponible, cubierto: trigoDisponible >= costoTrigo });
    return { enGracia, minutosParaFinGracia: Math.max(0, Math.round(MANTENIMIENTO.graciaMinutos - minutosDesdeFundacion)), items };
  }

  /** Slots de política disponibles para `cargo` según el nivel de Facción (el Gobernador escala con el nivel). */
  slotsPoliticaDisponibles(cargo: CargoTipo, nivelFaccion: number): number {
    return slotsDisponibles(cargo, nivelFaccion);
  }

  /** Producción por tick de cada edificio activo de extracción/producción primaria, agrupada por tipo. */
  produccionInfo(asentamiento: Asentamiento): ProduccionItem[] {
    const zona = this.getZonas().find((z) => z.asentamientoId === asentamiento.id);
    return produccionPorMinuto(asentamiento, this.getMapa(), zona?.poligono ?? []);
  }

  /** Producción y consumo estimados del edificio individual para el tooltip de la vista urbana. */
  edificioEconomiaInfo(asentamiento: Asentamiento, edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'estado' | 'posicion' | 'ambito'>): {
    produccion: { recurso: string; cantidadPorMinuto: number }[];
    consumo: { recurso: string; cantidadPorMinuto: number }[];
    consumoTotal: { recurso: string; cantidadPorMinuto: number }[];
  } {
    const activosDelTipo = asentamiento.edificios.filter((e) => e.tipo === edificio.tipo && e.estado === 'activo').length;
    if ((edificio.estado !== undefined && edificio.estado !== 'activo') || activosDelTipo === 0 || edificio.tipo === 'centroUrbano') return { produccion: [], consumo: [], consumoTotal: [] };

    const produccionAgregada = this.produccionInfo(asentamiento).filter((item) => item.tipo === edificio.tipo);
    const produccion = produccionAgregada.map((item) => ({ recurso: item.recurso, cantidadPorMinuto: item.cantidadPorMinuto / activosDelTipo }));
    const definicion = EDIFICIO_CATALOGO[edificio.tipo] as { niveles?: Record<number, { recetas?: { produce: string; produccionBase: number; consumePorUnidad: Record<string, number> }[] }> };
    const recetas = definicion.niveles?.[edificio.nivelInterno ?? 1]?.recetas ?? [];
    const consumo = recetas.flatMap((receta) => {
      const salida = produccion.find((item) => item.recurso === receta.produce)?.cantidadPorMinuto ?? 0;
      return Object.entries(receta.consumePorUnidad).map(([recurso, cantidad]) => ({ recurso, cantidadPorMinuto: cantidad * salida }));
    });
    const ratioArtesano = ratioManoObraArtesanos(asentamiento);
    const consumoTotal = recetas.flatMap((receta) => {
      const salidaTotal = receta.produccionBase * ratioArtesano * factorLineaProduccion(edificio as Edificio, receta, asentamiento);
      return Object.entries(receta.consumePorUnidad).map(([recurso, cantidad]) => ({ recurso, cantidadPorMinuto: cantidad * salidaTotal }));
    });
    return { produccion, consumo, consumoTotal };
  }

  /** Resumen militar de solo lectura para la interfaz del asentamiento. */
  poderMilitarInfo(asentamiento: Asentamiento): { soldados: number; poder: number } {
    return {
      soldados: asentamiento.escuadrones.reduce((total, escuadron) => total + escuadron.cantidad, 0),
      poder: asentamiento.escuadrones.reduce((total, escuadron) => total + poderEscuadron(escuadron, this.state.instante), 0),
    };
  }

  /** Demanda de mano de obra agregada (pesants) frente a lo que piden los edificios productores activos. */
  manoObraInfo(asentamiento: Asentamiento): ManoObraInfo {
    return calcularManoObraInfo(asentamiento);
  }

  /** Población actual vs. límite por clase (Doc 4.1): Pesants/Artesanos limitados por cupos de Vivienda,
   * Nobleza por la `capacidadNobles` del Palacio — solo lectura, no altera el motor. */
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

  /**
   * Funda un asentamiento. El fundador es siempre EL ACTOR que ejecuta el comando (Fase C2/C3): el backend
   * ya no acepta una lista de cofundadores — ver `session/comandos/fundarAsentamiento.ts` para el porqué.
   */
  async fundarAsentamiento(faccionId: string, posicion: { x: number; y: number }): Promise<void> {
    await this.despachar('fundarAsentamiento', { faccionId, posicion }, 'Fundación rechazada');
  }

  async lanzarCaravanaFundacion(origenAsentamientoId: string, destino: { x: number; y: number }, numJugadores: number): Promise<void> {
    await this.despachar('lanzarCaravanaFundacion', { origenAsentamientoId, destino, numJugadores }, 'Caravana de Fundación rechazada');
  }

  async desarmarCaravanaFundacion(caravanaId: string): Promise<void> {
    await this.despachar('desarmarCaravanaFundacion', { caravanaId }, 'No se pudo desarmar la caravana');
  }

  async crearFaccion(nombre: string): Promise<void> {
    await this.despachar('crearFaccion', { nombre }, 'Facción rechazada');
  }

  async alternarFaccionNpc(faccionId: string, activo: boolean): Promise<void> {
    await this.despachar('alternarFaccionNpc', { faccionId, activo }, 'Cesión al NPC rechazada');
  }

  /** ¿Esta Facción la juega el NPC? (`GameSessionState.faccionesNpcIds`, para la pestaña Facción). */
  esFaccionNpc(faccionId: string): boolean {
    return this.state.faccionesNpcIds.includes(faccionId);
  }

  async asignarRey(faccionId: string, jugadorId: string): Promise<void> {
    await this.despachar('asignarRey', { faccionId, jugadorId }, 'Rey rechazado');
  }

  async asignarEmbajador(faccionId: string, jugadorId: string): Promise<void> {
    await this.despachar('asignarEmbajador', { faccionId, jugadorId }, 'Embajador rechazado');
  }

  async asignarCargoLocal(asentamientoId: string, cargo: CargoTipo, jugadorId: string): Promise<void> {
    await this.despachar('asignarCargoLocal', { asentamientoId, cargo, jugadorId }, 'Cargo rechazado');
  }

  async comprarCasa(asentamientoId: string, jugadorId: string): Promise<void> {
    await this.despachar('comprarCasa', { asentamientoId, jugadorId }, 'Compra de casa rechazada');
  }

  async activarPolitica(asentamientoId: string, cargo: CargoTipo, politicaId: string): Promise<void> {
    await this.despachar('activarPolitica', { asentamientoId, cargo, politicaId }, 'Política rechazada');
  }

  async proponerRelacion(
    tipo: 'vasallaje' | 'alianza',
    faccionAId: string,
    faccionBId: string,
    tributoRecurso: string,
    tributoCantidad: number
  ): Promise<void> {
    await this.despachar(
      'proponerRelacion',
      { tipo, faccionAId, faccionBId, tributoRecurso: tributoRecurso as RecursoTipo, tributoCantidad },
      'Relación rechazada'
    );
  }

  async romperRelacion(relacionId: string, iniciadorFaccionId: string): Promise<void> {
    await this.despachar('romperRelacion', { relacionId, iniciadorFaccionId }, 'Ruptura rechazada');
  }

  async rebelionVasallo(relacionId: string): Promise<void> {
    await this.despachar('rebelionVasallo', { relacionId }, 'Rebelión rechazada');
  }

  async anexionar(faccionAId: string, faccionBId: string): Promise<void> {
    await this.despachar('anexionar', { faccionAId, faccionBId }, 'Anexión rechazada');
  }

  async fusionar(faccionAId: string, faccionBId: string, nuevoNombre: string, nuevoReyId: string): Promise<void> {
    await this.despachar('fusionar', { faccionAId, faccionBId, nuevoNombre, nuevoReyId }, 'Fusión rechazada');
  }

  async proponerTrueque(
    asentamientoAId: string,
    recursoA: string,
    cantidadA: number,
    asentamientoBId: string,
    recursoB: string,
    cantidadB: number
  ): Promise<void> {
    await this.despachar(
      'proponerTrueque',
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

  async colocarOrdenMercado(asentamientoId: string, tipo: 'compra' | 'venta', recurso: string, cantidad: number, precio: number | undefined): Promise<void> {
    await this.despachar(
      'colocarOrdenMercado',
      { asentamientoId, tipo, recurso: recurso as RecursoTipo, cantidad, precio },
      'Orden rechazada'
    );
  }

  async crearCaravana(asentamientoId: string): Promise<void> {
    await this.despachar('crearCaravana', { asentamientoId }, 'Caravana rechazada');
  }

  /** Solo lectura, para la pestaña Guerra/Acciones: cupo de flota, cuántas caravanas propias tiene el
   * asentamiento y en qué estado, y el cooldown de creación. */
  caravanasInfo(asentamiento: Asentamiento): {
    mercadoActivo: boolean;
    cupo: number;
    disponibles: number;
    enTransito: number;
    retornando: number;
    cooldownCreacionRestanteMin: number;
  } {
    const propias = this.state.caravanas.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === asentamiento.id);
    return {
      mercadoActivo: tieneMercadoActivoEngine(asentamiento),
      cupo: cupoCaravanasEngine(asentamiento),
      disponibles: propias.filter((c) => c.estado === 'disponible').length,
      enTransito: propias.filter((c) => c.estado === 'en_transito').length,
      retornando: propias.filter((c) => c.estado === 'retornando').length,
      // El motor devuelve una `Duracion` en ms; la interfaz la muestra en minutos de mundo.
      cooldownCreacionRestanteMin: Math.round(cooldownCaravanaRestanteEngine(asentamiento, this.state.instante) / 60_000),
    };
  }

  async reclutarTropa(asentamientoId: string, jugadorId: string, tropaId: string, origen: 'pesants' | 'artesanos'): Promise<void> {
    await this.despachar('reclutarTropa', { asentamientoId, jugadorId, tropaId, origen }, 'Reclutamiento rechazado');
  }

  /** Residentes de un asentamiento (Doc 2.5): fundadores + casas compradas, deduplicado. */
  jugadoresDeAsentamiento(asentamientoId: string): string[] {
    const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId);
    if (!asentamiento) return [];
    return [...new Set([...asentamiento.jugadoresFundadoresIds, ...asentamiento.casasCompradas])];
  }

  async anadirEdificioManualmente(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', tipo: EdificioTipo): Promise<void> {
    await this.despachar('anadirEdificioManualmente', { asentamientoId, cargo, tipo }, 'Añadir a la cola rechazado');
  }

  async quitarDeCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string): Promise<void> {
    await this.despachar('quitarDeCola', { asentamientoId, cargo, edificioId }, 'Quitar de la cola rechazado');
  }

  async moverEnCola(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string, direccion: 'arriba' | 'abajo'): Promise<void> {
    await this.despachar('moverEnCola', { asentamientoId, cargo, edificioId, direccion }, 'Reordenar cola rechazado');
  }

  async mejorarEdificioAhora(asentamientoId: string, cargo: 'gobernador' | 'maestroObras', edificioId: string): Promise<void> {
    await this.despachar('mejorarEdificioAhora', { asentamientoId, cargo, edificioId }, 'Mejorar edificio rechazado');
  }

  /** Estado de la próxima mejora de un edificio concreto (nivel, costo, si hay fondos hoy) — `null` si el
   * edificio no existe o no tiene mejora posible. */
  infoMejoraEdificio(asentamiento: Asentamiento, edificioId: string): EstadoMejoraEdificio | null {
    const edificio = asentamiento.edificios.find((e) => e.id === edificioId);
    if (!edificio) return null;
    const capital = encontrarCapital(asentamiento.faccionId, this.state.asentamientos);
    return estadoMejoraEdificioEngine(asentamiento, edificio, capital);
  }

  async pausarAutoConstruccion(asentamientoId: string): Promise<void> {
    await this.despachar('alternarAutoConstruccion', { asentamientoId, pausada: true }, 'Pausar auto-construcción rechazado');
  }

  async reanudarAutoConstruccion(asentamientoId: string): Promise<void> {
    await this.despachar('alternarAutoConstruccion', { asentamientoId, pausada: false }, 'Reanudar auto-construcción rechazado');
  }

  async calibrarReservaManual(asentamientoId: string, recurso: RecursoTipo, valor: number): Promise<void> {
    await this.despachar('calibrarReservaManual', { asentamientoId, recurso, valor }, 'Calibrar reserva rechazado');
  }

  async renombrarAsentamiento(asentamientoId: string, nombre: string): Promise<void> {
    await this.despachar('renombrarAsentamiento', { asentamientoId, nombre }, 'Renombrar rechazado');
  }

  async iniciarAsedio(atacanteId: string, defensorId: string, escuadronesCsv: string): Promise<void> {
    await this.despachar('iniciarAsedio', { atacanteId, defensorId, escuadronIds: idsNoVacios(escuadronesCsv) }, 'Asedio rechazado');
  }

  async combateCampoAbierto(asentamientoAId: string, escuadronesACsv: string, asentamientoBId: string, escuadronesBCsv: string): Promise<void> {
    await this.despachar(
      'combateCampoAbierto',
      {
        asentamientoAId,
        escuadronIdsA: idsNoVacios(escuadronesACsv),
        asentamientoBId,
        escuadronIdsB: idsNoVacios(escuadronesBCsv),
      },
      'Combate rechazado'
    );
  }

  async interceptarCaravana(atacanteId: string, escuadronesCsv: string, caravanaId: string): Promise<void> {
    await this.despachar('interceptarCaravana', { atacanteId, escuadronIds: idsNoVacios(escuadronesCsv), caravanaId }, 'Intercepción rechazada');
  }

  async atacarCampamentoBandidos(atacanteId: string, escuadronesCsv: string, campamentoId: string): Promise<void> {
    await this.despachar(
      'atacarCampamentoBandidos',
      { atacanteId, escuadronIds: idsNoVacios(escuadronesCsv), campamentoId },
      'Ataque a campamento rechazado'
    );
  }

  /** Re-pide el estado completo al servidor. El mundo avanza SOLO en el servidor (reloj de mundo, Fase D /
   * D5: un tick por minuto real, con catch-up tras reinicio) — la interfaz no lo empuja, solo vuelve a leer.
   * Lo llama el botón "Refrescar" y el auto-refresco de `main.ts`. */
  async refrescar(): Promise<void> {
    try {
      this.estadoCache = await consultarEstado(this.gameId);
      await this.sincronizarMapa(this.estadoCache.mapaId);
    } catch (err) {
      this.registrarRechazoEfimero(this.mensajeDeError(err));
    }
    this.notify();
  }

  /**
   * Descarta la partida actual y crea otra desde cero con la seed indicada — operación DESTRUCTIVA
   * (`crearOResumirPartida(..., forzar: true)`, ver `apiCliente.ts`). Pierde todo lo que hubiera en la
   * partida en curso; la interfaz debe confirmar con el usuario ANTES de llamar a esto, no lo hace `GameStore`.
   */
  async regenerarMundo(seed: number, region?: RegionId): Promise<void> {
    try {
      await crearOResumirPartida(this.gameId, seed, region, true);
      this.estadoCache = await consultarEstado(this.gameId);
      // El único punto donde `sincronizarMapa` de verdad pide algo: regenerar SIEMPRE reemplaza la seed, así
      // que `mapaId` cambia siempre — a diferencia de `despachar`/`refrescar`, donde suele ser un no-op.
      await this.sincronizarMapa(this.estadoCache.mapaId);
      this.mapaCache = null;
      this.zonasFusionadasCache = null;
      this.logEfimero = [];
    } catch (err) {
      this.registrarRechazoEfimero(this.mensajeDeError(err));
    }
    this.notify();
  }

  /** Serializa la simulación completa (mundo, asentamientos, facciones, log, historial de jugadores...) a
   * JSON — para descargar. Sigue siendo del jugador (guardar/exportar SU partida); a diferencia de
   * `importarSimulacion` (retirada, es administración: reemplazar la partida completa del servidor). */
  exportarSimulacion(): string {
    const mapa = this.mapaGeneradoCache.mapa;
    const payload: SimulacionExportada = {
      version: 2,
      exportadoEn: new Date().toISOString(),
      tick: this.state.tick,
      worldgenVersion: mapa.version,
      world: { config: mapa.config, recursos: this.getMapa().nodosConStock(), bosques: mapa.bosques },
      asentamientos: this.state.asentamientos,
      facciones: this.state.facciones,
      caravanas: this.state.caravanas,
      acuerdos: this.state.acuerdos,
      ordenes: this.state.ordenes,
      relaciones: this.state.relaciones,
      titulos: this.state.titulos,
      caminos: this.state.caminos,
      campamentosBandidos: this.state.campamentosBandidos,
      bandidosProximoSpawnEn: this.state.bandidosProximoSpawnEn,
      faccionesNpcIds: this.state.faccionesNpcIds,
      // El formato de archivo v2 guarda el log en texto (es anterior a `eventosDominio`): se proyecta al
      // exportar en vez de arrastrarlo en el estado.
      log: proyectarLog(this.state.eventosDominio),
      historialJugadores: this.state.historialJugadores,
    };
    return JSON.stringify(payload, null, 2);
  }

  /**
   * Heightmap (RAW 16-bit) + metadata (posiciones de nodos/bosques/ríos/asentamientos en metros)
   * del mapa actual, listos para Unity Terrain. Puro respecto al estado: no muta nada.
   */
  exportarMapaUnity(opciones?: OpcionesExportUnity): ExportUnityResultado {
    return exportarParaUnityTerrain(this.mapaGeneradoCache.mapa, this.state.asentamientos, opciones);
  }
}

/** Conecta con `gameId` (lo crea si no existe todavía — no destructivo) y devuelve un `GameStore` listo para
 * usar. `main.ts` la llama una vez al arrancar, antes de montar el resto de la interfaz. */
export function crearGameStore(gameId = 'local', seed = 1): Promise<GameStore> {
  return GameStore.crear(gameId, seed);
}
