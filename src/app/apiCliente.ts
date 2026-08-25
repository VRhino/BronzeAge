// Wrapper `fetch` delgado sobre `src/server/api.ts` — sin lógica de negocio, solo I/O. Compartido por
// `app/gameStore.ts` (cliente de JUGADOR, vía `main.ts`) y `admin.ts` (panel de administración): ambos hablan
// con el mismo backend, cada uno con el subconjunto de llamadas que le corresponde (Docs/Arquitectura/
// 4_Plan_Evolucion_Tareas.md, Fase B3 — migración de `main.ts`).
//
// Las rutas son relativas (`/partidas/...`): en dev, `vite.config.ts` las proxya al backend (mismo origen
// desde el navegador, sin CORS); en producción, se sirven detrás del mismo host que el estático.
import type { RegionId } from '../domain/types';
import type { GameSessionState } from '../session/gameSession';
import type { ResultadoComando } from '../session/comandos/tipos';
import type { TipoComando } from '../session/comandos/registro';

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
 * `POST /partidas`) — no destructivo. `forzar: true` SÍ lo es (descarta y empieza de cero): solo lo usa
 * `admin.ts`, nunca el bootstrap del cliente de jugador.
 */
export function crearOResumirPartida(gameId: string, seed: number, region?: RegionId, forzar?: boolean): Promise<ResumenPartida> {
  return peticion<ResumenPartida>('/partidas', {
    method: 'POST',
    body: JSON.stringify({ gameId, seed, region, forzar }),
  });
}

export function ejecutarComando<R = unknown>(gameId: string, tipo: TipoComando, params: unknown): Promise<RespuestaComando<R>> {
  return peticion<RespuestaComando<R>>(`/partidas/${encodeURIComponent(gameId)}/comandos`, {
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
