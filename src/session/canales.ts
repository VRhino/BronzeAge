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
//   - `asentamiento/<id>`: eventos de ESE asentamiento. Solo si es de la Facción propia — la autorización de
//     canal usa la MISMA regla que la proyección: nada de lo ajeno, ni siquiera en tiempo real.
//
// De ahí que atribuir un evento importe más de lo que parece: lo que sale sin `asentamientoId` se difunde por
// `mapa/general`, al que puede suscribirse CUALQUIER jugador de la partida. Los eventos de campaña se
// atribuyen a su asentamiento de origen justamente por esto (ver `avanzarEjercitos`).
import { heroesNombrados, type EventoDominio } from '../domain/eventos';
import { esCiudadano } from '../engine/faccion';
import { combateEn } from './batallas';
import type { GameSessionState } from './estado';
import { proyectarParaJugador } from './proyecciones/jugador';

const SIN_GEOMETRIA = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

/**
 * Sigue una batalla quien combate en ella o la ve ahora mismo bajo la niebla (doc 02 §3.5): el criterio exacto con el que la
 * proyección la enseña en el mapa, que se pregunta a la propia proyección para que no puedan separarse. Solo se evalúa al
 * suscribirse, y las geometrías no cambian si se ve o no una batalla.
 */
function puedeSeguirLaBatalla(estado: GameSessionState, heroeId: string, battleId: string): boolean {
  const batalla = estado.batallas.find((b) => b.id === battleId);
  if (!batalla) return false;
  return combateEn(batalla, heroeId) || proyectarParaJugador(estado, heroeId, SIN_GEOMETRIA).batallas.some((b) => b.battleId === battleId);
}

export const CANAL_GENERAL = 'mapa/general';

export function canalDeAsentamiento(asentamientoId: string): string {
  return `asentamiento/${asentamientoId}`;
}

export function canalDeBatalla(battleId: string): string {
  return `batalla/${battleId}`;
}

/** El canal personal de un héroe (2026-10-07): lo que le nombra (`heroesNombrados`) y no va ya por el canal general. Solo lo abre él. */
export function canalDeHeroe(heroeId: string): string {
  return `heroe/${heroeId}`;
}

/** La batalla de un evento `batalla.*` o `evento_pve.*`, si lo es (doc 02 §3.5). */
function batallaDe(evento: EventoDominio): string | undefined {
  if (!evento.codigo.startsWith('batalla.') && !evento.codigo.startsWith('evento_pve.')) return undefined;
  const id = (evento.payload as { battleId?: unknown } | undefined)?.battleId;
  return typeof id === 'string' ? id : undefined;
}

/** Todos los canales por los que va un evento: el de siempre; si es de una batalla, el de la batalla (doc 02 §3.5); y si no es global, el
 * personal de cada héroe que nombra (un global ya le llega por `mapa/general`). Sin esto, los eventos de «ninguna plaza» (`asentamientoId: ''`,
 * columnas salidas de un campamento) no los recibía nadie en tiempo real. */
export function canalesDeEvento(evento: EventoDominio): string[] {
  const battleId = batallaDe(evento);
  const personales = evento.asentamientoId === undefined ? [] : heroesNombrados(evento).map(canalDeHeroe);
  return [canalDeEvento(evento), ...(battleId === undefined ? [] : [canalDeBatalla(battleId)]), ...personales];
}

/** Canal al que pertenece un evento — el mismo criterio de "propio" que usa la proyección de jugador
 * (`eventosDominio` en `proyectarParaJugador`), expresado como nombre de canal en vez de filtro de array. */
export function canalDeEvento(evento: EventoDominio): string {
  return evento.asentamientoId === undefined ? CANAL_GENERAL : canalDeAsentamiento(evento.asentamientoId);
}

/**
 * Puede suscribirse `heroeId` al `canal` indicado, con el estado ACTUAL de la partida.
 *
 * Se evalúa en el momento de suscribirse, no en cada mensaje difundido — si una anexión o fusión cambiara la
 * Facción dueña de un asentamiento después, una suscripción ya activa no se re-valida sola (simplificación
 * conocida, igual que en la proyección: casos raros, y el cliente vuelve a suscribirse en cada reconexión).
 */
export function puedeSuscribirseA(estado: GameSessionState, heroeId: string, canal: string): boolean {
  if (canal === CANAL_GENERAL) return true;
  if (canal.startsWith('heroe/')) return canal === canalDeHeroe(heroeId);
  if (canal.startsWith('batalla/')) return puedeSeguirLaBatalla(estado, heroeId, canal.slice('batalla/'.length));
  const asentamientoId = canal.startsWith('asentamiento/') ? canal.slice('asentamiento/'.length) : undefined;
  if (asentamientoId === undefined) return false; // canal con forma desconocida: no autorizado, no un error

  const asentamiento = estado.asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) return false; // no existe: nada que suscribir, no se distingue de "no autorizado"

  const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
  return faccion !== undefined && esCiudadano(faccion, heroeId);
}
