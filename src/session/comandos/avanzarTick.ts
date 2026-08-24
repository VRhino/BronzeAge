import type { Mapa } from '../../world/mapa';
import { avanzarSimulacion, type ContextoSimulacion } from '../../engine/simulation';
import { conResultadoDeSimulacion, estadoSimulacionDe, type GameSessionState } from '../estado';
import { exito, type ContextoComando, type TransicionComando } from './tipos';

/**
 * Avanza un tick de simulación.
 *
 * No es un comando de jugador sino una operación DEL SISTEMA: la inicia el scheduler del runner, no una
 * persona (por eso su actor es `ACTOR_SISTEMA`). En producción no debe existir como endpoint público — el doc
 * 2 lo deja claro: "el servidor sigue siendo quien avance y resuelva los ticks". Se mantiene invocable a mano
 * para pruebas y para la consola de administración.
 *
 * ⚠️ Este es el único comando cuyo estado NO es totalmente reconstruible desde su valor de retorno: el motor
 * muta `estadoMapa` por dentro a través de la fachada `Mapa` (`extraer`/`avanzarRegeneracion`). Arreglarlo es
 * prerrequisito de la persistencia de Fase B3 — ver la tarea marcada con ⚠️ en el doc 4. Hasta entonces,
 * descartar el estado devuelto por un fallo de escritura NO revierte lo extraído de los yacimientos.
 */
export function avanzarTick(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  _params: void
): TransicionComando<void> {
  const tick = estado.tick + 1;
  const contexto: ContextoSimulacion = { tick, momento: ctx.momento, rng: ctx.rng };
  const resultado = avanzarSimulacion(estadoSimulacionDe(estado), mapa, contexto);

  const siguiente = conResultadoDeSimulacion({ ...estado, tick }, resultado);
  return exito(siguiente, resultado.eventosDominio);
}
