// Superficie de ADMINISTRACIÓN (`/admin/*`), Fase C3. Gobierna la partida como objeto: crearla/reabrirla,
// avanzar su tick, inspeccionar su estado completo.
//
// Requiere `administrador_global` (para lo que es de instancia) o una `Membresia` de administración en esa
// partida concreta (`administrador_partida`/`moderador`). La política de qué rol alcanza qué está en
// `acceso/rolesDePartida.ts`; aquí solo se cablea a rutas y códigos HTTP.
//
// Antes de C3 estos tres endpoints vivían sin prefijo y SIN autenticar (`POST /partidas`, `/tick`,
// `GET /partidas/:gameId`): cualquiera con acceso a red podía crear una partida, avanzarla o leerla entera.
// Separar las superficies es lo que permite cerrarlos sin romper la de jugador.
import type { FastifyInstance } from 'fastify';
import type { RegionId } from '../../domain/types';
import { puedeAdministrar, puedeCrearPartida, puedeDescartarPartida, rolEnPartida } from '../../acceso/rolesDePartida';
import type { RolTecnico } from '../../acceso/tipos';
import type { ActorDeComando } from '../../session/comandos/autorizacion';
import { PartidaYaAbiertaError } from '../registroDePartidas';
import { vistaAdminDeEstado } from '../../session/estado';
import { ESQUEMA_SESION_AUTH } from '../openapi';
import { ejecutarComandoHttp, ESQUEMA_EJECUTAR_COMANDO, type EjecutarComandoBody } from './comandos';
import { enviarMapa, ESQUEMA_MAPA } from './mapa';
import { ERROR_RESPUESTA, PARAMS_GAME_ID, RESUMEN_PARTIDA_RESPUESTA } from './esquemas';
import {
  mensajeDe,
  partidaNoAbierta,
  resolverActor,
  resumenDe,
  sinPermiso,
  sinSesion,
  type DependenciasDeRutas,
  type ParametrosGameId,
} from './contexto';

const SEGURIDAD_ADMIN = [{ [ESQUEMA_SESION_AUTH]: [] }];

interface CrearPartidaBody {
  gameId: string;
  seed: number;
  region?: RegionId;
  /** Descarta la partida abierta en este proceso (si la hay) y crea una limpia — operación destructiva: la
   * interfaz debe confirmarlo con el usuario antes de pedirlo. Exige `administrador_partida` o
   * `administrador_global`; un `moderador` NO puede (doc 5: sin acceso a regeneración de mundo). */
  forzar?: boolean;
}

const REGIONES: readonly RegionId[] = ['greciaContinental', 'anatolia', 'egeo', 'nilo', 'mesopotamia'];

const ESQUEMA_CREAR_PARTIDA = {
  description: 'Crea una partida nueva, o retoma la que ya hubiera en disco para ese gameId. Exige administrador_global.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
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
  response: {
    201: RESUMEN_PARTIDA_RESPUESTA,
    400: ERROR_RESPUESTA,
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    409: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_TICK = {
  description: 'Avanza un tick de la partida (auto-comercio + turno del NPC de gobernanza incluidos).',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_RESPUESTA },
} as const;

const ESQUEMA_ESTADO_COMPLETO = {
  description:
    'Estado COMPLETO de la partida, sin proyectar por audiencia (todas las Facciones, log global). ' +
    'Sin `mapa` (Fase C11): trae `mapaId` en su lugar — el mapa real se pide una vez por ' +
    'GET .../mapa/:mapaId, cacheable para siempre. ' +
    'Cuerpo de la respuesta no modelado en este esquema por su tamaño y forma variable (Fase C6, doc 4).',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA },
} as const;

const ESQUEMA_COMANDOS_ADMIN = {
  ...ESQUEMA_EJECUTAR_COMANDO,
  description:
    'Ejecuta un comando de partida con rol de administración. La matriz de autorización sigue mandando: casi ' +
    'todos los comandos exigen rol jugador y aquí se rechazan (403 rol_insuficiente) — tener acceso técnico ' +
    'no concede autoridad dentro del juego.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_RESPUESTA },
} as const;

