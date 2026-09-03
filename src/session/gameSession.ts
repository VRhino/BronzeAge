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
import { BALANCE_VERSION } from '../constants';
import { createRng, generarMapa, MAPA_DEFAULT, restaurarRng, WORLDGEN_VERSION, type RandomFn } from '../worldgen';
import { crearEstadoMapa, crearMapa, type EstadoMapa, type Mapa } from '../world/mapa';
import { GeneradorIds } from './idGenerator';
import { eventoAdministrativo, instanteDeTick, isoDeInstante, type GameSessionState } from './estado';
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
  /** `BALANCE_VERSION` (`constants.ts`) vigente al exportar — registro de qué balance corría, sin efecto
   * sobre la carga (a diferencia de `worldgenVersion`, un desajuste no se rechaza; ver el comentario de
   * `BALANCE_VERSION`). */
  balanceVersion: number;
  /**
   * Contador interno del RNG de partida (`RandomFn.estado()`), para que una sesión reconstruida CONTINÚE
   * la misma secuencia en vez de reiniciarla desde la seed del mundo — importa para la reconstrucción de
   * bugs de producción: sin esto, cargar el snapshot de un incidente y avanzar reproduce una continuación
   * cualquiera, no la que realmente ocurrió (ver Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3).
   *
   * Opcional porque el formato de archivo v2 de `GameStore` (anterior a esta pieza, ver
   * `GameStore.importarSimulacion`) no lo guarda: `importar` cae de vuelta a la seed en ese caso, con la
   * misma pérdida de continuidad que había antes de esto — documentada, no oculta.
   */
  estadoRng?: number;
}

export class GameSession {
  private estado: GameSessionState;
  private rng: RandomFn;
  private ids: GeneradorIds;
  /** Fachada de consulta del estado ACTUAL, cacheada para no reconstruirla en cada lectura. Se invalida
   * sola comparando referencias: en cuanto un comando cambia el mapa, `estado.estadoMapa` es otro objeto. */
  private mapaDeConsulta: { sobre: EstadoMapa; mapa: Mapa } | null = null;

