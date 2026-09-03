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
import type { Escuadron, Jugador } from '../domain/types';
import { LIDERAZGO, TROPAS_RECLUTABLES } from '../constants';

/**
 * Coste de liderazgo de una tropa (tipo), DERIVADO de su poder nominal completo (Doc 5.11.1).
 *
 * Se deriva y no se escribe a mano en el catálogo por una razón concreta: `poderBase` sigue siendo
 * PLACEHOLDER pendiente de calibración (Doc 5.8), así que once números escritos a mano se desincronizarían
 * del poder en cuanto se calibre. Con una fórmula, calibrar el poder recalcula el coste solo, y lo único que
 * se ajusta a mano es `LIDERAZGO.factorCoste`.
 *
 * Es `poderBase × unidadesPorDefecto` y no `poderBase` a secas porque lo que se comanda son SOLDADOS, no
 * estadísticas por soldado. Usa `unidadesPorDefecto` (constante del catálogo) y NO la cantidad actual del
 * escuadrón: si mirara la cantidad, un escuadrón de élite a media fuerza costaría menos y la tropa cara
 * pasaría a ser fieldeable "en dosis pequeñas", que es justo lo que el gate quiere impedir. El efecto
 * secundario conocido —un escuadrón mermado paga el precio completo— está anotado como punto abierto en
 * `Consideraciones/Movimiento_Ejercitos_Definicion.md` §7.
 *
 * Devuelve 0 para un `tropaId` que no exista en el catálogo: no es tarea de esta función rechazar tropas
 * inventadas (eso lo hace el reclutamiento), y cobrar por algo que no existe sería peor que no cobrar.
 */
export function costeLiderazgo(tropaId: string): number {
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) return 0;
  return tropa.poderBase * tropa.unidadesPorDefecto * LIDERAZGO.factorCoste;
}

/** Liderazgo efectivo de un jugador. Un jugador sin registro en el estado usa el base — así una partida
 * guardada de antes de esta mecánica no necesita migración ni deja a nadie a 0 (ver `Jugador`). */
export function liderazgoDe(jugador: Jugador | undefined): number {
  return jugador?.liderazgoBase ?? LIDERAZGO.base;
}

/** Liderazgo que consumen estos escuadrones juntos. El llamador es quien decide que todos son del MISMO
 * jugador: en un ejército de varios, cada uno se valida contra SU propio liderazgo por separado y no hay
 * tope agregado (Doc 5.11). */
export function liderazgoComprometido(escuadrones: readonly Escuadron[]): number {
  return escuadrones.reduce((acc, e) => acc + costeLiderazgo(e.tropaId), 0);
}

/** ¿Caben estos escuadrones en el liderazgo de este jugador? */
export function puedeLlevar(jugador: Jugador | undefined, escuadrones: readonly Escuadron[]): boolean {
  return liderazgoComprometido(escuadrones) <= liderazgoDe(jugador);
}

/** Lo que le queda libre a un jugador que ya lleva `escuadrones` comprometidos — para que la interfaz pueda
 * mostrar "38/50" y decidir qué apagar sin reimplementar la resta. */
export function liderazgoDisponible(jugador: Jugador | undefined, escuadrones: readonly Escuadron[]): number {
  return Math.max(0, liderazgoDe(jugador) - liderazgoComprometido(escuadrones));
}
