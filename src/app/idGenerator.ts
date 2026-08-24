// Contador secuencial de IDs de una partida (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase A4) — extraído
// de `GameStore` para poder trasladarlo intacto a un futuro `GameSession` de backend (Fase B) sin arrastrar el
// resto de responsabilidades de sesión/UI de `GameStore`. Deliberadamente mínimo: un entero que solo avanza,
// serializable tal cual (`actual()` devuelve un `number` plano) sin envoltorio adicional — no hay nada más que
// persistir. Mismo consumidor que `app/npcGobernanza.ts` (`ConfigNpcGobernanza.contadorInicial`/`contadorFinal`),
// que lleva su propia cuenta con el mismo patrón en vez de depender de esta clase, para no acoplar el motor de
// aplicación a un tipo concreto.
export class GeneradorIds {
  private valor: number;

  constructor(inicial = 0) {
    this.valor = inicial;
  }

  /** Devuelve el próximo id de la secuencia y avanza el contador (semántica postfix, como `contador++`). */
  siguiente(): number {
    return this.valor++;
  }

  /** Valor actual sin consumirlo — para pasarlo como `contadorInicial` a una llamada externa (ej. NPC). */
  actual(): number {
    return this.valor;
  }

  /** Fija el contador (ej. al `contadorFinal` devuelto por una llamada externa que generó ids por su cuenta). */
  fijar(valor: number): void {
    this.valor = valor;
  }
}
