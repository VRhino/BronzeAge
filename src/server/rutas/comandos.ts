// Ejecución de un comando de partida por HTTP, compartida por las dos superficies. Lo que cambia entre
// `/admin/*` y `/jugador/*` no es este flujo sino el ROL con el que se entra, y de eso ya se encargó el
// filtro de superficie antes de llamar aquí.
import type { FastifyReply } from 'fastify';
import { REGISTRO_COMANDOS, type TipoComando } from '../../session/comandos/registro';
import type { ManejadorComando } from '../../session/comandos/tipos';
import { verificarAutorizacion, type ActorDeComando } from '../../session/comandos/autorizacion';
import { ESQUEMAS_PARAMS } from '../../session/comandos/esquemas';
import type { RunnerDePartida } from '../runnerDePartida';
import type { HubDeDifusion } from '../difusion/hub';
import { mensajeDe, resumenDe } from './contexto';

export interface EjecutarComandoBody {
  tipo: string;
  params: unknown;
  /** Fase C5 (doc 4: "reconexión sin duplicar comandos"). Opcional: sin ella, cada petición se aplica tal
   * cual, igual que hasta ahora — la idempotencia es una protección que el cliente pide, no un requisito. */
  idempotencyKey?: string;
}

/**
 * Un `body` por comando (Fase C9): `tipo` fija QUÉ forma de `params` aplica (`ESQUEMAS_PARAMS`,
 * `session/comandos/esquemas.ts`), así que la única manera de expresarlo en JSON Schema es un `oneOf` con una
 * rama cerrada por comando — exactamente el mismo problema que un `switch` exhaustivo, resuelto con el mismo
 * mecanismo (`Object.entries` sobre un `Record` exhaustivo, no una lista que se pueda olvidar actualizar).
 *
 * ajv exige que EXACTAMENTE una rama valide: un `tipo` que no sea ninguno de los 30 no matchea ninguna
 * (`tipo: {const: ...}` en cada rama), así que la petición entera falla la validación — 400 antes de que
 * `ejecutarComandoHttp` llegue siquiera a mirar `tipo` (`esTipoComandoValido` de abajo queda como red de
 * seguridad para quien llame a `ejecutarComandoHttp` sin pasar por esta validación de Fastify, no como el
 * camino real de un cliente HTTP).
 */
const RAMAS_POR_COMANDO = Object.entries(ESQUEMAS_PARAMS).map(([tipo, esquemaParams]) => ({
  properties: {
    tipo: { const: tipo },
    params: esquemaParams,
    idempotencyKey: { type: 'string', minLength: 1 },
  },
  required: ['tipo', 'params'],
  additionalProperties: false,
}));

export const ESQUEMA_EJECUTAR_COMANDO = {
  body: { type: 'object', oneOf: RAMAS_POR_COMANDO },
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
 *
 * Tras un comando ACEPTADO, difunde sus eventos por WebSocket (`hub.difundir`, Fase C5) a quien esté
 * suscrito al canal que le corresponda a cada uno. Un comando rechazado no genera eventos que difundir
 * (`ResultadoComando.eventos` viene vacío), así que llamar a `difundir` siempre es seguro sin comprobar
 * `resultado.ok` aparte.
 *
 * `camposExtra` (Fase C6, doc 4: "respuesta de comando autosuficiente") añade campos a la respuesta de
 * ÉXITO sin que esta función tenga que saber qué son — hoy lo usa `/jugador/*` para adjuntar la proyección
 * propia (`session/proyecciones/jugador.ts`) y ahorrarle al cliente el segundo viaje que antes hacía falta
 * (comando + `GET` aparte). `/admin/*` no lo necesita: su `GET` de estado completo es barato de pedir aparte
 * y adjuntarlo aquí también repetiría el problema que esto viene a evitar.
 */
export async function ejecutarComandoHttp(
  reply: FastifyReply,
  runner: RunnerDePartida,
  cuerpo: EjecutarComandoBody,
  actor: ActorDeComando,
  actorId: string,
  hub: HubDeDifusion,
  camposExtra?: (runner: RunnerDePartida) => Record<string, unknown>
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
    const resultado = await runner.ejecutar(manejador, params, actorId, cuerpo.idempotencyKey);
    hub.difundir(runner.gameId, resultado.eventos);
    return reply.send({ ...resumenDe(runner), resultado, ...(camposExtra ? camposExtra(runner) : {}) });
  } catch (err) {
    // Solo un fallo de persistencia llega hasta aquí como excepción (ver `RunnerDePartida.aplicarYPersistir`).
    return reply.code(409).send({ error: mensajeDe(err) });
  }
}
