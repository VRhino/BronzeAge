// Wrapper `fetch` delgado sobre `src/server/api.ts` — sin lógica de negocio, solo I/O. Lo usa
// `app/gameStore.ts`, vía `main.ts`.
//
// **Este cliente habla la superficie de ADMINISTRACIÓN** (`/admin/*`, Fase C3): crea partidas y lee el
// estado completo, que son operaciones de administrador. El mundo avanza SOLO en el servidor (reloj de
// mundo, Fase D / D5) — el cliente no lo empuja, solo re-lee el estado. El cliente de JUGADOR (`/jugador/*`)
// vive en otro repositorio.
//
// Autenticación: login con el proveedor de desarrollo (`dev <sujetoId>`) y `sesionId` en memoria para el
// resto de peticiones. Es un apaño de desarrollo consciente — el sujeto sale de `VITE_USUARIO` y debe estar
// declarado en `ADMINISTRADORES` del servidor. Sustituirlo por un login real es cambiar solo `iniciarSesion`.
//
// Las rutas son relativas: en dev, `vite.config.ts` las proxya al backend (mismo origen desde el navegador,
// sin CORS); en producción, se sirven detrás del mismo host que el estático.
import type { RegionId } from '@motor/domain/types';
import type { EstadoAdmin, EventoDominioConVersion } from '@motor/session/estado';
import type { ResultadoComando } from '@motor/session/comandos/tipos';
import type { DatosDe, ParamsDe, TipoComando } from '@motor/session/comandos/registro';
import type { MapaGenerado } from '@motor/worldgen';

export interface ResumenPartida {
  gameId: string;
  /** Instante de MUNDO de la partida (ms desde época) — Fase D: la referencia temporal del contrato, en
   * lugar del `tick` interno del motor. */
  instante: number;
  version: number;
  /** Identidad del mapa vigente (Fase C11) — nunca el mapa en sí. Ver `obtenerMapa`. */
  mapaId: string;
}

export interface RespuestaComando<R = unknown> extends ResumenPartida {
  resultado: ResultadoComando<R>;
}

/** Fallo de la petición HTTP en sí (red, 4xx, 5xx) — no confundir con un `ResultadoComando.ok === false`,
 * que es un rechazo de DOMINIO y llega como respuesta 200 normal. */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    mensaje: string
  ) {
    super(mensaje);
  }
}

/** Sesión vigente, en memoria: se pierde al recargar y se vuelve a pedir. No se guarda en `localStorage` a
 * propósito — un identificador de sesión ahí sobrevive a la pestaña y es exactamente lo que no conviene
 * arrastrar cuando el mecanismo de autenticación real todavía no está decidido. */
let sesionId: string | null = null;

/** Sujeto con el que este cliente se identifica. Debe figurar en `ADMINISTRADORES` del servidor, si no el
 * backend responderá 403 al crear la partida. */
const SUJETO = import.meta.env.VITE_USUARIO ?? 'jefa';

/** Prefijo de versión del contrato (Fase C6, doc 4): `server/api.ts` sirve todo bajo `/v1`. */
const V1 = '/v1';

async function fetchJson<T>(url: string, opciones: RequestInit, cabeceraAuth: string): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url, {
      ...opciones,
      headers: {
        authorization: cabeceraAuth,
        // `content-type` solo con body: Fastify rechaza con 400 un `application/json` sobre un cuerpo vacío
        // (el tick no manda body) — el header solo tiene sentido cuando de verdad hay JSON que parsear.
        ...(opciones.body ? { 'content-type': 'application/json' } : {}),
        ...((opciones.headers as Record<string, string>) ?? {}),
      },
    });
  } catch {
    throw new ApiError(0, 'No se pudo contactar con el servidor. ¿Está corriendo `npm run server`?');
  }
  if (!res.ok) {
    const cuerpo = (await res.json().catch(() => ({}))) as { error?: string };
    throw new ApiError(res.status, cuerpo.error ?? `El servidor respondió ${res.status}.`);
  }
  return res.json() as Promise<T>;
}

async function iniciarSesion(): Promise<string> {
  const { sesionId: id } = await fetchJson<{ sesionId: string }>(`${V1}/sesiones`, { method: 'POST' }, `dev ${SUJETO}`);
  sesionId = id;
  return id;
}

