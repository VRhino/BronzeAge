// Superficie del SERVIDOR DE BATALLA de Conquest (doc 02 §3.2-§3.3): recoger los tickets y reportar asignación, inicio
// y tokens. No es `/admin` ni `/jugador`: entra con la credencial de un servidor declarado en `SERVIDORES_BATALLA`,
// nunca con la sesión de una persona. La ruta con la que cada jugador recoge SU token (§3.4) vive aquí también, porque
// es la otra mitad del mismo reparto: el secreto de cada uno nunca viaja por el canal colectivo.
//
// `battleId` es único entre partidas, así que estas rutas no llevan `gameId`: la batalla se busca en las partidas
// abiertas en este proceso. `ponytail:` recorrido lineal; un índice `battleId -> gameId` si llega a haber muchas.
import { readFileSync } from 'node:fs';
import Ajv from 'ajv';
import type { FastifyInstance, FastifyReply, FastifyRequest } from 'fastify';
import { puedeJugar } from '../../acceso/rolesDePartida';
import { SCHEMA_VERSION, type BattleServerAssignment, type InicioBatalla, type TokensBatalla } from '../../contratos/v1/dto';
import { batallasActivas, participacionesDe, type Batalla } from '../../session/batallas';
import { confirmarInicio, registrarAsignacion, registrarTokens, type ParamsDeServidor } from '../../session/comandos/batalla';
import { CODIGOS_ERROR } from '../../session/comandos/codigosDeError';
import type { ManejadorComando } from '../../session/comandos/tipos';
import { instanteDeTick } from '../../session/estado';
import { ESQUEMA_SERVIDOR_BATALLA, servidorDeCabecera } from '../identidad/servidoresDeBatalla';
import type { RunnerDePartida } from '../runnerDePartida';
import { ESQUEMA_SERVIDOR_BATALLA_AUTH, ESQUEMA_SESION_AUTH } from '../openapi';
import { ERROR_RESPUESTA } from './esquemas';
import { mensajeDe, partidaNoAbierta, resolverActor, sinPermiso, sinSesion, type DependenciasDeRutas } from './contexto';

/** El mismo schema que deserializa Conquest (doc 02 §6): lo que entra por aquí se valida contra él, no contra una copia. */
const ajv = new Ajv({ allErrors: true, strict: true, strictRequired: false, allowUnionTypes: true });
ajv.addSchema(JSON.parse(readFileSync(new URL('../../contratos/v1/contratos.schema.json', import.meta.url), 'utf8')), 'v1');

interface ParamsBatalla {
  battleId: string;
}

const PARAMS_BATALLA = { type: 'object', properties: { battleId: { type: 'string' } }, required: ['battleId'] } as const;

/** Un 409 por versión de contrato lleva además la que se esperaba (doc 02 §5): sin declararla, Fastify la recortaría. */
const ERROR_O_VERSION = {
  type: 'object',
  properties: { error: { type: 'string' }, schemaVersionEsperada: { type: 'integer' } },
  required: ['error'],
} as const;

function esquemaDeServidor(description: string) {
  return {
    description,
    tags: ['batallas'],
    security: [{ [ESQUEMA_SERVIDOR_BATALLA_AUTH]: [] }],
    params: PARAMS_BATALLA,
    response: { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_O_VERSION },
  };
}

const ESQUEMA_PENDIENTES = {
  description: "Batallas en 'convocando' sin servidor todavía (doc 02 §3.2). El orquestador de Conquest hace polling; sin efectos.",
  tags: ['batallas'],
  security: [{ [ESQUEMA_SERVIDOR_BATALLA_AUTH]: [] }],
  response: { 401: ERROR_RESPUESTA },
} as const;

