// Dónde está cada Jugador (Doc 1.10): el jugador como entidad SITUADA en el mundo.
//
// Este módulo existe porque la ubicación tiene DOS orígenes que tienen que coincidir: el estado la lleva
// explícita en `Jugador.ubicacion`, pero hay dos momentos en que no hay registro del que leerla —cuando un
// jugador actúa por primera vez y cuando se carga una partida anterior a esta mecánica— y en los dos hay que
// DEDUCIRLA de lo que el mundo ya sabe. Una sola función para los dos casos, o la partida migrada acabaría
// colocando a la gente en un sitio distinto del que la coloca el juego en marcha.
import type { Asentamiento, Ejercito, InteriorRecordado, Heroe, Point, RelacionPolitica, UbicacionHeroe } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { FUNDACION, MOVIMIENTO } from '../constants';
import { calcularRuta } from '../world/rutas';
import { evaluarViabilidadFundacion } from './settlement';
import { distancia } from '../world/geometria';
import type { Mapa } from '../world/mapa';
import type { RandomFn } from '../worldgen';
import { absorberColumna, alcanceDeVista, enLaPuertaDe, MovilizacionInvalidaError } from './ejercitos';
import { esResidente, puedeEntrarEn } from './pertenencia';
import { marcarVisto, rejillaDe, SIN_EXPLORAR } from './exploracion';

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
  heroeId: string,
  asentamientos: readonly Asentamiento[],
  ejercitos: readonly Ejercito[]
): UbicacionHeroe {
  const columna = ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId));
  if (columna) return { tipo: 'columna', ejercitoId: columna.id };

  const residencia = asentamientos.find((a) => esResidente(a, heroeId));
  if (residencia) return { tipo: 'asentamiento', asentamientoId: residencia.id };

  return { tipo: 'desconectado', punto: { x: 0, y: 0 } };
}

/**
 * La columna con la que APARECE en el mundo un héroe recién creado (Doc 1.3), en un punto de aparición: sin
 * tropas, sin carga y **SIN CASA**. Aparecer es nacer CON COLUMNA, no en un punto suelto: los tres sitios
 * donde un héroe puede estar son dentro de una plaza, dentro de una columna o fuera del mundo (Doc 1.10), y
 * sin ella no podría ni moverse ni fundar donde se para.
 *
 * `origenAsentamientoId` va vacio a proposito y no es un hueco mal tapado: un recien llegado no tiene a donde
 * replegarse, que es exactamente la condicion de HUERFANO que el motor ya sabe manejar (Doc 5.4 — si el
 * origen no existe, no hay donde reintegrar). Deja de serlo al fundar o al entrar en una Faccion.
 *
 * `politicaDeUnion: 'rechazar'` no es prudencia: una columna sin bandera no tiene Faccion a la que sumar a
 * quien se una, asi que aceptar compañia no significaria nada todavia.
 */
export function columnaDeAparicion(
  id: string,
  heroeId: string,
  mapa: Mapa,
  asentamientos: readonly Asentamiento[],
  rng: RandomFn,
  instante: Instante
): Ejercito {
  const punto = puntoDeAparicion(mapa, asentamientos, rng);
  return {
    id,
    faccionId: '',
    origenAsentamientoId: '',
    participantes: [{ heroeId, unidoEn: instante }],
    tipo: 'personal',
    liderId: heroeId,
    politicaDeUnion: 'rechazar',
    escuadrones: [],
    suministro: {},
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto },
    ruta: [],
    progreso: 0,
    posicionActual: punto,
    estado: 'estacionado',
  };
}

/** Coloca a varios héroes en el mismo sitio, dejando intacto a quien no esté en la lista. Se usa al fundar
 * — la única operación de hoy que sitúa a alguien sin moverlo, porque construir donde estás parado no es un
 * viaje. */
