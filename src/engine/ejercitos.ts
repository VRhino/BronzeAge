// Ejércitos (Doc 5.12): salir del asentamiento y moverse por el mapa.
//
// Este módulo cubre la COMPOSICIÓN de un ejército —movilizar, unirse, replegar, estacionar—, no su
// movimiento: eso es `avanzarEjercitos` en el tick. La ruta sí se calcula aquí, igual que una caravana
// calcula la suya al despacharse (`engine/trade.ts`), porque es parte de "salir", no de "avanzar".
//
// La regla que sostiene todo lo demás: los escuadrones se van DE VERDAD del asentamiento. No es una
// referencia ni una proyección — salen de `Asentamiento.escuadrones` y entran en `Ejercito.escuadrones`. Por
// eso la guarnición es lo único que defiende (Doc 5.12.4) sin necesidad de ningún predicado extra, y por eso
// `consumoRacionTropas` ya cuenta solo lo que se quedó en casa sin tocar una línea.
import type { Asentamiento, Ejercito, Escuadron, Jugador, Point } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { distancia } from '../world/geometria';
import { LOGISTICA, TROPAS_RECLUTABLES } from '../constants';
import { atribuir, type EventoCrudo } from '../domain/eventos';
import { avanzarPosicionEnRuta } from './movimiento';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { avanzarRacion, reservaDeTrigo } from './tropas';
import { puedeLlevar } from './liderazgo';
import { esResidente } from './pertenencia';

export class MovilizacionInvalidaError extends Error {}

export type ObjetivoEjercito = Ejercito['objetivo'];

/** Punto al que apunta un objetivo, para calcular la ruta. */
function puntoDeObjetivo(objetivo: ObjetivoEjercito, asentamientos: readonly Asentamiento[]): Point {
  if (objetivo.tipo === 'punto') return objetivo.punto;
  const destino = asentamientos.find((a) => a.id === objetivo.id);
  if (!destino) throw new MovilizacionInvalidaError('El asentamiento de destino no existe.');
  return destino.posicion;
}

/**
 * Escuadrones del jugador que existen, tienen soldados y le pertenecen. Rechaza en vez de filtrar en
 * silencio: pedir un escuadrón que no es tuyo o que ya salió a campaña es un error del llamador, y
 * tragárselo dejaría al jugador saliendo con menos tropa de la que creía.
 */
function seleccionarParaCampana(asentamiento: Asentamiento, jugadorId: string, escuadronIds: readonly string[]): Escuadron[] {
  if (escuadronIds.length === 0) throw new MovilizacionInvalidaError('Hay que llevarse al menos un escuadrón.');
  const elegidos: Escuadron[] = [];
  for (const id of escuadronIds) {
    const escuadron = asentamiento.escuadrones.find((e) => e.id === id);
    if (!escuadron) throw new MovilizacionInvalidaError(`El escuadrón ${id} no está en este asentamiento.`);
    if (escuadron.jugadorId !== jugadorId) throw new MovilizacionInvalidaError(`El escuadrón ${id} es de otro jugador.`);
    if (escuadron.cantidad <= 0) throw new MovilizacionInvalidaError(`El escuadrón ${id} está aniquilado.`);
    elegidos.push(escuadron);
  }
  return elegidos;
}

/**
 * Los participantes de una columna: jugadores DISTINTOS, no escuadrones (Doc 5.12.2). Es a la vez el número
 * de rombos que se dibujan en el mapa y el número de carros que lleva el ejército, porque las dos cosas
 * cuentan lo mismo: cuánta gente va ahí.
 */
export function participantesDe(escuadrones: readonly Escuadron[]): number {
  return new Set(escuadrones.map((e) => e.jugadorId)).size;
}

/** Capacidad del carro (Doc 5.13): FIJA por Jugador y aditiva — un ejército de cuatro lleva cuatro carros. */
export function capacidadCarroDe(escuadrones: readonly Escuadron[]): number {
  return participantesDe(escuadrones) * LOGISTICA.capacidadCarroPorJugador;
}

