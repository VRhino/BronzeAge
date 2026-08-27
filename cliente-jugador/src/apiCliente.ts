// Wrapper `fetch` sobre la superficie `/jugador/*` y `/sesiones` del backend (Fase C3) — sin lógica de negocio, solo I/O.
import type { MapaGenerado } from './terreno';

import type { Asentamiento, CaminoComercial, CampamentoBandido, Caravana, Faccion, ZonaFaccion, TrazadoAsentamiento } from './tiposDominio';

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
  facciones: Faccion[];
  asentamientos: Asentamiento[];
  caravanas: Caravana[];
  caminos: CaminoComercial[];
  campamentosBandidos: CampamentoBandido[];
  zonasFusionadas: ZonaFaccion[];
  trazadoPorAsentamiento: Record<string, TrazadoAsentamiento>;
  [campo: string]: unknown;
}

export interface RespuestaLogin {
  usuarioId: string;
  sesionId: string;
  expiraEn: string;
}

export interface RespuestaWhoami {
  usuarioId: string;
  esAdministradorGlobal: boolean;
  gameId?: string;
  rol?: string | null;
  jugadorId?: string | null;
}

export interface RespuestaComando {
  resultado: {
    ok: boolean;
    codigoError?: string;
  };
  proyeccion?: ProyeccionJugador;
  [campo: string]: unknown;
}

const V1 = '/v1';
const STORAGE_KEY_SESION = 'bac_jugador_sesion_id';
const STORAGE_KEY_USUARIO = 'bac_jugador_usuario';
const STORAGE_KEY_GAME_ID = 'bac_jugador_game_id';

let sesionIdMemoria: string | null = null;
let usuarioMemoria: string | null = null;

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

export async function loginConUsuario(usuario: string): Promise<RespuestaLogin> {
  const sujeto = usuario.trim();
  if (!sujeto) throw new ApiError(400, 'El nombre de usuario no puede estar vacío.');

  const res = await fetchJson<RespuestaLogin>(`${V1}/sesiones`, { method: 'POST' }, `dev ${sujeto}`);
  sesionIdMemoria = res.sesionId;
  usuarioMemoria = sujeto;
  return res;
}

export function guardarSesionLocal(sesionId: string, usuario: string, gameId: string): void {
  sesionIdMemoria = sesionId;
  usuarioMemoria = usuario;
  localStorage.setItem(STORAGE_KEY_SESION, sesionId);
  localStorage.setItem(STORAGE_KEY_USUARIO, usuario);
  localStorage.setItem(STORAGE_KEY_GAME_ID, gameId);
}

export function cargarSesionLocal(): { sesionId: string; usuario: string; gameId: string } | null {
  const sid = localStorage.getItem(STORAGE_KEY_SESION);
  const usr = localStorage.getItem(STORAGE_KEY_USUARIO);
  const gid = localStorage.getItem(STORAGE_KEY_GAME_ID);

  if (sid && usr && gid) {
    sesionIdMemoria = sid;
    usuarioMemoria = usr;
    return { sesionId: sid, usuario: usr, gameId: gid };
  }
  return null;
}

export function cerrarSesion(): void {
  sesionIdMemoria = null;
  usuarioMemoria = null;
  localStorage.removeItem(STORAGE_KEY_SESION);
  localStorage.removeItem(STORAGE_KEY_USUARIO);
  localStorage.removeItem(STORAGE_KEY_GAME_ID);
}

export function getUsuarioActual(): string | null {
  return usuarioMemoria;
}

async function peticion<T>(url: string, opciones: RequestInit = {}): Promise<T> {
  if (!sesionIdMemoria) {
    const local = cargarSesionLocal();
    if (!local) throw new ApiError(401, 'No hay ninguna sesión activa. Inicie sesión primero.');
  }

  try {
    return await fetchJson<T>(url, opciones, `sesion ${sesionIdMemoria}`);
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) {
      // Si la sesión expiró o es inválida, intentar re-autenticar automáticamente si conocemos el usuario
      if (usuarioMemoria) {
        const loginRes = await loginConUsuario(usuarioMemoria);
        sesionIdMemoria = loginRes.sesionId;
        const local = cargarSesionLocal();
        if (local) guardarSesionLocal(loginRes.sesionId, local.usuario, local.gameId);
        return fetchJson<T>(url, opciones, `sesion ${sesionIdMemoria}`);
      }
    }
    throw err;
  }
}

export function obtenerWhoami(gameId?: string): Promise<RespuestaWhoami> {
  const query = gameId ? `?gameId=${encodeURIComponent(gameId)}` : '';
  return peticion<RespuestaWhoami>(`${V1}/sesiones/actual${query}`);
}

/** Une al sujeto actual a la partida como jugador */
export function unirseAPartida(gameId: string): Promise<{ jugadorId: string }> {
  return peticion<{ jugadorId: string }>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/membresia`, { method: 'POST' });
}

export function consultarProyeccion(gameId: string): Promise<ProyeccionJugador> {
  return peticion<ProyeccionJugador>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}`);
}

/** El mapa como asset (Fase C11a): se pide una sola vez por `mapaId` y se cachea */
export function obtenerMapa(gameId: string, mapaId: string): Promise<MapaGenerado> {
  return peticion<MapaGenerado>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/mapa/${encodeURIComponent(mapaId)}`);
}

export function ejecutarComando(gameId: string, tipo: string, params: unknown): Promise<RespuestaComando> {
  return peticion<RespuestaComando>(`${V1}/jugador/partidas/${encodeURIComponent(gameId)}/comandos`, {
    method: 'POST',
    body: JSON.stringify({ tipo, params }),
  });
}
