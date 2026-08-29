// Partidas que ESTE proceso tiene abiertas. No es un caché de conveniencia: es lo que impide que dos
// peticiones de creación concurrentes para el mismo `gameId` acaben con dos `RunnerDePartida` distintos
// escribiendo el mismo archivo — justo el escenario que la versión de concurrencia de
// `persistenciaPartida.ts` está pensada para DETECTAR como síntoma, no algo que deba llegar a pasar.
//
// Extraído de `api.ts` en la Fase C3, al separarse las superficies: el mismo registro lo consultan las rutas
// de administración y las de jugador, y tenerlo como `Map` suelto dentro del constructor de rutas lo hacía
// invisible para cualquiera que no leyera esa función entera.
import type { RegionId } from '../domain/types';
import { RunnerDePartida } from './runnerDePartida';
import { listarPartidas, type ResumenPartidaEnDisco } from './persistenciaPartida';

export interface ConfiguracionPartida {
  seed: number;
  region?: RegionId;
}

export class PartidaYaAbiertaError extends Error {
  constructor(gameId: string) {
    super(`la partida '${gameId}' ya está abierta en este proceso.`);
    this.name = 'PartidaYaAbiertaError';
  }
}

export class RegistroDePartidas {
  private readonly runners = new Map<string, RunnerDePartida>();

  /**
   * Fuente de ticks (Fase C12, doc 4: "sin algo el mundo no avanza") — `undefined` por defecto: SIN
   * configurarlo, ninguna partida avanza sola, ni siquiera las de los tests (mismo criterio deliberado que
   * `ADMINISTRADORES`/`ORIGENES_PERMITIDOS` en `api.ts` — un default que activa algo por sí solo es el que
   * sobrevive hasta producción sin que nadie lo note, y aquí además arriesgaría dejar temporizadores reales
   * corriendo en cientos de servidores de prueba que nunca los paran explícitamente).
   *
   * Con "mundo = tiempo real" (doc 10 §2) el valor natural es `SIMULACION.duracionTickMs` (60 000): un tick
   * de mundo por minuto real. D5 cerró el mecanismo de verdad — el reloj de mundo de `RunnerDePartida` con
   * catch-up tras reinicio (`referenciaRelojInicialMs`); ya no es un metrónomo tonto. Cuánto debe medir un
   * tick de JUEGO (si algún día no es un minuto real) sigue siendo una decisión de ritmo pendiente, pero eso
   * es `duracionTickMs`, no este intervalo.
   *
   * `ahora` es el reloj de pared del proceso (el mismo que `deps.ahora` en `api.ts`) — se le pasa al runner
   * para que su reloj de mundo y su catch-up sean inyectables en tests, no solo el reloj real del sistema.
   */
  constructor(
    private readonly directorio: string,
    private readonly intervaloTickMs?: number,
    private readonly ahora: () => string = () => new Date().toISOString()
  ) {}

  obtener(gameId: string): RunnerDePartida | undefined {
    return this.runners.get(gameId);
  }

  /** Descubrimiento (Fase C12) — lee el directorio, no `this.runners`: ver el comentario de `listarPartidas`. */
  listar(): Promise<ResumenPartidaEnDisco[]> {
    return listarPartidas(this.directorio);
  }

  /**
   * Abre una partida: la crea, o RETOMA la que ya hubiera en disco para ese `gameId` (mismo criterio que
   * `RunnerDePartida.cargarOCrear`; `seed`/`region` se ignoran en ese caso). Es también la única forma de
   * que un proceso reiniciado vuelva a abrir una partida que ya existía en disco.
   *
   * Si ya está abierta EN ESTE PROCESO, lanza: reabrirla no tendría sentido y machacar la que corre sería
   * destructivo sin decirlo. Para eso está `descartarYCrear`, explícito y con su propio permiso.
   */
  async abrir(gameId: string, config: ConfiguracionPartida): Promise<RunnerDePartida> {
    if (this.runners.has(gameId)) throw new PartidaYaAbiertaError(gameId);
    const runner = await RunnerDePartida.cargarOCrear(gameId, config, { directorio: this.directorio, ahora: this.ahora });
    this.runners.set(gameId, runner);
    this.arrancarRelojSiConfigurado(runner);
    return runner;
  }

  /** Descarta la partida en curso y crea una limpia — NO reanuda el snapshot existente. Única operación
   * destructiva del registro; quien la expone debe exigir permiso aparte (`puedeDescartarPartida`).
   *
   * Persiste la nueva partida antes de devolverla (`crearYPersistir`, Fase C12): sin esto, el archivo en
   * disco seguiría siendo el de la partida DESCARTADA hasta el primer comando/tick — un reinicio del proceso
   * en ese hueco reviviría exactamente lo que `forzar: true` pedía borrar. */
  async descartarYCrear(gameId: string, config: ConfiguracionPartida): Promise<RunnerDePartida> {
    this.runners.delete(gameId);
    // `forzar: true`: la partida descartada puede seguir en disco con una version > 0 — este reemplazo,
    // que empieza en 0, es deliberado, no el conflicto de concurrencia que `guardarPartida` normalmente
    // detecta (ver su comentario).
    const runner = await RunnerDePartida.crearYPersistir(gameId, config, { directorio: this.directorio, ahora: this.ahora }, { forzar: true });
    this.runners.set(gameId, runner);
    this.arrancarRelojSiConfigurado(runner);
    return runner;
  }

  private arrancarRelojSiConfigurado(runner: RunnerDePartida): void {
    if (this.intervaloTickMs !== undefined) runner.iniciarRelojDeMundo(this.intervaloTickMs);
  }

  /** Detiene el reloj de mundo de cada partida abierta y espera a que su cola serial drene — apagado limpio
   * del proceso (`server/index.ts`). Sin esto, un `setInterval` por partida mantendría el proceso vivo y
   * podría dejar un tick a medio persistir al salir. */
  async cerrar(): Promise<void> {
    await Promise.all(
      [...this.runners.values()].map(async (runner) => {
        runner.detenerRelojDeMundo();
        await runner.esperarColaVacia();
      })
    );
  }
}
