// GameSession (Docs/Arquitectura/7_Diseno_GameSession.md): la partida como estado + reglas, síncrona, sin
// E/S, sin cola y sin reloj propio. Sustituye a `app/gameStore.ts` como dueña de la lógica de UNA partida —
// `GameStore` se convertirá en un adaptador delgado sobre esta clase (doc 7 §6, paso 2), sin que la UI
// cambie mientras dure la migración.
//
// Lo que NO hace a propósito, porque lo hace otra capa (doc 7 §2):
//   - Leer el reloj: todo `momento` lo pasa el llamador, nunca `Date.now()` aquí dentro.
//   - Cola de comandos, orden, concurrencia: eso es del futuro `RunnerDePartida`.
//   - HTTP, WebSocket, persistencia en disco: eso es del runner / la capa de transporte.
//   - Notificar a una UI (`subscribe`/`notify`) o llevar historial de depuración: eso es del cliente.
import type {
  AcuerdoTrueque,
  Asentamiento,
  CaminoComercial,
  CampamentoBandido,
  Caravana,
  Faccion,
  OrdenMercado,
  Point,
  RegionId,
  RelacionPolitica,
  Titulo,
} from '../domain/types';
import type { EventoDominio } from '../domain/eventos';
import { createRng, generarMapa, MAPA_DEFAULT, WORLDGEN_VERSION, type MapaGenerado, type RandomFn } from '../worldgen';
import { crearEstadoMapa, crearMapa, type EstadoMapa, type Mapa } from '../world/mapa';
import { avanzarSimulacion, type ContextoSimulacion, type EstadoSimulacion } from '../engine/simulation';
import { fundarAsentamiento as fundarAsentamientoEngine, FundacionInvalidaError } from '../engine/settlement';
import { crearFaccion as crearFaccionEngine } from '../engine/faccion';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza, type ResultadoNpcGobernanza } from './npcGobernanza';
import { GeneradorIds } from './idGenerator';
import { codigoDeErrorDominio } from './erroresDeDominio';
import { FUNDACION } from '../constants';

/** Entrada de log administrativo — mismo formato que ya usaba `GameStore.EventoLog`. Solo lo consulta un
 * administrador; nunca viaja a un jugador (Docs/Arquitectura/7_Diseno_GameSession.md §7.1). */
export interface EventoLogAdmin {
  tick: number;
  mensaje: string;
}

/**
 * Estado completo y serializable de una partida. Deliberadamente sin nada de UI (sin `Mapa` como fachada, sin
 * caché de render): eso se reconstruye a partir de aquí, nunca se guarda.
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
  /** Facciones que gobierna el NPC en vez de un jugador humano. Vive en la partida, no en el runner (Docs/
   * Arquitectura/7_Diseno_GameSession.md §7.2): cambia el resultado del tick, así que un reinicio con otra
   * configuración divergiría de lo que el snapshot dice haber pasado. Cambiarla es un comando administrativo;
   * en una partida real no cambia en caliente una vez elegida. */
  faccionesNpcIds: string[];
  tick: number;
  /** Se incrementa en cada mutación aceptada (tick o comando) — base del control de concurrencia optimista.
   * Un comando rechazado NUNCA la incrementa (ver `ResultadoComando`). */
  version: number;
  /** Administración (doc 7 §7.1): no viaja a jugadores. */
  log: EventoLogAdmin[];
  historialJugadores: Record<string, EventoLogAdmin[]>;
  /** Auditoría/replay de Fase E (doc 7 §7.1) — mismos hechos que `log`, en forma estructurada. Hoy todo sale
   * con `codigo: 'legado'` hasta que avance el marcador de migración de A5. */
  eventosDominio: EventoDominio[];
}

/**
 * Resultado de un comando: nunca texto de log (Docs/Arquitectura/2_Estudio_Evolucion_Backend_Multifrontend.md,
 * punto 6 — "los logs localizados no deben ser el único contrato"). `datos` lleva lo que el comando produjo
 * (ej. el id del asentamiento fundado) para que el llamador no tenga que releer el estado a ciegas.
 */
export interface ResultadoComando<T = void> {
  ok: boolean;
  datos?: T;
  /** Código estable (ver `erroresDeDominio.ts`) si `ok` es `false`. Nunca texto localizado. */
  codigoError?: string;
  eventos: EventoDominio[];
  /** Versión de la partida tras este comando. Igual a la anterior si `ok` es `false`: un comando rechazado
   * no muta nada. */
  version: number;
}

