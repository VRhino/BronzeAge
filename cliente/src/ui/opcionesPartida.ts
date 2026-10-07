// Lo que comparten la pestaña Mundo de la consola y la pantalla «sin partida»: las opciones de crear una partida y la lista de las
// que hay en el servidor.
import type { RegionId } from '@motor/domain/types';
import { fmtTiempoMundo } from '../app/gameStore';
import { esc } from './html';

/** Regiones geográficas disponibles (Fase 0.2, ver `worldgen/regiones.ts`) — nombre para el selector del
 * mundo. Mantenido a mano, igual que `BIOMA_NOMBRE`/`EDIFICIO_NOMBRE`: es presentación pura, no se deriva de
 * `worldgen/` (este cliente no puede importar de ahí). */
export const REGION_NOMBRE: Record<RegionId, string> = {
  greciaContinental: 'Grecia continental',
  anatolia: 'Anatolia',
  egeo: 'Egeo (archipiélago)',
  nilo: 'Nilo',
  mesopotamia: 'Mesopotamia',
};

/** Opciones del selector "Velocidad de tick" al regenerar (ms de reloj de PARED entre ticks). Un tick =
 * 1 minuto de mundo (`SIMULACION.duracionTickMs`), así que 1000 ms = 1 día de mundo cada 24 s reales. */
export const TICK_INTERVALOS: [number, string][] = [
  [500, '2 ticks/s (rápido)'],
  [1000, '1 tick/s'],
  [2000, '1 tick cada 2 s'],
  [5000, '1 tick cada 5 s'],
  [10000, '1 tick cada 10 s'],
  [30000, '1 tick cada 30 s'],
  [60000, '1 tick por minuto (tiempo real)'],
];

export function fmtIntervaloTick(ms: number | null | undefined): string {
  if (ms == null) return 'reloj parado (solo tick manual)';
  const conocido = TICK_INTERVALOS.find(([valor]) => valor === ms);
  if (conocido) return conocido[1];
  return ms % 1000 === 0 ? `1 tick cada ${ms / 1000} s` : `1 tick cada ${ms} ms`;
}

export const ID_PARTIDA_VALIDO = /^[A-Za-z0-9_.-]+$/;

/** Los `<option>` de región y de velocidad de tick. */
export const OPCIONES_REGION = `<option value="">Libre (procedural, sin sesgo)</option>${Object.entries(REGION_NOMBRE)
  .map(([id, nombre]) => `<option value="${id}">${nombre}</option>`)
  .join('')}`;
export const OPCIONES_TICK = `<option value="">Por defecto del servidor</option>${TICK_INTERVALOS.map(([ms, txt]) => `<option value="${ms}">${txt}</option>`).join('')}`;

/** Las partidas que el servidor conoce, como botones (`data-partida`); la `actual` va marcada y no se pulsa. */
export function htmlListaPartidas(partidas: readonly { gameId: string; instante: number; version: number }[], actual: string | null): string {
  if (partidas.length === 0) return '<p class="legend-note">No hay ninguna partida en el servidor.</p>';
  return partidas
    .map(
      (p) =>
        `<button type="button" role="listitem" class="partida-item${p.gameId === actual ? ' active' : ''}" data-partida="${esc(p.gameId)}" ${p.gameId === actual ? 'disabled aria-current="true"' : ''}><strong>${esc(p.gameId)}</strong><small>${p.gameId === actual ? 'conectada · ' : ''}${fmtTiempoMundo(p.instante)} · v${p.version}</small></button>`
    )
    .join('');
}
