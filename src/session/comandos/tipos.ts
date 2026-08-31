// Contrato común de los comandos de una partida (Docs/Arquitectura/7_Diseno_GameSession.md §3).
//
// Un comando es una FUNCIÓN PURA `(estado, mapa, ctx, params) -> { estado, resultado }`, no un método de una
// clase. La razón es concreta: `GameStore` tiene 34 comandos y 1582 líneas, y meterlos como métodos en
// `GameSession` reproduciría ese mismo objeto-dios en otra carpeta. Con un archivo por comando, `GameSession`
// se queda en un despachador pequeño para siempre, cada comando se prueba aislado, y —lo que importa de cara
// a la Fase C— los metadatos de autorización de cada uno (matriz del doc 5) pueden vivir junto a su lógica en
// vez de en una tabla paralela que se desincroniza.
import type { EventoDominio } from '../../domain/eventos';
import type { Instante } from '../../domain/tiempo';
import type { Mapa } from '../../world/mapa';
import type { RandomFn } from '../../worldgen';
import type { EventoDominioConVersion, GameSessionState } from '../estado';
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
  /** Instante de MUNDO (`Instante`, ms), derivado del tick por `GameSession.ejecutar` (`instanteDeTick`,
   * Fase D / doc 10). Ni el comando ni el llamador lo pasan: es función del `tick` y de nada más — el reloj
   * de pared no entra en el estado de partida. Es con lo que se fechan los campos `*En: Instante`. */
  instante: Instante;
  /** El mismo instante en ISO 8601, para fechar eventos (`EventoDominio.momento`). Redundante con `instante`
   * a propósito y de forma permanente: el núcleo puro no puede construir un `Date` (`isoDeInstante` vive en
   * `session/estado.ts`), así que quien ejecuta el comando lo pasa ya formateado — mismo criterio que
   * `ContextoSimulacion.momento`. */
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
  /** Con `version` estampada (Fase C13) — el mismo valor que `version` de aquí abajo, repetido en cada
   * evento para que un cliente que escucha por WebSocket (`hub.difundir`, Fase C5) sepa desde qué cursor
   * seguir (`GET .../eventos?desde=`, `session/estado.ts` `eventosDesde`) sin tener que mirar el envoltorio. */
  eventos: EventoDominioConVersion[];
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
 * Comando aceptado: incrementa `version` y acumula los eventos en `eventosDominio`.
 * **Único punto donde se incrementa la versión** — así no puede quedar un comando que mute el estado y se
 * olvide de subirla, que es justo lo que rompería el control de concurrencia optimista de Fase B3.
 *
 * `estado` debe llegar ya con los cambios de dominio aplicados; esta función solo añade lo transversal.
 *
 * Ya no escribe un `log` en texto en paralelo: era el MISMO hecho guardado dos veces en el estado (y
 * persistido dos veces en cada snapshot). El log que muestra la consola se DERIVA de `eventosDominio` en la
 * capa de presentación (`proyectarLog`, `session/estado.ts`) — ver doc 2 punto 6: el texto es presentación,
 * el contrato es el evento estructurado.
 */
export function exito<T>(estado: GameSessionState, eventos: EventoDominio[], datos?: T): TransicionComando<T> {
  const version = estado.version + 1;
  // Estampa `version` aquí y solo aquí (Fase C13) — mismo motivo que estampar la versión misma: es el único
  // punto que la conoce, y `eventos` llega desde el comando sin saber todavía a qué versión pertenece.
  const eventosConVersion: EventoDominioConVersion[] = eventos.map((e) => ({ ...e, version }));
  const estadoFinal: GameSessionState = {
    ...estado,
    version,
    eventosDominio: [...eventosConVersion, ...estado.eventosDominio],
  };
  return { estado: estadoFinal, resultado: { ok: true, datos, eventos: eventosConVersion, version } };
}

/**
 * Comando ACEPTADO que no cambió nada: no muta el estado y **no incrementa la versión**, porque no hay
 * mutación que versionar. No es un rechazo — pedir el estado en el que ya se está es legítimo, y es lo que
 * hace segura una reintentar por reconexión (doc 2, punto 10: idempotencia de comandos).
 */
export function sinCambios<T>(estado: GameSessionState, datos?: T): TransicionComando<T> {
  return { estado, resultado: { ok: true, datos, eventos: [], version: estado.version } };
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
