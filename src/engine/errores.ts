/**
 * Base de los errores de regla del motor: "esto no se puede hacer" (reclutar sin equipo, fundar sin sitio…). No son
 * fallos del programa sino respuestas esperadas, así que **no capturan la pila**: el NPC los lanza y los recoge miles
 * de veces por tick al probar qué puede hacer, y capturar la pila de cada uno se llevaba un tercio del tiempo del batch
 * (perfil del 2026-10-01). El mensaje dice todo lo que hace falta.
 */
export class ReglaInvalidaError extends Error {
  constructor(mensaje?: string) {
    const limite = Error.stackTraceLimit;
    Error.stackTraceLimit = 0;
    super(mensaje);
    Error.stackTraceLimit = limite;
  }
}
