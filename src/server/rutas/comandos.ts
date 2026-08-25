// Ejecución de un comando de partida por HTTP, compartida por las dos superficies. Lo que cambia entre
// `/admin/*` y `/jugador/*` no es este flujo sino el ROL con el que se entra, y de eso ya se encargó el
// filtro de superficie antes de llamar aquí.
import type { FastifyReply } from 'fastify';
import { REGISTRO_COMANDOS, type TipoComando } from '../../session/comandos/registro';
import type { ManejadorComando } from '../../session/comandos/tipos';
import { verificarAutorizacion, type ActorDeComando } from '../../session/comandos/autorizacion';
import type { RunnerDePartida } from '../runnerDePartida';
import { mensajeDe, resumenDe } from './contexto';

export interface EjecutarComandoBody {
  tipo: string;
  params: unknown;
}

export const ESQUEMA_EJECUTAR_COMANDO = {
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

/**
 * Valida el tipo, pasa la matriz de autorización y despacha. El actor llega YA resuelto desde la sesión y la
 * membresía — nunca de un id que el cuerpo de la petición afirme tener (doc 2, principio 3).
 *
 * `actorId` es lo que el motor registra como autor: el `jugadorId` de la membresía, o un id derivado del
 * usuario cuando actúa como administrador sin personaje en la partida (así una acción administrativa queda
 * distinguible en el log de una de jugador).
 */
export async function ejecutarComandoHttp(
  reply: FastifyReply,
  runner: RunnerDePartida,
  cuerpo: EjecutarComandoBody,
  actor: ActorDeComando,
  actorId: string
) {
  const { tipo, params } = cuerpo;
  if (!esTipoComandoValido(tipo)) {
    return reply.code(400).send({ error: `tipo de comando desconocido: '${tipo}'.` });
  }

  // `tipo` es `TipoComando` (unión), no un literal: la genérica de `verificarAutorizacion` no infiere un
  // único `T` de una unión, igual que `manejador` de abajo pierde su `P`/`R` en el mismo punto — misma
  // frontera "comando serializado sin validar" que asume el resto de la ruta.
  const chequeo = verificarAutorizacion(tipo, params as any, runner.getState(), actor);
  if (!chequeo.autorizado) {
    return reply.code(403).send({ error: `no autorizado (${chequeo.motivo})` });
  }

  // Dispatch genérico por nombre: el tipo específico de cada manejador (`P`/`R`) se pierde a propósito aquí
  // — es la frontera entre "comando serializado sin validar" y "comando tipado", igual que en cualquier
  // deserialización de un body HTTP. Sin esquema por comando todavía (doc 4, pendiente): un `params` con la
  // forma equivocada puede lanzar dentro del manejador en vez de devolver un rechazo limpio — cae en el
  // `catch` como cualquier otro fallo y responde 409, no un crash del proceso.
  const manejador = REGISTRO_COMANDOS[tipo] as ManejadorComando<unknown, unknown>;
  try {
    const resultado = await runner.ejecutar(manejador, params, actorId);
    return reply.send({ ...resumenDe(runner), resultado });
  } catch (err) {
    // Solo un fallo de persistencia llega hasta aquí como excepción (ver `RunnerDePartida.aplicarYPersistir`).
    return reply.code(409).send({ error: mensajeDe(err) });
  }
}
