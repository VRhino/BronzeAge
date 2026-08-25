// API HTTP administrativa mínima (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): crear partida,
// avanzar tick, consultar estado. `POST /partidas/:gameId/comandos` sí exige sesión y pasa por la matriz de
// autorización (Fase C2, doc 5) — es el único endpoint que ejecuta acciones de JUEGO en nombre de un actor.
//
// `POST /partidas`, `POST /partidas/:gameId/tick` y `GET /partidas/:gameId` siguen SIN exigir sesión: son
// deliberadamente de ADMINISTRACIÓN (crear/avanzar/inspeccionar la partida en bruto) y ya estaban flotando
// sin protección antes de C2 — protegerlas de verdad es CORS + versionado + separación de superficies
// admin/jugador (C3/C6), no algo que quepa colar aquí de paso. `GET /partidas/:gameId` en particular sigue
// devolviendo el estado completo sin proyección — fuga conocida, pendiente de C4.
//
// `crearServidor()` construye la instancia SIN escuchar ningún puerto — eso lo decide el llamador
// (`index.ts` en producción, los tests vía `.inject()`). Es el patrón recomendado de Fastify para probar
// rutas sin abrir sockets de verdad.
import Fastify, { type FastifyInstance } from 'fastify';
import type { RegionId } from '../domain/types';
import { REGISTRO_COMANDOS, type TipoComando } from '../session/comandos/registro';
import type { ManejadorComando } from '../session/comandos/tipos';
import { RunnerDePartida } from './runnerDePartida';
import { proveedoresPorDefecto } from './identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from './identidad/repositorioEnMemoria';
import {
  CabeceraAutorizacionInvalidaError,
  credencialDesdeCabecera,
  credencialOpcionalDesdeCabecera,
} from './identidad/cabeceraAutorizacion';
import { ProveedorDesconocidoError, autenticar, resolverSesion, type ContextoAutenticacion } from '../acceso/servicioAutenticacion';
import { CredencialInvalidaError, crearRegistroProveedores } from '../acceso/proveedorIdentidad';
import { verificarAutorizacion, type ActorDeComando } from '../session/comandos/autorizacion';

export interface OpcionesServidor {
  /** Directorio donde `persistenciaPartida.ts` guarda los snapshots. */
  directorio: string;
  /** Contexto de autenticación (proveedores + repositorio). Por defecto usa solo el proveedor de desarrollo
   * (`identidad/proveedorDesarrollo.ts`) sobre un repositorio en memoria — inyectable para tests o para un
   * proceso real con otros proveedores dados de alta. */
  identidad?: ContextoAutenticacion;
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

  const identidad: ContextoAutenticacion = opciones.identidad ?? {
    proveedores: crearRegistroProveedores(proveedoresPorDefecto()),
    repositorio: crearRepositorioIdentidadEnMemoria(),
  };

  /**
   * Login: `Authorization: <esquema-de-proveedor> <credencial>` (ej. `dev ana`, ver
   * `identidad/proveedorDesarrollo.ts`) -> `Usuario` (find-or-create) + `Sesion` nueva. El cliente guarda
   * `sesionId` y lo presenta en requests futuras como `Authorization: sesion <sesionId>`.
   *
   * Todavía no hay ningún endpoint que EXIJA esta sesión (eso es C2/C3 — autorización por comando y
   * superficies separadas admin/jugador); este es el mecanismo, listo para que esas fases lo consuman.
   */
  app.post('/sesiones', async (request, reply) => {
    try {
      const { usuario, sesion } = await autenticar(credencialDesdeCabecera(request.headers.authorization), identidad);
      return reply.code(201).send({ usuarioId: usuario.id, sesionId: sesion.id, expiraEn: sesion.expiraEn });
    } catch (err) {
      if (
        err instanceof CabeceraAutorizacionInvalidaError ||
        err instanceof ProveedorDesconocidoError ||
        err instanceof CredencialInvalidaError
      ) {
        return reply.code(401).send({ error: mensajeDe(err) });
      }
      throw err;
    }
  });

  /** Whoami: resuelve `Authorization: sesion <sesionId>` a quién es. Sin sesión válida, 401 — nunca lanza. */
  app.get('/sesiones/actual', async (request, reply) => {
    const resuelto = resolverSesion(credencialOpcionalDesdeCabecera(request.headers.authorization), identidad);
    if (!resuelto) return reply.code(401).send({ error: 'sesion ausente, invalida o expirada' });
    return reply.send({ usuarioId: resuelto.usuario.id, sesionId: resuelto.sesion.id, expiraEn: resuelto.sesion.expiraEn });
  });

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