/**
 * Carga el carro con trigo del almacén (Doc 5.13, Paso 6). Se lleva el MENOR de dos topes:
 *
 *  - el espacio que le queda al carro, y
 *  - lo que el asentamiento puede soltar sin bajar de su `reservaDeTrigo` — el mismo margen que ya frena a la
 *    auto-construcción y al reclutamiento. Sacar un ejército cuesta stock real, pero no puede ser la vía por
 *    la que un jugador vacía su propia ciudad y la deja en hambruna.
 *
 * Si no llega, **se sale con menos autonomía y punto**: el diseño dice explícitamente que no se bloquea la
 * salida. Impedir mover tropa porque la ciudad va justa de comida sería una regla mucho más dura que la
 * pedida, y además dejaría al jugador encerrado justo cuando más falta le hace maniobrar.
 *
 * La reserva se mide sobre el asentamiento del que los escuadrones YA se han ido: dejan de comer de aquí en
 * el mismo acto, así que seguir contándolos protegería a bocas que ya no están.
 */
function cargarCarro(
  asentamiento: Asentamiento,
  yaEnElCarro: number,
  capacidad: number
): { asentamiento: Asentamiento; cargado: number } {
  const espacio = Math.max(0, capacidad - yaEnElCarro);
  const disponible = Math.max(0, cantidadDisponible(asentamiento.almacen, 'trigo') - reservaDeTrigo(asentamiento));
  const cargado = Math.min(espacio, disponible);
  if (cargado <= 0) return { asentamiento, cargado: 0 };
  return { asentamiento: { ...asentamiento, almacen: descontarRecursos(asentamiento.almacen, { trigo: cargado }) }, cargado };
}

/** Tope de Liderazgo del jugador sobre lo que ESE jugador aporta (Doc 5.11): en un ejército de varios no hay
 * tope agregado, cada uno se valida contra el suyo. */
function exigirLiderazgo(jugador: Jugador | undefined, escuadrones: readonly Escuadron[]): void {
  if (!puedeLlevar(jugador, escuadrones)) {
    throw new MovilizacionInvalidaError('Estos escuadrones exceden el Liderazgo del jugador.');
  }
}

/**
 * Saca a un jugador de campaña con los escuadrones que elija (Doc 5.12.1). Salir SOLO es esto mismo con un
 * participante: no hay dos casos ni dos tipos.
 *
 * Sale con el carro cargado del almacén hasta donde llegue sin comprometer la despensa del asentamiento
 * (`cargarCarro`, Doc 5.13). Puede salir con el carro vacío si la ciudad ya iba justa: eso no se impide, se
 * paga en autonomía.
 */
export function movilizarEjercito(
  asentamiento: Asentamiento,
  jugador: Jugador | undefined,
  jugadorId: string,
  escuadronIds: readonly string[],
  objetivo: ObjetivoEjercito,
  asentamientos: readonly Asentamiento[],
  mapa: Mapa,
  id: string
): { asentamiento: Asentamiento; ejercito: Ejercito; trigoCargado: number } {
  if (!esResidente(asentamiento, jugadorId)) {
    throw new MovilizacionInvalidaError('Solo un residente puede sacar tropas de este asentamiento.');
  }
  if (objetivo.tipo === 'asentamiento' && objetivo.id === asentamiento.id) {
    throw new MovilizacionInvalidaError('El destino no puede ser el propio asentamiento de origen.');
  }

  const escuadrones = seleccionarParaCampana(asentamiento, jugadorId, escuadronIds);
  exigirLiderazgo(jugador, escuadrones);

  const destino = puntoDeObjetivo(objetivo, asentamientos);
  // El agua es infranqueable (`world/rutas.ts`): si no hay camino por tierra, no se sale. Un ejército no se
  // embarca — y una ruta recta de reserva sería precisamente una marcha por el mar.
  const ruta = calcularRuta(mapa, asentamiento.posicion, destino);
  if (!ruta) throw new MovilizacionInvalidaError('No hay ruta por tierra hasta ese destino.');
  const idsFuera = new Set(escuadrones.map((e) => e.id));
  const sinLosQueSalen = { ...asentamiento, escuadrones: asentamiento.escuadrones.filter((e) => !idsFuera.has(e.id)) };
  const carga = cargarCarro(sinLosQueSalen, 0, capacidadCarroDe(escuadrones));

  return {
    asentamiento: carga.asentamiento,
    ejercito: {
      id,
      faccionId: asentamiento.faccionId,
      origenAsentamientoId: asentamiento.id,
      escuadrones,
      suministro: { trigo: carga.cargado },
      caravanasAdjuntasIds: [],
      objetivo,
      ruta,
      progreso: 0,
      posicionActual: asentamiento.posicion,
      estado: 'marchando',
    },
    trigoCargado: carga.cargado,
  };
}

