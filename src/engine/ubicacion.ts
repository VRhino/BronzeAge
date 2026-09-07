// Dónde está cada Jugador (Doc 1.10): el jugador como entidad SITUADA en el mundo.
//
// Este módulo existe porque la ubicación tiene DOS orígenes que tienen que coincidir: el estado la lleva
// explícita en `Jugador.ubicacion`, pero hay dos momentos en que no hay registro del que leerla —cuando un
// jugador actúa por primera vez y cuando se carga una partida anterior a esta mecánica— y en los dos hay que
// DEDUCIRLA de lo que el mundo ya sabe. Una sola función para los dos casos, o la partida migrada acabaría
// colocando a la gente en un sitio distinto del que la coloca el juego en marcha.
import type { Asentamiento, Ejercito, InteriorRecordado, Jugador, Point, RelacionPolitica, UbicacionJugador } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { FUNDACION, MOVIMIENTO } from '../constants';
import { calcularRuta } from '../world/rutas';
import { evaluarViabilidadFundacion } from './settlement';
import { distancia } from '../world/geometria';
import type { Mapa } from '../world/mapa';
import type { RandomFn } from '../worldgen';
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
  ejercitos: readonly Ejercito[],
  /** Para APARECER en el mundo a quien el estado no conoce de nada (Doc 1.3). Omitirlo deja el comportamiento
   * de antes del spawn: se deduce, y quien no encaja en ningun sitio queda fuera del mundo. */
  aparicion?: { mapa: Mapa; rng: RandomFn }
): Jugador[] {
  if (jugadores.some((j) => j.id === jugadorId)) return jugadores as Jugador[];

  const deducida = ubicacionDeducida(jugadorId, asentamientos, ejercitos);
  // A quien el mundo NO conoce de nada —ni residencia ni columna— no se le deduce nada: se le hace APARECER
  // (Doc 1.3). `ubicacionDeducida` devolvia para el un punto de relleno, que era un marcador de que esto
  // faltaba.
  //
  // PENDIENTE: aparecer deberia dejarle ademas una COLUMNA, que es la unica forma de estar en el campo
  // (Doc 1.10) y lo que le permitiria moverse y fundar donde se para. Va con "se funda donde se esta", que es
  // el resto del paso 9 — ver `Consideraciones/Jugador_Situado_Definicion.md`.
  const ubicacion: UbicacionJugador =
    deducida.tipo === 'desconectado' && aparicion
      ? { tipo: 'desconectado', punto: puntoDeAparicion(aparicion.mapa, asentamientos, aparicion.rng) }
      : deducida;

  return [...jugadores, { id: jugadorId, liderazgoBase, ubicacion }];
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

/**
 * Donde aparece un jugador que entra en la partida (Doc 1.3): un punto ALEATORIO del mundo, en tierra firme
 * y lejos de lo ya fundado.
 *
 * Aparecer ahi es literal: no se elige un punto sobre el mapa desde fuera, se nace en campo abierto y se
 * camina. Esa caminata es la primera decision del juego y es lo que da sentido a explorar antes de
 * asentarse.
 *
 * **Es una costura a proposito.** Cuando el vestibulo pase a ser "apareces a las puertas de una ciudad que ya
 * funciona" (`Consideraciones/Entrada_Al_Mundo_Definicion.md`, decision 1), se cambia el cuerpo de esta
 * funcion y nada mas.
 *
 * Si el mundo esta tan lleno que ningun punto cumple la distancia minima, se AFLOJA esa exigencia antes que
 * dejar a alguien sin poder entrar: quedarse fuera de la partida es peor que aparecer cerca de un vecino.
 */
export function puntoDeAparicion(
  mapa: Mapa,
  asentamientos: readonly Asentamiento[],
  rng: RandomFn
): Point {
  const lejosDeTodo = (p: Point): boolean =>
    asentamientos.every((a) => distancia(p, a.posicion) >= FUNDACION.distanciaMinimaAparicion);

  let fundable: Point | undefined;
  for (let intento = 0; intento < FUNDACION.intentosDeAparicion; intento++) {
    const punto = { x: rng() * mapa.limites.ancho, y: rng() * mapa.limites.alto };
    // Se aparece donde se PODRIA fundar, aunque no sea buen sitio. No es una comodidad: aparecer en un lugar
    // donde fundar es imposible —agua, cima, dentro de la zona de otro— dejaria al recien llegado obligado a
    // caminar sin saberlo, y el juego no le habria dicho por que.
    //
    // Y la caminata NO pierde sentido, porque el liston es `fundable`, no `recomendable`: un sitio legal no
    // es un sitio bueno. Sin bosque al alcance, fundar ahi es una sentencia (ver `evaluarViabilidadFundacion`),
    // asi que sigue habiendo todas las razones para andar y mirar antes de plantar la primera piedra.
    if (!evaluarViabilidadFundacion(mapa, punto, asentamientos as Asentamiento[]).fundable) continue;
    if (!calcularRuta(mapa, punto, punto)) continue;
    if (lejosDeTodo(punto)) return punto;
    fundable ??= punto;
  }
  // Ninguno lejos: vale el primero fundable que salio. Y si tampoco hubo, el centro del mapa — un mundo sin
  // un solo punto habitable no es un mundo, pero devolver algo es mejor que romper la entrada.
  return fundable ?? { x: mapa.limites.ancho / 2, y: mapa.limites.alto / 2 };
}

/**
 * Desde donde funda este jugador (Doc 1.3): **se funda DONDE SE ESTA**, no en un punto elegido sobre el mapa.
 *
 * Solo se puede desde una columna, que es la unica forma de estar en el campo. Desde dentro de una plaza no
 * —ya estas en una ciudad— y desconectado tampoco.
 */
export function puntoDeFundacionDe(jugador: Jugador, ejercitos: readonly Ejercito[]): Point {
  if (jugador.ubicacion.tipo === 'asentamiento') {
    throw new MovilizacionInvalidaError('Se funda en campo abierto: hay que salir de la plaza primero.');
  }
  if (jugador.ubicacion.tipo === 'desconectado') {
    throw new MovilizacionInvalidaError('No estas en el mundo.');
  }
  const columna = ejercitos.find((e) => e.id === (jugador.ubicacion as { ejercitoId: string }).ejercitoId);
  if (!columna) throw new MovilizacionInvalidaError('Tu columna ya no existe.');
  return columna.posicionActual;
}