  /**
   * Unirse a una partida como jugador: `Authorization: sesion <id>` -> crea la `Membresia` (rol `'jugador'`)
   * que `/comandos` exige para todo lo que no sea de administración. Un `Usuario` solo puede tener una
   * `Membresia` por `gameId` (doc 5, "Preguntas abiertas" lo asume 1:1) — repetir la llamada tras la primera
   * es 409, no una migración silenciosa.
   *
   * La `Membresia` NO dice a qué Facción pertenece: eso lo decide el juego (`Faccion.ciudadanosIds`) y lo
   * deriva la autorización de ahí. Unirse a la partida y unirse a una Facción son dos hechos distintos.
   *
   * `Jugador` como entidad de almacenamiento propia (doc 5) queda diferida: hoy nada además de la
   * autorización necesita distinguirla de la `Membresia` que ya la referencia, así que crear un puerto de
   * persistencia solo para duplicar `jugadorId` sería infraestructura sin consumidor. Reutiliza el `id` del
   * `Usuario` como `jugadorId` — cumple el contrato del doc ("mismo valor que hoy usa el motor como
   * jugadorId") sin necesitar un generador de ids aparte.
   */
  app.post<{ Params: ParametrosGameId }>('/partidas/:gameId/jugadores', async (request, reply) => {
    const runner = runners.get(request.params.gameId);
    if (!runner) {
      return reply.code(404).send({ error: `la partida '${request.params.gameId}' no está abierta en este proceso.` });
    }
    const resuelto = resolverSesion(credencialOpcionalDesdeCabecera(request.headers.authorization), identidad);
    if (!resuelto) return reply.code(401).send({ error: 'sesion ausente, invalida o expirada' });

    if (identidad.repositorio.obtenerMembresia(resuelto.usuario.id, request.params.gameId)) {
      return reply.code(409).send({ error: 'ya existe una membresia de este usuario en esta partida' });
    }
    const jugadorId = resuelto.usuario.id;
    identidad.repositorio.otorgarMembresia({
      usuarioId: resuelto.usuario.id,
      gameId: request.params.gameId,
      jugadorId,
      rol: 'jugador',
      desde: new Date().toISOString(),
    });
    return reply.code(201).send({ jugadorId });
  });

  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/partidas/:gameId/comandos',
    { schema: ESQUEMA_EJECUTAR_COMANDO },
    async (request, reply) => {
      const runner = runners.get(request.params.gameId);
      if (!runner) {
        return reply.code(404).send({ error: `la partida '${request.params.gameId}' no está abierta en este proceso.` });
      }

      // Fase C2 (doc 5): actor SIEMPRE resuelto desde la sesión + membresía, nunca de un id que el body
      // afirme tener. Sin `Membresia` en esta partida no hay nada que autorizar — 403, no 401 (la sesión en
      // sí es válida; lo que falta es pertenencia a ESTA partida).
      const resuelto = resolverSesion(credencialOpcionalDesdeCabecera(request.headers.authorization), identidad);
      if (!resuelto) return reply.code(401).send({ error: 'sesion ausente, invalida o expirada' });
      const membresia = identidad.repositorio.obtenerMembresia(resuelto.usuario.id, request.params.gameId);
      if (!membresia) return reply.code(403).send({ error: 'sin membresia en esta partida' });

      const { tipo, params } = request.body;
      if (!esTipoComandoValido(tipo)) {
        return reply.code(400).send({ error: `tipo de comando desconocido: '${tipo}'.` });
      }

      const actor: ActorDeComando = { rol: membresia.rol, jugadorId: membresia.jugadorId };
      // `tipo` es `TipoComando` (unión), no un literal: la genérica de `verificarAutorizacion` no infiere un
      // único `T` de una unión, igual que `manejador` de abajo pierde su `P`/`R` en el mismo punto — misma
      // frontera "comando serializado sin validar" que ya asume el resto de la ruta.
      const chequeo = verificarAutorizacion(tipo, params as any, runner.getState(), actor);
      if (!chequeo.autorizado) {
        return reply.code(403).send({ error: `no autorizado (${chequeo.motivo})` });
      }

      // Dispatch genérico por nombre: el tipo específico de cada manejador (`P`/`R`) se pierde a propósito
      // aquí — es la frontera entre "comando serializado sin validar" y "comando tipado", igual que en
      // cualquier deserialización de un body HTTP. Sin esquema por comando todavía (queda para Fase C, doc
      // 5): un `params` con la forma equivocada puede llegar a lanzar dentro del manejador en vez de
      // devolver un rechazo limpio — cae en el `catch` de abajo como cualquier otro fallo y se responde 409,
      // no un crash del proceso.
      const manejador = REGISTRO_COMANDOS[tipo] as ManejadorComando<unknown, unknown>;
      const actorId = membresia.jugadorId ?? `admin:${resuelto.usuario.id}`;
      try {
        const resultado = await runner.ejecutar(manejador, params, actorId);
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
