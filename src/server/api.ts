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
import { REGISTRO_COMANDOS, type TipoComando } from '../session/comandos/registro';
import { ACTOR_LOCAL, type ManejadorComando } from '../session/comandos/tipos';
import { RunnerDePartida } from './runnerDePartida';

export interface OpcionesServidor {
  /** Directorio donde `persistenciaPartida.ts` guarda los snapshots. */
  directorio: string;
}

interface CrearPartidaBody {
  gameId: string;
  seed: number;
  region?: RegionId;
  /** Descarta la partida abierta en este proceso (si la hay) y crea una limpia — operación destructiva
   * (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): la interfaz debe confirmarlo con el usuario antes
   * de pedirlo (ver `GameStore.regenerarMundo`). Sin esto, `POST /partidas` sobre un `gameId` ya abierto solo
   * puede RETOMARLO (`cargarOCrear`), nunca tirarlo y empezar de cero. */
  forzar?: boolean;
}

interface ParametrosGameId {
  gameId: string;
}

interface EjecutarComandoBody {
  tipo: string;
  params: unknown;
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
      forzar: { type: 'boolean' },
    },
  },
} as const;

const ESQUEMA_EJECUTAR_COMANDO = {
  body: {
    type: 'object',
    required: ['tipo', 'params'],
    additionalProperties: false,
    properties: {
      tipo: { type: 'string', minLength: 1 },
      params: {},
    },
  },
} as const;

function esTipoComandoValido(tipo: string): tipo is TipoComando {
  return Object.prototype.hasOwnProperty.call(REGISTRO_COMANDOS, tipo);
}

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
    const { gameId, seed, region, forzar } = request.body;
    if (runners.has(gameId)) {
      if (!forzar) {
        return reply.code(409).send({ error: `la partida '${gameId}' ya está abierta en este proceso.` });
      }
      // `forzar`: descarta la partida en curso y crea una limpia — NO reanuda el snapshot existente, a
      // diferencia de `cargarOCrear` de más abajo. Es la única operación destructiva de esta API.
      runners.delete(gameId);
      const runner = RunnerDePartida.crear(gameId, { seed, region }, { directorio: opciones.directorio });
      runners.set(gameId, runner);
      return reply.code(201).send(resumenDe(runner));
    }
    const runner = await RunnerDePartida.cargarOCrear(gameId, { seed, region }, { directorio: opciones.directorio });
    runners.set(gameId, runner);
    return reply.code(201).send(resumenDe(runner));
  });

  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/partidas/:gameId/comandos',
    { schema: ESQUEMA_EJECUTAR_COMANDO },
    async (request, reply) => {
      const runner = runners.get(request.params.gameId);
      if (!runner) {
        return reply.code(404).send({ error: `la partida '${request.params.gameId}' no está abierta en este proceso.` });
      }
      const { tipo, params } = request.body;
      if (!esTipoComandoValido(tipo)) {
        return reply.code(400).send({ error: `tipo de comando desconocido: '${tipo}'.` });
      }
      // Dispatch genérico por nombre: el tipo específico de cada manejador (`P`/`R`) se pierde a propósito
      // aquí — es la frontera entre "comando serializado sin validar" y "comando tipado", igual que en
      // cualquier deserialización de un body HTTP. Sin esquema por comando todavía (queda para Fase C, doc
      // 5): un `params` con la forma equivocada puede llegar a lanzar dentro del manejador en vez de
      // devolver un rechazo limpio — cae en el `catch` de abajo como cualquier otro fallo y se responde 409,
      // no un crash del proceso.
      const manejador = REGISTRO_COMANDOS[tipo] as ManejadorComando<unknown, unknown>;
      try {
        const resultado = await runner.ejecutar(manejador, params, ACTOR_LOCAL);
        return reply.send({ ...resumenDe(runner), resultado });
      } catch (err) {
        // Igual que en /tick: solo un fallo de persistencia llega hasta aquí como excepción.
        return reply.code(409).send({ error: mensajeDe(err) });
      }
    }
  );

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
