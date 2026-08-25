// API HTTP administrativa mínima (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): crear partida,
// avanzar tick, consultar estado. Nada de autenticación ni autorización todavía — eso es Fase C (doc 5).
// Estos tres endpoints son deliberadamente de ADMINISTRACIÓN: sin protegerlos por rol técnico, no deben
// exponerse tal cual a un cliente de jugador real. `main.ts` (el cliente de navegador local) todavía no
// habla con esta API — esa es la tarea siguiente del doc 4, sin abordar.
//
// `crearServidor()` construye la instancia SIN escuchar ningún puerto — eso lo decide el llamador
// (`index.ts` en producción, los tests vía `.inject()`). Es el patrón recomendado de Fastify para probar
// rutas sin abrir sockets de verdad.
import Fastify, { type FastifyInstance } from 'fastify';
import type { RegionId } from '../domain/types';
import { RunnerDePartida } from './runnerDePartida';

export interface OpcionesServidor {
  /** Directorio donde `persistenciaPartida.ts` guarda los snapshots. */
  directorio: string;
}

interface CrearPartidaBody {
  gameId: string;
  seed: number;
  region?: RegionId;
}

interface ParametrosGameId {
  gameId: string;
}

const REGIONES: readonly RegionId[] = ['greciaContinental', 'anatolia', 'egeo', 'nilo', 'mesopotamia'];

const ESQUEMA_CREAR_PARTIDA = {
  body: {
    type: 'object',
    required: ['gameId', 'seed'],
    additionalProperties: false,
    properties: {
      gameId: { type: 'string', minLength: 1 },
      seed: { type: 'number' },
      region: { type: 'string', enum: REGIONES },
    },
  },
} as const;

interface ResumenPartida {
  gameId: string;
  tick: number;
  version: number;
}

function resumenDe(runner: RunnerDePartida): ResumenPartida {
  const estado = runner.getState();
  return { gameId: runner.gameId, tick: estado.tick, version: estado.version };
}

function mensajeDe(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function crearServidor(opciones: OpcionesServidor): FastifyInstance {
  const app = Fastify({ logger: false });

  /**
   * Partidas que ESTE proceso tiene abiertas. No es un caché de conveniencia: es lo que impide que dos
   * peticiones de creación concurrentes para el mismo `gameId` acaben con dos `RunnerDePartida` distintos
   * escribiendo el mismo archivo — justo el escenario que la versión de concurrencia de
   * `persistenciaPartida.ts` está pensada para DETECTAR como síntoma, no algo que deba llegar a pasar.
   */
  const runners = new Map<string, RunnerDePartida>();

  /**
   * Crea una partida nueva, o RETOMA la que ya hubiera en disco para ese `gameId` (mismo criterio que
   * `RunnerDePartida.cargarOCrear`, `seed`/`region` se ignoran en ese caso). No hay un endpoint aparte para
   * "reanudar": con solo estos tres endpoints mínimos, esta es la única forma de que un proceso reiniciado
   * vuelva a abrir una partida que ya existía en disco.
   */
  app.post<{ Body: CrearPartidaBody }>('/partidas', { schema: ESQUEMA_CREAR_PARTIDA }, async (request, reply) => {
    const { gameId, seed, region } = request.body;
    if (runners.has(gameId)) {
      return reply.code(409).send({ error: `la partida '${gameId}' ya está abierta en este proceso.` });
    }
    const runner = await RunnerDePartida.cargarOCrear(gameId, { seed, region }, { directorio: opciones.directorio });
    runners.set(gameId, runner);
    return reply.code(201).send(resumenDe(runner));
  });

  app.post<{ Params: ParametrosGameId }>('/partidas/:gameId/tick', async (request, reply) => {
    const runner = runners.get(request.params.gameId);
    if (!runner) {
      return reply.code(404).send({ error: `la partida '${request.params.gameId}' no está abierta en este proceso.` });
    }
    try {
      const resultado = await runner.avanzarTick();
      return reply.send({ ...resumenDe(runner), resultado });
    } catch (err) {
      // La única forma en que `avanzarTick` puede rechazar (no `resultado.ok === false`, que ya viene
      // dentro de `resultado`) es un fallo de la capa de persistencia — ver `RunnerDePartida.aplicarYPersistir`.
      return reply.code(409).send({ error: mensajeDe(err) });
    }
  });

  app.get<{ Params: ParametrosGameId }>('/partidas/:gameId', async (request, reply) => {
    const runner = runners.get(request.params.gameId);
    if (!runner) {
      return reply.code(404).send({ error: `la partida '${request.params.gameId}' no está abierta en este proceso.` });
    }
    return reply.send(runner.getState());
  });

  return app;
}
