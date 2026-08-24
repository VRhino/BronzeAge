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
   * Momento de simulación en que ocurrió el evento (ISO 8601). **Este es el campo temporal definitivo**: el
   * protocolo hacia los clientes debe usar SIEMPRE este y nunca `tick` (ver
   * Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md §4, regla (b)) — así el día que el motor pase
   * a tiempo real (Fase D) el contrato no cambia. Lo inyecta quien avanza la simulación
   * (`ContextoSimulacion.momento`, ver `engine/simulation.ts`); el motor nunca lee el reloj por su cuenta.
   */
  momento: string;
  /**
   * Tick en el que ocurrió el evento. **PROVISIONAL**: desaparece al completarse la Fase D — es la unidad
   * interna del motor, no un contrato temporal. Para cualquier consumidor externo, usar `momento`.
   */
  tick: number;
  /** Asentamiento al que se atribuye, si aplica (mismo criterio que hoy usa `simulation.ts` para prefijar
   * cada mensaje de un asentamiento con su id). Ausente en eventos globales (comercio, mercado, títulos...). */
  asentamientoId?: string;
}
