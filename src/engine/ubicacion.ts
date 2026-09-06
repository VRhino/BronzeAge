// Dónde está cada Jugador (Doc 1.10): el jugador como entidad SITUADA en el mundo.
//
// Este módulo existe porque la ubicación tiene DOS orígenes que tienen que coincidir: el estado la lleva
// explícita en `Jugador.ubicacion`, pero hay dos momentos en que no hay registro del que leerla —cuando un
// jugador actúa por primera vez y cuando se carga una partida anterior a esta mecánica— y en los dos hay que
// DEDUCIRLA de lo que el mundo ya sabe. Una sola función para los dos casos, o la partida migrada acabaría
// colocando a la gente en un sitio distinto del que la coloca el juego en marcha.
import type { Asentamiento, Ejercito, InteriorRecordado, Jugador, RelacionPolitica, UbicacionJugador } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { MOVIMIENTO } from '../constants';
import { absorberColumna, enLaPuertaDe, MovilizacionInvalidaError } from './ejercitos';
import { esResidente, puedeEntrarEn } from './pertenencia';

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

/**
 * Cruzar la puerta de una plaza (Doc 1.10.3). Devuelve lo que pasa, sin decidir nada de estado: si la
 * columna se disuelve dentro —y con qué asentamiento resultante— o si se queda aparcada a la puerta.
 *
 * Las cuatro condiciones, y qué tapa cada una:
 *
 *  1. **Solo en columna personal.** Un Ejército lleva a varios: dejarle cruzar la puerta desde dentro lo
 *     disolvería con su gente dentro, y sería además una salida encubierta que se salta al Líder (Doc
 *     5.14.2). Hay que separarse antes.
 *  2. **En la puerta**, no en las afueras: entrar es un acto, no un roce (Doc 5.12.3).
 *  3. **Con permiso** del Gobernador (Doc 1.10.5).
 *  4. Y entonces, **residencia o no**, que es lo que decide si la columna se deshace o espera fuera.
 */
export function cruzarLaPuerta(
  columna: Ejercito,
  asentamiento: Asentamiento,
  jugadorId: string,
  relaciones: readonly RelacionPolitica[]
): { asentamiento: Asentamiento; disuelveColumna: boolean } {
  if (columna.tipo === 'ejercito') {
    throw new MovilizacionInvalidaError('Vas en un ejército: hay que separarse antes de entrar en una plaza.');
  }
  if (!enLaPuertaDe(columna, asentamiento)) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${MOVIMIENTO.radioPuerta} de la plaza para entrar.`);
  }
  if (!puedeEntrarEn(asentamiento, jugadorId, columna.faccionId, relaciones)) {
    throw new MovilizacionInvalidaError('Esa plaza no te deja entrar.');
  }

  // En tu residencia la columna se DESHACE —tropas a la guarnición, carro al almacén— porque ahí tienes
  // todo delante y volver a salir vuelve a elegir. En cualquier otra se queda esperando intacta.
  return esResidente(asentamiento, jugadorId)
    ? { asentamiento: absorberColumna(asentamiento, columna, true), disuelveColumna: true }
    : { asentamiento, disuelveColumna: false };
}

/**
 * Salir de una plaza AJENA retomando la columna aparcada (Doc 1.10.3), sin pantalla de equipamiento.
 *
 * Desde tu residencia se rechaza a propósito en vez de hacer lo mismo en silencio: allí hay un roster entero
 * y un almacén que elegir, y eso es `salirAlMundo`.
 */
export function retomarColumna(asentamiento: Asentamiento, jugadorId: string): void {
  if (esResidente(asentamiento, jugadorId)) {
    throw new MovilizacionInvalidaError('De tu propia residencia se sale eligiendo tropas y carga, con `salirAlMundo`.');
  }
}

/**
 * Congela lo que este jugador estaba viendo del interior de esta plaza (Doc 1.10.1), para que al salir le
 * quede una foto fechada en vez de un agujero.
 *
 * Guarda solo la cola —`en_cola` y `en_construccion`—, el almacen y la guarnicion. Lo `activo` no entra: es
 * publico y viaja vivo en la ficha, asi que recordarlo seria guardar dos veces el mismo hecho.
 */
export function conFotoDelInterior(jugador: Jugador, asentamiento: Asentamiento, vistoEn: Instante): Jugador {
  const foto: InteriorRecordado = {
    vistoEn,
    almacen: asentamiento.almacen,
    cola: asentamiento.edificios.filter((e) => e.estado !== 'activo'),
    guarnicion: asentamiento.escuadrones,
  };
  return { ...jugador, plazasRecordadas: { ...jugador.plazasRecordadas, [asentamiento.id]: foto } };
}

/** Aplica la foto al jugador indicado, dejando intactos los demas. Envoltorio sobre `conFotoDelInterior` para
 * que el comando no tenga que buscar y reemplazar a mano. */
export function conFotoTomadaPor(
  jugadores: readonly Jugador[],
  jugadorId: string,
  asentamiento: Asentamiento,
  vistoEn: Instante
): Jugador[] {
  return jugadores.map((j) => (j.id === jugadorId ? conFotoDelInterior(j, asentamiento, vistoEn) : j));
}

/**
 * ¿Está este jugador DENTRO de esta plaza (Doc 2.5: la ciudadanía habilita, la presencia ejerce)?
 *
 * Sin registro se deduce, con la misma función que usan el alta y la proyección: quien nunca ha dado una
 * orden está donde el mundo dice que está, no en ninguna parte. Si divergieran, un jugador recién llegado no
 * podría hacer nada en su propia ciudad hasta haber hecho algo antes — que es imposible.
 */
export function estaEnAsentamiento(
  jugadores: readonly Jugador[],
  jugadorId: string,
  asentamientoId: string,
  asentamientos: readonly Asentamiento[],
  ejercitos: readonly Ejercito[]
): boolean {
  const registrada = jugadores.find((j) => j.id === jugadorId)?.ubicacion;
  const ubicacion = registrada ?? ubicacionDeducida(jugadorId, asentamientos, ejercitos);
  return ubicacion.tipo === 'asentamiento' && ubicacion.asentamientoId === asentamientoId;
}
