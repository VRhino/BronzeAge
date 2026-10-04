import type { Mapa } from '../../world/mapa';
import { avanzarSimulacion, type ContextoSimulacion } from '../../engine/simulation';
import { bloqueosDe, eventosDeBatalla, vencerBatallas } from '../batallas';
import { conResultadoDeSimulacion, estadoSimulacionDe, instanteDeTick, isoDeInstante, type GameSessionState } from '../estado';
import { exito, type ContextoComando, type TransicionComando } from './tipos';
import { salirDelMundo } from '../../engine/presencia';
import { PRESENCIA } from '../../constants';
import { columnaDe } from '../../engine/ejercitos';
import type { EventoCrudo } from '../../domain/eventos';
import { conMomento } from './eventos';

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
 *
 * Las batallas de Unity (doc 01 §15) se tocan aquí dos veces: las que vencieron su plazo se cierran antes que nada, y
 * lo que está en una activa no entra en la simulación (Doc 5.15.1). El tick no abre ninguna: los combates los pide
 * `atacar`, humano o bot.
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
  const momento = isoDeInstante(instante);

  const trasVencer = vencerBatallas(estado, instante);
  const bloqueos = bloqueosDe(trasVencer.estado, instante);
  const simulacion = estadoSimulacionDe(trasVencer.estado);
  const apartados = {
    ejercitos: simulacion.ejercitos.filter((e) => bloqueos.ejercitos.has(e.id)),
    caravanas: simulacion.caravanas.filter((c) => bloqueos.caravanas.has(c.id)),
  };
  const contexto: ContextoSimulacion = {
    instante,
    momento,
    rng: ctx.rng,
  };
  const resultado = avanzarSimulacion(
    {
      ...simulacion,
      ejercitos: simulacion.ejercitos.filter((e) => !bloqueos.ejercitos.has(e.id)),
      caravanas: simulacion.caravanas.filter((c) => !bloqueos.caravanas.has(c.id)),
    },
    mapa,
    contexto
  );

  // `estadoMapa` va aparte de `conResultadoDeSimulacion` porque no es estado del motor: `EstadoSimulacion` es
  // lo que el motor recibe y devuelve como juego, y el mapa es el mundo sobre el que se juega.
  const trasTick = conResultadoDeSimulacion(
    { ...trasVencer.estado, tick, estadoMapa: resultado.estadoMapa },
    {
      ...resultado,
      ejercitos: [...resultado.ejercitos, ...apartados.ejercitos],
      caravanas: [...resultado.caravanas, ...apartados.caravanas],
    }
  );
  const eventosDeBatallas = trasVencer.vencidas
    .flatMap((b) => eventosDeBatalla(trasTick, b, 'batalla.fallida', `La batalla ${b.id} no llegó a jugarse: nadie pierde nada.`))
    .map((e) => ({ ...e, momento }));
  const presencia = conSalidasDelMundo(trasTick, mapa, instante, bloqueos.heroes);
  return exito(presencia.estado, [...resultado.eventosDominio, ...eventosDeBatallas, ...conMomento({ ...ctx, momento }, presencia.eventos)]);
}

/**
 * Los que pidieron desconectarse y ya cumplieron su espera quedan desconectados (Doc 1.10.6), en orden de id; quien va en
 * columna y está perseguido espera un poco más (D66). Quien está en
 * una batalla de Unity espera a que termine (D33b, decisión del usuario 2026-10-04: hasta que exista el abandono de
 * batalla con Conquest).
 */
function conSalidasDelMundo(
  estado: GameSessionState,
  mapa: Mapa,
  instante: number,
  enBatalla: ReadonlySet<string>
): { estado: GameSessionState; eventos: EventoCrudo[] } {
  // En peligro no se sale (D66): si una columna le persigue, espera mientras dure y como mucho hasta el tope.
  const extra = PRESENCIA.topeAplazamientoMs - PRESENCIA.retardoDesconexionMs;
  const perseguido = (heroeId: string) => {
    const suya = columnaDe(estado.ejercitos, heroeId);
    return !!suya && estado.ejercitos.some((e) => e.id !== suya.id && e.persiguiendo?.tipo === 'ejercito' && e.persiguiendo.id === suya.id);
  };
  const salen = estado.heroes
    .filter((h) => h.desconectaEn !== undefined && h.desconectaEn <= instante && !h.fuera && !enBatalla.has(h.id))
    .filter((h) => instante >= h.desconectaEn! + extra || !perseguido(h.id))
    .map((h) => h.id)
    .sort();
  let actual = estado;
  const eventos: EventoCrudo[] = [];
  for (const heroeId of salen) {
    const r = salirDelMundo(actual, heroeId, mapa);
    actual = { ...actual, heroes: r.heroes, ejercitos: r.ejercitos, caravanas: r.caravanas };
    eventos.push(...r.eventos);
  }
  return { estado: actual, eventos };
}
