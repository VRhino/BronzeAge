// Liderazgo (Doc 5.11): cuánta tropa puede sacar a campaña un jugador.
//
// Módulo PURO y sin dependencias más allá del catálogo: no sabe de asentamientos, ni de ejércitos, ni del
// tick. Existe aparte precisamente para que lo consuman los dos lados sin duplicar la regla — el comando que
// moviliza (que la hace cumplir) y la interfaz (que muestra "38/50 pts" y apaga lo que no cabe antes de que
// el jugador lo intente). Una regla de juego escrita dos veces es una regla que acaba divergiendo.
//
// La distinción que sostiene toda la mecánica: **es un límite de SALIDA, no de posesión**. La guarnición no
// tiene tope de liderazgo; se puede poseer mucha más tropa de la que se puede sacar de una vez, y lo que se
// queda es lo único que defiende el asentamiento (Doc 5.12.4).
import type { Escuadron, Heroe } from '../domain/types';
import { LIDERAZGO, TROPAS_RECLUTABLES } from '../constants';

/**
 * Coste de Liderazgo de una tropa (tipo): el de su ESCALÓN (Doc 5.11.1).
 *
 * Hasta 2026-09-04 se derivaba de `poderBase × unidadesPorDefecto × factorCoste`, con el argumento de que
 * `poderBase` es placeholder y once cifras a mano se desincronizarían al calibrarlo. Ese argumento se cae con
 * el rediseño, y a propósito: **el coste ya no debe seguir al poder.** Si lo sigue de forma proporcional, el
 * poder por punto de Liderazgo sale idéntico para todas las tropas y elegir composición deja de ser una
 * decisión (ver `LIDERAZGO` en constants.ts). El escalón es justamente el grado de libertad que permite que
 * la élite cueste más de lo que rinde.
 *
 * NO mira la `cantidad` actual del escuadrón, solo su tipo: si la mirara, un escuadrón de élite a media
 * fuerza costaría menos y la tropa cara pasaría a ser fieldeable "en dosis pequeñas", que es justo lo que el
 * gate impide. El efecto secundario conocido —un escuadrón mermado paga el precio completo— sigue anotado
 * como punto abierto en `Consideraciones/Movimiento_Ejercitos_Definicion.md` §7.
 *
 * Devuelve 0 para un `tropaId` que no exista en el catálogo: no es tarea de esta función rechazar tropas
 * inventadas (eso lo hace el reclutamiento), y cobrar por algo que no existe sería peor que no cobrar.
 */
export function costeLiderazgo(tropaId: string): number {
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) return 0;
  return LIDERAZGO.costePorEscalon[tropa.escalon] ?? 0;
}

/** Liderazgo efectivo de un jugador. Un jugador sin registro en el estado usa el base — así una partida
 * guardada de antes de esta mecánica no necesita migración ni deja a nadie a 0 (ver `Jugador`). */
export function liderazgoDe(jugador: Heroe | undefined): number {
  return jugador?.liderazgoBase ?? LIDERAZGO.base;
}

/** Liderazgo que consumen estos escuadrones juntos. El llamador es quien decide que todos son del MISMO
 * jugador: en un ejército de varios, cada uno se valida contra SU propio liderazgo por separado y no hay
 * tope agregado (Doc 5.11). */
export function liderazgoComprometido(escuadrones: readonly Escuadron[]): number {
  return escuadrones.reduce((acc, e) => acc + costeLiderazgo(e.tropaId), 0);
}

/** ¿Caben estos escuadrones en el liderazgo de este jugador? */
export function puedeLlevar(jugador: Heroe | undefined, escuadrones: readonly Escuadron[]): boolean {
  return liderazgoComprometido(escuadrones) <= liderazgoDe(jugador);
}

/** Lo que le queda libre a un jugador que ya lleva `escuadrones` comprometidos — para que la interfaz pueda
 * mostrar "38/50" y decidir qué apagar sin reimplementar la resta. */
export function liderazgoDisponible(jugador: Heroe | undefined, escuadrones: readonly Escuadron[]): number {
  return Math.max(0, liderazgoDe(jugador) - liderazgoComprometido(escuadrones));
}
