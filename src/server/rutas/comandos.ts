// Ejecución de un comando de partida por HTTP, compartida por las dos superficies. Lo que cambia entre
// `/admin/*` y `/jugador/*` no es este flujo sino el ROL con el que se entra, y de eso ya se encargó el
// filtro de superficie antes de llamar aquí.
import type { FastifyError, FastifyReply, FastifyRequest } from 'fastify';
import { REGISTRO_COMANDOS, type TipoComando } from '../../session/comandos/registro';
import type { ManejadorComando } from '../../session/comandos/tipos';
import { verificarAutorizacion, type ActorDeComando } from '../../session/comandos/autorizacion';
import { ESQUEMAS_PARAMS } from '../../session/comandos/esquemas';
import type { RunnerDePartida } from '../runnerDePartida';
import type { HubDeDifusion } from '../difusion/hub';
import type { RegistroDeAuditoria } from '../auditoria';
import { mensajeDe, resolverActor, resumenDe, type DependenciasDeRutas, type ParametrosGameId } from './contexto';

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
 * Gancho `onError` para las rutas de comandos (Fase E2): audita los rechazos que NUNCA llegan a
 * `ejecutarComandoHttp`.
 *
 * Existe porque ajv valida el cuerpo ANTES del manejador (`ESQUEMA_EJECUTAR_COMANDO`, Fase C9): un `params`
 * con la forma equivocada se responde con 400 sin que la ruta llegue a ejecutarse, así que sin este gancho
 * ese rechazo sería el único invisible para la auditoría. Como el `oneOf` cierra por `tipo`, un `tipo`
 * inexistente cae también aquí — no en la red de seguridad de `esTipoComandoValido`, que solo alcanza a
 * quien llame a `ejecutarComandoHttp` sin pasar por Fastify.
 *
 * **Solo errores de validación** (`error.validation`, que Fastify rellena únicamente en ese caso). Un fallo
 * inesperado de otra clase no se audita como si fuera un cliente mandando basura: sería mentir sobre la
 * causa, y `ejecutarComandoHttp` ya registra los fallos que sí sabe explicar.
 *
 * El actor se resuelve igual que en la ruta —de la sesión, nunca del cuerpo— y si no hay sesión válida se
 * anota `'anonimo'`: una petición malformada sin credenciales sigue siendo un dato de operación, y perderla
 * dejaría ciego justo al caso de alguien sondeando la API sin haberse autenticado.
 */
export function auditarRechazoDeEsquema(deps: DependenciasDeRutas) {
  return async (request: FastifyRequest, _reply: FastifyReply, error: FastifyError): Promise<void> => {
    if (error.validation === undefined) return;
    const gameId = (request.params as Partial<ParametrosGameId>).gameId ?? '(sin gameId)';
    const resuelto = resolverActor(request, deps, gameId);
    const cuerpo = request.body as Partial<EjecutarComandoBody> | undefined;
    deps.auditoria.registrar({
      gameId,
      actor: resuelto?.actor.membresia?.jugadorId ?? resuelto?.usuario.id ?? 'anonimo',
      // `tipo` puede ser cualquier cosa (o faltar): es un cuerpo que NO pasó validación, así que se anota tal
      // cual llegó en vez de fingir que es un `TipoComando`.
      comando: typeof cuerpo?.tipo === 'string' ? cuerpo.tipo : '(sin tipo)',
      resultado: 'rechazado',
      causa: 'esquema',
      detalle: error.message,
    });
  };
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
  auditoria: RegistroDeAuditoria,
  camposExtra?: (runner: RunnerDePartida) => Record<string, unknown>
) {
  const { tipo, params } = cuerpo;
  // Cada salida de esta función escribe UNA línea de auditoría, y son cuatro: tipo inválido, sin permiso,
  // ejecutado (aceptado o rechazado por el dominio) y fallo de persistencia. Registrar en cada `return` en
  // vez de en un `finally` es a propósito — el `finally` tendría que reconstruir por qué se salió, que es
  // justo el dato que aquí se conoce de primera mano.
  const comun = { gameId: runner.gameId, actor: actorId, comando: tipo };

  if (!esTipoComandoValido(tipo)) {
    auditoria.registrar({ ...comun, resultado: 'rechazado', causa: 'esquema', detalle: 'tipo de comando desconocido' });
    return reply.code(400).send({ error: `tipo de comando desconocido: '${tipo}'.` });
  }

  // `tipo` es `TipoComando` (unión), no un literal: la genérica de `verificarAutorizacion` no infiere un
  // único `T` de una unión, igual que `manejador` de abajo pierde su `P`/`R` en el mismo punto — misma
  // frontera "comando serializado sin validar" que asume el resto de la ruta.
  const chequeo = verificarAutorizacion(tipo, params as any, runner.getState(), actor);
  if (!chequeo.autorizado) {
    // La línea de moderación por excelencia (E2 -> E3): el intento de actuar sobre lo que no es tuyo. Antes
    // de esto, un 403 no dejaba absolutamente ningún rastro.
    auditoria.registrar({ ...comun, resultado: 'rechazado', causa: 'autorizacion', detalle: chequeo.motivo });
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
    // `ok: false` aquí NO es un error: es el dominio diciendo que no (sin recursos, plaza ocupada...). Se
    // audita igual, y con su `codigoError` — es lo que convierte un "no me deja construir" en algo
    // comprobable. `instante` y `version` salen del resumen porque describen la partida DESPUÉS del comando.
    const resumen = resumenDe(runner);
    auditoria.registrar({
      ...comun,
      instante: resumen.instante,
      version: resultado.version,
      ...(resultado.ok
        ? { resultado: 'aceptado' as const }
        : { resultado: 'rechazado' as const, causa: 'dominio' as const, detalle: resultado.codigoError }),
    });
    return reply.send({ ...resumen, resultado, ...(camposExtra ? camposExtra(runner) : {}) });
  } catch (err) {
    // Solo un fallo de persistencia llega hasta aquí como excepción (ver `RunnerDePartida.aplicarYPersistir`).
    // Es el caso más importante de auditar de los cuatro: el comando SÍ se aplicó en memoria y NO se guardó,
    // así que esta línea es la única constancia de un estado que el snapshot no refleja.
    auditoria.registrar({ ...comun, resultado: 'rechazado', causa: 'persistencia', detalle: mensajeDe(err) });
    return reply.code(409).send({ error: mensajeDe(err) });
  }
}
