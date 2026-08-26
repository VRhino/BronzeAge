// Estado de una partida y sus transformaciones PURAS (Docs/Arquitectura/7_Diseno_GameSession.md).
//
// Vive aparte de `gameSession.ts` por dos razones: evita el ciclo de importación que habría entre la clase y
// sus manejadores de comando (los manejadores necesitan el tipo del estado, la clase necesita los
// manejadores), y deja claro que el estado es DATOS — no hay ningún método que lo mute en el sitio, solo
// funciones que devuelven un estado nuevo. Esa disciplina es lo que permite al runner de Fase B3 aplicar un
// comando, intentar persistirlo y DESCARTAR el estado nuevo si la escritura falla.
import type {
  AcuerdoTrueque,
  Asentamiento,
  CaminoComercial,
  CampamentoBandido,
  Caravana,
  Faccion,
  OrdenMercado,
  RelacionPolitica,
  Titulo,
} from '../domain/types';
import type { EventoDominio } from '../domain/eventos';
import type { MapaGenerado } from '../worldgen';
import type { EstadoMapa } from '../world/mapa';
import type { EstadoSimulacion } from '../engine/simulation';

/**
 * Entrada de log en texto. Ya no se guarda un log global en el estado (se deriva de `eventosDominio` con
 * `proyectarLog`); este tipo sigue siendo el de esa proyección y el de `historialJugadores`.
 *
 * Administración: nunca viaja a un jugador tal cual (Docs/Arquitectura/7_Diseno_GameSession.md §7.1 — el log
 * global narra lo que pasa en TODO el mundo, así que incluirlo en una proyección de jugador sería una fuga de
 * información de facciones rivales).
 */
export interface EventoLogAdmin {
  tick: number;
  mensaje: string;
}

/**
 * Estado completo y serializable de una partida. Sin nada de presentación: la fachada `Mapa` se deriva de
 * `mapa`/`estadoMapa` y nunca se guarda, igual que en `GameStore`.
 */
export interface GameSessionState {
  gameId: string;
  mapa: MapaGenerado;
  estadoMapa: EstadoMapa;
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  caminos: CaminoComercial[];
  campamentosBandidos: CampamentoBandido[];
  bandidosProximoSpawnTick: number;
  /** Facciones que gobierna el NPC en vez de un jugador humano. Vive en la partida y no en el runner
   * (doc 7 §7.2): cambia el resultado del tick, así que un reinicio con otra configuración divergiría de lo
   * que el snapshot dice haber pasado. En partida real no cambia en caliente una vez elegida. */
  faccionesNpcIds: string[];
  tick: number;
  /** Sube en cada mutación aceptada. Un comando rechazado NUNCA la incrementa — base del control de
   * concurrencia optimista de Fase B3. */
  version: number;
  historialJugadores: Record<string, EventoLogAdmin[]>;
  /**
   * Todo lo que ha ocurrido en la partida, en forma estructurada: la ÚNICA representación de los hechos que
   * se guarda. El log en texto que muestra la consola se deriva de aquí con `proyectarLog()` — antes se
   * persistía además un `log: EventoLogAdmin[]` en paralelo, que era el mismo hecho dos veces en el estado y
   * dos veces en cada snapshot, y una de las dos copias (texto plano) no se podía filtrar por audiencia.
   *
   * Administración: NO viaja a un jugador tal cual (doc 7 §7.1 — narra lo que pasa en TODO el mundo, así que
   * mandarlo entero sería una fuga de información de facciones rivales). Las proyecciones por audiencia de
   * Fase C filtran sobre `codigo`/`payload`, que es justamente para lo que existe.
   */
  eventosDominio: EventoDominio[];
}

/** Proyecta el estado de partida al subconjunto que consume el motor. El motor no conoce `gameId`, `version`,
 * logs ni `faccionesNpcIds` — y no debe. */
export function estadoSimulacionDe(estado: GameSessionState): EstadoSimulacion {
  return {
    asentamientos: estado.asentamientos,
    facciones: estado.facciones,
    caravanas: estado.caravanas,
    acuerdos: estado.acuerdos,
    ordenes: estado.ordenes,
    relaciones: estado.relaciones,
    titulos: estado.titulos,
    caminos: estado.caminos,
    campamentosBandidos: estado.campamentosBandidos,
    bandidosProximoSpawnTick: estado.bandidosProximoSpawnTick,
  };
}

/** Vuelca en el estado el resultado de un tick del motor. No toca `version` ni los logs: de eso se encarga
 * `exito()` (`comandos/tipos.ts`), que es el único sitio donde se incrementa la versión. */
export function conResultadoDeSimulacion(estado: GameSessionState, simulacion: EstadoSimulacion): GameSessionState {
  return {
    ...estado,
    asentamientos: simulacion.asentamientos,
    facciones: simulacion.facciones,
    caravanas: simulacion.caravanas,
    acuerdos: simulacion.acuerdos,
    ordenes: simulacion.ordenes,
    relaciones: simulacion.relaciones,
    titulos: simulacion.titulos,
    caminos: simulacion.caminos,
    campamentosBandidos: simulacion.campamentosBandidos,
    bandidosProximoSpawnTick: simulacion.bandidosProximoSpawnTick,
  };
}