export function registrarRutasDeAdmin(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /**
   * Crea una partida nueva, o RETOMA la que ya hubiera en disco para ese `gameId`. Operación de INSTANCIA:
   * solo `administrador_global`, porque en una partida que aún no existe no hay `Membresia` posible.
   *
   * Al crearla, otorga a quien la crea `Membresia` de `administrador_partida` sobre ella. Sin eso, un
   * administrador global no podría ejecutar ni siquiera los comandos que la matriz le reserva
   * (`alternarFaccionNpc`), porque esa matriz razona sobre roles DE PARTIDA.
   */
  app.post<{ Body: CrearPartidaBody }>('/admin/partidas', { schema: ESQUEMA_CREAR_PARTIDA }, async (request, reply) => {
    const { gameId, seed, region, forzar } = request.body;
    const resuelto = resolverActor(request, deps, gameId);
    if (!resuelto) return sinSesion(reply);
    if (!puedeCrearPartida(resuelto.actor)) {
      return sinPermiso(reply, 'crear o reabrir una partida exige rol administrador_global');
    }
    if (forzar && !puedeDescartarPartida(resuelto.actor)) {
      return sinPermiso(reply, 'descartar una partida exige rol administrador_partida o administrador_global');
    }

    const runner = forzar
      ? deps.partidas.descartarYCrear(gameId, { seed, region })
      : await deps.partidas.abrir(gameId, { seed, region }).catch((err: unknown) => {
          if (err instanceof PartidaYaAbiertaError) return undefined;
          throw err;
        });
    if (!runner) return reply.code(409).send({ error: `la partida '${gameId}' ya está abierta en este proceso.` });

    otorgarAdministracion(deps, resuelto.actor.usuarioId, gameId);
    return reply.code(201).send(resumenDe(runner));
  });

  app.post<{ Params: ParametrosGameId }>('/admin/partidas/:gameId/tick', { schema: ESQUEMA_TICK }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;

    try {
      const resultado = await acceso.runner.avanzarTick();
      deps.hub.difundir(acceso.runner.gameId, resultado.eventos);
      return reply.send({ ...resumenDe(acceso.runner), resultado });
    } catch (err) {
      // La única forma en que `avanzarTick` puede rechazar (no `resultado.ok === false`, que ya viene dentro
      // de `resultado`) es un fallo de la capa de persistencia.
      return reply.code(409).send({ error: mensajeDe(err) });
    }
  });

  /** Estado COMPLETO de la partida, sin proyección: todas las facciones, log global. Es exactamente por eso
   * que vive tras `/admin/*` — para un jugador sería una fuga (proyecciones por audiencia: Fase C4).
   * Sin `mapa` (Fase C11): viaja `mapaId`, y el mapa real se pide una vez por `GET .../mapa/:mapaId`.
   * `preciosReferencia` (auditoría de doc 9) se fusiona aquí, no en `vistaAdminDeEstado`: es la pieza impura
   * (caché con TTL de un minuto real) que solo `RunnerDePartida` puede calcular. */
  app.get<{ Params: ParametrosGameId }>('/admin/partidas/:gameId', { schema: ESQUEMA_ESTADO_COMPLETO }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;
    return reply.send({ ...vistaAdminDeEstado(acceso.runner.getState()), preciosReferencia: acceso.runner.preciosReferencia() });
  });

  /** El mapa como asset (Fase C11) — ver `mapa.ts`. Misma comprobación de administración que el resto de esta
   * superficie: el mapa no es secreto, pero la partida sí exige sesión para entrar en su gameId. */
  app.get<{ Params: ParametrosGameId & { mapaId: string } }>('/admin/partidas/:gameId/mapa/:mapaId', { schema: ESQUEMA_MAPA }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;
    enviarMapa(reply, acceso.runner);
  });

  /**
   * Comandos ejecutados como administrador. La matriz sigue mandando: casi todos los comandos son de rol
   * `jugador` y aquí se rechazarán con `rol_insuficiente`, que es lo correcto — tener acceso técnico no
   * concede autoridad dentro del juego (doc 5). Hoy solo `alternarFaccionNpc` admite administración.
   */
  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/admin/partidas/:gameId/comandos',
    { schema: ESQUEMA_COMANDOS_ADMIN },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;

      const rol = rolEnPartida(acceso.actorInstancia) as RolTecnico;
      const actor: ActorDeComando = { rol, jugadorId: acceso.actorInstancia.membresia?.jugadorId ?? null };
      // Un administrador sin personaje en la partida queda registrado como `admin:<usuarioId>`, para que su
      // huella en el log no se confunda con la de un jugador.
      const actorId = acceso.actorInstancia.membresia?.jugadorId ?? `admin:${acceso.actorInstancia.usuarioId}`;
      return ejecutarComandoHttp(reply, acceso.runner, request.body, actor, actorId, deps.hub);
    }
  );
}

/** `Membresia` de administración para quien abre la partida. Idempotente: reabrir una partida ya
 * administrada por ese usuario no duplica ni degrada nada. */
function otorgarAdministracion(deps: DependenciasDeRutas, usuarioId: string, gameId: string): void {
  if (deps.identidad.repositorio.obtenerMembresia(usuarioId, gameId)) return;
  deps.identidad.repositorio.otorgarMembresia({
    usuarioId,
    gameId,
    jugadorId: null,
    rol: 'administrador_partida',
    desde: deps.ahora(),
  });
}

type AccesoAdmin =
  | { ok: true; runner: import('../runnerDePartida').RunnerDePartida; actorInstancia: import('../../acceso/rolesDePartida').ActorDeInstancia }
  | { ok: false; respuesta: unknown };

/** Sesión + partida abierta + permiso de administración, en el orden en que importa: sin sesión no se
 * revela si la partida existe. */
function exigirAdministracion(
  request: { headers: Record<string, unknown>; params: unknown },
  reply: Parameters<typeof sinSesion>[0],
  deps: DependenciasDeRutas
): AccesoAdmin {
  const { gameId } = request.params as ParametrosGameId;
  const resuelto = resolverActor(request as never, deps, gameId);
  if (!resuelto) return { ok: false, respuesta: sinSesion(reply) };
  if (!puedeAdministrar(resuelto.actor)) {
    return { ok: false, respuesta: sinPermiso(reply, 'sin rol de administracion en esta partida') };
  }
  const runner = deps.partidas.obtener(gameId);
  if (!runner) return { ok: false, respuesta: partidaNoAbierta(reply, gameId) };
  return { ok: true, runner, actorInstancia: resuelto.actor };
}
