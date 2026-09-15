import type { Mapa } from '../../world/mapa';
import type { ContextoSimulacion } from '../../engine/simulation';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza } from '../npcGobernanza';
import { faccionesEnBatalla } from '../batallas';
import { conResultadoDeSimulacion, estadoSimulacionDe, type GameSessionState } from '../estado';
import { eventos as construirEventos } from './eventos';
import { exito, sinCambios, type ContextoComando, type TransicionComando } from './tipos';

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
  // Una Facción NPC con algo en una batalla activa no gobierna mientras dure: así no saca tropa de una plaza asediada ni
  // toca escuadras reservadas (Doc 5.15.1). `ponytail:` pausa la Facción entera; por plaza, si se nota en el juego.
  const enBatalla = faccionesEnBatalla(estado, ctx.instante);
  const faccionesIds = estado.faccionesNpcIds.filter((f) => !enBatalla.has(f));
  if (faccionesIds.length === 0) {
    return sinCambios(estado);
  }

  const contexto: ContextoSimulacion = { instante: ctx.instante, momento: ctx.momento, rng: ctx.rng };
  const config: ConfigNpcGobernanza = { faccionesIds, contadorInicial: ctx.ids.actual() };
  const resultado = avanzarNpcGobernanza(estadoSimulacionDe(estado), mapa, contexto, config);

  // Los ids que el motor generó dentro del NPC salieron de este mismo contador: se adelanta para que la
  // próxima acción manual no reutilice uno (ver `ConfigNpcGobernanza.contadorInicial`).
  ctx.ids.fijar(resultado.contadorFinal);

  const siguiente = conResultadoDeSimulacion(estado, resultado.estado);
  // `npcGobernanza.ts` sigue narrando en texto libre: es el NPC jugando como jugaría una persona, no un
  // comando, y su migración a `codigo`/`payload` es una pasada propia. Hasta entonces sus eventos salen con
  // un código que al menos permite FILTRARLOS como grupo (ej. no mandar el ruido del NPC a los clientes).
  const eventos = construirEventos(
    ctx,
    resultado.eventos.map((mensaje) => ({ codigo: 'npc.accion', mensaje: `[NPC] ${mensaje}` }))
  );

  return exito(siguiente, eventos);
}
