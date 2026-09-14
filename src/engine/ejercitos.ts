// Ejércitos (Doc 5.12): salir del asentamiento y moverse por el mapa.
//
// Este módulo cubre la COMPOSICIÓN de un ejército —movilizar, unirse, replegar, estacionar—, no su
// movimiento: eso es `avanzarEjercitos` en el tick. La ruta sí se calcula aquí, igual que una caravana
// calcula la suya al despacharse (`engine/trade.ts`), porque es parte de "salir", no de "avanzar".
//
// La regla que sostiene todo lo demás: los escuadrones se van DE VERDAD del campamento. Cambian de contenedor
// —de `campamento` a `ejercito`, doc 01 §13— y por eso la guarnición es lo único que defiende (Doc 5.12.4) y lo
// único que come de la plaza. Aquí se trabaja sobre vistas con la tropa puesta (`EjercitoConTropa`, ver
// `engine/tropa.ts`); quien llama las monta y las deshace.
import type { AcuerdoTrueque, Asentamiento, Caravana, Ejercito, Escuadron, Faccion, Heroe, Point, RelacionPolitica, UbicacionHeroe } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { distancia } from '../world/geometria';
import { LOGISTICA, MOVIMIENTO, TROPAS_RECLUTABLES, VISION } from '../constants';
import { capacidadCaravana, velocidadCaravana } from './caravanas';
import {
  alCampamento,
  campamentoDe,
  conEscolta,
  conEscuadrones,
  conTropa,
  defensaDe,
  heroesQueDefienden,
  indiceTropa,
  sinEscolta,
  sinTropa,
  type CaravanaConEscolta,
  type EjercitoConTropa,
  type IndiceTropa,
} from './tropa';
import { atribuir, type EventoCrudo } from '../domain/eventos';
import type { Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { asediarConEjercito, desalojarResidentes, encuentroEntreEjercitos, interceptarCaravanaConEjercito } from './combate';
import { avanzarPosicionEnRuta } from './movimiento';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { avanzarRacion, consumoRacionDeEscuadrones, reservaDeTrigo } from './tropas';
import { puedeLlevar } from './liderazgo';
import { esResidente, estanAliadas } from './pertenencia';
import { heridosEn, herir } from './heroe';

export class MovilizacionInvalidaError extends Error {}

/** Lo que se saca de mirar de cerca una columna (Doc 5.12.3). Cantidades SI —es lo que se cuenta al verla— y
 * nombre del jugador dueno de cada escuadron: saber a quien te enfrentas es la mitad del valor. */
export interface ComposicionColumna {
  ejercitoId: string;
  faccionId: string;
  heroesIds: string[];
  escuadrones: { tropaId: string; cantidad: number; heroeId: string }[];
}

/** Lo que se saca de mirar de cerca una caravana (Doc 5.12.3): QUE lleva y si va escoltada, nunca CUANTO. */
export interface ContenidoCaravana {
  caravanaId: string;
  escoltada: boolean;
  recursos: string[];
}

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
function seleccionarParaCampana(
  /** El campamento de la plaza de la que se sale (`campamentoDe`). */
  campamento: readonly Escuadron[],
  heroeId: string,
  escuadronIds: readonly string[],
  /** `salirAlMundo` sí admite salir con las manos vacías (Doc 1.10.2): el viajero sin tropas es una forma de
   * jugar, no un error. Movilizar una campaña contra un destino, no. */
  permitirVacio = false
): Escuadron[] {
  if (escuadronIds.length === 0 && !permitirVacio) throw new MovilizacionInvalidaError('Hay que llevarse al menos un escuadrón.');
  const elegidos: Escuadron[] = [];
  for (const id of escuadronIds) {
    const escuadron = campamento.find((e) => e.id === id);
    if (!escuadron) throw new MovilizacionInvalidaError(`El escuadrón ${id} no está en este asentamiento.`);
    if (escuadron.heroeId !== heroeId) throw new MovilizacionInvalidaError(`El escuadrón ${id} es de otro jugador.`);
    if (escuadron.cantidad <= 0) throw new MovilizacionInvalidaError(`El escuadrón ${id} está aniquilado.`);
    // En guarnición la maneja la IA y el héroe no puede usarla mientras siga asignada (Doc 5.15.3).
    if (escuadron.enGuarnicion) throw new MovilizacionInvalidaError(`El escuadrón ${id} está en la guarnición.`);
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
/** En qué columna va este jugador, si va en alguna. Lee `participantes` y no los escuadrones: un viajero sin
 * tropas también va dentro de la suya (Doc 5.12.1). */
export function columnaDe(ejercitos: readonly Ejercito[], heroeId: string): Ejercito | undefined {
  return ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId));
}

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
    adjuntasDe(ejercito, caravanas).reduce((suma, c) => suma + capacidadCaravana(c), 0)
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
/**
 * Carga el carro con lo que el jugador ELIGE (Doc 1.10.2), no solo con trigo.
 *
 * Tres topes, y cada uno tapa algo distinto:
 *
 *  1. **La capacidad del carro**, medida sobre el TOTAL de recursos y no por recurso: es un carro, no una
 *     estantería con un cajon por material.
 *  2. **Lo que hay en el almacén**, descontando la reserva de comida de la plaza — sacar el carro no puede
 *     dejar a la guarnición sin comer (misma regla que `cargarCarro`).
 *  3. **Nada negativo.** Sin esto, "cargar" -100 de trigo sería un depósito encubierto que se salta la
 *     reserva.
 *
 * Falla en vez de recortar en silencio: pedir más de lo que cabe es un error del que pide, y servirle menos
 * sin decirlo le hace salir de campaña creyendo que lleva provisiones que no lleva.
 *
 * *Pendiente (Doc 1.10.2):* el tope que el Tesorero podrá fijar sobre cuánto puede retirar cada jugador. Hoy
 * el único límite es físico.
 */
function cargarCarroElegido(
  asentamiento: Asentamiento,
  /** Ración de lo que se queda en el campamento: es lo que protege la reserva de trigo. */
  consumoTropas: number,
  carga: Readonly<Record<string, number>>,
  capacidad: number
): { asentamiento: Asentamiento; suministro: Record<string, number> } {
  const pedido = Object.entries(carga).filter(([, cantidad]) => cantidad !== 0);
  if (pedido.some(([, cantidad]) => cantidad < 0)) {
    throw new MovilizacionInvalidaError('No se puede cargar una cantidad negativa.');
  }
  const total = pedido.reduce((suma, [, cantidad]) => suma + cantidad, 0);
  if (total > capacidad) {
    throw new MovilizacionInvalidaError(`El carro admite ${capacidad} y se piden ${total}.`);
  }
  for (const [recurso, cantidad] of pedido) {
    const reservado = recurso === 'trigo' ? reservaDeTrigo(asentamiento, consumoTropas) : 0;
    if (cantidad > cantidadDisponible(asentamiento.almacen, recurso) - reservado) {
      throw new MovilizacionInvalidaError(`El almacén no tiene ${cantidad} de ${recurso} de sobra.`);
    }
  }
  const suministro = Object.fromEntries(pedido);
  return { asentamiento: { ...asentamiento, almacen: descontarRecursos(asentamiento.almacen, suministro) }, suministro };
}

function cargarCarro(
  asentamiento: Asentamiento,
  /** Ración de lo que se queda en el campamento de la plaza. */
  consumoTropas: number,
  yaEnElCarro: number,
  capacidad: number
): { asentamiento: Asentamiento; cargado: number } {
  const espacio = Math.max(0, capacidad - yaEnElCarro);
  const disponible = Math.max(0, cantidadDisponible(asentamiento.almacen, 'trigo') - reservaDeTrigo(asentamiento, consumoTropas));
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
  caravanas: readonly Caravana[],
  /** La ración del campamento de cada plaza, que protege su reserva de trigo. */
  consumoTropasDe: (plaza: Asentamiento) => number
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
  const carga = cargarCarro(plaza, consumoTropasDe(plaza), enElCarro, capacidadCargaDe(ejercito, caravanas));
  if (carga.cargado <= 0) return { ejercito, plaza: undefined, repuesto: 0 };

  return {
    ejercito: { ...ejercito, suministro: { ...ejercito.suministro, trigo: enElCarro + carga.cargado } },
    plaza: carga.asentamiento,
    repuesto: carga.cargado,
  };
}

/** Tope de Liderazgo del héroe sobre lo que ESE héroe aporta (Doc 5.11): en un ejército de varios no hay tope
 * agregado, cada uno se valida contra el suyo. Cuenta también lo que ya tiene fuera del campamento —en una
 * columna o de escolta—, porque el Liderazgo que ocupa una escolta sin héroe se suma a lo que lleva consigo
 * (Doc 3.13.4). */
function exigirLiderazgo(jugador: Heroe | undefined, escuadrones: readonly Escuadron[]): void {
  const nuevos = new Set(escuadrones.map((e) => e.id));
  const yaFuera = (jugador?.escuadrones ?? []).filter((e) => e.contenedor.tipo !== 'campamento' && !nuevos.has(e.id));
  if (!puedeLlevar(jugador, [...yaFuera, ...escuadrones])) {
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
  /** El campamento de la plaza (`campamentoDe`): de ahí salen los escuadrones, y lo que queda protege el trigo. */
  campamento: readonly Escuadron[],
  jugador: Heroe | undefined,
  heroeId: string,
  escuadronIds: readonly string[],
  objetivo: ObjetivoEjercito,
  asentamientos: readonly Asentamiento[],
  mapa: Mapa,
  id: string,
  /** Para fechar la entrada del que sale: la antigüedad decide la sucesión del líder (Doc 5.14.3). */
  instante: Instante,
  /** Qué hacer con quien pida unirse por el camino (Doc 5.14.1). Se fija aquí y no cambia. Por defecto
   * `rechazar`: lo prudente es que la columna salga con quien salió salvo que su Líder diga otra cosa. */
  politicaDeUnion: Ejercito['politicaDeUnion'] = 'rechazar'
): { asentamiento: Asentamiento; ejercito: EjercitoConTropa; trigoCargado: number } {
  // Tu tropa está en tu campamento, y el campamento es tu residencia (Doc 5.15.2).
  if (!esResidente(asentamiento, heroeId)) {
    throw new MovilizacionInvalidaError('Solo se sale de campaña desde tu residencia: ahí está tu campamento.');
  }
  if (objetivo.tipo === 'asentamiento' && objetivo.id === asentamiento.id) {
    throw new MovilizacionInvalidaError('El destino no puede ser el propio asentamiento de origen.');
  }

  const escuadrones = seleccionarParaCampana(campamento, heroeId, escuadronIds);
  exigirLiderazgo(jugador, escuadrones);

  const destino = puntoDeObjetivo(objetivo, asentamientos);
  // El agua es infranqueable (`world/rutas.ts`): si no hay camino por tierra, no se sale. Un ejército no se
  // embarca — y una ruta recta de reserva sería precisamente una marcha por el mar.
  const ruta = calcularRuta(mapa, asentamiento.posicion, destino);
  if (!ruta) throw new MovilizacionInvalidaError('No hay ruta por tierra hasta ese destino.');
  const idsFuera = new Set(escuadrones.map((e) => e.id));
  const carga = cargarCarro(asentamiento, consumoRacionDeEscuadrones(campamento.filter((e) => !idsFuera.has(e.id))), 0, capacidadCarrosDe(1));

  return {
    asentamiento: carga.asentamiento,
    ejercito: {
      id,
      faccionId: asentamiento.faccionId,
      origenAsentamientoId: asentamiento.id,
      participantes: [{ heroeId, unidoEn: instante }],
      // Movilizar es salir CONTRA un destino, y eso es lo que hace un ejército aunque salga uno solo (Doc
      // 5.12.1): rumbo fijo desde el primer paso, y otros pueden sumarse por el camino.
      tipo: 'ejercito',
      liderId: heroeId,
      politicaDeUnion,
      escuadronIds: [...idsFuera],
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
/**
 * Salir al mundo (Doc 1.10.2): el jugador deja su residencia y aparece en el mapa **junto a la plaza**, sin
 * destino.
 *
 * Es hermana de `movilizarEjercito` y la diferencia no es de tamaño sino de intención, que es lo que fija el
 * `tipo` de la columna para siempre (Doc 5.12.1): movilizar es salir CONTRA un destino y hace un `ejercito`;
 * esto es salir a lo tuyo y hace una columna `personal`, que rectifica el rumbo cuando quiere, no lleva
 * caravanas y a la que nadie se une.
 *
 * Tres cosas que la separan de movilizar, todas del canon:
 *
 *  - **Se puede salir sin tropas.** El viajero solo es una forma de jugar (Doc 5.12.1), no un caso raro.
 *  - **La carga se ELIGE**, no se rellena de trigo hasta arriba.
 *  - **Nace `estacionado` donde la plaza**, no marchando: el destino es un `marcharA` posterior.
 */
export function salirAlMundo(
  asentamiento: Asentamiento,
  /** El campamento de la plaza (`campamentoDe`). */
  campamento: readonly Escuadron[],
  jugador: Heroe | undefined,
  heroeId: string,
  escuadronIds: readonly string[],
  carga: Readonly<Record<string, number>>,
  /** Las columnas del mundo: salir estando ya fuera no es salir, y quien lo sabe es el mundo. */
  ejercitos: readonly Ejercito[],
  id: string,
  instante: Instante
): { asentamiento: Asentamiento; ejercito: EjercitoConTropa } {
  if (!esResidente(asentamiento, heroeId)) {
    throw new MovilizacionInvalidaError('Solo se sale al mundo desde la propia residencia.');
  }
  if (columnaDe(ejercitos, heroeId)) {
    throw new MovilizacionInvalidaError('Ya estás fuera: no se puede salir dos veces.');
  }

  const escuadrones = seleccionarParaCampana(campamento, heroeId, escuadronIds, true);
  exigirLiderazgo(jugador, escuadrones);

  const idsFuera = new Set(escuadrones.map((e) => e.id));
  const cargado = cargarCarroElegido(
    asentamiento,
    consumoRacionDeEscuadrones(campamento.filter((e) => !idsFuera.has(e.id))),
    carga,
    capacidadCarrosDe(1)
  );

  return {
    asentamiento: cargado.asentamiento,
    ejercito: {
      id,
      faccionId: asentamiento.faccionId,
      origenAsentamientoId: asentamiento.id,
      participantes: [{ heroeId, unidoEn: instante }],
      tipo: 'personal',
      liderId: heroeId,
      // A una columna personal no se une nadie (Doc 5.12.1), así que su política no significa nada. Se pone
      // la prudente para que, si alguna vez se leyera por descuido, no abra una puerta que no existe.
      politicaDeUnion: 'rechazar',
      escuadronIds: [...idsFuera],
      escuadrones,
      suministro: cargado.suministro,
      caravanasAdjuntasIds: [],
      // Sin destino todavía: el objetivo es donde está. Un `marcharA` posterior es lo que la pone en camino.
      objetivo: { tipo: 'punto', punto: asentamiento.posicion },
      ruta: [],
      progreso: 0,
      posicionActual: asentamiento.posicion,
      estado: 'estacionado',
    },
  };
}

/**
 * Mete una columna entera dentro de un asentamiento: los escuadrones al campamento de sus héroes y el carro al
 * almacén (Doc 1.10.3). Solo tiene sentido para quien reside ahí, que es donde está su campamento: el llamador
 * se ocupa de los demás.
 *
 * Es la MISMA operación que hace un ejército al llegar a casa replegado, y por eso vive aquí y no duplicada
 * en los dos sitios: si divergieran, volver a casa andando y volver a casa entrando por la puerta dejarían
 * la plaza en estados distintos.
 */
export function absorberColumna(
  asentamiento: Asentamiento,
  ejercito: EjercitoConTropa,
  devolverSuministro: boolean
): { asentamiento: Asentamiento; tropa: Escuadron[] } {
  const almacen = devolverSuministro
    ? Object.entries(ejercito.suministro).reduce((acc, [recurso, cantidad]) => agregarRecurso(acc, recurso, cantidad), asentamiento.almacen)
    : asentamiento.almacen;
  return { asentamiento: { ...asentamiento, almacen }, tropa: alCampamento(ejercito.escuadrones) };
}

/** ¿Está la columna en la PUERTA de esta plaza (Doc 1.10.3)? Entrar es una acción que se ofrece al estar
 * cerca, nunca algo que pase solo por pasar por al lado (Doc 5.12.3). */
export function enLaPuertaDe(ejercito: Ejercito, asentamiento: Asentamiento): boolean {
  return distancia(ejercito.posicionActual, asentamiento.posicion) <= MOVIMIENTO.radioPuerta;
}

/**
 * `guarnecer` (Ocupacion §2.3): un EJÉRCITO en la puerta de una plaza de su Facción entra y se deshace: su
 * tropa vuelve al campamento y su carro al almacén (`absorberColumna`).
 *
 * **Solo si todos los que van dentro residen ahí** (decisión del usuario, 2026-09-14): el campamento de un
 * héroe está en su residencia (Doc 5.15.2), así que volcar la tropa en una plaza ajena ya no tiene dónde
 * dejarla. Reforzar otra plaza es entrar y defenderla en persona.
 *
 * Las **caravanas adjuntas** (§2.3d) NO se pierden: pasan a `'aparcada'` en la plaza anfitriona — siguen
 * siendo de su origen, no las usa la anfitriona, y salen luego enganchadas a un ejército o enviadas a casa.
 *
 * Puro: no toca `session` ni decide dónde queda el jugador (eso es del comando, `situarHeroes`).
 */
export function guarnecer(
  asentamiento: Asentamiento,
  ejercito: EjercitoConTropa,
  caravanas: readonly Caravana[]
): { asentamiento: Asentamiento; tropa: Escuadron[]; caravanasAparcadas: Caravana[] } {
  if (ejercito.tipo !== 'ejercito') {
    throw new MovilizacionInvalidaError('Solo un ejército guarnece: una columna personal no trae tropa que volcar en la guarnición.');
  }
  if (ejercito.faccionId !== asentamiento.faccionId) {
    throw new MovilizacionInvalidaError('Solo se guarnece una plaza de tu propia Facción.');
  }
  if (!ejercito.participantes.every((p) => esResidente(asentamiento, p.heroeId))) {
    throw new MovilizacionInvalidaError('Solo se guarnece la plaza donde residen todos los que van en el ejército: ahí está su campamento.');
  }
  if (!enLaPuertaDe(ejercito, asentamiento)) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${MOVIMIENTO.radioPuerta} de la plaza para guarnecerla.`);
  }

  const caravanasAparcadas = adjuntasDe(ejercito, caravanas).map((c) => ({
    ...c,
    estado: 'aparcada' as const,
    posicionActual: asentamiento.posicion,
  }));

  return { ...absorberColumna(asentamiento, ejercito, true), caravanasAparcadas };
}

export function unirseAEjercito(
  ejercito: EjercitoConTropa,
  asentamiento: Asentamiento,
  /** El campamento de la plaza (`campamentoDe`). */
  campamento: readonly Escuadron[],
  jugador: Heroe | undefined,
  heroeId: string,
  escuadronIds: readonly string[],
  /** Para fechar su entrada: la antigüedad decide la sucesión del líder (Doc 5.14.3). */
  instante: Instante,
  /** Las del mundo: el que se une llena hasta la capacidad TOTAL de la columna, adjuntas incluidas. */
  caravanas: readonly Caravana[] = []
): { asentamiento: Asentamiento; ejercito: EjercitoConTropa; trigoCargado: number } {
  if (!esResidente(asentamiento, heroeId)) {
    throw new MovilizacionInvalidaError('Solo un residente puede sacar tropas de este asentamiento.');
  }
  if (ejercito.faccionId !== asentamiento.faccionId) {
    throw new MovilizacionInvalidaError('No se puede unir tropas a un ejército de otra Facción.');
  }
  if (distancia(ejercito.posicionActual, asentamiento.posicion) > LOGISTICA.radioReabastecimiento) {
    throw new MovilizacionInvalidaError('El ejército está demasiado lejos del asentamiento para recoger tropas.');
  }

  const escuadrones = seleccionarParaCampana(campamento, heroeId, escuadronIds);
  // Solo lo que aporta ESTE héroe cuenta contra SU liderazgo, incluido lo que ya lleve fuera (`exigirLiderazgo`).
  exigirLiderazgo(jugador, escuadrones);

  const idsFuera = new Set(escuadrones.map((e) => e.id));
  const escuadronesTotales = [...ejercito.escuadrones, ...escuadrones];
  // Sumar más tropas a un ejército en el que YA vas es legítimo y no te convierte en dos participantes — ni
  // aporta un carro nuevo, que es lo que el tope de carga de abajo mide.
  const yaDentro = ejercito.participantes.some((p) => p.heroeId === heroeId);
  const participantes = yaDentro ? ejercito.participantes : [...ejercito.participantes, { heroeId, unidoEn: instante }];
  const enElCarro = ejercito.suministro['trigo'] ?? 0;
  const carga = cargarCarro(
    asentamiento,
    consumoRacionDeEscuadrones(campamento.filter((e) => !idsFuera.has(e.id))),
    enElCarro,
    capacidadCargaDe({ ...ejercito, participantes }, caravanas)
  );

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
  // 'aparcada' (Ocupacion §2.3d): una adjunta que otro ejército dejó en una plaza de la Facción al guarnecer.
  // Cualquier ejército de la Facción puede recogerla — sigue siendo de su origen, no de la plaza anfitriona.
  if (caravana.estado !== 'disponible' && caravana.estado !== 'aparcada') {
    throw new MovilizacionInvalidaError('Solo se puede enganchar una caravana disponible o aparcada, no una ya despachada.');
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
  const espacio = capacidadCaravana(caravana) - yaCargado;
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
export function replegarEjercito(
  ejercito: Ejercito,
  origen: Asentamiento | undefined,
  mapa: Mapa,
  /** Quién lo pide. Cancelar es del Líder y solo suyo (Doc 5.14.3): el rumbo lo acordaron varios y deshacerlo
   * no puede ser cosa de uno cualquiera. No atrapa a nadie — el que no quiera seguir se separa. Omitirlo es
   * el camino del sistema (llegar, disolverse), que no tiene actor. */
  quienLoPide?: string
): Ejercito {
  if (ejercito.estado === 'regresando') throw new MovilizacionInvalidaError('El ejército ya está regresando.');
  if (quienLoPide !== undefined && ejercito.liderId !== quienLoPide) {
    throw new MovilizacionInvalidaError('Cancelar la marcha es del Líder; el que no quiera seguir puede separarse.');
  }
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
/**
 * Fija o RECTIFICA el destino de una columna personal (Doc 5.12.1): clic en un punto y se recalcula la ruta
 * desde donde esté, tantas veces como el jugador quiera.
 *
 * **Solo para columnas `personal`, y el criterio es el `tipo`, no cuánta gente va dentro.** Un ejército
 * reducido a un solo miembro sigue siendo un ejército y sigue sin poder cambiar de rumbo: el destino se
 * acordó entre varios al salir, y su única salida es cancelar y volver (Doc 5.12.6). Derivarlo de
 * `participantes.length` reabriría justo el agujero que §1.1f cerró.
 *
 * La ruta se recalcula SIEMPRE desde `posicionActual`, no desde el origen: rectificar a mitad de camino es
 * el caso normal, no la excepción.
 */
export function marcharA(
  ejercito: Ejercito,
  jugador: Heroe,
  objetivo: ObjetivoEjercito,
  asentamientos: readonly Asentamiento[],
  mapa: Mapa
): Ejercito {
  if (ejercito.tipo !== 'personal') {
    throw new MovilizacionInvalidaError('El rumbo de un ejército no se cambia: se cancela y se vuelve.');
  }
  // Su columna puede estar aparcada a la puerta de una plaza mientras él está DENTRO (Doc 1.10.3): la
  // columna no se mueve sola, hay que volver a ella primero.
  if (jugador.ubicacion.tipo !== 'columna') {
    throw new MovilizacionInvalidaError('Estás dentro de una plaza: hay que salir antes de ponerse en marcha.');
  }

  const destino = puntoDeObjetivo(objetivo, asentamientos);
  // El agua es infranqueable, igual que al movilizar: un viajero tampoco se embarca.
  const ruta = calcularRuta(mapa, ejercito.posicionActual, destino);
  if (!ruta) throw new MovilizacionInvalidaError('No hay ruta por tierra hasta ese destino.');

  // Elegir un destino nuevo es dejar de ir detras de alguien (Doc 5.12.3): no hay forma de marchar a un
  // punto Y perseguir a la vez, y dejar la presa puesta la haria reaparecer en el proximo tick.
  return { ...ejercito, estado: 'marchando', objetivo, ruta, progreso: 0, persiguiendo: undefined };
}

/**
 * Funde una columna personal dentro de un Ejército que tiene delante (Doc 5.14.1).
 *
 * Lo que aporta es **lo que ya lleva encima** —sus escuadrones y su carro—, no tropas frescas de casa: eso
 * es `unirseAEjercito`, que valida otra geometría (pasar cerca de TU plaza). Aquí la geometría es estar uno
 * junto al otro.
 *
 * Y el precio está en la última línea: **adopta el destino del ejército**, que ya no puede rectificar. Es lo
 * que hace que marchar acompañado cueste algo, sin ninguna regla extra que lo imponga (Doc 5.12.1).
 */
export function unirseEnCampo(ejercito: EjercitoConTropa, columna: EjercitoConTropa, instante: Instante): EjercitoConTropa {
  validarUnionEnCampo(ejercito, columna);

  const suministro = { ...ejercito.suministro };
  for (const [recurso, cantidad] of Object.entries(columna.suministro)) {
    suministro[recurso] = (suministro[recurso] ?? 0) + cantidad;
  }

  return {
    ...ejercito,
    participantes: [...ejercito.participantes, ...columna.participantes.map((p) => ({ ...p, unidoEn: instante }))],
    escuadrones: [...ejercito.escuadrones, ...columna.escuadrones],
    suministro,
    // La petición atendida se retira: ya no hay nada que contestar.
    peticionesDeUnion: ejercito.peticionesDeUnion?.filter((p) => !columna.participantes.some((q) => q.heroeId === p.heroeId)),
  };
}

/** Las condiciones de unirse en campo (Doc 5.14.1), aparte porque también las comprueba quien solo pide unirse. */
function validarUnionEnCampo(ejercito: Ejercito, columna: Ejercito): void {
  if (ejercito.id === columna.id) throw new MovilizacionInvalidaError('Ya vas en esa columna.');
  if (ejercito.politicaDeUnion === 'rechazar') {
    throw new MovilizacionInvalidaError('Esa columna no admite a nadie más.');
  }
  if (ejercito.tipo !== 'ejercito') {
    throw new MovilizacionInvalidaError('Dos viajeros que se cruzan no forman un ejército.');
  }
  if (columna.tipo !== 'personal') {
    throw new MovilizacionInvalidaError('Un ejército no se une a otro ejército.');
  }
  if (ejercito.faccionId !== columna.faccionId) {
    throw new MovilizacionInvalidaError('Un ejército lo componen ciudadanos de una sola Facción.');
  }
  if (distancia(ejercito.posicionActual, columna.posicionActual) > LOGISTICA.radioEncuentro) {
    throw new MovilizacionInvalidaError('Hay que estar uno junto al otro para unirse en campo.');
  }
}

/**
 * Saca a un jugador de un Ejército y le devuelve la libertad de movimiento (Doc 5.14.2): se lleva lo suyo y
 * nace como columna personal donde estaba.
 *
 * Dos separaciones que el juego no permite, y ya no coinciden desde que un Invitado no puede ser Líder —que
 * es una figura retirada, pero la regla se quedó enunciada aparte y conviene que siga estando:
 *
 *  - **El Líder no se separa.** Para irse cede antes el liderazgo.
 *  - **El último tampoco.** Si no, la columna quedaría vacía en campo abierto con sus caravanas y su
 *    suministro tirados; su salida es cancelar y volver (Doc 5.12.6).
 *
 * El origen NO cambia: la columna nueva hereda el asentamiento del que salió el ejército, no el suyo. Es
 * donde se replegará.
 */
export function separarseDelEjercito(
  ejercito: EjercitoConTropa,
  heroeId: string,
  id: string
): { ejercito: EjercitoConTropa; columna: EjercitoConTropa } {
  const dentro = ejercito.participantes.find((p) => p.heroeId === heroeId);
  if (!dentro) throw new MovilizacionInvalidaError('No vas en ese ejército.');
  if (ejercito.tipo !== 'ejercito') {
    throw new MovilizacionInvalidaError('De una columna personal no te separas: es tuya.');
  }
  if (ejercito.liderId === heroeId) {
    throw new MovilizacionInvalidaError('El Líder no puede separarse: primero tiene que ceder el liderazgo.');
  }
  if (ejercito.participantes.length <= 1) {
    throw new MovilizacionInvalidaError('Eres el último: hay que cancelar la marcha, no vaciar la columna.');
  }
  return desgajar(ejercito, heroeId, id);
}

/**
 * Saca a un héroe de una columna con lo suyo y, como mucho, un carro, como columna personal donde estaba.
 * Sin validar: separarse (arriba) y volver a una casa en la que no se reside (`avanzarEjercitos`) ponen cada
 * uno sus condiciones.
 */
function desgajar(ejercito: EjercitoConTropa, heroeId: string, id: string): { ejercito: EjercitoConTropa; columna: EjercitoConTropa } {
  const dentro = ejercito.participantes.find((p) => p.heroeId === heroeId)!;
  const suyos = ejercito.escuadrones.filter((e) => e.heroeId === heroeId);
  // Se lleva COMO MUCHO un carro, que es lo que aportó (Doc 5.13). Se reparte a prorrata sobre lo que haya:
  // el carro es común mientras se marcha junto, así que no hay "su" trigo que devolver, solo una parte.
  const total = Object.values(ejercito.suministro).reduce((suma, c) => suma + c, 0);
  const seLleva = Math.min(capacidadCarrosDe(1), total);
  const fraccion = total > 0 ? seLleva / total : 0;
  const suministroColumna: Record<string, number> = {};
  const suministroResto: Record<string, number> = {};
  for (const [recurso, cantidad] of Object.entries(ejercito.suministro)) {
    const parte = cantidad * fraccion;
    if (parte > 0) suministroColumna[recurso] = parte;
    if (cantidad - parte > 0) suministroResto[recurso] = cantidad - parte;
  }

  const idsSuyos = new Set(suyos.map((e) => e.id));
  return {
    ejercito: {
      ...ejercito,
      participantes: ejercito.participantes.filter((p) => p.heroeId !== heroeId),
      escuadrones: ejercito.escuadrones.filter((e) => !idsSuyos.has(e.id)),
      suministro: suministroResto,
    },
    columna: {
      id,
      faccionId: ejercito.faccionId,
      // El origen se HEREDA (Doc 5.14.2): separarse no inventa una casa nueva.
      origenAsentamientoId: ejercito.origenAsentamientoId,
      participantes: [dentro],
      tipo: 'personal',
      liderId: heroeId,
      politicaDeUnion: 'rechazar',
      escuadronIds: suyos.map((e) => e.id),
      escuadrones: suyos,
      suministro: suministroColumna,
      caravanasAdjuntasIds: [],
      objetivo: { tipo: 'punto', punto: ejercito.posicionActual },
      ruta: [],
      progreso: 0,
      posicionActual: ejercito.posicionActual,
      estado: 'estacionado',
    },
  };
}

/** Eleva a otro integrante a Líder (Doc 5.14.3). Es el único camino para que el Líder pueda irse. */
export function cederLiderazgo(ejercito: Ejercito, liderActualId: string, sucesorId: string): Ejercito {
  if (ejercito.liderId !== liderActualId) throw new MovilizacionInvalidaError('Solo el Líder cede el liderazgo.');
  if (sucesorId === liderActualId) throw new MovilizacionInvalidaError('Ya eres el Líder.');
  if (!ejercito.participantes.some((p) => p.heroeId === sucesorId)) {
    throw new MovilizacionInvalidaError('El sucesor tiene que ir dentro de la columna.');
  }
  return { ...ejercito, liderId: sucesorId };
}

/** ¿Sigue viva esta petición? La caducidad se evalúa AL LEER (Doc 5.14.1): nada se dispara a los 10 s. */
function peticionViva(peticion: { expiraEn: Instante }, ahora: Instante): boolean {
  return ahora < peticion.expiraEn;
}

/**
 * Anota una petición de unión con su caducidad (Doc 5.14.1), reemplazando la que ese jugador tuviera y
 * barriendo de paso las que ya vencieron — el único momento en que alguien las mira es este y el de
 * contestarlas, así que aquí es donde se limpian sin necesidad de temporizador.
 *
 * Valida la unión ANTES de anotar: pedirle al Líder que decida sobre algo que el motor va a rechazar
 * igualmente es hacerle perder los diez segundos que tiene.
 */
export function anotarPeticionDeUnion(ejercito: Ejercito, columna: Ejercito, ahora: Instante, expiraEn: Instante): Ejercito {
  validarUnionEnCampo(ejercito, columna);
  const heroeId = columna.liderId;
  const vivas = (ejercito.peticionesDeUnion ?? []).filter((p) => peticionViva(p, ahora) && p.heroeId !== heroeId);
  return { ...ejercito, peticionesDeUnion: [...vivas, { heroeId, pedidoEn: ahora, expiraEn }] };
}

/**
 * Retira una petición contestada (Doc 5.14.1), comprobando lo que la hace contestable: que quien responde
 * sea el Líder, y que la petición **siga viva**. La caducidad se mira aquí, al leer, porque nada la barrió
 * al vencer.
 */
export function retirarPeticionDeUnion(ejercito: Ejercito, liderId: string, solicitanteId: string, ahora: Instante): Ejercito {
  if (ejercito.liderId !== liderId) {
    throw new MovilizacionInvalidaError('Solo el Líder contesta las peticiones de unión.');
  }
  const peticion = (ejercito.peticionesDeUnion ?? []).find((p) => p.heroeId === solicitanteId);
  if (!peticion) throw new MovilizacionInvalidaError('No hay ninguna petición de ese jugador.');
  if (!peticionViva(peticion, ahora)) {
    throw new MovilizacionInvalidaError('Esa petición ya caducó: el silencio cuenta como un no.');
  }
  return { ...ejercito, peticionesDeUnion: (ejercito.peticionesDeUnion ?? []).filter((p) => p.heroeId !== solicitanteId) };
}

/**
 * Lo que se distingue de una columna ajena al acercarse a mirarla (Doc 5.12.3): que tropas la componen y de
 * quien son.
 *
 * Es la telemetria que la proyeccion prohibe a distancia de vista, y aqui se concede porque **se paga**: hay
 * que meterse dentro del anillo de inspeccion (40), y el observado recibe aviso. Obtener informacion deja de
 * ser gratis y pasa a ser una jugada con riesgo.
 */
export function inspeccionarColumna(observador: Ejercito, objetivo: EjercitoConTropa): ComposicionColumna {
  if (observador.id === objetivo.id) throw new MovilizacionInvalidaError('Esa columna es la tuya.');
  if (distancia(observador.posicionActual, objetivo.posicionActual) > MOVIMIENTO.radioInspeccion) {
    throw new MovilizacionInvalidaError(`Hay que acercarse a menos de ${MOVIMIENTO.radioInspeccion} para inspeccionar.`);
  }
  return {
    ejercitoId: objetivo.id,
    faccionId: objetivo.faccionId,
    heroesIds: objetivo.participantes.map((p) => p.heroeId),
    escuadrones: objetivo.escuadrones
      .filter((e) => e.cantidad > 0)
      .map((e) => ({ tropaId: e.tropaId, cantidad: e.cantidad, heroeId: e.heroeId })),
  };
}

/** Lo que se distingue de una caravana al acercarse: si lleva escolta y QUE carga, nunca cuanto (Doc 5.12.3).
 * Es lo mismo que ya se ve de lejos — inspeccionar una caravana no anade nada salvo certeza, y por eso
 * tampoco cuesta mas que acercarse. */
export function inspeccionarCaravana(observador: Ejercito, objetivo: Caravana, escoltada: boolean): ContenidoCaravana {
  if (distancia(observador.posicionActual, objetivo.posicionActual) > MOVIMIENTO.radioInspeccion) {
    throw new MovilizacionInvalidaError(`Hay que acercarse a menos de ${MOVIMIENTO.radioInspeccion} para inspeccionar.`);
  }
  return {
    caravanaId: objetivo.id,
    escoltada,
    recursos: Object.keys(objetivo.contenido)
      .filter((r) => (objetivo.contenido[r] ?? 0) > 0)
      .sort(),
  };
}

/**
 * ¿Se puede tocar esta columna? Solo mientras lleve algún héroe sano (Doc 5.16.4): los heridos no persiguen, no
 * se les persigue ni entran en batallas, así que una columna de solo heridos es intocable. Vale igual para quien
 * ataca: sin un héroe sano no hay quien lleve a la tropa.
 */
export function tieneHeroeSano(ejercito: Ejercito, heridos: ReadonlySet<string>): boolean {
  return ejercito.participantes.some((p) => !heridos.has(p.heroeId));
}

/** Lo que entra en una batalla: las escuadras de un héroe herido no combaten (Doc 5.16.4; decisión del usuario,
 * 2026-09-14: sin su héroe solo combaten la escolta de una caravana y la guarnición). */
function sinHeridos(ejercito: EjercitoConTropa, heridos: ReadonlySet<string>): EjercitoConTropa {
  return { ...ejercito, escuadrones: ejercito.escuadrones.filter((e) => !heridos.has(e.heroeId)) };
}

/** Devuelve a la columna las escuadras que `sinHeridos` dejó fuera, intactas y en su orden. */
function conApartadas(trasCombate: EjercitoConTropa, antes: EjercitoConTropa): EjercitoConTropa {
  const peleadas = new Map(trasCombate.escuadrones.map((e) => [e.id, e]));
  return { ...trasCombate, escuadrones: antes.escuadrones.map((e) => peleadas.get(e.id) ?? e) };
}

function exigirTocables(atacante: Ejercito, defensor: Ejercito, heridos: ReadonlySet<string>): void {
  if (!tieneHeroeSano(atacante, heridos)) throw new MovilizacionInvalidaError('Todos los héroes de tu columna están heridos: no pueden entrar en batalla.');
  if (!tieneHeroeSano(defensor, heridos)) throw new MovilizacionInvalidaError('Esa columna solo lleva héroes heridos: no se la puede tocar.');
}

/**
 * Lo que le pasa al DERROTADO en campo abierto (Doc 5.12.3, 5.16.4): pierde la mitad de su carro, igual una
 * columna personal que un Ejercito, cuyo `suministro` es ya el de todos sus miembros, y sus héroes quedan heridos
 * (`vencidos`: los hiere el llamador, que es quien tiene los héroes). La mitad entera aunque alguno ya fuera herido
 * y no combatiera: el carro es de la columna (decisión del usuario, 2026-09-14).
 *
 * Perder la mitad y no todo es deliberado: dejarle algo es lo que hace que valga la pena seguir el viaje en
 * vez de reiniciarlo, y lo que distingue un robo de una ruina.
 */
function trasDerrota<E extends Ejercito>(perdedor: E): { perdedor: E; botin: Record<string, number>; vencidos: string[] } {
  const botin: Record<string, number> = {};
  const queda: Record<string, number> = {};
  for (const [recurso, cantidad] of Object.entries(perdedor.suministro)) {
    const robado = cantidad * MOVIMIENTO.fraccionRobada;
    if (robado > 0) botin[recurso] = robado;
    if (cantidad - robado > 0) queda[recurso] = cantidad - robado;
  }
  const derrotado = { ...perdedor, suministro: queda };
  // Soltar la presa sigue siendo solo de la columna personal, como antes de que el Ejercito perdiera el carro.
  return {
    perdedor: perdedor.tipo === 'personal' ? { ...derrotado, persiguiendo: undefined } : derrotado,
    botin,
    vencidos: perdedor.participantes.map((p) => p.heroeId),
  };
}

/** El ganador carga el botin de `trasDerrota` hasta su `capacidad`: un ladron sin sitio deja lo que no le cabe. */
function cargarBotin<E extends Ejercito>(ganador: E, botin: Record<string, number>, capacidad: number): E {
  const suministro = { ...ganador.suministro };
  let yaLleva = Object.values(suministro).reduce((suma, c) => suma + c, 0);
  for (const [recurso, cantidad] of Object.entries(botin)) {
    const cogido = Math.min(Math.max(0, capacidad - yaLleva), cantidad);
    if (cogido > 0) {
      suministro[recurso] = (suministro[recurso] ?? 0) + cogido;
      yaLleva += cogido;
    }
  }
  return { ...ganador, suministro };
}

/**
 * Atacar a una columna que tienes delante (Doc 5.12.3). Sustituye al choque que el tick resolvia solo por
 * geometria: acercarse ya no basta, hay que pedirlo.
 *
 * Los héroes del perdedor quedan heridos (`vencidos`, Doc 5.16.4) y pierde la mitad de su carro en favor del
 * ganador, sea viajero o Ejercito. El botin va limitado por la capacidad del que lo coge: un ladron sin sitio deja
 * lo que no le cabe. Las escuadras de los heridos no combaten.
 */
export function atacarColumna(
  atacante: EjercitoConTropa,
  defensor: EjercitoConTropa,
  facciones: Faccion[],
  relaciones: readonly RelacionPolitica[],
  /** Las del mundo: de aquí salen las adjuntas del que gane, sea quien sea, para saber cuánto botín le cabe. */
  caravanas: readonly Caravana[],
  /** Los héroes heridos ahora (`heridosEn`). */
  heridos: ReadonlySet<string>,
  rng: RandomFn
): { atacante: EjercitoConTropa; defensor: EjercitoConTropa; facciones: Faccion[]; eventos: EventoCrudo[]; vencidos: string[] } {
  if (atacante.id === defensor.id) throw new MovilizacionInvalidaError('Esa columna es la tuya.');
  if (atacante.faccionId === defensor.faccionId || estanAliadas(relaciones, atacante.faccionId, defensor.faccionId)) {
    throw new MovilizacionInvalidaError('No se ataca a los tuyos ni a un aliado.');
  }
  if (distancia(atacante.posicionActual, defensor.posicionActual) > LOGISTICA.radioEncuentro) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${LOGISTICA.radioEncuentro} para atacar.`);
  }
  exigirTocables(atacante, defensor, heridos);

  const choque = encuentroEntreEjercitos(sinHeridos(atacante, heridos), sinHeridos(defensor, heridos), facciones, rng);
  const gano = choque.a.escuadrones.some((e) => e.cantidad > 0) && !choque.b.escuadrones.some((e) => e.cantidad > 0);
  // Quien pierde es quien se queda sin nadie en pie; si los dos siguen enteros no hay derrota que castigar.
  const perdedor = gano ? conApartadas(choque.b, defensor) : conApartadas(choque.a, atacante);
  const ganador = gano ? conApartadas(choque.a, atacante) : conApartadas(choque.b, defensor);
  const secuela = trasDerrota(perdedor);

  const conBotin = cargarBotin(ganador, secuela.botin, capacidadCargaDe(ganador, caravanas));

  return {
    atacante: gano ? conBotin : secuela.perdedor,
    defensor: gano ? secuela.perdedor : conBotin,
    facciones: choque.facciones,
    eventos: choque.eventos,
    vencidos: secuela.vencidos,
  };
}

/** Interceptar una caravana que tienes delante (Doc 5.12.3). Mismo cambio que `atacarColumna`: lo que antes
 * disparaba la geometria ahora lo pide el jugador. Si la caravana se zafa, pierde el atacante y sus héroes quedan
 * heridos (`vencidos`, Doc 5.16.4); la escolta de la caravana no tiene héroe. */
export function interceptar(
  ejercito: EjercitoConTropa,
  caravana: CaravanaConEscolta,
  capacidadCarga: number,
  heridos: ReadonlySet<string>,
  rng: RandomFn
): ReturnType<typeof interceptarCaravanaConEjercito> & { vencidos: string[] } {
  if (!tieneHeroeSano(ejercito, heridos)) throw new MovilizacionInvalidaError('Todos los héroes de tu columna están heridos: no pueden entrar en batalla.');
  if (distancia(ejercito.posicionActual, caravana.posicionActual) > LOGISTICA.radioEncuentro) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${LOGISTICA.radioEncuentro} para interceptar.`);
  }
  const r = interceptarCaravanaConEjercito(sinHeridos(ejercito, heridos), caravana, capacidadCarga, rng);
  return { ...r, ejercito: conApartadas(r.ejercito, ejercito), vencidos: r.capturada ? [] : ejercito.participantes.map((p) => p.heroeId) };
}

/**
 * Fijar a quien persigues (Doc 5.12.3). No es un destino: es un objetivo que se mueve, y la ruta se
 * recalcula cada tick hacia donde este.
 *
 * Termina de tres formas: al alcanzarlo (15, y entonces hay combate porque ya lo elegiste), al rectificar el
 * rumbo con `marcharA` y al soltarlo. Mientras la presa solo lleve héroes heridos no hay combate (Doc 5.16.4).
 */
export function perseguir(
  ejercito: Ejercito,
  objetivo: { tipo: 'ejercito' | 'caravana'; id: string },
  heridos: ReadonlySet<string>,
  /** La columna perseguida, si es una: a una de solo heridos no se la puede perseguir. */
  presa?: Ejercito
): Ejercito {
  if (objetivo.tipo === 'ejercito' && objetivo.id === ejercito.id) {
    throw new MovilizacionInvalidaError('No puedes perseguirte a ti mismo.');
  }
  if (!tieneHeroeSano(ejercito, heridos)) throw new MovilizacionInvalidaError('Todos los héroes de tu columna están heridos: no pueden perseguir.');
  if (presa && !tieneHeroeSano(presa, heridos)) throw new MovilizacionInvalidaError('Esa columna solo lleva héroes heridos: no se la puede perseguir.');
  return { ...ejercito, persiguiendo: objetivo, estado: 'marchando' };
}

/** Soltar la presa. Tambien lo hace cualquier `marcharA`: elegir un destino nuevo es dejar de ir detras de
 * alguien. */
export function dejarDePerseguir(ejercito: Ejercito): Ejercito {
  return { ...ejercito, persiguiendo: undefined };
}

/**
 * Hasta donde ve esta columna (Doc 1.10, Doc 5.12.7). Una con tropa despliega batidores y alcanza
 * `VISION.ejercito`; **un jugador viajando solo, no**, y ve `VISION.jugadorSolo`.
 *
 * Es el mismo criterio que la velocidad (`velocidadDeEjercito`): lo que decide no es de quien sea la columna
 * sino si lleva soldados en pie. Un escuadron aniquilado no ve mas que un hombre solo, igual que no frena
 * mas que un hombre solo.
 */
export function alcanceDeVista(ejercito: Ejercito, tropa: IndiceTropa): number {
  return ejercito.escuadronIds.some((id) => (tropa.get(id)?.cantidad ?? 0) > 0) ? VISION.ejercito : VISION.jugadorSolo;
}

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
export function velocidadDeEjercito(ejercito: EjercitoConTropa, caravanas: readonly Caravana[] = []): number {
  const velocidades = ejercito.escuadrones
    // Un escuadrón aniquilado persiste como IDENTIDAD (Doc 5.4), pero no como gente que camine: no frena a
    // nadie. Sin este filtro, perder hasta el último hombre de la tropa pesada seguiría lastrando la columna.
    .filter((e) => e.cantidad > 0)
    .map((e) => TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)?.velocidad)
    .filter((v): v is number => v !== undefined);
  // Las caravanas adjuntas entran en el MISMO mínimo (Doc 5.13.2): una comercial va a 16, así que engancharla
  // baja una fuerza ligera de 20 a 16 y le quita la capacidad de cazar caravanas. Ahí está el equilibrio de la
  // escolta, sin ninguna regla extra: no se puede escoltar y depredar a la vez.
  for (const c of adjuntasDe(ejercito, caravanas)) velocidades.push(velocidadCaravana(c));
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
function sinSoldados(ejercito: EjercitoConTropa): boolean {
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
  /** Dueños de las escuadras (`engine/tropa.ts`), y dónde queda cada uno al volver a casa. */
  heroes: readonly Heroe[];
}

export interface ResultadoAvanceEjercitos {
  ejercitos: Ejercito[];
  asentamientos: Asentamiento[];
  /** Las del mundo, menos las que se hayan perdido con un ejército derrotado (Doc 5.13.2). */
  caravanas: Caravana[];
  /** Las Facciones, que un asedio puede tocar: XP de combate y conquista, y la penalización de reputación por
   * atacar a un Aliado. Vuelven tal cual si en el tick no hubo ningún asedio. */
  facciones: Faccion[];
  /** Con sus escuadras al día —bajas, hambre, vuelta al campamento— y situados quienes volvieron a casa. */
  heroes: Heroe[];
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
      heroes: [...contexto.heroes],
      eventos: [],
    };
  }

  const eventos: EventoCrudo[] = [];
  const porId = new Map(asentamientos.map((a) => [a.id, a]));
  const supervivientes: EjercitoConTropa[] = [];
  let faccionesActuales = [...facciones];
  // Las escuadras viven en sus héroes (`engine/tropa.ts`): las columnas y caravanas se recorren con la tropa
  // puesta y se deshacen al final. Lo que cambia FUERA de una columna —el campamento que defiende un asedio, la
  // tropa que vuelve a casa— se aplica en el momento, porque el siguiente asedio o reposte tiene que verlo.
  let heroes = [...contexto.heroes];
  /** Heridos al empezar el tick (Doc 5.16.4): sus escuadras no asedian, y un ejército de solo heridos espera. */
  const heridos = heridosEn(heroes, instante);
  const indice = indiceTropa(heroes);
  let caravanasActuales: CaravanaConEscolta[] = caravanas.map((c) => conEscolta(c, indice));
  const consumoTropasDe = (plaza: Asentamiento): number => consumoRacionDeEscuadrones(campamentoDe(plaza, heroes));
  const situar = (ids: readonly string[], ubicacion: UbicacionHeroe): void => {
    const aSituar = new Set(ids);
    heroes = heroes.map((h) => (aSituar.has(h.id) ? { ...h, ubicacion } : h));
  };

  /**
   * Vuelve a casa (Doc 5.12.6). Quien reside en el origen entra: su tropa al campamento y el carro al almacén.
   * Quien no, se queda a la puerta en su propia columna, con lo suyo y un carro, porque su campamento está en
   * otra plaza. Sin origen (cayó) no hay adónde volver: la tropa vuelve a los campamentos de sus héroes.
   */
  const reintegrar = (ejercito: EjercitoConTropa): boolean => {
    const origen = porId.get(ejercito.origenAsentamientoId);
    if (!origen) {
      heroes = conEscuadrones(heroes, alCampamento(ejercito.escuadrones));
      return false;
    }
    let resto = ejercito;
    for (const p of ejercito.participantes) {
      if (esResidente(origen, p.heroeId)) continue;
      const separado = desgajar(resto, p.heroeId, `${ejercito.id}-${p.heroeId}`);
      resto = separado.ejercito;
      supervivientes.push({ ...separado.columna, estado: 'estacionado' });
      situar([p.heroeId], { tipo: 'columna', ejercitoId: separado.columna.id });
    }
    const absorbida = absorberColumna(origen, resto, true);
    porId.set(origen.id, absorbida.asentamiento);
    heroes = conEscuadrones(heroes, absorbida.tropa);
    situar(
      resto.participantes.map((p) => p.heroeId),
      { tipo: 'asentamiento', asentamientoId: origen.id }
    );
    return true;
  };

  for (const original of [...ejercitos].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)).map((e) => conTropa(e, indice))) {
    // 1. Comer. La MISMA regla del hambre que la guarnición, solo que de otra despensa (Doc 5.13).
    const factorConsumo = original.estado === 'estacionado' ? LOGISTICA.factorConsumoEstacionado : 1;
    const trigoEnCarro = original.suministro['trigo'] ?? 0;
    const racion = avanzarRacion(original.escuadrones, trigoEnCarro, factorConsumo, participantesDe(original));
    // `avanzarRacion` narra la deserción sin saber si es guarnición o campaña; aquí sí se sabe de quién es
    // esa columna, y sin atribuirla el evento saldría GLOBAL — o sea, contando a todo el mundo que a un
    // rival se le están desertando los hombres (Doc 5.12.7).
    eventos.push(...racion.eventos.map((e) => atribuir(e, original.origenAsentamientoId)));

    let ejercito: EjercitoConTropa = {
      ...original,
      escuadrones: racion.escuadrones,
      suministro: { ...original.suministro, trigo: trigoEnCarro - racion.trigoConsumido },
    };

    // 2. ¿Se quedó sin nadie DENTRO? Se disuelve y las identidades vacías vuelven a casa a poder rellenarse.
    // Perder todos los soldados ya no basta: los jugadores siguen ahí y ahora viajan a pie (Doc 5.12.1).
    if (sinNadieDentro(ejercito)) {
      const origen = porId.get(ejercito.origenAsentamientoId);
      if (origen) porId.set(origen.id, absorberColumna(origen, ejercito, true).asentamiento);
      heroes = conEscuadrones(heroes, alCampamento(ejercito.escuadrones));
      const volvieron = origen !== undefined;
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
    const reposte = repostarSiPuede(ejercito, porId, relaciones, caravanas, consumoTropasDe);
    if (reposte.plaza) {
      ejercito = { ...ejercito, suministro: reposte.ejercito.suministro };
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
        const volvieron = reintegrar(ejercito);
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
      if (objetivo && objetivo.faccionId !== ejercito.faccionId && !tieneHeroeSano(ejercito, heridos)) {
        // Todos sus héroes están heridos: espera a la puerta y asedia cuando alguno sane (Doc 5.16.4; decisión del
        // usuario, 2026-09-14). Sigue `marchando` con la ruta acabada, así la llegada se vuelve a mirar cada tick.
        supervivientes.push(ejercito);
        continue;
      }
      if (objetivo && objetivo.faccionId !== ejercito.faccionId) {
        const defensores = heroesQueDefienden(objetivo, heroes, heridos);
        const asedio = asediarConEjercito(sinHeridos(ejercito, heridos), objetivo, defensaDe(objetivo, heroes, heridos), faccionesActuales, [...relaciones], instante, rng);
        ejercito = conApartadas(asedio.ejercito, ejercito);
        porId.set(objetivo.id, asedio.defensor);
        heroes = conEscuadrones(heroes, asedio.tropaDefensora);
        faccionesActuales = asedio.facciones;
        // Los héroes del bando que pierde quedan heridos (Doc 5.16.4). Sin combate —plaza vacía u ocupada— no pierde nadie.
        if (asedio.tropaDefensora.length > 0) {
          heroes = herir(heroes, asedio.conquistado ? defensores.map((h) => h.id) : ejercito.participantes.map((p) => p.heroeId), instante);
        }
        // Conquistar no convierte al ejército en guarnición (Doc 5.15.5): acampa a la puerta, y los residentes
        // derrotados se van con su campamento a 0 a la plaza más cercana de su Facción.
        if (asedio.conquistado) {
          const desalojo = desalojarResidentes(objetivo, [...porId.values()], heroes);
          for (const a of desalojo.asentamientos) porId.set(a.id, a);
          heroes = desalojo.heroes;
        }
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
  // Los heridos de nuevo, no los del principio: quien perdió un asedio en este tick ya no persigue a nadie.
  const conEncuentros = resolverEncuentros(supervivientes, caravanasActuales, faccionesActuales, relaciones, porId, heridosEn(heroes, instante), rng);
  eventos.push(...conEncuentros.eventos);

  // Se deshacen las vistas: cada escuadra vuelve a su héroe, marcada donde acabó. La escolta de una caravana
  // capturada ya viene a 0 y al campamento (Doc 5.15.4).
  const tropaFinal: Escuadron[] = [...conEncuentros.escoltasPerdidas];
  const ejercitosFinal = conEncuentros.ejercitos.map((vista) => {
    const { ejercito, tropa } = sinTropa(vista);
    tropaFinal.push(...tropa);
    return ejercito;
  });
  const caravanasFinal = conEncuentros.caravanas.map((vista) => {
    const { caravana, tropa } = sinEscolta(vista);
    tropaFinal.push(...tropa);
    return caravana;
  });

  return {
    ejercitos: ejercitosFinal,
    asentamientos: [...porId.values()],
    caravanas: caravanasFinal,
    facciones: conEncuentros.facciones,
    heroes: herir(conEscuadrones(heroes, tropaFinal), conEncuentros.vencidos, instante),
    eventos,
  };
}

/**
 * Los encuentros de un tick (Doc 5.12.3) — **y ya no salen de la geometría**.
 *
 * Antes, dos columnas enemigas que pasaban a menos de 15 se masacraban solas dentro del tick. Ahora un
 * encuentro exige que alguien lo haya PEDIDO: o con `atacar`/`interceptar` desde el menú, que se resuelven
 * en su comando y no aquí, o **persiguiendo** — y esta funcion es la que cierra las persecuciones cuando el
 * perseguidor alcanza a su presa. Elegir ir detrás de alguien ES elegir el combate; lo que desaparece es
 * pelear por haber pasado cerca.
 *
 * Las reglas que lo acotan siguen siendo las mismas, y siguen haciendo falta:
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
  ejercitos: readonly EjercitoConTropa[],
  caravanas: readonly CaravanaConEscolta[],
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  /** `asentamientoId -> Asentamiento` del tick ya avanzado: de aquí sale de qué Facción es cada caravana. */
  asentamientosPorId: ReadonlyMap<string, Asentamiento>,
  /** Los héroes heridos ahora: ni persiguen, ni se les alcanza, ni sus escuadras combaten (Doc 5.16.4). */
  heridos: ReadonlySet<string>,
  rng: RandomFn
): {
  ejercitos: EjercitoConTropa[];
  caravanas: CaravanaConEscolta[];
  facciones: Faccion[];
  eventos: EventoCrudo[];
  escoltasPerdidas: Escuadron[];
  /** Los héroes de los bandos que perdieron: los hiere quien tiene los héroes. */
  vencidos: string[];
} {
  const eventos: EventoCrudo[] = [];
  if (ejercitos.length === 0) {
    return { ejercitos: [...ejercitos], caravanas: [...caravanas], facciones: [...facciones], eventos, escoltasPerdidas: [], vencidos: [] };
  }

  const porId = new Map(ejercitos.map((e) => [e.id, e]));
  const escoltasPerdidas: Escuadron[] = [];
  const vencidos: string[] = [];
  let caravanasVivas = [...caravanas];
  let faccionesActuales = [...facciones];
  const yaChocaron = new Set<string>();

  const enemiga = (a: string, b: string) => a !== b && !estanAliadas(relaciones, a, b);
  const porIdAsc = (a: { id: string }, b: { id: string }) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);

  for (const id of [...porId.keys()].sort()) {
    if (yaChocaron.has(id)) continue;
    const ejercito = porId.get(id)!;
    // Pelea lo que no está herido (Doc 5.16.4): sin héroe sano no se persigue, y sin soldados sanos no se choca.
    if (!tieneHeroeSano(ejercito, heridos) || sinSoldados(sinHeridos(ejercito, heridos))) continue;

    // **Solo se resuelve lo que se persigue.** Sin presa fijada no hay encuentro, por muy cerca que se pase.
    const presaFijada = ejercito.persiguiendo;
    if (!presaFijada) continue;

    const rivales =
      presaFijada.tipo === 'ejercito'
        ? [...porId.values()]
            .filter((o) => o.id === presaFijada.id)
            .filter((o) => !yaChocaron.has(o.id) && !sinSoldados(sinHeridos(o, heridos)) && enemiga(ejercito.faccionId, o.faccionId))
            .filter((o) => tieneHeroeSano(o, heridos))
            .filter((o) => distancia(o.posicionActual, ejercito.posicionActual) <= LOGISTICA.radioEncuentro)
        : [];
    const presas =
      presaFijada.tipo === 'caravana'
        ? caravanasVivas
            .filter((c) => c.id === presaFijada.id)
            .filter((c) => c.estado !== 'adjunta' && c.estado !== 'disponible')
            .filter((c) => {
              const duena = asentamientosPorId.get(c.origenAsentamientoId)?.faccionId;
              // Sin dueño identificable no se puede decidir si es enemiga, así que no se toca.
              return duena !== undefined && enemiga(ejercito.faccionId, duena);
            })
            .filter((c) => distancia(c.posicionActual, ejercito.posicionActual) <= LOGISTICA.radioEncuentro)
        : [];

    const masCerca = <T extends { id: string; posicionActual: Point }>(lista: T[]): T | undefined =>
      [...lista].sort((x, y) => {
        const dx = distancia(x.posicionActual, ejercito.posicionActual);
        const dy = distancia(y.posicionActual, ejercito.posicionActual);
        return dx !== dy ? dx - dy : porIdAsc(x, y);
      })[0];

    const rival = masCerca(rivales);
    if (rival) {
      const choque = encuentroEntreEjercitos(sinHeridos(ejercito, heridos), sinHeridos(rival, heridos), faccionesActuales, rng);
      // Alcanzada la presa, la persecución termina: se persigue para pelear, y ya se peleo. Al que cae le
      // toca lo mismo que en un ataque (`trasDerrota`): sus héroes heridos, que impide rematarlo en cadena el
      // minuto siguiente, y la mitad del carro para el otro.
      const gano = choque.a.escuadrones.some((e) => e.cantidad > 0) && !choque.b.escuadrones.some((e) => e.cantidad > 0);
      const perdio = choque.b.escuadrones.some((e) => e.cantidad > 0) && !choque.a.escuadrones.some((e) => e.cantidad > 0);
      let cazador = conApartadas(choque.a, ejercito);
      let alcanzado = conApartadas(choque.b, rival);
      if (gano) {
        const secuela = trasDerrota(alcanzado);
        alcanzado = secuela.perdedor;
        cazador = cargarBotin(cazador, secuela.botin, capacidadCargaDe(cazador, caravanasVivas));
        vencidos.push(...secuela.vencidos);
      } else if (perdio) {
        const secuela = trasDerrota(cazador);
        cazador = secuela.perdedor;
        alcanzado = cargarBotin(alcanzado, secuela.botin, capacidadCargaDe(alcanzado, caravanasVivas));
        vencidos.push(...secuela.vencidos);
      }
      porId.set(ejercito.id, { ...cazador, persiguiendo: undefined });
      porId.set(rival.id, alcanzado);
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
      const emboscada = interceptarCaravanaConEjercito(sinHeridos(ejercito, heridos), presa, capacidadCargaDe(ejercito, caravanasVivas), rng);
      porId.set(ejercito.id, { ...conApartadas(emboscada.ejercito, ejercito), persiguiendo: undefined });
      // Si la caravana se zafa, perdió el que la perseguía (Doc 5.16.4).
      if (!emboscada.capturada) vencidos.push(...ejercito.participantes.map((p) => p.heroeId));
      caravanasVivas = emboscada.caravana
        ? caravanasVivas.map((c) => (c.id === presa.id ? emboscada.caravana! : c))
        : caravanasVivas.filter((c) => c.id !== presa.id);
      // Escolta sin héroe (Doc 3.13.4) que vuelve a 0 al campamento tras perder la caravana (Doc 5.15.4).
      escoltasPerdidas.push(...emboscada.escoltaPerdida);
      for (const e of emboscada.eventos) eventos.push(atribuir(e, ejercito.origenAsentamientoId));
      yaChocaron.add(ejercito.id);
    }
  }

  return { ejercitos: [...porId.values()], caravanas: caravanasVivas, facciones: faccionesActuales, eventos, escoltasPerdidas, vencidos };
}

