// Superficie de JUGADOR (`/jugador/*`). Todo lo que un cliente de jugador necesita: unirse a una partida,
// leer su proyección del estado, y ejecutar comandos en ella.
//
// El `GET` de lectura (Fase C4, Slice 1) NO es el estado completo: pasa por `proyectarParaJugador`, que hoy
// solo expone la Facción propia — nada de las demás. Es deliberadamente conservador, ver el comentario de
// cabecera de `session/proyecciones/jugador.ts` para el porqué (la niebla de guerra completa del doc 6
// depende de un número de balance que no está definido en ningún doc de este repo).
//
// Un administrador NO pasa este filtro aunque tenga acceso total a la partida: para jugar hace falta ser
// jugador (doc 5, "Diferencia entre rol técnico y cargo de juego").
import type { FastifyInstance } from 'fastify';
import { puedeJugar } from '../../acceso/rolesDePartida';
import type { ActorDeComando } from '../../session/comandos/autorizacion';
import { proyectarParaJugador } from '../../session/proyecciones/jugador';
import { ESQUEMA_SESION_AUTH } from '../openapi';
import { ejecutarComandoHttp, ESQUEMA_EJECUTAR_COMANDO, type EjecutarComandoBody } from './comandos';
import { enviarMapa, ESQUEMA_MAPA } from './mapa';
import { ERROR_RESPUESTA, PARAMS_GAME_ID } from './esquemas';
import {
  partidaNoAbierta,
  resolverActor,
  sinPermiso,
  sinSesion,
  type DependenciasDeRutas,
  type ParametrosGameId,
} from './contexto';

const SEGURIDAD_JUGADOR = [{ [ESQUEMA_SESION_AUTH]: [] }];

const ESQUEMA_MEMBRESIA = {
  description: 'Unirse a una partida como jugador: crea la Membresia (rol jugador) que exige el resto de esta superficie.',
  tags: ['jugador'],
  security: SEGURIDAD_JUGADOR,
  params: PARAMS_GAME_ID,
  response: {
    201: {
      type: 'object',
      properties: { jugadorId: { type: 'string' } },
      required: ['jugadorId'],
    },
    401: ERROR_RESPUESTA,
    404: ERROR_RESPUESTA,
    409: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_PROYECCION = {
  description:
    'Proyección del jugador (Fase C4 Slice 1): su Facción completa, las demás solo con metadatos públicos. ' +
    'Sin `mapa` (Fase C11): trae `mapaId` en su lugar — ver GET .../mapa/:mapaId. ' +
    'Cuerpo no modelado en este esquema por su tamaño y forma variable (ver session/proyecciones/jugador.ts).',
  tags: ['jugador'],
  security: SEGURIDAD_JUGADOR,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA },
} as const;

const ESQUEMA_COMANDOS_JUGADOR = {
  ...ESQUEMA_EJECUTAR_COMANDO,
  description:
    'Ejecuta un comando de partida como jugador. La respuesta incluye `proyeccion` (Fase C6: "respuesta ' +
    'autosuficiente") con el estado propio ya actualizado, para no necesitar un GET aparte tras cada comando.',
  tags: ['jugador'],
  security: SEGURIDAD_JUGADOR,
  params: PARAMS_GAME_ID,
  response: { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_RESPUESTA },
} as const;

export function registrarRutasDeJugador(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /**
   * Unirse a una partida como jugador: crea la `Membresia` (rol `'jugador'`) que exige el resto de esta
   * superficie. Un `Usuario` solo puede tener una `Membresia` por partida (doc 5) — repetir la llamada es
   * 409, no una migración silenciosa de rol.
   *
   * La `Membresia` NO dice a qué Facción pertenece: eso lo decide el juego (`Faccion.ciudadanosIds`) y la
   * autorización lo deriva de ahí. Unirse a la partida y unirse a una Facción son hechos distintos.
   *
   * Reutiliza el `id` del `Usuario` como `jugadorId` — cumple el contrato del doc 5 ("mismo valor que hoy
   * usa el motor como jugadorId") sin necesitar un generador de ids aparte.
   */
  app.post<{ Params: ParametrosGameId }>('/jugador/partidas/:gameId/membresia', { schema: ESQUEMA_MEMBRESIA }, async (request, reply) => {
    const { gameId } = request.params;
    const resuelto = resolverActor(request, deps, gameId);
    if (!resuelto) return sinSesion(reply);
    if (!deps.partidas.obtener(gameId)) return partidaNoAbierta(reply, gameId);

    if (deps.identidad.repositorio.obtenerMembresia(resuelto.usuario.id, gameId)) {
      return reply.code(409).send({ error: 'ya existe una membresia de este usuario en esta partida' });
    }
    const jugadorId = resuelto.usuario.id;
    deps.identidad.repositorio.otorgarMembresia({
      usuarioId: resuelto.usuario.id,
      gameId,
      jugadorId,
      rol: 'jugador',
      desde: deps.ahora(),
    });
    return reply.code(201).send({ jugadorId });
  });

  /**
   * Proyección de jugador (Fase C4, Slice 1): la Facción propia completa, las demás solo con sus metadatos
   * públicos — nunca sus asentamientos. No es el mismo endpoint que `/admin/partidas/:gameId`: aquel expone
   * `GameSessionState` en bruto, este siempre pasa por `proyectarParaJugador`.
   */
  app.get<{ Params: ParametrosGameId }>('/jugador/partidas/:gameId', { schema: ESQUEMA_PROYECCION }, async (request, reply) => {
    const { gameId } = request.params;
    const resuelto = resolverActor(request, deps, gameId);
    if (!resuelto) return sinSesion(reply);
    if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');

    const runner = deps.partidas.obtener(gameId);
    if (!runner) return partidaNoAbierta(reply, gameId);

    const jugadorId = resuelto.actor.membresia!.jugadorId!;
    return reply.send(proyectarParaJugador(runner.getState(), jugadorId));
  });

  /** El mapa como asset (Fase C11) — ver `mapa.ts`. La proyección solo trae `mapaId`; el mapa real se pide
   * aquí, una vez, y se cachea para siempre en el cliente. */
  app.get<{ Params: ParametrosGameId & { mapaId: string } }>('/jugador/partidas/:gameId/mapa/:mapaId', { schema: ESQUEMA_MAPA }, async (request, reply) => {
    const { gameId } = request.params;
    const resuelto = resolverActor(request, deps, gameId);
    if (!resuelto) return sinSesion(reply);
    if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');

    const runner = deps.partidas.obtener(gameId);
    if (!runner) return partidaNoAbierta(reply, gameId);
    enviarMapa(reply, runner);
  });

  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/jugador/partidas/:gameId/comandos',
    { schema: ESQUEMA_COMANDOS_JUGADOR },
    async (request, reply) => {
      const { gameId } = request.params;
      const resuelto = resolverActor(request, deps, gameId);
      if (!resuelto) return sinSesion(reply);
      if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');

      const runner = deps.partidas.obtener(gameId);
      if (!runner) return partidaNoAbierta(reply, gameId);

      const jugadorId = resuelto.actor.membresia!.jugadorId!;
      const actor: ActorDeComando = { rol: 'jugador', jugadorId };
      return ejecutarComandoHttp(reply, runner, request.body, actor, jugadorId, deps.hub, (r) => ({
        proyeccion: proyectarParaJugador(r.getState(), jugadorId),
      }));
    }
  );
}
