// Dónde está cada Jugador (Doc 1.10): el jugador como entidad SITUADA en el mundo.
//
// Este módulo existe porque la ubicación tiene DOS orígenes que tienen que coincidir: el estado la lleva
// explícita en `Jugador.ubicacion`, pero hay dos momentos en que no hay registro del que leerla —cuando un
// jugador actúa por primera vez y cuando se carga una partida anterior a esta mecánica— y en los dos hay que
// DEDUCIRLA de lo que el mundo ya sabe. Una sola función para los dos casos, o la partida migrada acabaría
// colocando a la gente en un sitio distinto del que la coloca el juego en marcha.
import type { Asentamiento, Ejercito, Jugador, UbicacionJugador } from '../domain/types';
import { esResidente } from './pertenencia';

/**
 * Dónde está un jugador según lo que el mundo sabe de él, sin consultar su registro.
 *
 * El ORDEN importa y es el del canon: **la columna gana a la residencia**. Un jugador de campaña sigue
 * residiendo en su ciudad —eso es ciudadanía, no presencia (Doc 2.5)— pero está en el camino, no dentro.
 *
 * Quien no está en ninguno de los dos no está en el mundo. No es un error: es el huérfano, el que aún no ha
 * fundado, o alguien de quien la partida solo conserva el rastro (cargos, historial). Se le da un punto
 * PLACEHOLDER porque el estado no tiene ninguno que darle — el spawn de la entrada en partida es lo que le
 * pondrá uno de verdad.
 */
export function ubicacionDeducida(
  jugadorId: string,
  asentamientos: readonly Asentamiento[],
  ejercitos: readonly Ejercito[]
): UbicacionJugador {
  const columna = ejercitos.find((e) => e.participantes.some((p) => p.jugadorId === jugadorId));
  if (columna) return { tipo: 'columna', ejercitoId: columna.id };

  const residencia = asentamientos.find((a) => esResidente(a, jugadorId));
  if (residencia) return { tipo: 'asentamiento', asentamientoId: residencia.id };

  return { tipo: 'desconectado', punto: { x: 0, y: 0 } };
}

/**
 * Devuelve la lista de jugadores con la garantía de que `jugadorId` tiene registro, deduciendo su ubicación
 * si hay que crearlo. Idempotente: si ya está, devuelve la MISMA lista, y por eso se puede llamar en el
 * camino de todos los comandos sin copiar el array en cada uno.
 *
 * No toca al que ya existe. Mover a alguien es cosa de los comandos de movimiento, no de aparecer.
 */
export function conJugadorAsegurado(
  jugadores: readonly Jugador[],
  jugadorId: string,
  liderazgoBase: number,
  asentamientos: readonly Asentamiento[],
  ejercitos: readonly Ejercito[]
): Jugador[] {
  if (jugadores.some((j) => j.id === jugadorId)) return jugadores as Jugador[];
  return [...jugadores, { id: jugadorId, liderazgoBase, ubicacion: ubicacionDeducida(jugadorId, asentamientos, ejercitos) }];
}

/** Coloca a varios jugadores en el mismo sitio, dejando intacto a quien no esté en la lista. Se usa al fundar
 * — la única operación de hoy que sitúa a alguien sin moverlo, porque construir donde estás parado no es un
 * viaje. Al que no tenga registro no lo inventa: de eso se encarga el alta de `GameSession`, que corre
 * después en el mismo comando. */
export function situarJugadores(jugadores: readonly Jugador[], ids: readonly string[], ubicacion: UbicacionJugador): Jugador[] {
  const aSituar = new Set(ids);
  return jugadores.map((j) => (aSituar.has(j.id) ? { ...j, ubicacion } : j));
}
