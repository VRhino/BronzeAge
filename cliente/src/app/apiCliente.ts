// Wrapper `fetch` delgado sobre `src/server/api.ts` — sin lógica de negocio, solo I/O. Lo usa
// `app/gameStore.ts`, vía `main.ts` (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3 — migración de
// `main.ts`). No hay separación real todavía entre cliente de jugador y herramienta de administración —
// `main.ts` sirve a los dos propósitos por ahora, a propósito.
//
// Las rutas son relativas (`/partidas/...`): en dev, `vite.config.ts` las proxya al backend (mismo origen
// desde el navegador, sin CORS); en producción, se sirven detrás del mismo host que el estático.
import type { RegionId } from '@motor/domain/types';
import type { GameSessionState } from '@motor/session/gameSession';
import type { ResultadoComando } from '@motor/session/comandos/tipos';
import type { DatosDe, ParamsDe, TipoComando } from '@motor/session/comandos/registro';

export interface ResumenPartida {
  gameId: string;
  tick: number;
  version: number;
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

async function peticion<T>(url: string, opciones?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url, {
      ...opciones,
      // Solo con body: Fastify rechaza con 400 un `content-type: application/json` sobre un cuerpo vacío
      // (POST /tick no manda body) — el header solo tiene sentido cuando de verdad hay JSON que parsear.
      headers: opciones?.body ? { 'content-type': 'application/json', ...(opciones?.headers ?? {}) } : opciones?.headers,
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

/**
 * Crea la partida si no existe, o la retoma si ya hay un runner abierto en el proceso (`server/api.ts`,
 * `POST /partidas`) — no destructivo. `forzar: true` SÍ lo es (descarta y empieza de cero): lo usa
 * `GameStore.regenerarMundo`, nunca el bootstrap de `GameStore.crear`.
 */
export function crearOResumirPartida(gameId: string, seed: number, region?: RegionId, forzar?: boolean): Promise<ResumenPartida> {
  return peticion<ResumenPartida>('/partidas', {
    method: 'POST',
    body: JSON.stringify({ gameId, seed, region, forzar }),
  });
}

/** `T` fija a la vez la forma de `params` (`ParamsDe<T>`) y la de `resultado.datos` (`DatosDe<T>`) contra el
 * propio `REGISTRO_COMANDOS` — un `params` con un campo de menos, de más o del tipo equivocado no compila,
 * en vez de viajar como `unknown` y reventar dentro del manejador (ver `server/api.ts`). */
export function ejecutarComando<T extends TipoComando>(gameId: string, tipo: T, params: ParamsDe<T>): Promise<RespuestaComando<DatosDe<T>>> {
  return peticion<RespuestaComando<DatosDe<T>>>(`/partidas/${encodeURIComponent(gameId)}/comandos`, {
    method: 'POST',
    body: JSON.stringify({ tipo, params }),
  });
}

export function avanzarTick(gameId: string): Promise<RespuestaComando<void>> {
  return peticion<RespuestaComando<void>>(`/partidas/${encodeURIComponent(gameId)}/tick`, { method: 'POST' });
}

export function consultarEstado(gameId: string): Promise<GameSessionState> {
  return peticion<GameSessionState>(`/partidas/${encodeURIComponent(gameId)}`);
}
