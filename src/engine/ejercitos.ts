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
import type { AcuerdoTrueque, Asentamiento, Caravana, Ejercito, Escuadron, Faccion, Jugador, Point, RelacionPolitica } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { distancia } from '../world/geometria';
import { CARAVANA_CATALOGO, LOGISTICA, MOVIMIENTO, TROPAS_RECLUTABLES } from '../constants';
import { atribuir, type EventoCrudo } from '../domain/eventos';
import type { Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { asediarConEjercito, encuentroEntreEjercitos, interceptarCaravanaConEjercito } from './combate';
import { avanzarPosicionEnRuta } from './movimiento';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { avanzarRacion, reservaDeTrigo } from './tropas';
import { puedeLlevar } from './liderazgo';
import { esResidente, estanAliadas } from './pertenencia';

export class MovilizacionInvalidaError extends Error {}

/** Fase A5 — payload de `ejercito.caravanas_perdidas` (Doc 5.13.2). */
export interface PayloadCaravanasPerdidas {
  ejercitoId: string;
  caravanaIds: string[];
}

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
/**
 * Cuánta gente va DENTRO de la columna (Doc 5.12.1). Lee `participantes`, no los escuadrones: un jugador que
 * sale sin tropas —o que las pierde todas— sigue yendo dentro, y antes esto devolvía 0 por él.
 */
export function participantesDe(ejercito: Pick<Ejercito, 'participantes'>): number {
  return ejercito.participantes.length;
}

/**
 * Capacidad de carga total del ejército (Doc 5.13): el carro FIJO de cada Jugador —aditivo, un ejército de
 * cuatro lleva cuatro carros— más lo que aporten las caravanas adjuntas (Doc 5.13.2).
 *
 * `caravanas` son las del mundo, no solo las adjuntas: se filtran aquí por `caravanasAdjuntasIds` para que el
 * llamador no tenga que resolverlas. Omitirlas devuelve solo los carros, que es lo correcto para un ejército
 * que todavía no ha enganchado ninguna.
 */
export function capacidadCargaDe(ejercito: Ejercito, caravanas: readonly Caravana[] = []): number {
  return (
    capacidadCarrosDe(participantesDe(ejercito)) +
    adjuntasDe(ejercito, caravanas).reduce((suma, c) => suma + CARAVANA_CATALOGO[c.tipo].capacidad, 0)
  );
}

/** Solo los carros de los Jugadores, sin caravanas. Existe aparte porque al MOVILIZAR todavía no hay ejército
 * al que preguntarle sus adjuntas: se está creando — por eso toma el número y no la columna. */
function capacidadCarrosDe(participantes: number): number {
  return participantes * LOGISTICA.capacidadCarroPorJugador;
}

/** Las caravanas que este ejército lleva enganchadas, resueltas contra la lista del mundo. Una id que ya no
 * corresponda a ninguna caravana viva se ignora en silencio: perder una caravana es un hecho del juego (la
 * derrota del ejército, Doc 5.13.2), no un estado inconsistente que haya que reparar. */
function adjuntasDe(ejercito: Ejercito, caravanas: readonly Caravana[]): Caravana[] {
  if (ejercito.caravanasAdjuntasIds.length === 0) return [];
  const ids = new Set(ejercito.caravanasAdjuntasIds);
  return caravanas.filter((c) => ids.has(c.id));
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
  relaciones: readonly RelacionPolitica[],
  caravanas: readonly Caravana[]
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
  const carga = cargarCarro(plaza, enElCarro, capacidadCargaDe(ejercito, caravanas));
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
  id: string,
  /** Para fechar la entrada del que sale: la antigüedad decide la sucesión del líder (Doc 5.14.3). */
  instante: Instante
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
  const carga = cargarCarro(sinLosQueSalen, 0, capacidadCarrosDe(1));

  return {
    asentamiento: carga.asentamiento,
    ejercito: {
      id,
      faccionId: asentamiento.faccionId,
      origenAsentamientoId: asentamiento.id,
      participantes: [{ jugadorId, unidoEn: instante }],
      // Movilizar es salir CONTRA un destino, y eso es lo que hace un ejército aunque salga uno solo (Doc
      // 5.12.1): rumbo fijo desde el primer paso, y otros pueden sumarse por el camino.
      tipo: 'ejercito',
      liderId: jugadorId,
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
  escuadronIds: readonly string[],
  /** Para fechar su entrada: la antigüedad decide la sucesión del líder (Doc 5.14.3). */
  instante: Instante,
  /** Las del mundo: el que se une llena hasta la capacidad TOTAL de la columna, adjuntas incluidas. */
  caravanas: readonly Caravana[] = []
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
  // Sumar más tropas a un ejército en el que YA vas es legítimo y no te convierte en dos participantes — ni
  // aporta un carro nuevo, que es lo que el tope de carga de abajo mide.
  const yaDentro = ejercito.participantes.some((p) => p.jugadorId === jugadorId);
  const participantes = yaDentro ? ejercito.participantes : [...ejercito.participantes, { jugadorId, unidoEn: instante }];
  const enElCarro = ejercito.suministro['trigo'] ?? 0;
  const carga = cargarCarro(sinLosQueSalen, enElCarro, capacidadCargaDe({ ...ejercito, participantes, escuadrones: escuadronesTotales }, caravanas));

  return {
    asentamiento: carga.asentamiento,
    ejercito: {
      ...ejercito,
      participantes,
      escuadrones: escuadronesTotales,
      suministro: { ...ejercito.suministro, trigo: enElCarro + carga.cargado },
    },
    trigoCargado: carga.cargado,
  };
}

/**
 * Engancha una caravana al ejército como tren de suministros (Doc 5.13.2).
 *
 * Cuatro condiciones, y cada una tapa un agujero distinto:
 *
 *  1. **Misma Facción.** Un ejército no requisa la flota de otro.
 *  2. **Disponible**, no en tránsito ni retornando: una caravana ya despachada está cumpliendo un trueque, y
 *     secuestrarla a media entrega rompería el acuerdo por detrás.
 *  3. **Al alcance**, el mismo `radioReabastecimiento` que rige recoger refuerzos y repostar — engancharse a
 *     una columna que está al otro lado del mapa sería teletransportar el tren de suministros.
 *  4. **No enganchada ya** a este ejército.
 *
 * Lo que NO se comprueba es el cupo de Mercado: la caravana ya existe y ya lo ocupaba. Lo que el diseño llama
 * "cuesta comercio" (Doc 5.13.2) es exactamente esto — mientras marcha con el ejército no está comerciando —,
 * y sale solo de que la caravana deje de estar disponible.
 */
export function adjuntarCaravana(
  ejercito: Ejercito,
  caravana: Caravana,
  origen: Asentamiento | undefined
): { ejercito: Ejercito; caravana: Caravana } {
  if (ejercito.caravanasAdjuntasIds.includes(caravana.id)) {
    throw new MovilizacionInvalidaError('Esa caravana ya va con este ejército.');
  }
  if (!origen || origen.faccionId !== ejercito.faccionId) {
    throw new MovilizacionInvalidaError('La caravana no es de la Facción de este ejército.');
  }
  if (caravana.estado !== 'disponible') {
    throw new MovilizacionInvalidaError('Solo se puede enganchar una caravana que esté disponible, no una ya despachada.');
  }
  if (distancia(caravana.posicionActual, ejercito.posicionActual) > LOGISTICA.radioReabastecimiento) {
    throw new MovilizacionInvalidaError('La caravana está demasiado lejos del ejército.');
  }
  return {
    ejercito: { ...ejercito, caravanasAdjuntasIds: [...ejercito.caravanasAdjuntasIds, caravana.id] },
    caravana: { ...caravana, estado: 'adjunta', posicionActual: ejercito.posicionActual },
  };
}

/** Suelta una caravana del ejército. Se queda donde esté la columna en ese momento — no vuelve sola a casa,
 * igual que un ejército no se teletransporta al replegarse (Doc 5.12.6). */
export function soltarCaravana(ejercito: Ejercito, caravana: Caravana): { ejercito: Ejercito; caravana: Caravana } {
  if (!ejercito.caravanasAdjuntasIds.includes(caravana.id)) {
    throw new MovilizacionInvalidaError('Esa caravana no va con este ejército.');
  }
  return {
    ejercito: { ...ejercito, caravanasAdjuntasIds: ejercito.caravanasAdjuntasIds.filter((id) => id !== caravana.id) },
    caravana: { ...caravana, estado: 'disponible', posicionActual: ejercito.posicionActual },
  };
}

/**
 * Carga mercancía en una caravana adjunta, del almacén de una plaza al alcance (Doc 5.13.3).
 *
 * **Decisión del usuario (2026-09-04): una caravana enganchada deja de comportarse como las automáticas.** El
 * jugador que la engancha elige QUÉ carga y a dónde la lleva — son viajes conscientes, no reparto por score.
 * Y eso no es un desvío del diseño sino su destino: `asignarCaravanasATrueque` lleva desde que existe
 * declarando que el reparto automático es "el sustituto de Fase 0" y que "en el diseño objetivo el jugador
 * elige la caravana, la carga y la escolta a mano". La escolta es la primera parte que llega ahí.
 *
 * Se carga de donde se pueda repostar —propia siempre, aliada con la opción abierta—: la misma puerta y la
 * misma geografía que el trigo.
 *
 * **Sin reserva de mantenimiento**, a diferencia del carro: cargar mercancía para comerciar es lo mismo que
 * ya hace el comercio automático, que reparte contra `cantidadDisponible` a secas. Exigir aquí una reserva
 * que el otro camino no tiene haría que la misma acción costase distinto según quién la ordene.
 */
export function cargarCaravanaAdjunta(
  ejercito: Ejercito,
  caravana: Caravana,
  plaza: Asentamiento,
  recurso: string,
  cantidad: number,
  relaciones: readonly RelacionPolitica[]
): { caravana: Caravana; plaza: Asentamiento; cargado: number } {
  if (!ejercito.caravanasAdjuntasIds.includes(caravana.id)) {
    throw new MovilizacionInvalidaError('Esa caravana no va con este ejército.');
  }
  if (cantidad <= 0) throw new MovilizacionInvalidaError('La cantidad a cargar tiene que ser positiva.');
  if (distancia(plaza.posicion, ejercito.posicionActual) > LOGISTICA.radioReabastecimiento) {
    throw new MovilizacionInvalidaError('El ejército está demasiado lejos de ese asentamiento.');
  }
  if (!puedeRepostarEn(ejercito, plaza, relaciones)) {
    throw new MovilizacionInvalidaError('Ese asentamiento no abre su almacén a este ejército.');
  }

  const yaCargado = Object.values(caravana.contenido).reduce((a, b) => a + b, 0);
  const espacio = CARAVANA_CATALOGO[caravana.tipo].capacidad - yaCargado;
  if (espacio <= 0) throw new MovilizacionInvalidaError('La caravana ya va llena.');

  const cargado = Math.min(cantidad, espacio, cantidadDisponible(plaza.almacen, recurso));
  if (cargado <= 0) throw new MovilizacionInvalidaError(`No hay ${recurso} en el almacén de ${plaza.id}.`);

  return {
    caravana: { ...caravana, contenido: { ...caravana.contenido, [recurso]: (caravana.contenido[recurso] ?? 0) + cargado } },
    plaza: { ...plaza, almacen: descontarRecursos(plaza.almacen, { [recurso]: cargado }) },
    cargado,
  };
}

/**
 * Qué lado de un trueque le toca cumplir a este ejército, y cuánto le falta (Doc 3.2 + 5.13.3).
 *
 * Es la consulta que necesita la interfaz descrita por el usuario: "al interactuar con un asentamiento, ver
 * los trueques activos, el faltante por TU parte, y entregar de lo que cargas". Vive en el motor y no en el
 * cliente porque decide una regla —de qué lado estás y cuánto debes—, no una presentación.
 *
 * `null` si el acuerdo no está activo, si ninguno de sus dos lados es de la Facción del ejército, o si ese
 * lado ya está saldado.
 */
export function ladoPendienteParaEjercito(
  ejercito: Ejercito,
  acuerdo: AcuerdoTrueque,
  asentamientos: readonly Asentamiento[]
): { lado: 'A' | 'B'; recurso: string; faltante: number; destinoId: string } | null {
  if (acuerdo.estado !== 'activo') return null;
  const faccionDe = (id: string) => asentamientos.find((a) => a.id === id)?.faccionId;

  // El lado que DEBE es el de la Facción del ejército; el destino de la entrega es el otro.
  const candidatos: { lado: 'A' | 'B'; deudorId: string; destinoId: string; recurso: string; faltante: number }[] = [
    {
      lado: 'A',
      deudorId: acuerdo.asentamientoAId,
      destinoId: acuerdo.asentamientoBId,
      recurso: acuerdo.recursoA,
      faltante: acuerdo.cantidadTotalA - acuerdo.cantidadEntregadaA,
    },
    {
      lado: 'B',
      deudorId: acuerdo.asentamientoBId,
      destinoId: acuerdo.asentamientoAId,
      recurso: acuerdo.recursoB,
      faltante: acuerdo.cantidadTotalB - acuerdo.cantidadEntregadaB,
    },
  ];

  const mio = candidatos.find((c) => faccionDe(c.deudorId) === ejercito.faccionId && c.faltante > 0);
  return mio ? { lado: mio.lado, recurso: mio.recurso, faltante: mio.faltante, destinoId: mio.destinoId } : null;
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
 * (comercial va a 16). No hace falta ninguna regla más para separar los dos roles — y lo mismo vale para las
 * caravanas adjuntas, que entran en el mismo mínimo (Doc 5.13.2).
 *
 * Una columna SIN escuadrones no es un caso degenerado: es un jugador viajando solo (Doc 5.12.1), y va a
 * `MOVIMIENTO.velocidadJugador` — más rápido que cualquier tropa, porque no arrastra impedimenta. Antes esto
 * devolvía 0, o sea que se quedaba clavado en el sitio.
 */
export function velocidadDeEjercito(ejercito: Ejercito, caravanas: readonly Caravana[] = []): number {
  const velocidades = ejercito.escuadrones
    // Un escuadrón aniquilado persiste como IDENTIDAD (Doc 5.4), pero no como gente que camine: no frena a
    // nadie. Sin este filtro, perder hasta el último hombre de la tropa pesada seguiría lastrando la columna.
    .filter((e) => e.cantidad > 0)
    .map((e) => TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)?.velocidad)
    .filter((v): v is number => v !== undefined);
  // Las caravanas adjuntas entran en el MISMO mínimo (Doc 5.13.2): una comercial va a 16, así que engancharla
  // baja una fuerza ligera de 20 a 16 y le quita la capacidad de cazar caravanas. Ahí está el equilibrio de la
  // escolta, sin ninguna regla extra: no se puede escoltar y depredar a la vez.
  for (const c of adjuntasDe(ejercito, caravanas)) velocidades.push(CARAVANA_CATALOGO[c.tipo].velocidad);
  if (velocidades.length === 0) {
    // Sin escuadrones y sin adjuntas: o va gente dentro —y entonces es un viajero— o no queda nadie y la
    // columna está a punto de disolverse, en cuyo caso da igual a qué velocidad no se mueve.
    return participantesDe(ejercito) > 0 ? MOVIMIENTO.velocidadJugador : 0;
  }
  return Math.min(...velocidades);
}

/** ¿No le queda un solo soldado en pie? Un escuadrón persiste como identidad con `cantidad: 0` (Doc 5.4), así
 * que "sin soldados" es que NINGUNO tenga hombres, no que la lista esté vacía. Es lo que decide si puede
 * combatir, no si sigue existiendo: para eso está `sinNadieDentro`. */
function sinSoldados(ejercito: Ejercito): boolean {
  return ejercito.escuadrones.every((e) => e.cantidad <= 0);
}

/**
 * ¿Se quedó sin NADIE? Es la condición de disolución (Doc 5.13.4), y no es la misma que quedarse sin
 * soldados: una columna cuyos escuadrones caen todos sigue teniendo dentro a sus jugadores, que ahora viajan
 * a pie. Se disuelve cuando ya no va nadie — lo que hoy solo ocurre al replegarse.
 */
function sinNadieDentro(ejercito: Ejercito): boolean {
  return ejercito.participantes.length === 0;
}

/**
 * Todo lo que el tick de ejércitos necesita del mundo. Es un objeto y no siete parámetros sueltos porque a
 * partir del Paso 9 lo son: `mapa` para moverse, `facciones`/`relaciones`/`instante`/`rng` para el asedio, y
 * `caravanas` para las adjuntas. Con esa lista, el orden posicional dejaba de decir nada en la llamada.
 */
export interface ContextoAvanceEjercitos {
  asentamientos: readonly Asentamiento[];
  caravanas: readonly Caravana[];
  facciones: readonly Faccion[];
  relaciones: readonly RelacionPolitica[];
  mapa: Mapa;
  instante: Instante;
  rng: RandomFn;
}

export interface ResultadoAvanceEjercitos {
  ejercitos: Ejercito[];
  asentamientos: Asentamiento[];
  /** Las del mundo, menos las que se hayan perdido con un ejército derrotado (Doc 5.13.2). */
  caravanas: Caravana[];
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
export function avanzarEjercitos(ejercitos: readonly Ejercito[], contexto: ContextoAvanceEjercitos): ResultadoAvanceEjercitos {
  const { asentamientos, caravanas, facciones, relaciones, mapa, instante, rng } = contexto;
  if (ejercitos.length === 0) {
    return {
      ejercitos: [...ejercitos],
      asentamientos: [...asentamientos],
      caravanas: [...caravanas],
      facciones: [...facciones],
      eventos: [],
    };
  }

  const eventos: EventoCrudo[] = [];
  const porId = new Map(asentamientos.map((a) => [a.id, a]));
  const supervivientes: Ejercito[] = [];
  let faccionesActuales = [...facciones];
  let caravanasActuales = [...caravanas];

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
    const racion = avanzarRacion(original.escuadrones, trigoEnCarro, factorConsumo, participantesDe(original));
    // `avanzarRacion` narra la deserción sin saber si es guarnición o campaña; aquí sí se sabe de quién es
    // esa columna, y sin atribuirla el evento saldría GLOBAL — o sea, contando a todo el mundo que a un
    // rival se le están desertando los hombres (Doc 5.12.7).
    eventos.push(...racion.eventos.map((e) => atribuir(e, original.origenAsentamientoId)));

    let ejercito: Ejercito = {
      ...original,
      escuadrones: racion.escuadrones,
      suministro: { ...original.suministro, trigo: trigoEnCarro - racion.trigoConsumido },
    };

    // 2. ¿Se quedó sin nadie DENTRO? Se disuelve y las identidades vacías vuelven a casa a poder rellenarse.
    // Perder todos los soldados ya no basta: los jugadores siguen ahí y ahora viajan a pie (Doc 5.12.1).
    if (sinNadieDentro(ejercito)) {
      const volvieron = reintegrar(ejercito, true);
      // Las caravanas adjuntas se pierden con él (Doc 5.13.2). El canon lo dice de un ejército DERROTADO, y
      // aquí se aplica también al que se deshace por hambre: en los dos casos deja de existir en campo
      // abierto, y lo que vuelve a casa son las IDENTIDADES de sus escuadrones (Doc 5.13.4), no bienes
      // físicos. Una caravana sin nadie que la lleve no se teletransporta a ninguna parte.
      const perdidas = adjuntasDe(ejercito, caravanasActuales);
      if (perdidas.length > 0) {
        const ids = new Set(perdidas.map((c) => c.id));
        caravanasActuales = caravanasActuales.filter((c) => !ids.has(c.id));
        eventos.push({
          codigo: 'ejercito.caravanas_perdidas',
          asentamientoId: ejercito.origenAsentamientoId,
          mensaje: `Con el ejército ${ejercito.id} se pierden ${perdidas.length} caravana(s) adjunta(s).`,
          payload: { ejercitoId: ejercito.id, caravanaIds: perdidas.map((c) => c.id) } satisfies PayloadCaravanasPerdidas,
        });
      }
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
    const reposte = repostarSiPuede(ejercito, porId, relaciones, caravanas);
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

    // Las adjuntas van enganchadas: su posición es la del ejército, no una ruta propia. Sin esto seguirían
    // marcando el punto donde se engancharon y el mapa mentiría sobre dónde está el tren de suministros.
    if (ejercito.caravanasAdjuntasIds.length > 0) {
      const ids = new Set(ejercito.caravanasAdjuntasIds);
      caravanasActuales = caravanasActuales.map((c) => (ids.has(c.id) ? { ...c, posicionActual: ejercito.posicionActual } : c));
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
        // no tiene. Sigue abierto: el Paso 11 retiró `combateCampoAbierto`, que tenía el mismo problema,
        // pero retirar el comando no resolvió la falta de audiencia por Facción — solo dejó de duplicarla.
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

  // --- FASE 2: encuentros por proximidad (Doc 5.12.3, Paso 10) ---
  //
  // Va en una segunda pasada y no dentro del bucle de arriba porque un encuentro depende de dónde acabaron
  // TODOS: resolverlo mientras la mitad de las columnas aún no se ha movido daría choques con posiciones de
  // dos momentos distintos, y el resultado dependería del orden del array.
  const conEncuentros = resolverEncuentros(supervivientes, caravanasActuales, faccionesActuales, relaciones, porId, instante, rng);
  eventos.push(...conEncuentros.eventos);

  return {
    ejercitos: conEncuentros.ejercitos,
    asentamientos: [...porId.values()],
    caravanas: conEncuentros.caravanas,
    facciones: conEncuentros.facciones,
    eventos,
  };
}

/**
 * Los encuentros de un tick: quién se cruza con quién, y qué pasa (Doc 5.12.3).
 *
 * Nadie los ordena — salen de la geometría. Las reglas que los acotan:
 *
 *  - **Orden canónico por id.** Cada encuentro consume RNG, así que el orden decide el resultado. Ordenar por
 *    id lo ancla al DATO y no a cómo quedara el array (§9 de la revisión por consejo: sin esto el determinismo
 *    se rompe aunque la secuencia global del tick sea correcta).
 *  - **Un encuentro por ejército y tick.** Sin eso, tres columnas juntas se trituran en cascada dentro del
 *    mismo minuto y el resultado depende de a quién se mire primero.
 *  - **Los aliados no se cruzan.** Dos columnas amigas compartiendo ruta se masacrarían solas cada tick, que
 *    es lo contrario de lo que una alianza significa. Tampoco las de la misma Facción, claro.
 *  - **Una caravana escoltada no es un objetivo blando**: el que se topa con ella se topa con su ejército, y
 *    eso ya es un encuentro entre ejércitos. Por eso las 'adjunta' se saltan al buscar caravanas.
 *  - **El más cercano primero.** Entre varios al alcance, el que se cruza de verdad es el que tienes encima;
 *    a igual distancia decide el id, para que no lo decida el orden de la lista.
 */
function resolverEncuentros(
  ejercitos: readonly Ejercito[],
  caravanas: readonly Caravana[],
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  /** `asentamientoId -> Asentamiento` del tick ya avanzado: de aquí sale de qué Facción es cada caravana. */
  asentamientosPorId: ReadonlyMap<string, Asentamiento>,
  instante: Instante,
  rng: RandomFn
): { ejercitos: Ejercito[]; caravanas: Caravana[]; facciones: Faccion[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  if (ejercitos.length === 0) {
    return { ejercitos: [...ejercitos], caravanas: [...caravanas], facciones: [...facciones], eventos };
  }

  const porId = new Map(ejercitos.map((e) => [e.id, e]));
  let caravanasVivas = [...caravanas];
  let faccionesActuales = [...facciones];
  const yaChocaron = new Set<string>();

  const enemiga = (a: string, b: string) => a !== b && !estanAliadas(relaciones, a, b);
  const porIdAsc = (a: { id: string }, b: { id: string }) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);

  for (const id of [...porId.keys()].sort()) {
    if (yaChocaron.has(id)) continue;
    const ejercito = porId.get(id)!;
    if (sinSoldados(ejercito)) continue;

    // Candidatos: ejércitos enemigos que aún no han chocado, y caravanas enemigas SIN escolta.
    const rivales = [...porId.values()]
      .filter((o) => o.id !== id && !yaChocaron.has(o.id) && !sinSoldados(o) && enemiga(ejercito.faccionId, o.faccionId))
      .filter((o) => distancia(o.posicionActual, ejercito.posicionActual) <= LOGISTICA.radioEncuentro);
    const presas = caravanasVivas
      .filter((c) => c.estado !== 'adjunta' && c.estado !== 'disponible')
      .filter((c) => {
        const duena = asentamientosPorId.get(c.origenAsentamientoId)?.faccionId;
        // Sin dueño identificable no se puede decidir si es enemiga, así que no se toca.
        return duena !== undefined && enemiga(ejercito.faccionId, duena);
      })
      .filter((c) => distancia(c.posicionActual, ejercito.posicionActual) <= LOGISTICA.radioEncuentro);

    const masCerca = <T extends { id: string; posicionActual: Point }>(lista: T[]): T | undefined =>
      [...lista].sort((x, y) => {
        const dx = distancia(x.posicionActual, ejercito.posicionActual);
        const dy = distancia(y.posicionActual, ejercito.posicionActual);
        return dx !== dy ? dx - dy : porIdAsc(x, y);
      })[0];

    // Un ejército enemigo manda sobre una caravana: es la amenaza real, y dejarla pasar para saquear un carro
    // sería absurdo.
    const rival = masCerca(rivales);
    if (rival) {
      const choque = encuentroEntreEjercitos(ejercito, rival, faccionesActuales, instante, rng);
      porId.set(ejercito.id, choque.a);
      porId.set(rival.id, choque.b);
      faccionesActuales = choque.facciones;
      for (const e of choque.eventos) {
        eventos.push(atribuir(e, ejercito.origenAsentamientoId));
        eventos.push(atribuir(e, rival.origenAsentamientoId));
      }
      yaChocaron.add(ejercito.id);
      yaChocaron.add(rival.id);
      continue;
    }

    const presa = masCerca(presas);
    if (presa) {
      const emboscada = interceptarCaravanaConEjercito(
        ejercito,
        presa,
        capacidadCargaDe(ejercito, caravanasVivas),
        instante,
        rng
      );
      porId.set(ejercito.id, emboscada.ejercito);
      if (emboscada.capturada) caravanasVivas = caravanasVivas.filter((c) => c.id !== presa.id);
      for (const e of emboscada.eventos) eventos.push(atribuir(e, ejercito.origenAsentamientoId));
      yaChocaron.add(ejercito.id);
    }
  }

  return { ejercitos: [...porId.values()], caravanas: caravanasVivas, facciones: faccionesActuales, eventos };
}

