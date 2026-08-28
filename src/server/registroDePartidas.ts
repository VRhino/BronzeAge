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
   * El intervalo en sí es un PLACEHOLDER (igual que los valores de `constants.ts`, ver su cabecera): cada
   * cuánto debe avanzar el mundo es una decisión de RITMO DE JUEGO, no de arquitectura, y no está tomada en
   * ningún doc de este repo — E1 (Fase E) es quien cierra esto de verdad, con recuperación de eventos
   * vencidos tras un reinicio. Esto es solo "que exista alguna fuente", no la definitiva.
   */
  constructor(
    private readonly directorio: string,
    private readonly intervaloTickMs?: number
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
    const runner = await RunnerDePartida.cargarOCrear(gameId, config, { directorio: this.directorio });
    this.runners.set(gameId, runner);
    this.arrancarTicksSiConfigurado(runner);
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
    const runner = await RunnerDePartida.crearYPersistir(gameId, config, { directorio: this.directorio }, { forzar: true });
    this.runners.set(gameId, runner);
    this.arrancarTicksSiConfigurado(runner);
    return runner;
  }

  private arrancarTicksSiConfigurado(runner: RunnerDePartida): void {
    if (this.intervaloTickMs !== undefined) runner.iniciarTicksAutomaticos(this.intervaloTickMs);
  }
}
