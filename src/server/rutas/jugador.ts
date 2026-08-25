// Superficie de JUGADOR (`/jugador/*`), Fase C3. Todo lo que un cliente de jugador necesita: unirse a una
// partida y ejecutar comandos en ella.
//
// NO tiene endpoint de lectura de estado, y es deliberado: hoy el único `GET` de partida devuelve el estado
// completo (todas las facciones, log global), que para un jugador es una fuga. Exponerlo aquí "de momento"
// sería justo la clase de atajo que luego nadie retira. Llega con las proyecciones por audiencia (Fase C4).
//
// Un administrador NO pasa este filtro aunque tenga acceso total a la partida: para jugar hace falta ser
// jugador (doc 5, "Diferencia entre rol técnico y cargo de juego").
import type { FastifyInstance } from 'fastify';
import { puedeJugar } from '../../acceso/rolesDePartida';
import type { ActorDeComando } from '../../session/comandos/autorizacion';
import { ejecutarComandoHttp, ESQUEMA_EJECUTAR_COMANDO, type EjecutarComandoBody } from './comandos';
import {
  partidaNoAbierta,
  resolverActor,
  sinPermiso,
  sinSesion,
  type DependenciasDeRutas,
  type ParametrosGameId,
} from './contexto';

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
  app.post<{ Params: ParametrosGameId }>('/jugador/partidas/:gameId/membresia', async (request, reply) => {
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

  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/jugador/partidas/:gameId/comandos',
    { schema: ESQUEMA_EJECUTAR_COMANDO },
    async (request, reply) => {
      const { gameId } = request.params;
      const resuelto = resolverActor(request, deps, gameId);
      if (!resuelto) return sinSesion(reply);
      if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');

      const runner = deps.partidas.obtener(gameId);
      if (!runner) return partidaNoAbierta(reply, gameId);

      const jugadorId = resuelto.actor.membresia!.jugadorId;
      const actor: ActorDeComando = { rol: 'jugador', jugadorId };
      return ejecutarComandoHttp(reply, runner, request.body, actor, jugadorId ?? resuelto.usuario.id);
    }
  );
}
