import type { Mapa } from '../../world/mapa';
import type { ContextoSimulacion } from '../../engine/simulation';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza } from '../npcGobernanza';
import { conResultadoDeSimulacion, estadoSimulacionDe, eventoLegado, type GameSessionState } from '../estado';
import { exito, type ContextoComando, type TransicionComando } from './tipos';

/**
 * Turno del NPC de gobernanza para las Facciones de `faccionesNpcIds`.
 *
 * Operación del sistema, como `avanzarTick`, y siempre DESPUÉS de él — nunca dentro de `avanzarSimulacion`:
 * el NPC decide con las funciones públicas del motor exactamente igual que lo haría un jugador pulsando
 * botones, así que no forma parte de las reglas del tick (ver cabecera de `session/npcGobernanza.ts`).
 *
 * Sin Facciones cedidas al NPC no hace nada y **no incrementa la versión** — no habría mutación que versionar.
 */
export function avanzarFaccionesNpc(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  _params: void
): TransicionComando<void> {
  if (estado.faccionesNpcIds.length === 0) {
    return { estado, resultado: { ok: true, eventos: [], version: estado.version } };
  }

  const contexto: ContextoSimulacion = { tick: estado.tick, momento: ctx.momento, rng: ctx.rng };
  const config: ConfigNpcGobernanza = { faccionesIds: estado.faccionesNpcIds, contadorInicial: ctx.ids.actual() };
  const resultado = avanzarNpcGobernanza(estadoSimulacionDe(estado), mapa, contexto, config);

  // Los ids que el motor generó dentro del NPC salieron de este mismo contador: se adelanta para que la
  // próxima acción manual no reutilice uno (ver `ConfigNpcGobernanza.contadorInicial`).
  ctx.ids.fijar(resultado.contadorFinal);

  const siguiente = conResultadoDeSimulacion(estado, resultado.estado);
  const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, `[NPC] ${mensaje}`));

  return exito(siguiente, eventos);
}