/**
 * Petición autenticada. Si la sesión falta o el servidor la rechaza (401: caducó, o el proceso se reinició y
 * la perdió — hoy vive en memoria), entra una vez y reintenta. Un segundo 401 se propaga: reintentar en
 * bucle solo convertiría un problema de credenciales en una tormenta de peticiones.
 */
async function peticion<T>(url: string, opciones: RequestInit = {}): Promise<T> {
  const id = sesionId ?? (await iniciarSesion());
  try {
    return await fetchJson<T>(url, opciones, `sesion ${id}`);
  } catch (err) {
    if (!(err instanceof ApiError) || err.status !== 401) throw err;
    return fetchJson<T>(url, opciones, `sesion ${await iniciarSesion()}`);
  }
}

/**
 * Crea la partida si no existe, o la retoma desde el snapshot en disco — no destructivo. `forzar: true` SÍ
 * lo es (descarta y empieza de cero): lo usa `GameStore.regenerarMundo`, nunca el bootstrap de
 * `GameStore.crear`. Exige rol `administrador_global`; `forzar`, además, no lo permite un `moderador`.
 */
export function crearOResumirPartida(
  gameId: string,
  seed: number,
  region?: RegionId,
  forzar?: boolean,
  intervaloTickMs?: number
): Promise<ResumenPartida> {
  return peticion<ResumenPartida>(`${V1}/admin/partidas`, {
    method: 'POST',
    body: JSON.stringify({ gameId, seed, region, forzar, intervaloTickMs }),
  });
}

/** `T` fija a la vez la forma de `params` (`ParamsDe<T>`) y la de `resultado.datos` (`DatosDe<T>`) contra el
 * propio `REGISTRO_COMANDOS` — un `params` con un campo de menos, de más o del tipo equivocado no compila,
 * en vez de viajar como `unknown` y reventar dentro del manejador.
 *
 * Va por la superficie de administración, así que la matriz de autorización lo evalúa con rol de
 * administrador: rechazará con 403 todo lo que sea de jugador (que es casi todo). Ver `rutas/admin.ts`. */
export function ejecutarComando<T extends TipoComando>(gameId: string, tipo: T, params: ParamsDe<T>): Promise<RespuestaComando<DatosDe<T>>> {
  return peticion<RespuestaComando<DatosDe<T>>>(`${V1}/admin/partidas/${encodeURIComponent(gameId)}/comandos`, {
    method: 'POST',
    body: JSON.stringify({ tipo, params }),
  });
}

/** Sin `mapa` (Fase C11): trae `mapaId` en su lugar. Ver `obtenerMapa` para pedir el mapa real. */
export function consultarEstado(gameId: string): Promise<EstadoAdmin> {
  return peticion<EstadoAdmin>(`${V1}/admin/partidas/${encodeURIComponent(gameId)}`);
}

/**
 * Cursor incremental de eventos (Fase C13, y desde el 2026-09-05 la ÚNICA vía: `eventosDominio` dejó de
 * viajar dentro de la lectura de estado, donde era el 87-88 % del payload y crecía sin techo).
 *
 * `desde` es una `version` de partida, no una fecha ni un índice: se pide `0` la primera vez y después la
 * mayor `version` ya vista. Devuelve solo lo posterior, así que el coste de mantener el log al día deja de
 * depender de lo larga que sea la partida.
 */
export function consultarEventos(gameId: string, desde: number): Promise<{ eventos: EventoDominioConVersion[] }> {
  return peticion<{ eventos: EventoDominioConVersion[] }>(
    `${V1}/admin/partidas/${encodeURIComponent(gameId)}/eventos?desde=${desde}`
  );
}

/**
 * El mapa como asset (Fase C11): NUNCA cambia durante la partida, así que se pide una sola vez por `mapaId` —
 * `GameStore` decide cuándo llamar a esto comparando el `mapaId` que trae cada respuesta contra el que tiene
 * cacheado, no esta función. `:mapaId` en la URL es lo que le permite al navegador cachear la respuesta para
 * siempre sin volver a preguntarle al servidor (`Cache-Control: immutable`, ver `server/rutas/mapa.ts`).
 */
export function obtenerMapa(gameId: string, mapaId: string): Promise<MapaGenerado> {
  return peticion<MapaGenerado>(`${V1}/admin/partidas/${encodeURIComponent(gameId)}/mapa/${encodeURIComponent(mapaId)}`);
}
