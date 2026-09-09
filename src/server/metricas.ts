// Métricas de operación del proceso (Fase E3, doc 4: "duración de tick/procesamiento, tamaño de cola, tasa
// de errores, clientes conectados").
//
// Este archivo NO mide nada: ENSAMBLA. Cada número lo lleva ya quien lo conoce de primera mano —
// `RunnerDePartida` (cola, tick, ráfagas), `RegistroDeAuditoria` (recuento por resultado, fallos de
// escritura) y `HubDeDifusion` (conexiones)— y aquí solo se juntan en una instantánea coherente. Es
// deliberado: un colector global al que todo el mundo empuja números acaba siendo un segundo sitio donde la
// verdad puede divergir del sitio donde ocurre.
//
// **Por qué existen estas métricas y no otras.** Salen de una pregunta concreta, no de una lista genérica:
// la re-medición de escala del 2026-09-05 (doc 6 §1) dejó ver que el riesgo real para la cola serial no es un
// tick lento sino la RÁFAGA DE CATCH-UP tras una caída (~79 min de cola bloqueada en el peor caso). Por eso
// `ultimaRafagaTicks`/`mayorRafagaTicks` viajan junto a `colaPendiente`: son las dos caras del mismo
// problema, y sin ellas un servidor atascado solo se ve desde fuera como "no responde".
import type { RegistroDeAuditoria, RecuentoDeComandos } from './auditoria';
import type { HubDeDifusion } from './difusion/hub';
import type { RegistroDePartidas } from './registroDePartidas';
import type { MetricasDePartida } from './runnerDePartida';

/** Lo que sabe el proceso de sí mismo. Separado de lo de cada partida porque su ciclo de vida es otro: esto
 * arranca con el proceso, las partidas van y vienen. */
export interface MetricasDeProceso {
  /** Segundos desde que arrancó el proceso. Contexto imprescindible para leer cualquier acumulado de abajo:
   * 300 rechazos en un mes y 300 en cinco minutos no son el mismo servidor. */
  arribaSegundos: number;
  /** Memoria residente y heap, en MB. Lo más barato que delata una fuga. */
  memoriaMb: { rss: number; heapUsado: number; heapTotal: number };
  /** Partidas abiertas EN ESTE PROCESO — no las que hay en disco (eso lo dice `GET /admin/partidas`). */
  partidasAbiertas: number;
}

export interface Metricas {
  momento: string;
  proceso: MetricasDeProceso;
  /** Comandos por resultado desde el arranque, con las cuatro causas de rechazo separadas. */
  comandos: RecuentoDeComandos;
  /** Escrituras de auditoría perdidas. **Cualquier valor > 0 es un incidente**: significa que hay comandos
   * que ocurrieron y de los que no queda constancia (ver la decisión "un fallo de escritura no tumba el
   * comando" en `auditoria.ts`). */
  auditoriaFallida: number;
  partidas: (MetricasDePartida & { conexiones: number })[];
}

export interface FuentesDeMetricas {
  partidas: RegistroDePartidas;
  auditoria: RegistroDeAuditoria;
  hub: HubDeDifusion;
  ahora: () => string;
  /** Inyectable para poder congelarlo en un test — mismo criterio que el reloj. Por defecto, el del proceso. */
  arribaSegundos?: () => number;
}

const MB = 1024 * 1024;

/** Instantánea de todas las métricas. Barata a propósito (lee contadores ya llevados, no recorre estado de
 * juego): tiene que poder pedirse cada pocos segundos sin que consultarla sea lo que ralentice el servidor. */
export function recogerMetricas(fuentes: FuentesDeMetricas): Metricas {
  const memoria = process.memoryUsage();
  const abiertas = fuentes.partidas.abiertas();

  return {
    momento: fuentes.ahora(),
    proceso: {
      arribaSegundos: Math.round(fuentes.arribaSegundos ? fuentes.arribaSegundos() : process.uptime()),
      memoriaMb: {
        rss: Math.round(memoria.rss / MB),
        heapUsado: Math.round(memoria.heapUsed / MB),
        heapTotal: Math.round(memoria.heapTotal / MB),
      },
      partidasAbiertas: abiertas.length,
    },
    comandos: fuentes.auditoria.recuento,
    auditoriaFallida: fuentes.auditoria.fallos,
    partidas: abiertas.map((runner) => ({
      ...runner.metricas(),
      conexiones: fuentes.hub.conexionesAbiertas(runner.gameId),
    })),
  };
}
