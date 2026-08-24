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

/** Entrada de log administrativo. Solo la consulta un administrador; nunca viaja a un jugador
 * (Docs/Arquitectura/7_Diseno_GameSession.md §7.1 — el log global narra lo que pasa en TODO el mundo, así
 * que incluirlo en una proyección de jugador sería una fuga de información de facciones rivales). */
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
  /** Administración, no viaja a jugadores (ver `EventoLogAdmin`). */
  log: EventoLogAdmin[];
  historialJugadores: Record<string, EventoLogAdmin[]>;
  /** Auditoría y replay de Fase E: los mismos hechos que `log`, en forma estructurada. Hoy todo sale con
   * `codigo: 'legado'` hasta que avance la migración por subsistemas de A5. */
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

/** Evento "legado": el mismo texto que ya se registraba, envuelto en la forma estructurada. Mientras un
 * subsistema no tenga su propio `codigo`, esta es la única forma en que produce eventos (ver
 * `domain/eventos.ts` y el marcador de migración de A5 en el doc 4). */
export function eventoLegado(momento: string, tick: number, mensaje: string, asentamientoId?: string): EventoDominio {
  return { codigo: 'legado', mensaje, momento, tick, asentamientoId };
}

/** Añade una entrada al historial de un jugador concreto (administración, igual que `log`). */
export function conHistorialDeJugador(estado: GameSessionState, jugadorId: string, mensaje: string): GameSessionState {
  if (!jugadorId) return estado;
  const previo = estado.historialJugadores[jugadorId] ?? [];
  return {
    ...estado,
    historialJugadores: { ...estado.historialJugadores, [jugadorId]: [{ tick: estado.tick, mensaje }, ...previo] },
  };
}
