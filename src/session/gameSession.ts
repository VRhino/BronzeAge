// GameSession (Docs/Arquitectura/7_Diseno_GameSession.md): dueña de UNA partida. Síncrona, sin E/S, sin cola
// y sin reloj propio.
//
// Es DELIBERADAMENTE pequeña: sostiene el estado actual y despacha comandos, nada más. La lógica de cada
// comando vive en `comandos/`, un archivo por comando. El motivo es medido, no estético: `GameStore` acumula
// 34 comandos en 1582 líneas, y convertir esos métodos en métodos de esta clase habría reproducido el mismo
// objeto-dios que el doc 2 le critica, solo que en otra carpeta.
//
// Lo que NO hace, porque es de otra capa (doc 7 §2):
//   - Leer el reloj: el `momento` lo pasa siempre el llamador.
//   - Cola, orden de comandos, concurrencia: del futuro `RunnerDePartida`.
//   - HTTP, WebSocket, persistencia en disco: del runner y de la capa de transporte.
//   - Notificar a una UI o llevar historial de depuración: del cliente.
import type { RegionId } from '../domain/types';
import { createRng, generarMapa, MAPA_DEFAULT, WORLDGEN_VERSION, type RandomFn } from '../worldgen';
import { crearEstadoMapa, crearMapa, type Mapa } from '../world/mapa';
import { GeneradorIds } from './idGenerator';
import { eventoLegado, type GameSessionState } from './estado';
import { avanzarAutoComercio } from './comandos/avanzarAutoComercio';
import { avanzarFaccionesNpc } from './comandos/avanzarFaccionesNpc';
import { avanzarTick } from './comandos/avanzarTick';
import {
  ACTOR_SISTEMA,
  type ActorId,
  type ContextoComando,
  type ManejadorComando,
  type ResultadoComando,
} from './comandos/tipos';

export type { GameSessionState } from './estado';
export type { ResultadoComando } from './comandos/tipos';

/** Datos de un `exportar()`, listos para serializar. La persistencia real (escritura atómica, versión de
 * concurrencia en disco, estado del RNG) llega en Fase B3 — ver doc 4. */
export interface PartidaExportada {
  state: GameSessionState;
  siguienteId: number;
  worldgenVersion: number;
}

export class GameSession {
  private estado: GameSessionState;
  private rng: RandomFn;
  private ids: GeneradorIds;
  /** Fachada de consultas espaciales sobre `estado.mapa`/`estado.estadoMapa`. Una sola instancia para toda la
   * vida de la sesión: `GameSession` no lleva historial (a diferencia de `GameStore`, que cacheaba una por
   * foto), así que no hay nada que invalidar. `extraer`/`avanzarRegeneracion` mutan `estadoMapa` en el sitio,
   * por eso la misma fachada sigue siendo válida tick tras tick. */
  private mapaFachada: Mapa;