/**
 * Deriva el log administrativo en texto a partir de los eventos estructurados. Es PRESENTACIÓN, no estado: no
 * se guarda ni viaja como tal — el servidor manda `eventosDominio` y quien pinta la consola llama a esto.
 *
 * Un evento atribuido a un asentamiento se prefija con su id, que es lo que hacía `avanzarSimulacion` cuando
 * el tick llevaba su propio array de texto (`eventos`, retirado por redundante). Al pasar el log a derivarse
 * de `eventosDominio` —cuyos `mensaje` NO llevan el prefijo, porque el id va aparte en `asentamientoId`— la
 * consola había empezado a mostrar "granja completado." sin decir de qué asentamiento. Reconstruirlo aquí lo
 * deja en un solo sitio, en vez de en cada emisor.
 */
export function proyectarLog(eventos: readonly EventoDominio[]): EventoLogAdmin[] {
  return eventos.map((e) => ({
    tick: e.tick,
    mensaje: e.asentamientoId ? `${e.asentamientoId}: ${e.mensaje}` : e.mensaje,
  }));
}

/**
 * Hecho administrativo que NO es un comando de partida (hoy solo los cambios de balance, ver
 * `GameSession.registrarEventoAdministrativo`). Mantiene `mensaje` como texto libre a propósito: es un rastro
 * temporal, y lo sustituye la auditoría real de Fase C (actor, fecha, versión previa y nueva).
 */
export function eventoAdministrativo(momento: string, tick: number, mensaje: string): EventoDominio {
  return { codigo: 'administrativo', mensaje, momento, tick };
}

/**
 * Identidad del mapa de una partida (Fase C11, doc 9: "el mapa deja de ser estado, es un asset"). Función
 * PURA de `MapaGenerado.config` (`seed` + `region` — `ancho`/`alto` no varían nunca hoy, siempre
 * `MAPA_DEFAULT`, así que no hace falta incluirlos) y del algoritmo de generación (`version`,
 * `WORLDGEN_VERSION`). Determinista: el mismo mapa siempre produce el mismo id, y solo cambia si el mapa
 * cambia de verdad (`regenerarMundo`/`forzar`, o una subida de `WORLDGEN_VERSION`).
 *
 * No es un hash: es legible a propósito, para poder leer un id en un log o una URL y saber de qué mapa se
 * trata sin decodificar nada. Nunca se guarda — se deriva cada vez que hace falta, igual que `proyectarLog`.
 */
export function idDeMapa(mapa: GameSessionState['mapa']): string {
  return `v${mapa.version}-s${mapa.config.seed}${mapa.config.region ? `-${mapa.config.region}` : ''}`;
}

/**
 * Vista de administración del estado completo: TODO sin filtrar por audiencia (a diferencia de
 * `proyectarParaJugador`, esto sigue siendo "todas las Facciones, log global" — Fase C3), pero con `mapa`
 * sustituido por `mapaId` (Fase C11). El mapa no cambia nunca durante la partida (125 KB medidos, idénticos
 * byte a byte del tick 0 al 200 — doc 6 §6.4): mandarlo entero en cada lectura de estado es la misma fuga de
 * ancho de banda que ya se corrigió en la proyección de jugador, solo que aquí no se había notado porque el
 * cliente de administración es el único que la sufre hoy.
 *
 * El cliente pide el mapa real, una vez, por `GET .../mapa/:mapaId` — servible con cache eterna porque el id
 * ya captura su identidad completa.
 *
 * `preciosReferencia` (auditoría de doc 9, 2026-08-26) NO lo rellena esta función: es una regla de entrada
 * PRIVILEGIADA (necesita el almacén de TODOS los asentamientos), calculada con caché de un minuto real en
 * `RunnerDePartida.preciosReferencia()` — impuro, vive en `server/`, no aquí. Se declara en este tipo porque
 * es lo que de verdad viaja por el cable (mismo patrón que `RespuestaComando.proyeccion` en C6: el tipo
 * describe el CONTRATO, no todo tiene que salir de una sola función pura). Quien construye la respuesta HTTP
 * es responsable de fusionarlo — ver `server/rutas/admin.ts` y `jugador.ts`.
 */
export type EstadoAdmin = Omit<GameSessionState, 'mapa'> & { mapaId: string; preciosReferencia: Record<string, number> };

/** Devuelve todo MENOS `preciosReferencia`: esa pieza es impura (TTL real) y la añade el llamador HTTP —
 * spread sobre este resultado más `{ preciosReferencia: runner.preciosReferencia() }` completa un `EstadoAdmin`. */
export function vistaAdminDeEstado(estado: GameSessionState): Omit<EstadoAdmin, 'preciosReferencia'> {
  const { mapa, ...resto } = estado;
  return { ...resto, mapaId: idDeMapa(mapa) };
}

/** Añade una entrada al historial de un jugador concreto (administración, igual que el log). */
export function conHistorialDeJugador(estado: GameSessionState, jugadorId: string, mensaje: string): GameSessionState {
  if (!jugadorId) return estado;
  const previo = estado.historialJugadores[jugadorId] ?? [];
  return {
    ...estado,
    historialJugadores: { ...estado.historialJugadores, [jugadorId]: [{ tick: estado.tick, mensaje }, ...previo] },
  };
}
