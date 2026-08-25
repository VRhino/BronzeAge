// Contrato común de los comandos de una partida (Docs/Arquitectura/7_Diseno_GameSession.md §3).
//
// Un comando es una FUNCIÓN PURA `(estado, mapa, ctx, params) -> { estado, resultado }`, no un método de una
// clase. La razón es concreta: `GameStore` tiene 34 comandos y 1582 líneas, y meterlos como métodos en
// `GameSession` reproduciría ese mismo objeto-dios en otra carpeta. Con un archivo por comando, `GameSession`
// se queda en un despachador pequeño para siempre, cada comando se prueba aislado, y —lo que importa de cara
// a la Fase C— los metadatos de autorización de cada uno (matriz del doc 5) pueden vivir junto a su lógica en
// vez de en una tabla paralela que se desincroniza.
import type { EventoDominio } from '../../domain/eventos';
import type { Mapa } from '../../world/mapa';
import type { RandomFn } from '../../worldgen';
import type { GameSessionState } from '../estado';
import type { GeneradorIds } from '../idGenerator';
import { codigoDeErrorDominio } from '../erroresDeDominio';
import type { CodigoError } from './codigosDeError';

/** Quién ejecuta el comando. Hoy es una cadena libre; en la Fase C pasa a ser el `Jugador` resuelto desde la
 * sesión autenticada, NUNCA un id que el cliente elija (doc 2, principio 3). */
export type ActorId = string;

/** Actor de las operaciones que inicia el propio servidor (tick programado, turno del NPC), no una persona.
 * Se corresponde con el rol técnico `servicio_npc` del doc 5. */
export const ACTOR_SISTEMA: ActorId = 'sistema';

/** Actor de la consola local de administración (`app/gameStore.ts`), donde todavía no hay autenticación y
 * quien pulsa el botón es siempre el mismo. Desaparece en la Fase C, cuando el actor lo resuelva la sesión
 * autenticada en vez de fijarlo el cliente (doc 2, principio 3). */
export const ACTOR_LOCAL: ActorId = 'local';

/**
 * Contexto de ejecución de un comando: lo que NO es dato del comando pero hace falta para resolverlo.
 * Deliberadamente simétrico a `ContextoSimulacion` (`engine/simulation.ts`) — y por el mismo motivo: el
 * momento y la aleatoriedad se inyectan, nunca se leen aquí dentro.
 */
export interface ContextoComando {
  /** Momento de simulación (ISO 8601). Lo decide el llamador; ningún comando llama a `Date.now()`. */
  momento: string;
  actor: ActorId;
  rng: RandomFn;
  /** Generador de ids de la partida. Es un servicio con estado propio (avanza al consumirlo), a diferencia
   * del resto del contexto, que son datos. */
  ids: GeneradorIds;
}

/** Lo que un comando devuelve al llamador. Nunca texto de log: el mensaje localizado es presentación, y el
 * contrato con un cliente remoto tiene que ser un código estable (doc 2, punto 6). */
export interface ResultadoComando<T = void> {
  ok: boolean;
  datos?: T;
  /** Código estable de dominio si `ok` es `false` (catálogo cerrado, ver `codigosDeError.ts`). */
  codigoError?: CodigoError;
  eventos: EventoDominio[];
  /** Versión de la partida tras el comando. Idéntica a la previa si fue rechazado. */
  version: number;
}

/** Un comando devuelve el estado resultante junto al resultado — nunca muta el que recibe. Si fue rechazado,
 * `estado` es exactamente el mismo objeto que entró. */
export interface TransicionComando<T = void> {
  estado: GameSessionState;
  resultado: ResultadoComando<T>;
}

export type ManejadorComando<P, R> = (
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  params: P
) => TransicionComando<R>;

/**
 * Comando aceptado: incrementa `version`, vuelca los eventos en el log administrativo y en `eventosDominio`.
 * **Único punto donde se incrementa la versión** — así no puede quedar un comando que mute el estado y se
 * olvide de subirla, que es justo lo que rompería el control de concurrencia optimista de Fase B3.
 *
 * `estado` debe llegar ya con los cambios de dominio aplicados; esta función solo añade lo transversal.
 */
export function exito<T>(estado: GameSessionState, eventos: EventoDominio[], datos?: T): TransicionComando<T> {
  const version = estado.version + 1;
  const estadoFinal: GameSessionState = {
    ...estado,
    version,
    log: [...eventos.map((e) => ({ tick: e.tick, mensaje: e.mensaje })), ...estado.log],
    eventosDominio: [...eventos, ...estado.eventosDominio],
  };
  return { estado: estadoFinal, resultado: { ok: true, datos, eventos, version } };
}

/** Comando rechazado: devuelve el estado SIN TOCAR (mismo objeto) y sin subir la versión. */
export function rechazo<T>(estado: GameSessionState, codigoError: CodigoError): TransicionComando<T> {
  return { estado, resultado: { ok: false, codigoError, eventos: [], version: estado.version } };
}

/**
 * Traduce una excepción del motor a un rechazo. Si no es un error de dominio reconocido la RELANZA: un error
 * inesperado es un bug, y convertirlo en un `ResultadoComando` lo enterraría en un "comando rechazado"
 * indistinguible de un rechazo legítimo.
 */
export function rechazoDesdeError<T>(estado: GameSessionState, err: unknown): TransicionComando<T> {
  const codigo = codigoDeErrorDominio(err);
  if (codigo === undefined) throw err;
  return rechazo(estado, codigo);
}