/**
 * Un jugador se suma a un ejército ya en campaña (Doc 5.12.1) con escuadrones de SU asentamiento.
 *
 * Exige proximidad: el ejército tiene que estar pasando por (o parado en) el asentamiento del que se une. Sin
 * eso, unirse sería teletransportar refuerzos al otro extremo del mapa — y como el radio es el mismo que el
 * del reabastecimiento, "por dónde puede pasar a recogerte" y "dónde puede repostar" son la misma geografía.
 *
 * El que se une trae SU carro y lo carga de SU asentamiento (Doc 5.13). El tope se mide contra la capacidad
 * total de la columna ya con él dentro, no contra "un carro más": si el que se une YA era participante
 * —sumar más escuadrones a un ejército en el que ya vas es legítimo— no aparece ningún carro nuevo, y sin ese
 * tope repetir la operación sería una bomba de trigo infinita desde el almacén.
 */
export function unirseAEjercito(
  ejercito: Ejercito,
  asentamiento: Asentamiento,
  jugador: Jugador | undefined,
  jugadorId: string,
  escuadronIds: readonly string[]
): { asentamiento: Asentamiento; ejercito: Ejercito; trigoCargado: number } {
  if (!esResidente(asentamiento, jugadorId)) {
    throw new MovilizacionInvalidaError('Solo un residente puede sacar tropas de este asentamiento.');
  }
  if (ejercito.faccionId !== asentamiento.faccionId) {
    throw new MovilizacionInvalidaError('No se puede unir tropas a un ejército de otra Facción.');
  }
  if (distancia(ejercito.posicionActual, asentamiento.posicion) > LOGISTICA.radioReabastecimiento) {
    throw new MovilizacionInvalidaError('El ejército está demasiado lejos del asentamiento para recoger tropas.');
  }

  const escuadrones = seleccionarParaCampana(asentamiento, jugadorId, escuadronIds);
  // Solo lo que aporta ESTE jugador cuenta contra SU liderazgo, incluido lo que ya tuviera dentro.
  const suyosYaDentro = ejercito.escuadrones.filter((e) => e.jugadorId === jugadorId);
  exigirLiderazgo(jugador, [...suyosYaDentro, ...escuadrones]);

  const idsFuera = new Set(escuadrones.map((e) => e.id));
  const sinLosQueSalen = { ...asentamiento, escuadrones: asentamiento.escuadrones.filter((e) => !idsFuera.has(e.id)) };
  const escuadronesTotales = [...ejercito.escuadrones, ...escuadrones];
  const enElCarro = ejercito.suministro['trigo'] ?? 0;
  const carga = cargarCarro(sinLosQueSalen, enElCarro, capacidadCarroDe(escuadronesTotales));

  return {
    asentamiento: carga.asentamiento,
    ejercito: {
      ...ejercito,
      escuadrones: escuadronesTotales,
      suministro: { ...ejercito.suministro, trigo: enElCarro + carga.cargado },
    },
    trigoCargado: carga.cargado,
  };
}

/**
 * Manda el ejército de vuelta a casa (Doc 5.12.6).
 *
 * Replegar y "cancelar la marcha" son la MISMA operación aplicada desde estados distintos, así que hay un
 * solo comando y no dos que hagan lo mismo:
 *
 * - **marchando** -> da media vuelta: invierte la ruta Y el progreso, de modo que la posición actual no se
 *   mueve ni un punto y desanda exactamente el camino que trajo. Esto es "cancelar la marcha".
 * - **estacionado** -> no hay camino que desandar (ya llegó y se quedó), así que se calcula una ruta nueva
 *   desde donde está hasta el asentamiento de origen.
 *
 * En ninguno de los dos casos se teletransporta: volver cuesta el mismo camino que costó ir, y se sigue
 * comiendo del carro durante el regreso.
 */
export function replegarEjercito(ejercito: Ejercito, origen: Asentamiento | undefined, mapa: Mapa): Ejercito {
  if (ejercito.estado === 'regresando') throw new MovilizacionInvalidaError('El ejército ya está regresando.');
  if (!origen) throw new MovilizacionInvalidaError('El ejército no tiene asentamiento al que volver.');

  const objetivo: ObjetivoEjercito = { tipo: 'asentamiento', id: origen.id };

  if (ejercito.estado === 'marchando') {
    return {
      ...ejercito,
      estado: 'regresando',
      objetivo,
      ruta: [...ejercito.ruta].reverse(),
      progreso: 1 - ejercito.progreso,
    };
  }

  const vuelta = calcularRuta(mapa, ejercito.posicionActual, origen.posicion);
  if (!vuelta) throw new MovilizacionInvalidaError('No hay ruta por tierra de vuelta a casa desde aquí.');

  return { ...ejercito, estado: 'regresando', objetivo, ruta: vuelta, progreso: 0 };
}

