// Superficie de sesión: login y whoami. No es ni de administración ni de jugador — es la puerta por la que
// se entra a cualquiera de las dos, así que no lleva prefijo.
import type { FastifyInstance } from 'fastify';
import { ProveedorDesconocidoError, autenticar } from '../../acceso/servicioAutenticacion';
import { CredencialInvalidaError } from '../../acceso/proveedorIdentidad';
import { CabeceraAutorizacionInvalidaError, credencialDesdeCabecera } from '../identidad/cabeceraAutorizacion';
import { rolEnPartida } from '../../acceso/rolesDePartida';
import { mensajeDe, resolverActor, sinSesion, type DependenciasDeRutas } from './contexto';

export function registrarRutasDeSesion(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /**
   * Login: `Authorization: <esquema-de-proveedor> <credencial>` (ej. `dev ana`, ver
   * `identidad/proveedorDesarrollo.ts`) -> `Usuario` (find-or-create) + `Sesion` nueva. El cliente guarda
   * `sesionId` y lo presenta en el resto de peticiones como `Authorization: sesion <sesionId>`.
   */
  app.post('/sesiones', async (request, reply) => {
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
  app.get<{ Querystring: { gameId?: string } }>('/sesiones/actual', async (request, reply) => {
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
