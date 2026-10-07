// Gestión de respaldos desde la superficie de administración (Fase E2): verlos, hacer uno ahora, restaurar y borrar. Son la red de
// seguridad del borrado de partidas: los de una partida borrada siguen aquí y restaurar uno la devuelve. Todo exige
// administrador_global, y solo existe con el almacén en disco (`directorioRespaldos`).
import type { FastifyInstance, FastifyReply } from 'fastify';
import { puedeCrearPartida } from '../../acceso/rolesDePartida';
import { ESQUEMA_SESION_AUTH } from '../openapi';
import { borrarRespaldo, gameIdDeRespaldo, listarTodosLosRespaldos, respaldarPartida, restaurarPartida, RespaldoInservibleError } from '../respaldos';
import { listarPartidas } from '../persistenciaPartida';
import { recuperarPartida } from '../identidad/olvidarPartida';
import { ERROR_RESPUESTA, PARAMS_GAME_ID } from './esquemas';
import { resolverActor, resumenDe, sinPermiso, sinSesion, type DependenciasDeRutas, type ParametrosGameId } from './contexto';

const SEGURIDAD = [{ [ESQUEMA_SESION_AUTH]: [] }];
const PROPIEDADES_RESPALDO = { gameId: { type: 'string' }, momento: { type: 'string' }, archivo: { type: 'string' }, bytes: { type: 'integer' } } as const;
const RESPALDO = { type: 'object', properties: PROPIEDADES_RESPALDO, required: ['gameId', 'momento', 'archivo', 'bytes'] } as const;
const ERRORES = { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 501: ERROR_RESPUESTA };
const ARCHIVO = { type: 'object', properties: { archivo: { type: 'string', maxLength: 200 } }, required: ['archivo'] } as const;

export function registrarRutasDeRespaldos(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /** Sesión, rol y almacén en disco: lo común a todas. Devuelve el directorio, o `undefined` si ya ha respondido. */
  const exigir = (request: Parameters<typeof resolverActor>[0], reply: FastifyReply): string | undefined => {
    const resuelto = resolverActor(request, deps);
    if (!resuelto) {
      sinSesion(reply);
      return undefined;
    }
    if (!puedeCrearPartida(resuelto.actor)) {
      sinPermiso(reply, 'gestionar respaldos exige rol administrador_global');
      return undefined;
    }
    if (!deps.directorioRespaldos) {
      reply.code(501).send({ error: 'este servidor no guarda respaldos en disco (almacén remoto).' });
      return undefined;
    }
    return deps.directorioRespaldos;
  };

  app.get(
    '/admin/respaldos',
    {
      schema: {
        description: 'Los respaldos de todas las partidas, también de las borradas, del más reciente al más antiguo. `partidaExiste` dice si la partida sigue en el servidor.',
        tags: ['admin'],
        security: SEGURIDAD,
        response: {
          200: {
            type: 'object',
            properties: {
              respaldos: {
                type: 'array',
                items: { type: 'object', properties: { ...PROPIEDADES_RESPALDO, partidaExiste: { type: 'boolean' } }, required: [...RESPALDO.required, 'partidaExiste'] },
              },
            },
            required: ['respaldos'],
          },
          ...ERRORES,
        },
      },
    },
    async (request, reply) => {
      const directorio = exigir(request, reply);
      if (!directorio) return reply;
      const existen = new Set((await listarPartidas(deps.almacen)).map((p) => p.gameId));
      const respaldos = await listarTodosLosRespaldos(directorio);
      return reply.send({ respaldos: respaldos.map((r) => ({ ...r, partidaExiste: existen.has(r.gameId) })) });
    }
  );

  app.post<{ Params: ParametrosGameId }>(
    '/admin/partidas/:gameId/respaldos',
    { schema: { description: 'Hace un respaldo de la partida ahora.', tags: ['admin'], security: SEGURIDAD, params: PARAMS_GAME_ID, response: { 201: RESPALDO, ...ERRORES } } },
    async (request, reply) => {
      const directorio = exigir(request, reply);
      if (!directorio) return reply;
      const { gameId } = request.params;
      // Abierta: que lo último esté en disco antes de copiarlo (el guardado va por su cola, tras lo pendiente).
      await deps.partidas.obtener(gameId)?.guardar();
      const respaldo = await respaldarPartida(directorio, gameId, deps.ahora());
      if (!respaldo) return reply.code(404).send({ error: `la partida '${gameId}' no existe.` });
      return reply.code(201).send(respaldo);
    }
  );

  app.post<{ Body: { archivo: string } }>(
    '/admin/respaldos/restaurar',
    {
      schema: {
        description:
          'Restaura una partida desde un respaldo (también una ya borrada). Si estaba abierta se cierra antes; después se abre con el estado ' +
          'restaurado y vuelven las membresías que cerró su borrado.',
        tags: ['admin'],
        security: SEGURIDAD,
        body: ARCHIVO,
        response: {
          200: {
            type: 'object',
            properties: { gameId: { type: 'string' }, version: { type: 'integer' }, membresiasReactivadas: { type: 'integer' } },
            required: ['gameId', 'version', 'membresiasReactivadas'],
          },
          ...ERRORES,
        },
      },
    },
    async (request, reply) => {
      const directorio = exigir(request, reply);
      if (!directorio) return reply;
      const { archivo } = request.body;
      const gameId = gameIdDeRespaldo(archivo);
      if (!gameId) return reply.code(400).send({ error: `'${archivo}' no es un nombre de respaldo.` });
      // Restaurar exige la partida cerrada: si no, su runner escribiría encima el estado viejo que tiene en memoria.
      await deps.partidas.cerrarUna(gameId);
      try {
        await restaurarPartida(directorio, gameId, archivo);
      } catch (err) {
        if (err instanceof RespaldoInservibleError) return reply.code(400).send({ error: err.message });
        throw err;
      }
      const runner = await deps.partidas.abrir(gameId, { seed: 1 });
      const conHeroe = new Set(runner.getState().heroes.flatMap((h) => (h.jugadorId ? [h.jugadorId] : [])));
      const membresiasReactivadas = recuperarPartida(deps.identidad.repositorio, gameId, conHeroe);
      return reply.send({ gameId, version: resumenDe(runner).version, membresiasReactivadas });
    }
  );

  app.delete<{ Params: { archivo: string } }>(
    '/admin/respaldos/:archivo',
    {
      schema: {
        description: 'Borra un respaldo y sus adjuntos.',
        tags: ['admin'],
        security: SEGURIDAD,
        params: ARCHIVO,
        response: { 200: { type: 'object', properties: { archivo: { type: 'string' } }, required: ['archivo'] }, ...ERRORES },
      },
    },
    async (request, reply) => {
      const directorio = exigir(request, reply);
      if (!directorio) return reply;
      const { archivo } = request.params;
      if (!(await borrarRespaldo(directorio, archivo))) return reply.code(404).send({ error: `no hay un respaldo '${archivo}'.` });
      return reply.send({ archivo });
    }
  );
}