const ESQUEMA_ASIGNACION_JUGADOR = {
  description: 'Tu token para entrar en una batalla en la que combates (doc 02 §3.4). 404 mientras no haya asignación para ti.',
  tags: ['jugador'],
  security: [{ [ESQUEMA_SESION_AUTH]: [] }],
  params: {
    type: 'object',
    properties: { gameId: { type: 'string' }, battleId: { type: 'string' } },
    required: ['gameId', 'battleId'],
  },
  response: {
    200: {
      type: 'object',
      properties: {
        battleId: { type: 'string' },
        instancia: {
          type: 'object',
          properties: { host: { type: 'string' }, puerto: { type: 'integer' }, protocolo: { type: 'string' } },
          required: ['host', 'puerto', 'protocolo'],
        },
        token: { type: 'string' },
        expiraEn: { type: 'string' },
      },
      required: ['battleId', 'instancia', 'token', 'expiraEn'],
    },
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    404: ERROR_RESPUESTA,
  },
} as const;

export function registrarRutasDeBatalla(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /** El servidor declarado que presenta la cabecera; si no lo es, responde 401 y devuelve `undefined`. */
  const servidor = (request: FastifyRequest, reply: FastifyReply): string | undefined => {
    const id = servidorDeCabecera(request.headers.authorization, deps.servidoresBatalla);
    if (!id) void reply.code(401).send({ error: `credencial de servidor de batalla ausente o no declarada ('${ESQUEMA_SERVIDOR_BATALLA} <token>')` });
    return id;
  };

  const buscar = (battleId: string): { runner: RunnerDePartida; batalla: Batalla } | undefined => {
    for (const runner of deps.partidas.abiertas()) {
      const batalla = runner.getState().batallas.find((b) => b.id === battleId);
      if (batalla) return { runner, batalla };
    }
    return undefined;
  };

  const noExiste = (reply: FastifyReply, battleId: string) =>
    reply.code(404).send({ error: `la batalla '${battleId}' no está en ninguna partida abierta.` });

  app.get('/batallas/pendientes', { schema: ESQUEMA_PENDIENTES }, async (request, reply) => {
    if (!servidor(request, reply)) return reply;
    const batallas = deps.partidas.abiertas().flatMap((runner) => {
      const estado = runner.getState();
      return batallasActivas(estado, instanteDeTick(estado.tick))
        .filter((b) => b.estado === 'convocando')
        .map((b) => ({ battleId: b.id, gameId: runner.gameId, ticketRevision: b.ticket.ticketRevision }));
    });
    return reply.send({ batallas });
  });

  app.get<{ Params: ParamsBatalla }>(
    '/batallas/:battleId/ticket',
    { schema: esquemaDeServidor('El BattleTicket vigente (doc 02 §3.2), recuperable en cualquier momento.') },
    async (request, reply) => {
      if (!servidor(request, reply)) return reply;
      const hallada = buscar(request.params.battleId);
      return hallada ? reply.send(hallada.batalla.ticket) : noExiste(reply, request.params.battleId);
    }
  );

  app.get<{ Params: ParamsBatalla }>(
    '/batallas/:battleId/incorporaciones',
    { schema: esquemaDeServidor('Los héroes que se han unido después del ticket, en orden (IncorporacionBatalla, doc 01 §15).') },
    async (request, reply) => {
      if (!servidor(request, reply)) return reply;
      const hallada = buscar(request.params.battleId);
      return hallada ? reply.send({ incorporaciones: hallada.batalla.incorporaciones }) : noExiste(reply, request.params.battleId);
    }
  );

  /** Comprueba forma y versión del mensaje contra el contrato y lo aplica en la cola de su partida. */
  const recibir =
    <M extends { schemaVersion: number; battleId: string }>(definicion: string, manejador: ManejadorComando<ParamsDeServidor<M>, void>) =>
    async (request: FastifyRequest<{ Params: ParamsBatalla }>, reply: FastifyReply) => {
      const servidorId = servidor(request, reply);
      if (!servidorId) return reply;
      const cuerpo = request.body as Partial<M> | undefined;
      if (cuerpo?.schemaVersion !== SCHEMA_VERSION) {
        return reply.code(409).send({ error: 'schemaVersion no reconocida.', schemaVersionEsperada: SCHEMA_VERSION });
      }
      const validar = ajv.getSchema(`v1#/definitions/${definicion}`)!;
      if (!validar(cuerpo)) return reply.code(400).send({ error: ajv.errorsText(validar.errors) });
      if (cuerpo.battleId !== request.params.battleId) return reply.code(400).send({ error: 'el battleId del cuerpo no es el de la ruta.' });

      const hallada = buscar(request.params.battleId);
      if (!hallada) return noExiste(reply, request.params.battleId);
      try {
        const resultado = await hallada.runner.ejecutar(manejador, { servidorId, mensaje: cuerpo as M }, `${ESQUEMA_SERVIDOR_BATALLA}:${servidorId}`);
        deps.hub.difundir(hallada.runner.gameId, resultado.eventos);
        if (!resultado.ok) {
          return reply.code(resultado.codigoError === CODIGOS_ERROR.batallaNoExiste ? 404 : 409).send({ error: resultado.codigoError ?? 'rechazado' });
        }
        const estado = hallada.runner.getState().batallas.find((b) => b.id === hallada.batalla.id)!.estado;
        return reply.send({ battleId: hallada.batalla.id, estado });
      } catch (err) {
        // Solo un fallo al persistir llega aquí, y entonces no se aplicó nada (`RunnerDePartida.aplicarYPersistir`).
        return reply.code(409).send({ error: mensajeDe(err) });
      }
    };

  app.post(
    '/batallas/:battleId/asignacion',
    { schema: esquemaDeServidor('Conquest reporta su BattleServerAssignment (doc 02 §3.3). Gana la primera; repetirla es idempotente.') },
    recibir<BattleServerAssignment>('BattleServerAssignment', registrarAsignacion)
  );
  app.post(
    '/batallas/:battleId/inicio',
    { schema: esquemaDeServidor('Conquest confirma que la partida real empezó (InicioBatalla, doc 02 §3.3).') },
    recibir<InicioBatalla>('InicioBatalla', confirmarInicio)
  );
  app.post(
    '/batallas/:battleId/tokens',
    { schema: esquemaDeServidor('Tokens de los humanos que se unieron después de la asignación (TokensBatalla, doc 02 §3.3).') },
    recibir<TokensBatalla>('TokensBatalla', registrarTokens)
  );

  app.get<{ Params: { gameId: string; battleId: string } }>(
    '/jugador/partidas/:gameId/batallas/:battleId/asignacion',
    { schema: ESQUEMA_ASIGNACION_JUGADOR },
    async (request, reply) => {
      const { gameId, battleId } = request.params;
      const resuelto = resolverActor(request, deps, gameId);
      if (!resuelto) return sinSesion(reply);
      if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');
      const runner = deps.partidas.obtener(gameId);
      if (!runner) return partidaNoAbierta(reply, gameId);

      const estado = runner.getState();
      const batalla = batallasActivas(estado, instanteDeTick(estado.tick)).find((b) => b.id === battleId);
      if (!batalla) return reply.code(404).send({ error: `no hay ninguna batalla activa '${battleId}' en esta partida.` });
      const heroe = estado.heroes.find((h) => h.jugadorId === resuelto.actor.membresia!.jugadorId);
      if (!heroe || !participacionesDe(batalla).some((p) => p.participante.heroeId === heroe.id)) {
        return sinPermiso(reply, 'no combates en esta batalla');
      }
      const token = batalla.asignacion?.tokensParticipante.find((t) => t.heroeId === heroe.id);
      if (!batalla.asignacion || !token) return reply.code(404).send({ error: 'todavía no hay asignación para ti en esta batalla.' });
      return reply.send({ battleId, instancia: batalla.asignacion.instancia, token: token.token, expiraEn: token.expiraEn });
    }
  );
}