  private constructor(estado: GameSessionState, ids: GeneradorIds, rng: RandomFn) {
    this.estado = estado;
    this.ids = ids;
    this.rng = rng;
    this.mapaFachada = crearMapa(estado.mapa, estado.estadoMapa);
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }): GameSession {
    const estado: GameSessionState = {
      gameId,
      mapa: generarMapa({ ...MAPA_DEFAULT, seed: config.seed, region: config.region }),
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
    return new GameSession(estado, new GeneradorIds(), createRng(config.seed));
  }

  static importar(payload: PartidaExportada): GameSession {
    const ids = new GeneradorIds();
    ids.fijar(payload.siguienteId);
    // El RNG se reinicia desde la seed del mundo: su estado interno consumido todavía no se persiste (misma
    // limitación conocida que ya tenía `GameStore`; se resuelve en Fase B3, ver doc 4).
    return new GameSession(payload.state, ids, createRng(payload.state.mapa.config.seed));
  }

  get gameId(): string {
    return this.estado.gameId;
  }

  getState(): Readonly<GameSessionState> {
    return this.estado;
  }

  getMapa(): Mapa {
    return this.mapaFachada;
  }

  exportar(): PartidaExportada {
    return { state: this.estado, siguienteId: this.ids.actual(), worldgenVersion: WORLDGEN_VERSION };
  }

  /**
   * Anota en el log administrativo un hecho que NO es un comando de partida — hoy solo los cambios de
   * balance (`app/balanceConfig.ts`), que mutan `constants.ts` en el sitio y por tanto son configuración del
   * PROCESO, no estado de esta partida (ver Docs/Arquitectura/7_Diseno_GameSession.md §2.ter).
   *
   * Existe para no tener que fingir que esos cambios son transiciones de estado: no pasan por un comando, no
   * tienen `ResultadoComando` y no producen eventos de dominio de juego. Lo único que hacen es dejar rastro,
   * que es lo que la consola de administración ya mostraba.
   *
   * **Temporal, y a sustituir en Fase C** por la auditoría real que pide el doc 2 (punto 8): actor, fecha,
   * versión anterior y nueva, sobre un balance versionado POR PARTIDA en vez de global. Cuando eso exista,
   * este método desaparece.
   *
   * Sí incrementa `version` porque el log es parte del estado persistido: cualquier cambio de
   * `GameSessionState` tiene que versionarse para que el control de concurrencia optimista siga siendo válido.
   */
  registrarEventoAdministrativo(momento: string, mensaje: string): void {
    const evento = eventoLegado(momento, this.estado.tick, mensaje);
    this.estado = {
      ...this.estado,
      version: this.estado.version + 1,
      log: [{ tick: evento.tick, mensaje }, ...this.estado.log],
      eventosDominio: [evento, ...this.estado.eventosDominio],
    };
  }

  /**
   * Ejecuta un comando y adopta el estado resultante. **Único punto donde `this.estado` cambia** — todo lo
   * demás son funciones puras, así que no hay forma de mutar la partida sin pasar por aquí.
   *
   * El comando se pasa por referencia en vez de por nombre para conservar los tipos de sus parámetros y de
   * sus datos de retorno. Un registro por nombre (`{ 'fundarAsentamiento': ... }`) hará falta en la Fase C,
   * cuando la API reciba comandos serializados; se construirá sobre estos mismos manejadores.
   */
  ejecutar<P, R>(manejador: ManejadorComando<P, R>, params: P, opciones: { momento: string; actor?: ActorId }): ResultadoComando<R> {
    const ctx: ContextoComando = {
      momento: opciones.momento,
      actor: opciones.actor ?? ACTOR_SISTEMA,
      rng: this.rng,
      ids: this.ids,
    };
    const transicion = manejador(this.estado, this.mapaFachada, ctx, params);
    this.estado = transicion.estado;
    return transicion.resultado;
  }

  // --- Operaciones del sistema ---
  //
  // Atajos con nombre para las dos operaciones que NO inicia un jugador sino el propio servidor (el scheduler
  // del runner). Se distinguen de los comandos de jugador a propósito: en producción no deben quedar
  // expuestas como endpoints públicos (doc 2: "el servidor sigue siendo quien avance y resuelva los ticks").

  avanzarTick(momento: string): ResultadoComando<void> {
    return this.ejecutar(avanzarTick, undefined, { momento, actor: ACTOR_SISTEMA });
  }

  /** Trueque automático de simulación (apagado por defecto). Va DESPUÉS del tick y ANTES del NPC de
   * gobernanza — mismo orden que tenía en `GameStore`. */
  avanzarAutoComercio(momento: string): ResultadoComando<void> {
    return this.ejecutar(avanzarAutoComercio, undefined, { momento, actor: ACTOR_SISTEMA });
  }

  avanzarFaccionesNpc(momento: string): ResultadoComando<void> {
    return this.ejecutar(avanzarFaccionesNpc, undefined, { momento, actor: ACTOR_SISTEMA });
  }
}
