// FIXTURE DE TESTS, no es del juego. Fundar a pie desapareció del juego (D7, D19: se funda con la Caravana de Fundación, `fundar`); queda
// aquí lo que hacía —aparecer con una columna en un punto al azar y fundar donde se está— porque muchos tests necesitan una plaza
// como punto de partida y no son tests de cómo se funda.
import type { Asentamiento, Ejercito, Heroe, Point } from '../../domain/types';
import type { Instante } from '../../domain/tiempo';
import { FUNDACION } from '../../constants';
import type { Mapa } from '../../world/mapa';
import { calcularRuta } from '../../world/rutas';
import { distancia } from '../../world/geometria';
import type { RandomFn } from '../../worldgen';
import { MovilizacionInvalidaError } from '../../engine/ejercitos';
import { evaluarViabilidadFundacion, exigirPuertaDeFundacion, fundarAsentamiento as fundarAsentamientoEngine } from '../../engine/settlement';
import { esCiudadano } from '../../engine/faccion';
import { salirDeCampamentos } from '../../engine/mercenarios';
import { cruzarLaPuerta, situarHeroes } from '../../engine/ubicacion';
import { conEscuadrones } from '../../engine/tropa';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from '../comandos/tipos';
import { comando, conExploracionFundida, conTropaDe, exigirJugador } from '../comandos/ayudas';
import { desdeCrudos, evento } from '../comandos/eventos';

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
    escuadronIds: [],
    suministro: {},
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto },
    ruta: [],
    progreso: 0,
    posicionActual: punto,
    estado: 'estacionado',
  };
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
  if (jugador.ubicacion.tipo === 'asentamiento' || jugador.ubicacion.tipo === 'mercenarios') {
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

export interface PayloadAsentamientoFundado {
  asentamientoId: string;
  faccionId: string;
  posicion: Point;
  heroesIds: string[];
}

export interface ParamsFundarAsentamiento {
  faccionId: string;
}

/**
 * Funda un asentamiento nuevo para una Facción existente. El fundador es EL ACTOR: recibe casa y, con ella,
 * ciudadanía inmediata de la Facción (Doc 1.2/1.3).
 *
 * **Se funda DONDE SE ESTA** (Doc 1.3): la posición no la elige el cliente, sale de la columna del fundador
 * (`puntoDeFundacionDe`) — hay que estar en campo abierto, ni dentro de una plaza ni desconectado. Fundar es
 * ENTRAR en lo que se acaba de levantar, así que esa columna se deshace dentro (`cruzarLaPuerta`), con sus
 * tropas y su carga si llevaba alguna: casi nunca, porque la columna con la que se aparece nace vacía, pero
 * un ciudadano que funda de campaña con su propia columna sí puede llegar con algo.
 *
 * Autorización (`comandos/autorizacion.ts`): rol `jugador`, y ser ya ciudadano de esa Facción — salvo que no
 * sea ciudadano de ninguna, porque fundar es una de las vías de ENTRAR en una (la otra es pedirlo al Rey, `solicitarIngreso`).
 *
 * **Fundación grupal diferida.** El Doc 1.2/1.3 admite hasta 5 fundadores juntos, y el motor lo soporta
 * (`fundarAsentamiento` de `engine/settlement.ts` recibe una lista). No se expone aquí porque falta lo que la
 * haría legítima: un mecanismo de CONSENTIMIENTO. Aceptar una lista de cofundadores del cliente permitiría
 * meter a cualquier jugador en una Facción sin que él lo pidiera —y, como un jugador solo puede pertenecer a
 * una (Doc 0), dejarlo bloqueado para entrar en la que quería—. Eso es una vía de acoso, no una función.
 *
 * Hasta la Fase C2 este comando fabricaba sus fundadores (`jugador-<faccionId>-<n>`) a partir de un
 * `numJugadores`, herencia de cuando no había identidad real. Con la ciudadanía ya derivada del estado de
 * juego para autorizar (ver `Membresia` en `acceso/tipos.ts`), esos ids ficticios dejaban al jugador real sin
 * ninguna forma de hacerse ciudadano: creaba la Facción, fundaba, y la ciudadanía se la quedaban cinco
 * jugadores que no existían.
 *
 * La gobernanza NPC no pasa por aquí: funda con sus héroes bot (`fundarAsentamientosIniciales`,
 * `session/npcGobernanza.ts`).
 */
export const fundarAsentamiento = comando<ParamsFundarAsentamiento, { asentamientoId: string }>((estado, mapa, ctx, params) => {
  const heroesIds = [ctx.actor];

  // La puerta de entrada al mundo (`Consideraciones/Entrada_Al_Mundo_Definicion.md`): cuánta gente hace falta
  // para fundar, y si hay que haber sido ciudadano antes. Hoy las dos palancas están abiertas para las
  // primeras pruebas; lo que se decida después se enchufa en `exigirPuertaDeFundacion` y no aquí.
  //
  // "Ya fue ciudadano" se resuelve con `salidasFaccionPorHeroe`, que es el registro de quien ALGUNA VEZ
  // dejó una Facción, más la ciudadanía vigente. Un jugador que nunca ha estado en ninguna no aparece en
  // ninguno de los dos.
  const yaFueCiudadano =
    estado.salidasFaccionPorHeroe[ctx.actor] !== undefined || estado.facciones.some((f) => esCiudadano(f, ctx.actor));
  exigirPuertaDeFundacion(heroesIds, yaFueCiudadano);

  const fundador = exigirJugador(estado, ctx.actor);
  const { posicion, columna } = puntoDeFundacionDe(fundador, estado.ejercitos);

  const resultado = fundarAsentamientoEngine(
    mapa,
    estado.facciones,
    params.faccionId,
    posicion,
    heroesIds,
    estado.asentamientos,
    ctx.instante
  );

  // Fundar es ENTRAR en lo que se acaba de levantar (Doc 1.10): el fundador ya es residente
  // (`fundarAsentamientoEngine` lo puso en `heroesFundadoresIds`), así que la columna con la que llegó se
  // deshace dentro — sus tropas al campamento, su carro al almacén — igual que al cruzar la puerta de
  // cualquier otra residencia. Reutiliza `cruzarLaPuerta` en vez de repetir la regla: es la MISMA entrada,
  // solo que a una plaza que nace en este mismo instante.
  const cruce = cruzarLaPuerta(conTropaDe(estado, columna), resultado.asentamiento, ctx.actor, estado.relaciones);

  let siguiente: GameSessionState = {
    ...estado,
    asentamientos: [...estado.asentamientos, cruce.asentamiento],
    facciones: resultado.facciones,
    ejercitos: cruce.disuelveColumna ? estado.ejercitos.filter((e) => e.id !== columna.id) : estado.ejercitos,
    // Quien funda reside en lo que levanta: deja el campamento de mercenarios donde residiera (Doc 1.9b).
    campamentosMercenarios: salirDeCampamentos(estado.campamentosMercenarios, ...heroesIds),
    // Única colocación que hace este comando, y hace falta porque `cruzarLaPuerta` no toca `Jugador.ubicacion`
    // — solo fusiona tropas y carga en el asentamiento.
    heroes: situarHeroes(conEscuadrones(estado.heroes, cruce.tropa), heroesIds, { tipo: 'asentamiento', asentamientoId: resultado.asentamiento.id }),
  };

  const nombreFaccion = resultado.facciones.find((f) => f.id === params.faccionId)?.nombre ?? params.faccionId;
  for (const heroeId of heroesIds) {
    // Lo que anduvo sin bandera pasa a ser conocimiento de la Facción que acaba de fundar (Doc 1.3).
    siguiente = conExploracionFundida(siguiente, heroeId, params.faccionId);
    siguiente = conHistorialDeJugador(siguiente, heroeId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
  }

  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'fundacion.asentamiento_fundado',
        mensaje: `${nombreFaccion} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`,
        payload: {
          asentamientoId: resultado.asentamiento.id,
          faccionId: params.faccionId,
          posicion,
          heroesIds,
        } satisfies PayloadAsentamientoFundado,
        asentamientoId: resultado.asentamiento.id,
      }),
      ...desdeCrudos(ctx, resultado.eventos),
    ],
    { asentamientoId: resultado.asentamiento.id }
  );
});