/** Planta el ejército donde está (Doc 5.12.3): deja de avanzar y pasa a consumo reducido. Aparcar en un paso
 * de montaña es una jugada legítima, y por eso el consumo baja pero nunca llega a 0. */
export function estacionarEjercito(ejercito: Ejercito): Ejercito {
  if (ejercito.estado === 'estacionado') throw new MovilizacionInvalidaError('El ejército ya está estacionado.');
  return { ...ejercito, estado: 'estacionado', objetivo: { tipo: 'punto', punto: ejercito.posicionActual } };
}

// --- Avance en el tick ---

/**
 * Velocidad de marcha del ejército: la de su escuadrón MÁS LENTO (Doc 5.12.5).
 *
 * De aquí sale sola la distinción entre una partida de incursión y un ejército de asedio: meter un solo
 * escuadrón pesado (12) en una fuerza ligera (20) la frena a 12 y le quita la capacidad de cazar caravanas
 * (comercial va a 16). No hace falta ninguna regla más para separar los dos roles.
 *
 * Un ejército sin escuadrones vivos no se mueve — pero eso no debería llegar aquí: `disolverSiVacio` lo
 * retira antes (Doc 5.13.4).
 */
export function velocidadDeEjercito(ejercito: Ejercito): number {
  const velocidades = ejercito.escuadrones
    .map((e) => TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)?.velocidad)
    .filter((v): v is number => v !== undefined);
  return velocidades.length === 0 ? 0 : Math.min(...velocidades);
}

/** ¿Se quedó sin nadie? Un escuadrón persiste como identidad con `cantidad: 0` (Doc 5.4), así que "vacío" es
 * que NINGUNO tenga soldados, no que la lista esté vacía — que es justo lo que `resolverCombate` no distingue
 * y lo que dejaría marchar a un ejército fantasma (Doc 5.13.4). */
function sinSoldados(ejercito: Ejercito): boolean {
  return ejercito.escuadrones.every((e) => e.cantidad <= 0);
}

export interface ResultadoAvanceEjercitos {
  ejercitos: Ejercito[];
  asentamientos: Asentamiento[];
  eventos: EventoCrudo[];
}

/**
 * Un tick de todos los ejércitos en campaña: comer, moverse, llegar (Doc 5.12/5.13).
 *
 * NO consume aleatoriedad. Ni el hambre, ni el movimiento, ni la disolución la necesitan — y eso es
 * deliberado: colocada al final de la cadena del tick y sin tocar el RNG, el guardián de determinismo sigue
 * verde SIN modificarlo, que es una verificación más fuerte que actualizarlo. El RNG entrará cuando la
 * llegada dispare combate (Paso 7), y ahí sí habrá que ordenar canónicamente las resoluciones.
 *
 * Orden dentro de cada ejército, y por qué:
 *  1. **Comer primero.** Un ejército que se queda sin suministro este tick pierde moral este tick, avance
 *     incluido — si se moviera antes de comer, la última jornada saldría gratis.
 *  2. **Disolver si se quedó sin nadie** (Doc 5.13.4), antes de moverlo: si no, marcharía como fantasma.
 *  3. **Mover**, salvo estacionado (que acampa pero sigue comiendo, a `factorConsumoEstacionado`).
 *  4. **Llegar**: `regresando` reintegra la tropa y el sobrante en casa; cualquier otro destino deja el
 *     ejército acampado donde llegó. Que la llegada a un asentamiento enemigo dispare un asedio es el Paso 7:
 *     hasta entonces, plantarse es la conducta neutra y no rompe nada.
 */