  private constructor(estado: GameSessionState, ids: GeneradorIds, rng: RandomFn) {
    this.estado = estado;
    this.ids = ids;
    this.rng = rng;
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }): GameSession {
    const estado: GameSessionState = {
      gameId,
      mapa: generarMapa({ ...MAPA_DEFAULT, seed: config.seed, region: config.region }),
      estadoMapa: crearEstadoMapa(),
      asentamientos: [],
      facciones: [],
      caravanas: [],
      jugadores: [],
      ejercitos: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
      campamentosBandidos: [],
      bandidosProximoSpawnEn: instanteDeTick(0),
      faccionesNpcIds: [],
      tick: 0,
      version: 0,
      historialJugadores: {},
      eventosDominio: [],
      salidasFaccionPorJugador: {},
    };
    return new GameSession(estado, new GeneradorIds(), createRng(config.seed));
  }

  static importar(payload: PartidaExportada): GameSession {
    const ids = new GeneradorIds();
    ids.fijar(payload.siguienteId);
    // Continúa la secuencia exacta si el payload trae `estadoRng` (Fase B3). Sin él —formato de archivo v2,
    // que es anterior a esto— se reinicia desde la seed del mundo, con la pérdida de continuidad ya conocida.
    const rng = payload.estadoRng !== undefined ? restaurarRng(payload.estadoRng) : createRng(payload.state.mapa.config.seed);
    return new GameSession(payload.state, ids, rng);
  }

  get gameId(): string {
    return this.estado.gameId;
  }

  getState(): Readonly<GameSessionState> {
    return this.estado;
  }

  /**
   * Fachada de consulta sobre el estado actual. Es de SOLO LECTURA en la práctica aunque el tipo exponga
   * `extraer`/`avanzarRegeneracion`: escribe en su propia copia, que nadie adopta, así que un uso indebido se
   * pierde en vez de corromper la partida.
   */
  getMapa(): Mapa {
    if (this.mapaDeConsulta?.sobre !== this.estado.estadoMapa) {
      this.mapaDeConsulta = { sobre: this.estado.estadoMapa, mapa: crearMapa(this.estado.mapa, this.estado.estadoMapa) };
    }
    return this.mapaDeConsulta.mapa;
  }

  exportar(): PartidaExportada {
    return {
      state: this.estado,
      siguienteId: this.ids.actual(),
      worldgenVersion: WORLDGEN_VERSION,
      balanceVersion: BALANCE_VERSION,
      estadoRng: this.rng.estado(),
    };
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
   * Sí incrementa `version` porque el evento es parte del estado persistido: cualquier cambio de
   * `GameSessionState` tiene que versionarse para que el control de concurrencia optimista siga siendo válido.
   */
  registrarEventoAdministrativo(mensaje: string): void {
    const version = this.estado.version + 1;
    this.estado = {
      ...this.estado,
      version,
      eventosDominio: [{ ...eventoAdministrativo(this.estado.tick, mensaje), version }, ...this.estado.eventosDominio],
    };
  }

  /**
   * Ejecuta un comando y adopta el estado resultante. **Único punto donde `this.estado` cambia** — todo lo
   * demás son funciones puras, así que no hay forma de mutar la partida sin pasar por aquí.
   *
   * El comando se pasa por referencia en vez de por nombre para conservar los tipos de sus parámetros y de
   * sus datos de retorno. Un registro por nombre (`{ 'fundarAsentamiento': ... }`) hará falta en la Fase C,
   * cuando la API reciba comandos serializados; se construirá sobre estos mismos manejadores.
   *
   * El comando recibe una fachada `Mapa` RECIÉN CREADA sobre una copia del estado del mapa. Es lo que cierra
   * el último resquicio por el que un comando podía dejar rastro sin devolverlo: si no adopta lo que escribió
   * en la fachada, se descarta con ella.
   *
   * `ctx.momento` se DERIVA del tick actual (`instanteDeTick`, Fase D / doc 10), no lo pasa el llamador —
   * antes `RunnerDePartida` inyectaba aquí `Date.now()` y ese reloj de pared terminaba persistido en el
   * estado (`eventosDominio[].momento`), haciendo que el mismo comando con la misma seed produjera snapshots
   * distintos. El tiempo de mundo es función del tick y de nada más.
   */
  ejecutar<P, R>(manejador: ManejadorComando<P, R>, params: P, opciones: { actor?: ActorId } = {}): ResultadoComando<R> {
    const instante = instanteDeTick(this.estado.tick);
    const ctx: ContextoComando = {
      instante,
      momento: isoDeInstante(instante),
      actor: opciones.actor ?? ACTOR_SISTEMA,
      rng: this.rng,
      ids: this.ids,
    };
    const anterior = this.estado;
    const mapa = crearMapa(anterior.mapa, anterior.estadoMapa);
    const transicion = manejador(anterior, mapa, ctx, params);

    // Un comando ACEPTADO que escribió en el mapa y no devolvió el resultado estaría perdiendo ese cambio en
    // silencio. Es un bug de programación, no un estado de juego posible, así que se rompe fuerte en vez de
    // dejar que la partida siga con los yacimientos desincronizados de lo que dice su versión.
    // (Un comando RECHAZADO devuelve el estado intacto a propósito: ahí descartar el mapa es lo correcto.)
    if (transicion.resultado.ok && mapa.mutacionesAplicadas > 0 && transicion.estado.estadoMapa === anterior.estadoMapa) {
      throw new Error('Un comando modificó el mapa pero no devolvió `estadoMapa` en su estado resultante.');
    }

    this.estado = transicion.estado;
    return transicion.resultado;
  }

  // --- Operaciones del sistema ---
  //
  // Atajos con nombre para las operaciones que NO inicia un jugador sino el propio servidor (el scheduler
  // del runner). Se distinguen de los comandos de jugador a propósito: en producción no deben quedar
  // expuestas como endpoints públicos (doc 2: "el servidor sigue siendo quien avance y resuelva los ticks").
  // Ninguna recibe `momento`: el instante de mundo lo deriva `ejecutar` del tick (Fase D).

  avanzarTick(): ResultadoComando<void> {
    return this.ejecutar(avanzarTick, undefined, { actor: ACTOR_SISTEMA });
  }

  /** Trueque automático de simulación (apagado por defecto). Va DESPUÉS del tick y ANTES del NPC de
   * gobernanza — mismo orden que tenía en `GameStore`. */
  avanzarAutoComercio(): ResultadoComando<void> {
    return this.ejecutar(avanzarAutoComercio, undefined, { actor: ACTOR_SISTEMA });
  }

  avanzarFaccionesNpc(): ResultadoComando<void> {
    return this.ejecutar(avanzarFaccionesNpc, undefined, { actor: ACTOR_SISTEMA });
  }
}
