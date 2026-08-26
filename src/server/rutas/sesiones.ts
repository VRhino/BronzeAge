// Superficie de sesión: login y whoami. No es ni de administración ni de jugador — es la puerta por la que
// se entra a cualquiera de las dos, así que no lleva prefijo propio (más allá de `/v1`).
import type { FastifyInstance } from 'fastify';
import { ProveedorDesconocidoError, autenticar } from '../../acceso/servicioAutenticacion';
import { CredencialInvalidaError } from '../../acceso/proveedorIdentidad';
import { CabeceraAutorizacionInvalidaError, credencialDesdeCabecera } from '../identidad/cabeceraAutorizacion';
import { rolEnPartida } from '../../acceso/rolesDePartida';
import { ESQUEMA_CREDENCIAL_PROVEEDOR, ESQUEMA_SESION_AUTH } from '../openapi';
import { ERROR_RESPUESTA } from './esquemas';
import { mensajeDe, resolverActor, sinSesion, type DependenciasDeRutas } from './contexto';

const ESQUEMA_LOGIN = {
  description: 'Login: verifica la credencial con el proveedor que atiende su esquema y devuelve una Sesion nueva.',
  tags: ['sesiones'],
  security: [{ [ESQUEMA_CREDENCIAL_PROVEEDOR]: [] }],
  response: {
    201: {
      type: 'object',
      properties: {
        usuarioId: { type: 'string' },
        sesionId: { type: 'string' },
        expiraEn: { type: 'string', format: 'date-time' },
      },
      required: ['usuarioId', 'sesionId', 'expiraEn'],
    },
    401: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_WHOAMI = {
  description: 'Whoami. Con `gameId`, informa además del rol con el que actúa en esa partida.',
  tags: ['sesiones'],
  security: [{ [ESQUEMA_SESION_AUTH]: [] }],
  querystring: {
    type: 'object',
    properties: { gameId: { type: 'string' } },
  },
  response: {
    200: {
      type: 'object',
      properties: {
        usuarioId: { type: 'string' },
        esAdministradorGlobal: { type: 'boolean' },
        gameId: { type: 'string' },
        rol: { type: ['string', 'null'] },
        jugadorId: { type: ['string', 'null'] },
      },
      required: ['usuarioId', 'esAdministradorGlobal'],
    },
    401: ERROR_RESPUESTA,
  },
} as const;

export function registrarRutasDeSesion(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /**
   * Login: `Authorization: <esquema-de-proveedor> <credencial>` (ej. `dev ana`, ver
   * `identidad/proveedorDesarrollo.ts`) -> `Usuario` (find-or-create) + `Sesion` nueva. El cliente guarda
   * `sesionId` y lo presenta en el resto de peticiones como `Authorization: sesion <sesionId>`.
   */
  app.post('/sesiones', { schema: ESQUEMA_LOGIN }, async (request, reply) => {
    try {
      const { usuario, sesion } = await autenticar(credencialDesdeCabecera(request.headers.authorization), deps.identidad);
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

  /**
   * Whoami. Con `?gameId=` responde además con qué rol actúa en ESA partida — es lo que permite a un cliente
   * saber qué superficie tiene disponible sin ir probando endpoints y coleccionando 403.
   */
  app.get<{ Querystring: { gameId?: string } }>('/sesiones/actual', { schema: ESQUEMA_WHOAMI }, async (request, reply) => {
    const resuelto = resolverActor(request, deps, request.query.gameId);
    if (!resuelto) return sinSesion(reply);
    return reply.send({
      usuarioId: resuelto.usuario.id,
      esAdministradorGlobal: resuelto.actor.esAdministradorGlobal,
      ...(request.query.gameId === undefined
        ? {}
        : { gameId: request.query.gameId, rol: rolEnPartida(resuelto.actor) ?? null, jugadorId: resuelto.actor.membresia?.jugadorId ?? null }),
    });
  });
}
