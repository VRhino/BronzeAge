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
import type { Asentamiento, Ejercito, Escuadron, Faccion, Jugador, Point, RelacionPolitica } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { distancia } from '../world/geometria';
import { LOGISTICA, TROPAS_RECLUTABLES } from '../constants';
import { atribuir, type EventoCrudo } from '../domain/eventos';
import type { Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { asediarConEjercito } from './combate';
import { avanzarPosicionEnRuta } from './movimiento';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { avanzarRacion, reservaDeTrigo } from './tropas';
import { puedeLlevar } from './liderazgo';
import { esResidente, estanAliadas } from './pertenencia';

export class MovilizacionInvalidaError extends Error {}

/** Fase A5 — payload de `ejercito.reabastecido` (ver `repostarSiPuede`). */
export interface PayloadEjercitoReabastecido {
  ejercitoId: string;
  /** La plaza que puso el trigo — propia, o aliada con la opción abierta. */
  asentamientoId: string;
  trigoRepuesto: number;
}

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

/**
 * ¿Puede este ejército repostar en esta plaza? (Doc 5.13, Paso 8). Tres casos y ninguno negociable:
 *
 *  - **Propia**: siempre. No hace falta permiso para abrir tu propio almacén a tu propia columna.
 *  - **Aliada**: solo si esa plaza tiene `permiteReabastecerAliados` activo. Repostar cuesta stock REAL al
 *    que lo da, así que es una decisión suya, no un derecho del que pasa por ahí.
 *  - **Neutral u hostil**: nunca.
 */
function puedeRepostarEn(ejercito: Ejercito, plaza: Asentamiento, relaciones: readonly RelacionPolitica[]): boolean {
  if (plaza.faccionId === ejercito.faccionId) return true;
  return (plaza.permiteReabastecerAliados ?? false) && estanAliadas(relaciones, ejercito.faccionId, plaza.faccionId);
}

/**
 * Repostar al pasar (Doc 5.13, Paso 8): si el ejército está dentro de `LOGISTICA.radioReabastecimiento` de
 * una plaza donde tiene derecho a hacerlo, rellena el carro de su almacén.
 *
 * Es EXACTAMENTE la misma operación que cargar al salir —`cargarCarro`, con sus dos topes: el espacio libre
 * del carro y lo que la plaza puede soltar sin bajar de su reserva de comida—, solo que el almacén es otro.
 * Que sea la misma función es lo que garantiza que repostar en una ciudad ajena no pueda vaciarla por debajo
 * de lo que su propia gente necesita, sin ninguna regla nueva que mantener en paralelo.
 *
 * Si hay varias plazas al alcance se elige la MÁS CERCANA, y a igual distancia la de id menor: hace falta un
 * criterio total porque de aquí sale un gasto real y el orden no puede depender de cómo quedara el array.
 *
 * Sin límite de veces: un ejército acampado junto a una plaza amiga repone cada tick, que es precisamente lo
 * que convierte "sostener un paso de montaña" en una posición sostenible (Doc 5.12.3) en vez de una cuenta
 * atrás. Repostar no es gratis para nadie — sale del almacén de quien lo da.
 */
function repostarSiPuede(
  ejercito: Ejercito,
  porId: Map<string, Asentamiento>,
  relaciones: readonly RelacionPolitica[]
): { ejercito: Ejercito; plaza: Asentamiento | undefined; repuesto: number } {
  const alcance = [...porId.values()]
    .filter(
      (a) =>
        distancia(a.posicion, ejercito.posicionActual) <= LOGISTICA.radioReabastecimiento &&
        puedeRepostarEn(ejercito, a, relaciones)
    )
    .sort((a, b) => {
      const da = distancia(a.posicion, ejercito.posicionActual);
      const db = distancia(b.posicion, ejercito.posicionActual);
      return da !== db ? da - db : a.id < b.id ? -1 : 1;
    });

  const plaza = alcance[0];
  if (!plaza) return { ejercito, plaza: undefined, repuesto: 0 };

  const enElCarro = ejercito.suministro['trigo'] ?? 0;
  const carga = cargarCarro(plaza, enElCarro, capacidadCarroDe(ejercito.escuadrones));
  if (carga.cargado <= 0) return { ejercito, plaza: undefined, repuesto: 0 };

  return {
    ejercito: { ...ejercito, suministro: { ...ejercito.suministro, trigo: enElCarro + carga.cargado } },
    plaza: carga.asentamiento,
    repuesto: carga.cargado,
  };
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
  /** Las Facciones, que un asedio puede tocar: XP de combate y conquista, y la penalización de reputación por
   * atacar a un Aliado. Vuelven tal cual si en el tick no hubo ningún asedio. */
  facciones: Faccion[];
  eventos: EventoCrudo[];
}

/**
 * Un tick de todos los ejércitos en campaña: comer, moverse, llegar (Doc 5.12/5.13).
 *
 * Consume aleatoriedad SOLO cuando un asedio llega a resolverse contra una plaza defendida (Paso 7). Todo lo
 * demás —hambre, movimiento, disolución, y la conquista de una plaza desguarnecida— es mudo de RNG, así que
 * una partida sin asedios hace exactamente las mismas llamadas que antes de que existiera esta mecánica y el
 * guardián de determinismo sigue verde SIN tocarlo.
 *
 * Y como el RNG ya entra aquí, los ejércitos se recorren en **orden canónico por id**: el orden del array es
 * determinista pero arbitrario, y dos estados equivalentes con los ejércitos en distinto orden consumirían la
 * secuencia aleatoria de forma distinta. Ordenar por id lo ancla al DATO y no a cómo quedó el array (§9,
 * hallazgo de la revisión cruzada; el Paso 10 lo necesitará igual para los encuentros).
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
  mapa: Mapa,
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): ResultadoAvanceEjercitos {
  if (ejercitos.length === 0) {
    return { ejercitos: [...ejercitos], asentamientos: [...asentamientos], facciones: [...facciones], eventos: [] };
  }

  const eventos: EventoCrudo[] = [];
  const porId = new Map(asentamientos.map((a) => [a.id, a]));
  const supervivientes: Ejercito[] = [];
  let faccionesActuales = [...facciones];

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

  for (const original of [...ejercitos].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) {
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

    // 3b. Repostar al pasar (Doc 5.13). Va DESPUÉS de moverse —se repone donde uno acaba, no donde estaba— y
    // ANTES de resolver la llegada, para que un ejército que se planta en una plaza propia entre en el asedio
    // o acampe ya con el carro lleno.
    const reposte = repostarSiPuede(ejercito, porId, relaciones);
    if (reposte.plaza) {
      ejercito = reposte.ejercito;
      porId.set(reposte.plaza.id, reposte.plaza);
      eventos.push({
        codigo: 'ejercito.reabastecido',
        asentamientoId: reposte.plaza.id,
        mensaje: `El ejército ${ejercito.id} repone ${Math.floor(reposte.repuesto)} de trigo en ${reposte.plaza.id}.`,
        payload: {
          ejercitoId: ejercito.id,
          asentamientoId: reposte.plaza.id,
          trigoRepuesto: reposte.repuesto,
        } satisfies PayloadEjercitoReabastecido,
      });
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
      // Llegó a su destino. Si ese destino es un asentamiento de otra Facción, la llegada ES el asedio
      // (Doc 5.12.4) — y se resuelve UNA vez, al cruzar el final de la ruta. No se repite mientras el
      // ejército siga acampado ahí: un asedio por tick convertiría cualquier plaza en una picadora de carne
      // sin que nadie hubiera decidido nada. Lo que pase después con un ejército parado junto a una ciudad
      // enemiga es el Paso 10 (encuentros por proximidad).
      const objetivo = ejercito.objetivo.tipo === 'asentamiento' ? porId.get(ejercito.objetivo.id) : undefined;
      if (objetivo && objetivo.faccionId !== ejercito.faccionId) {
        const asedio = asediarConEjercito(ejercito, objetivo, faccionesActuales, [...relaciones], instante, rng);
        ejercito = asedio.ejercito;
        porId.set(objetivo.id, asedio.defensor);
        faccionesActuales = asedio.facciones;
        // A quién se le cuenta. Un evento se atribuye a UN asentamiento y lo ve la Facción que lo posee, así
        // que un choque entre dos hay que narrarlo dos veces o alguien se queda sin enterarse:
        //
        //  - Siempre al **hogar del atacante**: es lo único que la Facción atacante posee con seguridad
        //    (no reside en la plaza que ataca), y es quien tiene que saber cómo le fue a su columna.
        //  - Y a la **plaza asediada**, SOLO si resistió. Si cae, pasa a manos del atacante, y atribuirle
        //    también el evento se lo enseñaría dos veces al mismo jugador — duplicado en el log, medido en
        //    vivo antes de esta condición.
        //
        // Queda un hueco conocido: al vencido no le llega la noticia de su propia derrota, porque pierde el
        // asentamiento por el que la vería. Taparlo pide una audiencia por FACCIÓN que el modelo de eventos
        // no tiene — el mismo agujero anotado para `combateCampoAbierto` en el Paso 11.
        for (const e of asedio.eventos) {
          eventos.push(atribuir(e, ejercito.origenAsentamientoId));
          if (!asedio.conquistado) eventos.push(atribuir(e, objetivo.id));
        }
      } else {
        eventos.push({
          codigo: 'ejercito.llega',
          asentamientoId: ejercito.origenAsentamientoId,
          mensaje: `El ejército ${ejercito.id} llega a su destino y acampa.`,
          payload: { ejercitoId: ejercito.id, objetivo: ejercito.objetivo },
        });
      }
      ejercito = { ...ejercito, estado: 'estacionado' };
    }

    supervivientes.push(ejercito);
  }

  return { ejercitos: supervivientes, asentamientos: [...porId.values()], facciones: faccionesActuales, eventos };
}
