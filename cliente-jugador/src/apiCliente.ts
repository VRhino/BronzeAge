// Wrapper `fetch` delgado sobre la superficie `/jugador/*` del backend (Fase C3) — sin lógica de negocio,
// solo I/O. Mismo patrón que `cliente/src/app/apiCliente.ts` (login dev, sesión en memoria), pero SIN ningún
// import de tipos del servidor: los tipos de esta función son los mínimos que este boilerplate necesita, no
// un reflejo de `session/estado.ts`. Un cliente de verdad generaría estos tipos desde `GET /v1/openapi.json`
// (Fase C9) en vez de escribirlos a mano.
import type { MapaGenerado } from './terreno';

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    mensaje: string
  ) {
    super(mensaje);
  }
}

export interface ProyeccionJugador {
  gameId: string;
  tick: number;
  version: number;
  jugadorId: string;
  faccionId: string | null;
  mapaId: string;
  facciones: unknown[];
  asentamientos: unknown[];
  [campo: string]: unknown;
}

let sesionId: string | null = null;

const SUJETO = import.meta.env.VITE_USUARIO ?? 'ana';
const V1 = '/v1';

async function fetchJson<T>(url: string, opciones: RequestInit, cabeceraAuth: string): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url, {
      ...opciones,
      headers: {
        authorization: cabeceraAuth,
        ...(opciones.body ? { 'content-type': 'application/json' } : {}),
        ...((opciones.headers as Record<string, string>) ?? {}),
      },
    });
  } catch {
    throw new ApiError(0, 'No se pudo contactar con el servidor. ¿Está corriendo `npm run server` en el backend?');
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

async function peticion<T>(url: string, opciones: RequestInit = {}): Promise<T> {
  const id = sesionId ?? (await iniciarSesion());
  try {
    return await fetchJson<T>(url, opciones, `sesion ${id}`);
  } catch (err) {
    if (!(err instanceof ApiError) || err.status !== 401) throw err;
    return fetchJson<T>(url, opciones, `sesion ${await iniciarSesion()}`);
  }
}

/** Une al sujeto actual a la partida como jugador — idempotente en la práctica (un segundo intento da 409,
 * que el llamador puede ignorar igual que hace `cliente/` con la creación de partida). */
export function unirseAPartida(gameId: string): Promise<{ jugadorId: string }> {
  return peticion<{ jugadorId: string }>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/membresia`, { method: 'POST' });
}

export function consultarProyeccion(gameId: string): Promise<ProyeccionJugador> {
  return peticion<ProyeccionJugador>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}`);
}

/** El mapa como asset (Fase C11a): se pide una sola vez por `mapaId` y se cachea para siempre — ver
 * `main.ts`, que decide CUÁNDO llamar a esto comparando el `mapaId` de la proyección contra el cacheado. */
export function obtenerMapa(gameId: string, mapaId: string): Promise<MapaGenerado> {
  return peticion<MapaGenerado>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/mapa/${encodeURIComponent(mapaId)}`);
}

/** Sin tipar `params`/`resultado.datos` por comando (a diferencia de `cliente/`, que sí puede porque importa
 * `ParamsDe<T>`/`DatosDe<T>` del servidor): la forma de cada comando está en `GET /v1/openapi.json`
 * (Fase C9) — es el contrato que un cliente sin motor debe leer, no adivinar contra el código fuente. */
export function ejecutarComando(gameId: string, tipo: string, params: unknown): Promise<{ resultado: { ok: boolean; codigoError?: string } }> {
  return peticion(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/comandos`, {
    method: 'POST',
    body: JSON.stringify({ tipo, params }),
  });
}
