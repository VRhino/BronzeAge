// Canales de difusión en tiempo real (Fase C5, doc 6 §2): a qué puede suscribirse un jugador sobre la
// conexión WebSocket única, y qué canal le corresponde a cada evento de dominio.
//
// Vive en `session/` y no en `server/` por el mismo motivo que `comandos/autorizacion.ts`: qué puede ver un
// jugador es negocio (cruza identidad técnica con pertenencia de juego), no transporte. El transporte —abrir
// el socket, parsear los mensajes JSON, mantener la lista de conexiones— es infraestructura y vive en
// `server/difusion/`.
//
// Dos canales, calcados de los ejemplos del doc 6 §2:
//   - `mapa/general`: eventos SIN `asentamientoId` (globales — diplomacia, Facciones, tick). Abierto a
//     cualquiera con `Membresia` de jugador en la partida.
//   - `asentamiento/<id>`: eventos de ESE asentamiento. Solo si es de la Facción propia — Slice 1 de C4
//     (`session/proyecciones/jugador.ts`) todavía no tiene niebla de guerra, así que la autorización de canal
//     usa la MISMA regla que la proyección: nada de lo ajeno, ni siquiera en tiempo real.
import type { EventoDominio } from '../domain/eventos';
import { esCiudadano } from '../engine/faccion';
import type { GameSessionState } from './estado';

export const CANAL_GENERAL = 'mapa/general';

export function canalDeAsentamiento(asentamientoId: string): string {
  return `asentamiento/${asentamientoId}`;
}

/** Canal al que pertenece un evento — el mismo criterio de "propio" que usa la proyección de jugador
 * (`eventosDominio` en `proyectarParaJugador`), expresado como nombre de canal en vez de filtro de array. */
export function canalDeEvento(evento: EventoDominio): string {
  return evento.asentamientoId === undefined ? CANAL_GENERAL : canalDeAsentamiento(evento.asentamientoId);
}

/**
 * Puede suscribirse `jugadorId` al `canal` indicado, con el estado ACTUAL de la partida.
 *
 * Se evalúa en el momento de suscribirse, no en cada mensaje difundido — si una anexión o fusión cambiara la
 * Facción dueña de un asentamiento después, una suscripción ya activa no se re-valida sola (simplificación
 * conocida, igual que en la proyección: casos raros, y el cliente vuelve a suscribirse en cada reconexión).
 */
export function puedeSuscribirseA(estado: GameSessionState, jugadorId: string, canal: string): boolean {
  if (canal === CANAL_GENERAL) return true;
  const asentamientoId = canal.startsWith('asentamiento/') ? canal.slice('asentamiento/'.length) : undefined;
  if (asentamientoId === undefined) return false; // canal con forma desconocida: no autorizado, no un error

  const asentamiento = estado.asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) return false; // no existe: nada que suscribir, no se distingue de "no autorizado"

  const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
  return faccion !== undefined && esCiudadano(faccion, jugadorId);
}
