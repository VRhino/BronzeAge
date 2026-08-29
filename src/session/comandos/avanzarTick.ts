import type { Mapa } from '../../world/mapa';
import { avanzarSimulacion, type ContextoSimulacion } from '../../engine/simulation';
import { conResultadoDeSimulacion, estadoSimulacionDe, instanteDeTick, isoDeInstante, type GameSessionState } from '../estado';
import { exito, type ContextoComando, type TransicionComando } from './tipos';

/**
 * Avanza un tick de simulación.
 *
 * No es un comando de jugador sino una operación DEL SISTEMA: la inicia el scheduler del runner, no una
 * persona (por eso su actor es `ACTOR_SISTEMA`). En producción no debe existir como endpoint público — el doc
 * 2 lo deja claro: "el servidor sigue siendo quien avance y resuelva los ticks". Se mantiene invocable a mano
 * para pruebas y para la consola de administración.
 *
 * Es el único comando que toca el mapa (los yacimientos se agotan al extraer y se regeneran al cabo de un
 * cooldown). Ese cambio viaja en `ResultadoTick.estadoMapa` y se adopta aquí explícitamente, igual que el
 * resto del estado: la fachada `Mapa` trabaja sobre su propia copia, así que el estado que entró no se toca
 * y descartar la transición —un fallo al persistir, Fase B3— revierte también los yacimientos.
 */
export function avanzarTick(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  _params: void
): TransicionComando<void> {
  const tick = estado.tick + 1;
  // El tick lleva el mundo de `instanteDeTick(estado.tick)` a `instanteDeTick(tick)`: sus eventos se fechan
  // con el instante RESULTANTE. `ctx.instante` que llega aquí es el del tick ANTERIOR (lo derivó
  // `GameSession.ejecutar` del `this.estado.tick` de entonces), por eso no se reutiliza.
  const instante = instanteDeTick(tick);
  const contexto: ContextoSimulacion = { instante, momento: isoDeInstante(instante), rng: ctx.rng };
  const resultado = avanzarSimulacion(estadoSimulacionDe(estado), mapa, contexto);

  // `estadoMapa` va aparte de `conResultadoDeSimulacion` porque no es estado del motor: `EstadoSimulacion` es
  // lo que el motor recibe y devuelve como juego, y el mapa es el mundo sobre el que se juega.
  const siguiente = conResultadoDeSimulacion({ ...estado, tick, estadoMapa: resultado.estadoMapa }, resultado);
  return exito(siguiente, resultado.eventosDominio);
}
