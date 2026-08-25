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

  constructor(private readonly directorio: string) {}

  obtener(gameId: string): RunnerDePartida | undefined {
    return this.runners.get(gameId);
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
    return runner;
  }

  /** Descarta la partida en curso y crea una limpia — NO reanuda el snapshot existente. Única operación
   * destructiva del registro; quien la expone debe exigir permiso aparte (`puedeDescartarPartida`). */
  descartarYCrear(gameId: string, config: ConfiguracionPartida): RunnerDePartida {
    this.runners.delete(gameId);
    const runner = RunnerDePartida.crear(gameId, config, { directorio: this.directorio });
    this.runners.set(gameId, runner);
    return runner;
  }
}