export function situarHeroes(heroes: readonly Heroe[], ids: readonly string[], ubicacion: UbicacionHeroe): Heroe[] {
  const aSituar = new Set(ids);
  return heroes.map((j) => (aSituar.has(j.id) ? { ...j, ubicacion } : j));
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
  heroeId: string,
  relaciones: readonly RelacionPolitica[]
): { asentamiento: Asentamiento; disuelveColumna: boolean } {
  if (columna.tipo === 'ejercito') {
    throw new MovilizacionInvalidaError('Vas en un ejército: hay que separarse antes de entrar en una plaza.');
  }
  if (!enLaPuertaDe(columna, asentamiento)) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${MOVIMIENTO.radioPuerta} de la plaza para entrar.`);
  }
  if (!puedeEntrarEn(asentamiento, heroeId, columna.faccionId, relaciones)) {
    throw new MovilizacionInvalidaError('Esa plaza no te deja entrar.');
  }

  // En tu residencia la columna se DESHACE —tropas a la guarnición, carro al almacén— porque ahí tienes
  // todo delante y volver a salir vuelve a elegir. En cualquier otra se queda esperando intacta.
  return esResidente(asentamiento, heroeId)
    ? { asentamiento: absorberColumna(asentamiento, columna, true), disuelveColumna: true }
    : { asentamiento, disuelveColumna: false };
}

/**
 * Salir de una plaza AJENA retomando la columna aparcada (Doc 1.10.3), sin pantalla de equipamiento.
 *
 * Desde tu residencia se rechaza a propósito en vez de hacer lo mismo en silencio: allí hay un roster entero
 * y un almacén que elegir, y eso es `salirAlMundo`.
 */
export function retomarColumna(asentamiento: Asentamiento, heroeId: string): void {
  if (esResidente(asentamiento, heroeId)) {
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
export function conFotoDelInterior(jugador: Heroe, asentamiento: Asentamiento, vistoEn: Instante): Heroe {
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
  heroes: readonly Heroe[],
  heroeId: string,
  asentamiento: Asentamiento,
  vistoEn: Instante
): Heroe[] {
  return heroes.map((j) => (j.id === heroeId ? conFotoDelInterior(j, asentamiento, vistoEn) : j));
}

/**
 * ¿Está este jugador DENTRO de esta plaza (Doc 2.5: la ciudadanía habilita, la presencia ejerce)?
 *
 * Sin registro se deduce, con la misma función que usan el alta y la proyección: quien nunca ha dado una
 * orden está donde el mundo dice que está, no en ninguna parte. Si divergieran, un jugador recién llegado no
 * podría hacer nada en su propia ciudad hasta haber hecho algo antes — que es imposible.
 */
export function estaEnAsentamiento(
  heroes: readonly Heroe[],
  heroeId: string,
  asentamientoId: string,
  asentamientos: readonly Asentamiento[],
  ejercitos: readonly Ejercito[]
): boolean {
  const registrada = heroes.find((j) => j.id === heroeId)?.ubicacion;
  const ubicacion = registrada ?? ubicacionDeducida(heroeId, asentamientos, ejercitos);
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
 *
 * Devuelve tambien la columna entera, y no solo el punto: fundar es ENTRAR en la plaza que se acaba de
 * levantar, y quien llama necesita la columna completa para hacerla entrar (`cruzarLaPuerta`) — si llevaba
 * tropas o carga (nunca las lleva la de aparicion, pero SI puede llevarlas la de un ciudadano que funda una
 * plaza nueva para su propia Faccion en marcha), pasan a la guarnicion y al almacen igual que en cualquier
 * otra entrada.
 */
export function puntoDeFundacionDe(jugador: Heroe, ejercitos: readonly Ejercito[]): { posicion: Point; columna: Ejercito } {
  if (jugador.ubicacion.tipo === 'asentamiento') {
    throw new MovilizacionInvalidaError('Se funda en campo abierto: hay que salir de la plaza primero.');
  }
  if (jugador.ubicacion.tipo === 'desconectado') {
    throw new MovilizacionInvalidaError('No estas en el mundo.');
  }
  const ubicacion = jugador.ubicacion;
  const columna = ejercitos.find((e) => e.id === ubicacion.ejercitoId);
  if (!columna) throw new MovilizacionInvalidaError('Tu columna ya no existe.');
  return { posicion: columna.posicionActual, columna };
}

/**
 * Graba en `Jugador.exploracionPersonal` lo que ve quien todavia no tiene bandera (Doc 1.3): sin Faccion no
 * hay `MemoriaFaccion` en la que anotarlo, y sin esto el tramo entre aparecer y fundar seria un paseo a
 * ciegas SIN REGISTRO.
 *
 * Solo aplica a quien esta en una columna HUERFANA (`faccionId === ''`): un jugador con Faccion ya graba en
 * `memoriaPorFaccion` (`engine/memoria.ts`), asi que tocar aqui su registro personal seria grabar el mismo
 * hecho dos veces. Se llama al FINAL del tick, igual que `grabarLoVisto` y por el mismo motivo: lo que se
 * graba es donde acabo la columna, no de donde salio.
 */
export function grabarExploracionPersonal(
  heroes: readonly Heroe[],
  ejercitos: readonly Ejercito[],
  limites: { ancho: number; alto: number }
): Heroe[] {
  const rejilla = rejillaDe(limites);
  let salida: Heroe[] = heroes as Heroe[];

  heroes.forEach((jugador, indice) => {
    const ubicacion = jugador.ubicacion;
    if (ubicacion.tipo !== 'columna') return;
    const columna = ejercitos.find((e) => e.id === ubicacion.ejercitoId);
    if (!columna || columna.faccionId !== '') return;

    const explorado = marcarVisto(jugador.exploracionPersonal ?? SIN_EXPLORAR, rejilla, columna.posicionActual, alcanceDeVista(columna));
    if (explorado === (jugador.exploracionPersonal ?? SIN_EXPLORAR)) return;

    if (salida === heroes) salida = [...heroes];
    salida[indice] = { ...jugador, exploracionPersonal: explorado };
  });

  return salida;
}
