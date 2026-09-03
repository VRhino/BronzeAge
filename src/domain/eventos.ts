// Eventos de dominio (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase A5) — el primer paso de una
// migración deliberadamente incremental: separar los eventos de un tick de los mensajes de log en texto que
// hoy produce cada subsistema del motor (`engine/construction.ts`, `engine/trade.ts`, etc., todos devuelven
// `eventos: string[]`). Un log en texto no es un contrato válido entre servidor y cliente (Docs/Arquitectura/
// 2_Estudio_Evolucion_Backend_Multifrontend.md, punto 6: "los logs localizados son una presentación; no deben
// ser el único contrato entre servidor y frontend").
//
// Esta primera versión NO migra ningún subsistema todavía: envuelve el mismo texto que ya se generaba
// (`mensaje`) bajo un código genérico (`codigo: 'legado'`) más metadata que ya estaba implícita en cómo
// `engine/simulation.ts` arma el log (a qué asentamiento pertenece, en qué tick ocurrió). Migrar un subsistema
// consiste en darle su propio `codigo` estable y mover lo que hoy es solo texto a `payload` tipado — subsistema
// por subsistema, sin tocar los demás cada vez.
export interface EventoDominio {
  /**
   * Código corto y estable del tipo de evento (ej. futuro `construccion.completada`,
   * `poblacion.hambruna.muerte`). Todavía no hay catálogo cerrado de códigos: mientras un subsistema no se
   * haya migrado, sus eventos usan `'legado'` y llevan el mensaje ya formateado en `mensaje`.
   */
  codigo: string;
  /** Mensaje en texto ya formateado — única fuente de verdad mientras dure la migración (ver `codigo`). */
  mensaje: string;
  /**
   * Instante de MUNDO en que ocurrió el evento (ISO 8601). **El único campo temporal del evento** (Fase D
   * cerrada — el `tick` provisional se retiró; doc 6 §4 regla (b)): derivado del tick por quien avanza la
   * simulación (`isoDeInstante(instanteDeTick(tick))`, ver `engine/simulation.ts` / `session/comandos/eventos.ts`),
   * nunca leído del reloj de pared.
   */
  momento: string;
  /** Asentamiento al que se atribuye, si aplica (mismo criterio que hoy usa `simulation.ts` para prefijar
   * cada mensaje de un asentamiento con su id). Ausente en eventos globales (comercio, mercado, títulos...). */
  asentamientoId?: string;
  /** Datos estructurados del evento, específicos de cada `codigo` (ej. `{ edificioId, edificioTipo,
   * nivelNuevo }` para `construccion.mejora_completada`) — lo que antes solo vivía interpolado dentro de
   * `mensaje`. `unknown` a propósito: no hay (ni debe haber) un único tipo que abarque los ~30 códigos
   * distintos; cada subsistema exporta su propia interfaz de payload junto al código que la usa, y un
   * consumidor que filtra por `codigo` sabe con qué forma castear. Ausente en eventos `'legado'` (subsistemas
   * todavía sin migrar, ver `EventoCrudo`) y en cualquier evento migrado sin datos extra que estructurar. */
  payload?: unknown;
}

/**
 * Lo que un subsistema del motor (`engine/*.ts`) empuja a su array `eventos` DENTRO de un `avanzarX`, antes de
 * que `engine/simulation.ts` le añada el contexto que el subsistema no conoce (`momento` y `asentamientoId` —
 * el `tick` provisional se retiró al cerrar la Fase D, doc 10 §8). Dos formas, a propósito, para que la
 * migración por subsistema (Docs/Arquitectura/
 * 4_Plan_Evolucion_Tareas.md, Fase A5, marcador 13 subsistemas) sea de uno en uno sin tocar los demás:
 *
 * - Un `string` plano: atajo "legado", mismo comportamiento que existía antes de A5 — se envuelve como
 *   `{ codigo: 'legado', mensaje: <el string> }`. Así es como emite un subsistema TODAVÍA sin migrar.
 * - Un objeto `{ codigo, mensaje, payload? }`: subsistema ya migrado — código estable en vez de `'legado'`,
 *   y los datos que antes solo estaban interpolados en el texto, estructurados en `payload`.
 *
 * `mensaje` se mantiene en AMBOS casos porque sigue siendo la única fuente del log en texto que ya muestra la
 * interfaz (`GameStore`/`main.ts`) — migrar un subsistema añade estructura, no le quita nada a nadie que ya
 * consuma el texto.
 */
export type EventoCrudo =
  | string
  | {
      codigo: string;
      mensaje: string;
      payload?: unknown;
      /**
       * Asentamiento al que se atribuye ESTE evento concreto, cuando el subsistema lo sabe y quien lo llama
       * no. `engine/simulation.ts` atribuye por lotes —le pasa un `asentamientoId` a `comoEventosDominio`
       * porque lo sabe por el bucle que recorre asentamiento a asentamiento—, pero un subsistema GLOBAL que
       * itera sobre otra cosa (los ejércitos, cada uno con su origen distinto) no cabe en ese molde: sin
       * esto sus eventos salen sin atribuir, y un evento sin atribuir es GLOBAL, o sea visible para todo el
       * mundo en `proyectarParaJugador`. Ahí es donde deja de ser una cuestión de forma: narrar a todos que
       * "el ejército X llega a su destino" es exactamente la telemetría de rival que Doc 5.12.7 prohíbe.
       *
       * Cuando está, gana sobre la atribución por lotes. Es el mismo campo, con el mismo significado, que
       * `EventoDeComando` (`session/comandos/eventos.ts`) ya tenía por el lado de los comandos.
       */
      asentamientoId?: string;
    };

/**
 * Atribuye a un asentamiento un evento que ya venía hecho. Existe para el caso en que quien PRODUCE el
 * evento no sabe a quién atribuirlo y quien lo consume sí: `avanzarRacion` narra la deserción de un
 * escuadrón sin saber si ese escuadrón está en una guarnición o en un carro a media marcha, y es
 * `avanzarEjercitos` —que sí lo sabe— quien le pone el origen del ejército antes de acumularlo.
 *
 * Respeta una atribución previa: si el evento ya venía atribuido, ese dato es más específico que el del
 * llamador y no se pisa.
 */
export function atribuir(evento: EventoCrudo, asentamientoId: string): EventoCrudo {
  if (typeof evento === 'string') return { codigo: 'legado', mensaje: evento, asentamientoId };
  return evento.asentamientoId === undefined ? { ...evento, asentamientoId } : evento;
}
