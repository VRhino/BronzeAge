// Constantes de INFRAESTRUCTURA del servidor: cómo se sirve, se recorta y se protege una partida. NO son reglas del juego.
//
// Las reglas del juego (lo que cambia el mundo simulado) viven en `constants.ts`, se sirven en `GET /v1/balance` y las
// lee el motor. Estas no: el motor no las ve, no afectan a un cálculo de la simulación y cambiarlas no altera el
// resultado de una partida (sí su coste, su memoria o cuándo se descarta un atraso). Los dos grupos no se mezclan.

export const INFRA = {
  /**
   * Cuántos ticks atrasados acepta ejecutar de golpe una pasada de `sincronizarConReloj` (`server/runnerDePartida.ts`).
   * Por encima de eso, el tiempo pendiente **se descarta** en vez de ejecutarse.
   *
   * **No es un tope de rendimiento, es dónde se traza la frontera** entre las dos cosas que producen un
   * atraso, que se parecen mucho vistas desde aquí:
   *
   *  - **Deriva del temporizador**, que SÍ hay que recuperar: `setInterval` no dispara exacto, y unas décimas
   *    por disparo se acumulan hasta valer un tick entero cada ~10 minutos. Si no se recuperaran, el mundo
   *    correría más lento que el tiempo real y "1 tick = 1 minuto" dejaría de ser cierto. Esto produce 2
   *    ticks de atraso como mucho.
   *  - **El proceso no estuvo sirviendo** (host suspendido, event loop bloqueado un minuto largo, salto de
   *    reloj por una corrección NTP), que NO hay que recuperar por la misma razón por la que no se recupera
   *    un reinicio. Esto produce decenas o cientos.
   *
   * 5 separa las dos con holgura por los dos lados. Es el único número elegido a ojo de todo esto; súbelo si
   * alguna vez se ven `ticksOmitidos` con el servidor sano.
   */
  ticksMaximosPorPasada: 5,

  /** Cuántas claves de idempotencia recuerda cada partida (`RunnerDePartida.ejecutar`); al pasarse se olvida la más antigua. */
  limiteIdempotencia: 500,

  /**
   * Cuántos eventos conserva `estado.eventosDominio` en memoria. El historial completo vive en el JSONL de la partida
   * (`server/eventosDePartida.ts`); el estado solo guarda los últimos para servir los cursores recientes y para que
   * `exito()` no copie, en cada comando, un array que crece sin techo (a los 4 500 ticks eran 59 000 eventos y más de
   * 1,6 ms por copia, tres veces por tick; `Consideraciones/Auditoria_Tick_Eventos.md`, N2a).
   */
  eventosMaximosEnMemoria: 5_000,
} as const;