function estadoSimulacionDe(estado: GameSessionState): EstadoSimulacion {
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

export class GameSession {
  readonly gameId: string;
  private state: GameSessionState;
  private ids: GeneradorIds;
  private rng: RandomFn;
  /** Fachada de consultas espaciales sobre `state.mapa`/`state.estadoMapa`. Una sola instancia para toda la
   * vida de la sesión (a diferencia de `GameStore`, que cachea una por foto de historial): `GameSession` no
   * lleva historial, así que no hay nada que invalidar salvo al reemplazar el mapa entero (`regenerarMundo`/
   * `importar`). `Mapa.extraer`/`avanzarRegeneracion` mutan `estadoMapa` en el sitio, por eso la misma fachada
   * sigue siendo válida tick tras tick. */
  private mapaFachada: Mapa;

  private constructor(gameId: string, state: GameSessionState, ids: GeneradorIds, rng: RandomFn) {
    this.gameId = gameId;
    this.state = state;
    this.ids = ids;
    this.rng = rng;
    this.mapaFachada = crearMapa(state.mapa, state.estadoMapa);
  }

  /** Crea una partida nueva, con mundo recién generado a partir de `seed`. */
  static crear(gameId: string, config: { seed: number; region?: RegionId }): GameSession {
    const mapaGenerado = generarMapa({ ...MAPA_DEFAULT, seed: config.seed, region: config.region });
    const state: GameSessionState = {
      gameId,
      mapa: mapaGenerado,
      estadoMapa: crearEstadoMapa(),
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
      version: 0,
      log: [],
      historialJugadores: {},
      eventosDominio: [],
    };
    return new GameSession(gameId, state, new GeneradorIds(), createRng(config.seed));
  }

  getState(): Readonly<GameSessionState> {
    return this.state;
  }

  getMapa(): Mapa {
    return this.mapaFachada;
  }

  // --- Registro interno (doc 7 §7.1: log/eventosDominio son administración, nunca viajan a jugadores) ---

  private registrar(momento: string, mensaje: string, asentamientoId?: string): EventoDominio {
    this.state.log = [{ tick: this.state.tick, mensaje }, ...this.state.log];
    const evento: EventoDominio = { codigo: 'legado', mensaje, momento, tick: this.state.tick, asentamientoId };
    this.state.eventosDominio = [evento, ...this.state.eventosDominio];
    return evento;
  }

  private registrarJugador(jugadorId: string, mensaje: string): void {
    if (!jugadorId) return;
    const lista = this.state.historialJugadores[jugadorId] ?? [];
    this.state.historialJugadores[jugadorId] = [{ tick: this.state.tick, mensaje }, ...lista];
  }

  // --- Comandos ---

  /**
   * Funda un asentamiento — primer comando migrado (representativo del patrón que seguirán los ~35 de
   * `GameStore`, ver Docs/Arquitectura/4_Plan_Evolucion_Tareas.md Fase B, paso "mover comandos por grupos").
   */
  fundarAsentamiento(momento: string, faccionId: string, posicion: Point, numJugadores: number): ResultadoComando<{ asentamientoId: string }> {
    const n = Math.min(FUNDACION.maxJugadoresFundacionGrupal, Math.max(1, numJugadores || 1));
    const jugadoresIds = Array.from({ length: n }, (_, i) => `jugador-${faccionId}-${i + 1}`);
    try {
      const resultado = fundarAsentamientoEngine(this.mapaFachada, this.state.facciones, faccionId, posicion, jugadoresIds, this.state.asentamientos, this.state.tick);
      this.state.asentamientos = [...this.state.asentamientos, resultado.asentamiento];
      this.state.facciones = resultado.facciones;
      this.state.version += 1;

      const nombreFaccion = this.state.facciones.find((f) => f.id === faccionId)?.nombre ?? faccionId;
      const eventos: EventoDominio[] = [
        this.registrar(momento, `${nombreFaccion} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`, resultado.asentamiento.id),
      ];
      for (const jugadorId of jugadoresIds) {
        this.registrarJugador(jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
      }
      return { ok: true, datos: { asentamientoId: resultado.asentamiento.id }, eventos, version: this.state.version };
    } catch (err) {
      return this.comandoRechazado(err, FundacionInvalidaError);
    }
  }

  /** Crea una Facción nueva. Segundo comando migrado — deliberadamente simple (sin efectos secundarios sobre
   * asentamientos) para que el patrón de `comandoRechazado` se vea sin ruido alrededor. */
  crearFaccion(momento: string, nombre: string): ResultadoComando<{ faccionId: string }> {
    const nombreLimpio = nombre.trim();
    if (!nombreLimpio) {
      return { ok: false, codigoError: 'faccion.nombre_vacio', eventos: [], version: this.state.version };
    }
    if (this.state.facciones.some((f) => f.nombre.toLowerCase() === nombreLimpio.toLowerCase())) {
      return { ok: false, codigoError: 'faccion.nombre_duplicado', eventos: [], version: this.state.version };
    }
    const nueva = crearFaccionEngine(`faccion-custom-${this.ids.siguiente()}`, nombreLimpio);
    this.state.facciones = [...this.state.facciones, nueva];
    this.state.version += 1;
    const eventos = [this.registrar(momento, `Se crea la Facción "${nueva.nombre}".`)];
    return { ok: true, datos: { faccionId: nueva.id }, eventos, version: this.state.version };
  }

  /** Traduce un error capturado a `ResultadoComando` rechazado si es del tipo de dominio esperado (o de
   * cualquier otro reconocido en `erroresDeDominio.ts`); relanza si no — un error no reconocido es un bug,
   * no un rechazo de comando, y no debe convertirse en un resultado silencioso. */
  private comandoRechazado<T>(err: unknown, ...tiposEsperados: Function[]): ResultadoComando<T> {
    const codigo = codigoDeErrorDominio(err);
    if (codigo === undefined || (tiposEsperados.length > 0 && !tiposEsperados.some((t) => err instanceof t))) throw err;
    return { ok: false, codigoError: codigo, eventos: [], version: this.state.version };
  }

  // --- Ciclo de partida ---

  /** Avanza un tick. `momento` lo decide el llamador (nunca `Date.now()` aquí) — ver cabecera del archivo. */
  avanzarTick(momento: string): ResultadoComando<void> {
    this.state.tick += 1;
    const contexto: ContextoSimulacion = { tick: this.state.tick, momento, rng: this.rng };
    const resultado = avanzarSimulacion(estadoSimulacionDe(this.state), this.mapaFachada, contexto);

    this.state = {
      ...this.state,
      asentamientos: resultado.asentamientos,
      facciones: resultado.facciones,
      caravanas: resultado.caravanas,
      acuerdos: resultado.acuerdos,
      ordenes: resultado.ordenes,
      relaciones: resultado.relaciones,
      titulos: resultado.titulos,
      caminos: resultado.caminos,
      campamentosBandidos: resultado.campamentosBandidos,
      bandidosProximoSpawnTick: resultado.bandidosProximoSpawnTick,
      version: this.state.version + 1,
      log: [...resultado.eventos.map((mensaje) => ({ tick: this.state.tick, mensaje })), ...this.state.log],
      eventosDominio: [...resultado.eventosDominio, ...this.state.eventosDominio],
    };

    return { ok: true, eventos: resultado.eventosDominio, version: this.state.version };
  }

  /** Turno del NPC de gobernanza para las Facciones de `faccionesNpcIds` — mismo orden que usa `GameStore`:
   * siempre DESPUÉS del tick del motor, nunca dentro de `avanzarSimulacion`. */
  avanzarFaccionesNpc(momento: string): ResultadoComando<void> {
    if (this.state.faccionesNpcIds.length === 0) return { ok: true, eventos: [], version: this.state.version };

    const contexto: ContextoSimulacion = { tick: this.state.tick, momento, rng: this.rng };
    const config: ConfigNpcGobernanza = { faccionesIds: this.state.faccionesNpcIds, contadorInicial: this.ids.actual() };
    const resultado: ResultadoNpcGobernanza = avanzarNpcGobernanza(estadoSimulacionDe(this.state), this.mapaFachada, contexto, config);

    this.ids.fijar(resultado.contadorFinal);
    const eventos = resultado.eventos.map((mensaje) => this.registrar(momento, `[NPC] ${mensaje}`));
    this.state = {
      ...this.state,
      asentamientos: resultado.estado.asentamientos,
      facciones: resultado.estado.facciones,
      caravanas: resultado.estado.caravanas,
      acuerdos: resultado.estado.acuerdos,
      campamentosBandidos: resultado.estado.campamentosBandidos,
      bandidosProximoSpawnTick: resultado.estado.bandidosProximoSpawnTick,
      version: this.state.version + (resultado.eventos.length > 0 ? 1 : 0),
    };

    return { ok: true, eventos, version: this.state.version };
  }

  // --- Serialización (Fase B3 le añadirá persistencia real por encima de esto) ---

  /** Formato mínimo de exportación — se ampliará en B3 (versión de concurrencia en disco, RNG, etc., ver
   * Docs/Arquitectura/4_Plan_Evolucion_Tareas.md). Por ahora basta para reconstruir la partida en memoria. */
  exportar(): { state: GameSessionState; siguienteId: number; worldgenVersion: number } {
    return { state: this.state, siguienteId: this.ids.actual(), worldgenVersion: WORLDGEN_VERSION };
  }

  static importar(payload: { state: GameSessionState; siguienteId: number }): GameSession {
    const ids = new GeneradorIds();
    ids.fijar(payload.siguienteId);
    // El RNG se reinicia desde la seed del mundo: el estado interno consumido no se persiste todavía (Fase
    // B3, ver Docs/Arquitectura/7_Diseno_GameSession.md — misma limitación conocida que ya tenía GameStore).
    const rng = createRng(payload.state.mapa.config.seed);
    return new GameSession(payload.state.gameId, payload.state, ids, rng);
  }
}