export function avanzarEjercitos(
  ejercitos: readonly Ejercito[],
  asentamientos: readonly Asentamiento[],
  mapa: Mapa
): ResultadoAvanceEjercitos {
  if (ejercitos.length === 0) return { ejercitos: [...ejercitos], asentamientos: [...asentamientos], eventos: [] };

  const eventos: EventoCrudo[] = [];
  const porId = new Map(asentamientos.map((a) => [a.id, a]));
  const supervivientes: Ejercito[] = [];

  /** Devuelve escuadrones (y opcionalmente suministro) al asentamiento de origen. Si ya no existe, se pierden
   * con él: sus jugadores quedan huérfanos (Doc 5.4) y no hay dónde reintegrar. */
  const reintegrar = (ejercito: Ejercito, devolverSuministro: boolean): boolean => {
    const origen = porId.get(ejercito.origenAsentamientoId);
    if (!origen) return false;
    const almacen = devolverSuministro
      ? Object.entries(ejercito.suministro).reduce((acc, [recurso, cantidad]) => agregarRecurso(acc, recurso, cantidad), origen.almacen)
      : origen.almacen;
    porId.set(origen.id, { ...origen, escuadrones: [...origen.escuadrones, ...ejercito.escuadrones], almacen });
    return true;
  };

  for (const original of ejercitos) {
    // 1. Comer. La MISMA regla del hambre que la guarnición, solo que de otra despensa (Doc 5.13).
    const factorConsumo = original.estado === 'estacionado' ? LOGISTICA.factorConsumoEstacionado : 1;
    const trigoEnCarro = original.suministro['trigo'] ?? 0;
    const racion = avanzarRacion(original.escuadrones, trigoEnCarro, factorConsumo);
    // `avanzarRacion` narra la deserción sin saber si es guarnición o campaña; aquí sí se sabe de quién es
    // esa columna, y sin atribuirla el evento saldría GLOBAL — o sea, contando a todo el mundo que a un
    // rival se le están desertando los hombres (Doc 5.12.7).
    eventos.push(...racion.eventos.map((e) => atribuir(e, original.origenAsentamientoId)));

    let ejercito: Ejercito = {
      ...original,
      escuadrones: racion.escuadrones,
      suministro: { ...original.suministro, trigo: trigoEnCarro - racion.trigoConsumido },
    };

    // 2. ¿Se quedó sin nadie? Se disuelve y las identidades vacías vuelven a casa a poder rellenarse.
    if (sinSoldados(ejercito)) {
      const volvieron = reintegrar(ejercito, true);
      eventos.push({
        codigo: 'ejercito.disuelto',
        asentamientoId: ejercito.origenAsentamientoId,
        mensaje: volvieron
          ? `El ejército ${ejercito.id} se deshace sin un solo soldado en pie; sus estandartes vuelven a ${ejercito.origenAsentamientoId}.`
          : `El ejército ${ejercito.id} se deshace sin un solo soldado en pie, y ya no tiene asentamiento al que volver.`,
        payload: { ejercitoId: ejercito.id, origenAsentamientoId: ejercito.origenAsentamientoId, reintegrado: volvieron },
      });
      continue;
    }

    // 3. Mover (estacionado acampa: no avanza, pero ya comió arriba).
    if (ejercito.estado !== 'estacionado') {
      const avance = avanzarPosicionEnRuta(mapa, ejercito.ruta, ejercito.progreso, velocidadDeEjercito(ejercito));
      ejercito = { ...ejercito, progreso: avance.progreso, posicionActual: avance.posicion };
    }

    // 4. Llegar.
    if (ejercito.estado !== 'estacionado' && ejercito.progreso >= 1) {
      if (ejercito.estado === 'regresando') {
        const volvieron = reintegrar(ejercito, true);
        eventos.push({
          codigo: 'ejercito.regresa',
          asentamientoId: ejercito.origenAsentamientoId,
          mensaje: volvieron
            ? `El ejército ${ejercito.id} vuelve a ${ejercito.origenAsentamientoId} y se reincorpora a la guarnición.`
            : `El ejército ${ejercito.id} llega a donde estaba su hogar y no encuentra nada a lo que volver.`,
          payload: { ejercitoId: ejercito.id, origenAsentamientoId: ejercito.origenAsentamientoId, reintegrado: volvieron },
        });
        continue;
      }
      // Llegó a su destino: acampa. El asedio lo añade el Paso 7.
      ejercito = { ...ejercito, estado: 'estacionado' };
      eventos.push({
        codigo: 'ejercito.llega',
        asentamientoId: ejercito.origenAsentamientoId,
        mensaje: `El ejército ${ejercito.id} llega a su destino y acampa.`,
        payload: { ejercitoId: ejercito.id, objetivo: ejercito.objetivo },
      });
    }

    supervivientes.push(ejercito);
  }

  return { ejercitos: supervivientes, asentamientos: [...porId.values()], eventos };
}
