// RunnerDePartida (Docs/Arquitectura/7_Diseno_GameSession.md §4, §8.4): la pieza que faltaba entre
// `GameSession` (síncrona, sin E/S, sin cola — doc 7 §2) y un backend real. Aquí, y solo aquí, viven:
//
//  - La COLA SERIAL por partida que exige el doc 2 (principio 5): dos llamadas a `ejecutar`/`avanzarTick`
//    lanzadas sin esperar la primera se aplican en el orden en que llegaron, nunca intercaladas.
//  - El ciclo "aplicar -> persistir -> confirmar" del doc 7 §2(a): si la escritura a disco falla, el comando
//    se descarta — `GameSession` vuelve a como estaba antes de aplicarlo, no se queda a medias.
//  - El scheduler de ticks automáticos (opcional: nada obliga a usarlo, `avanzarTick()` sigue invocable a
//    mano igual que hoy).
//
// Lo que NO hace todavía, a propósito: difusión a clientes (necesita el protocolo de suscripciones de
// Fase C, que no existe) y nada de HTTP/WebSocket (es la capa de encima, no esta).
//
// Contrato ASÍNCRONO desde el principio (doc 7 §8.4), aunque hoy por dentro no espere nada más que la propia
// escritura a disco: migrar a cesión cooperativa del event loop o a un worker (si algún día hiciera falta,
// ver doc 7 §8.3) es, por diseño, un cambio DENTRO de este archivo — invisible para quien lo llama.
import type { RegionId } from '../domain/types';
import { GameSession, type PartidaExportada, type ResultadoComando } from '../session/gameSession';
import type { ActorId, ManejadorComando } from '../session/comandos/tipos';
import { cargarPartida, guardarPartida } from './persistenciaPartida';

export interface OpcionesRunner {
  /** Directorio donde vive el snapshot de esta partida (`persistenciaPartida.ts`). */
  directorio: string;
  /** Reloj inyectado — igual que en toda `session/`, nunca se lee `Date.now()` sin pasar por aquí. Los tests
   * inyectan uno controlado; por defecto, el reloj real. */
  ahora?: () => string;
}

export class RunnerDePartida {
  private sesion: GameSession;
  private readonly directorio: string;
  private readonly ahora: () => string;
  private temporizador: ReturnType<typeof setInterval> | null = null;

  /**
   * Cadena de la cola serial. INVARIANTE: siempre es una promesa que RESUELVE (nunca rechaza) — cada
   * `encolar` la reemplaza por una versión saneada del trabajo que acaba de encadenar. Así un trabajo que
   * falla no "envenena" la cadena para los que vengan después; el rechazo real se lleva por separado en la
   * promesa que se devuelve al llamador de ESE trabajo.
   */
  private cola: Promise<void> = Promise.resolve();

  private constructor(sesion: GameSession, opciones: OpcionesRunner) {
    this.sesion = sesion;
    this.directorio = opciones.directorio;
    this.ahora = opciones.ahora ?? (() => new Date().toISOString());
  }

  static crear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): RunnerDePartida {
    return new RunnerDePartida(GameSession.crear(gameId, config), opciones);
  }

  /** Cierra el hueco entre "servidor recién arrancado" y "partida en curso": si ya hay un snapshot para
   * `gameId`, lo carga (en continuidad de RNG, ver `persistenciaPartida.ts`); si no, crea una partida nueva
   * con `config`. `config` se ignora si se carga un snapshot existente. */
  static async cargarOCrear(gameId: string, config: { seed: number; region?: RegionId }, opciones: OpcionesRunner): Promise<RunnerDePartida> {
    const existente = await cargarPartida(opciones.directorio, gameId);
    return new RunnerDePartida(existente ?? GameSession.crear(gameId, config), opciones);
  }

  get gameId(): string {
    return this.sesion.gameId;
  }

  getState() {
    return this.sesion.getState();
  }

  /** Ejecuta un comando de jugador y espera su turno en la cola. Rechaza con lo que lance la persistencia si
   * la escritura falla — el comando en sí no se pierde en silencio, pero tampoco queda aplicado sin estar
   * guardado. */
  ejecutar<P, R>(manejador: ManejadorComando<P, R>, params: P, actor?: ActorId): Promise<ResultadoComando<R>> {
    return this.encolar(() =>
      this.aplicarYPersistir((sesion) => sesion.ejecutar(manejador, params, { momento: this.ahora(), actor }))
    );
  }

  /** Avanza un tick, en la misma cola que los comandos de jugador — comparten la misma restricción (doc 7
   * §8.1: "el estado solo admite un mutador a la vez"), así que comparten la misma cola. */
  avanzarTick(): Promise<ResultadoComando<void>> {
    return this.encolar(() => this.aplicarYPersistir((sesion) => sesion.avanzarTick(this.ahora())));
  }

  /**
   * Programa `avanzarTick()` cada `intervaloMs`, a través de la misma cola serial que los comandos —
   * `setInterval` solo dispara la llamada, el orden real lo sigue decidiendo la cola. Un tick que tarde más
   * que `intervaloMs` no se solapa consigo mismo: la siguiente llamada simplemente espera su turno como
   * cualquier otro trabajo encolado.
   */
  iniciarTicksAutomaticos(intervaloMs: number): void {
    if (this.temporizador) return; // ya en marcha: no duplicar el intervalo
    this.temporizador = setInterval(() => {
      void this.avanzarTick();
    }, intervaloMs);
  }

  detenerTicksAutomaticos(): void {
    if (!this.temporizador) return;
    clearInterval(this.temporizador);
    this.temporizador = null;
  }

  /**
   * Se resuelve cuando la cola queda vacía: todo lo encolado hasta este instante ha terminado, con éxito o
   * con error. `detenerTicksAutomaticos` impide que se ENCOLE trabajo nuevo, pero no cancela el que ya
   * estaba en cola (un tick a medio persistir no se aborta) — esto es para esperar a que ese resto drene.
   * Pensado tanto para un apagado limpio del proceso como para pruebas que necesiten un punto determinista
   * después de parar el scheduler.
   */
  async esperarColaVacia(): Promise<void> {
    await this.cola;
  }

  private encolar<T>(trabajo: () => Promise<T>): Promise<T> {
    const resultado = this.cola.then(trabajo);
    this.cola = resultado.then(
      () => undefined,
      () => undefined
    );
    return resultado;
  }

  /** El ciclo "aplicar -> persistir -> confirmar" del doc 7 §2(a). `GameSession` ya adoptó el estado nuevo
   * dentro de `operacion` cuando esta función se entera de si hubo error — por eso, si la persistencia
   * falla, no basta con "no aplicar": hay que reconstruir `GameSession` desde el snapshot previo a la
   * operación. `previo` es barato de capturar (`exportar()` solo copia referencias, no clona, ver
   * `GameSession.exportar`), así que capturarlo en cada operación no es un coste real. */
  private async aplicarYPersistir<R>(operacion: (sesion: GameSession) => ResultadoComando<R>): Promise<ResultadoComando<R>> {
    const previo: PartidaExportada = this.sesion.exportar();
    const resultado = operacion(this.sesion);

    if (!resultado.ok) return resultado; // rechazado: GameSession no cambió nada, no hay nada que persistir

    try {
      await guardarPartida(this.directorio, this.sesion, this.ahora());
      return resultado;
    } catch (err) {
      this.sesion = GameSession.importar(previo);
      throw err;
    }
  }
}
