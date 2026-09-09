import type { AcuerdoTrueque, Asentamiento, CaminoComercial, Caravana, Escuadron, Faccion, Point } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payload de `comercio.entrega_escoltada` (Doc 5.13.3). */
export interface PayloadEntregaEscoltada {
  caravanaId: string;
  acuerdoId: string;
  destinoId: string;
  recurso: string;
  entregado: number;
  comision: number;
}

/** Fase A5 — payloads de los eventos de este subsistema (ver `avanzarCaravanas`/`asignarCaravanasATrueque`). */
export interface PayloadCaravanaLlega {
  caravanaId: string;
  tipo: Caravana['tipo'];
  origenId: string;
  destinoId: string;
  contenido: Record<string, number>;
  comision: number;
}
export interface PayloadTruequeCumplido {
  acuerdoId: string;
  asentamientoAId: string;
  asentamientoBId: string;
}
export interface PayloadTruequeExpirado {
  acuerdoId: string;
}
export interface PayloadCaravanaSale {
  origenId: string;
  destinoId: string;
  cantidad: number;
  recurso: string;
}
import { ANIMAL_CATALOGO, ASIGNACION_CARAVANA, CARAVANA_PREPARACION, CARRO_CATALOGO, COMISION, REPUTACION, TRUEQUE } from '../constants';
import type { AnimalTipo, CarroTipo } from '../domain/types';
import { capacidadCaravana, devolverEscoltaAGuarnicion, velocidadCaravana } from './caravanas';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { COSTE_MOVIMIENTO } from '../worldgen';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { agregarRecurso, cantidadDisponible, descontarRecursos, tieneRecursos } from './almacen';
import { buscarCamino } from './caminos';
import { calcularPrecioReferencia } from './market';
import { cupoCaravanas, cupoEscolta, puedeCrearCaravana, cooldownCaravanaRestante, tieneMercadoActivo } from './asentamientoQuery';
import { avanzarPosicionEnRuta } from './movimiento';
import { factorCapacidadCaravana, factorComisionExterna, factorVelocidadCaravana } from './politicas';
import { aplicarAjustesReputacion, factorComisionPorReputacion, type AjusteReputacion } from './reputacion';

export class TruequeInvalidoError extends Error {}
export class CaravanaInvalidaError extends Error {}

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Propone un acuerdo de trueque entre dos asentamientos (Doc 3.2). "Funciona en ambas direcciones": cada lado
 * se compromete a entregar su propio recurso.
 *
 * **Nace `'propuesto'`, y no obliga a nadie hasta que el otro lado acepta** (`aceptarTrueque`,
 * `Consideraciones/Comercio_Fisico_Definicion.md` decision 5). Hasta 2026-09-07 nacia `'activo'` en el mismo
 * acto, con este motivo escrito aqui: *"el flujo real de 'el Tesorero de B acepta' requiere un jugador
 * interactivo real, fuera de alcance del prototipo"*. Ese motivo caduco con el jugador situado — ahora hay
 * alguien concreto, en un sitio concreto, a quien preguntarle.
 *
 * Lo que esto desbloquea: proponerle un trueque a un JUGADOR ya no le compromete recursos sin su permiso, asi
 * que las Facciones NPC pueden hacerlo (hasta ahora se lo prohibian a si mismas por eso mismo, ver
 * `npcGobernanza`).
 */
export function proponerTrueque(
  asentamientos: Asentamiento[],
  asentamientoAId: string,
  asentamientoBId: string,
  recursoA: string,
  recursoB: string,
  cantidadTotalA: number,
  cantidadTotalB: number,
  instante: Instante,
  contador = 0
): AcuerdoTrueque {
  if (asentamientoAId === asentamientoBId) {
    throw new TruequeInvalidoError('Un asentamiento no puede acordar trueque consigo mismo.');
  }
  const a = asentamientos.find((s) => s.id === asentamientoAId);
  const b = asentamientos.find((s) => s.id === asentamientoBId);
  if (!a || !b) throw new TruequeInvalidoError('Alguno de los asentamientos no existe.');
  if (cantidadTotalA <= 0 || cantidadTotalB <= 0) {
    throw new TruequeInvalidoError('Las cantidades pactadas deben ser mayores que 0.');
  }

  return {
    id: `trueque-${asentamientoAId}-${asentamientoBId}-${contador}`,
    asentamientoAId,
    asentamientoBId,
    recursoA,
    recursoB,
    cantidadTotalA,
    cantidadTotalB,
    cantidadEntregadaA: 0,
    cantidadEntregadaB: 0,
    creadoEn: instante,
    // Mientras esta propuesto, el plazo es para CONTESTAR. `aceptarTrueque` lo vuelve a contar desde el si.
    expiraEn: sumar(instante, minutos(TRUEQUE.plazoMinutosPorDefecto)),
    estado: 'propuesto',
  };
}

/**
 * El otro lado dice que si (Doc 3.2). Solo entonces el acuerdo obliga, y solo entonces las caravanas empiezan
 * a mirarlo (`asignarCaravanasATrueque` filtra por `'activo'`).
 *
 * **El plazo se vuelve a contar desde aqui**, y no es un detalle: si se conservara el original, una propuesta
 * contestada al filo nacería ya sin tiempo material de cumplirse, y el que acepta de buena fe se comería la
 * penalizacion por incumplir (Doc 2.7) sin haber podido hacer nada.
 */
export function aceptarTrueque(acuerdo: AcuerdoTrueque, instante: Instante): AcuerdoTrueque {
  exigirSinContestar(acuerdo);
  return { ...acuerdo, estado: 'activo', expiraEn: sumar(instante, minutos(TRUEQUE.plazoMinutosPorDefecto)) };
}

/**
 * El otro lado dice que no (Doc 3.2). Queda `'rechazado'` en vez de borrarse: una respuesta es informacion, y
 * quien propuso tiene derecho a saber que le han contestado y no que se le ha olvidado a nadie.
 */
export function rechazarTrueque(acuerdo: AcuerdoTrueque): AcuerdoTrueque {
  exigirSinContestar(acuerdo);
  return { ...acuerdo, estado: 'rechazado' };
}

/** Un acuerdo solo se contesta una vez, y solo mientras sigue siendo una propuesta. */
function exigirSinContestar(acuerdo: AcuerdoTrueque): void {
  if (acuerdo.estado !== 'propuesto') {
    throw new TruequeInvalidoError(`El trueque ${acuerdo.id} ya no esta pendiente de respuesta (${acuerdo.estado}).`);
  }
}

/**
 * Crea una caravana comercial VACÍA (revamp, Doc 3.13.2): un activo PERSISTENTE que cuenta contra el
 * `cupoCaravanas` del Mercado (+ política "Ampliación de Flota") y no se desmantela voluntariamente. Nace
 * 'disponible' y SIN carros — no puede viajar hasta que se le añada al menos un carro con animal
 * (`agregarCarroACaravana` + `comprarAnimalParaCaravana`). El coste está en las piezas, no en el casco.
 */
export function crearCaravanaVacia(
  asentamiento: Asentamiento,
  caravanasExistentes: Caravana[],
  instante: Instante,
  contador = 0
): { asentamiento: Asentamiento; caravana: Caravana } {
  if (!tieneMercadoActivo(asentamiento)) {
    throw new CaravanaInvalidaError('El asentamiento necesita un Mercado activo para construir caravanas.');
  }
  if (!puedeCrearCaravana(asentamiento, instante)) {
    throw new CaravanaInvalidaError(
      `Cooldown de creación de caravanas: faltan ~${Math.round(cooldownCaravanaRestante(asentamiento, instante) / 60_000)} min para poder crear otra desde este asentamiento.`
    );
  }
  const cupo = cupoCaravanas(asentamiento);
  const propias = caravanasExistentes.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === asentamiento.id).length;
  if (propias >= cupo) {
    throw new CaravanaInvalidaError(`Cupo de caravanas alcanzado (${propias}/${cupo}).`);
  }
  const caravana: Caravana = {
    id: `caravana-comercial-${asentamiento.id}-${contador}`,
    tipo: 'comercial',
    origenAsentamientoId: asentamiento.id,
    contenido: {},
    posicionActual: asentamiento.posicion,
    progreso: 0,
    estado: 'disponible',
    carros: [],
    reservadaManual: false,
  };
  return { asentamiento: { ...asentamiento, ultimaCaravanaCreadaEn: instante }, caravana };
}

/** Solo se reconfigura una caravana parada en su origen — una en ruta lleva carga y no se toca. */
function exigirReconfigurable(caravana: Caravana): void {
  if (caravana.tipo !== 'comercial' || caravana.carros === undefined) {
    throw new CaravanaInvalidaError('Solo las caravanas comerciales del revamp se componen con piezas.');
  }
  if (caravana.estado !== 'disponible') {
    throw new CaravanaInvalidaError('La caravana está en viaje: solo se reconfigura estando disponible en el origen.');
  }
}

/**
 * Añade un carro a una caravana disponible (Doc 3.13.2). El básico se fabrica en el Mercado; el reforzado
 * exige Carpintería activa. El carro nace SIN animal — hay que comprárselo aparte.
 */
export function agregarCarroACaravana(
  caravana: Caravana,
  asentamiento: Asentamiento,
  tipoCarro: CarroTipo
): { caravana: Caravana; asentamiento: Asentamiento } {
  exigirReconfigurable(caravana);
  const carro = CARRO_CATALOGO[tipoCarro];
  if (carro.fabrica === 'carpinteria' && !asentamiento.edificios.some((e) => e.tipo === 'carpinteria' && e.estado === 'activo')) {
    throw new CaravanaInvalidaError(`El carro ${tipoCarro} se fabrica en la Carpintería, y este asentamiento no tiene una activa.`);
  }
  if (!tieneRecursos(asentamiento.almacen, carro.costo)) {
    throw new CaravanaInvalidaError(`No hay recursos suficientes para fabricar el carro ${tipoCarro}.`);
  }
  return {
    caravana: { ...caravana, carros: [...caravana.carros!, { tipoCarro }] },
    asentamiento: { ...asentamiento, almacen: descontarRecursos(asentamiento.almacen, carro.costo) },
  };
}

/**
 * Compra un animal y lo asigna a un carro sin tracción de una caravana disponible (Doc 3.13.2). Un carro
 * lleva como mucho un animal; para cambiarlo hay que tener el carro libre.
 */
export function comprarAnimalParaCaravana(
  caravana: Caravana,
  asentamiento: Asentamiento,
  carroIndice: number,
  tipoAnimal: AnimalTipo
): { caravana: Caravana; asentamiento: Asentamiento } {
  exigirReconfigurable(caravana);
  const carro = caravana.carros![carroIndice];
  if (!carro) throw new CaravanaInvalidaError(`La caravana no tiene un carro en la posición ${carroIndice}.`);
  if (carro.animal !== undefined) throw new CaravanaInvalidaError('Ese carro ya lleva un animal.');
  const costo = ANIMAL_CATALOGO[tipoAnimal].costo;
  if (!tieneRecursos(asentamiento.almacen, costo)) {
    throw new CaravanaInvalidaError(`No hay recursos suficientes para comprar un ${tipoAnimal}.`);
  }
  return {
    caravana: { ...caravana, carros: caravana.carros!.map((c, i) => (i === carroIndice ? { ...c, animal: tipoAnimal } : c)) },
    asentamiento: { ...asentamiento, almacen: descontarRecursos(asentamiento.almacen, costo) },
  };
}

/**
 * Construye una caravana comercial COMPLETA con la configuración por defecto — casco + 1 carro básico + 1 buey
 * (500/16, coste 20 madera + 12 oro). Es lo que usan el NPC de gobernanza y el laboratorio
 * (`simulacionAutoComercio`), que no componen caravanas a mano. Encadena `crearCaravanaVacia` +
 * `agregarCarroACaravana` + `comprarAnimalParaCaravana`, así que pasa por las mismas comprobaciones (y cobra
 * lo mismo) que el jugador.
 */
export function construirCaravanaComercial(
  asentamiento: Asentamiento,
  caravanasExistentes: Caravana[],
  instante: Instante,
  contador = 0
): { asentamiento: Asentamiento; caravana: Caravana } {
  const vacia = crearCaravanaVacia(asentamiento, caravanasExistentes, instante, contador);
  const conCarro = agregarCarroACaravana(vacia.caravana, vacia.asentamiento, 'basico');
  const conAnimal = comprarAnimalParaCaravana(conCarro.caravana, conCarro.asentamiento, 0, 'buey');
  return { asentamiento: conAnimal.asentamiento, caravana: conAnimal.caravana };
}

/**
 * Ruta que sigue una caravana entre dos asentamientos (Fase 0.3): reusa el Camino Comercial del par si existe
 * —con la polilínea orientada origen→destino— y si no la calcula por pathfinding. `undefined` = no hay ruta
 * por tierra (el agua es infranqueable, Doc 3.10): la caravana no sale.
 */
function calcularRutaComercial(
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  origen: Asentamiento,
  destino: Asentamiento
): Point[] | undefined {
  const camino = buscarCamino(caminos, origen.id, destino.id);
  const orientada = camino
    ? camino.asentamientoAId === origen.id
      ? camino.puntos
      : [...camino.puntos].reverse()
    : undefined;
  return orientada ?? calcularRuta(mapa, origen.posicion, destino.posicion) ?? undefined;
}

/**
 * Lanza una caravana comercial A MANO (Doc 3.13.3): el jugador elige carga, destino y —opcionalmente— una
 * escolta de escuadrones (Doc 3.13.4). La carga se reserva del almacén ya, y la caravana pasa por el estado
 * `'preparando'` en el origen —`kPorCarro × (nº carros − 1)` ticks, 0 para una de un solo carro— antes de
 * salir. `cancelarPreparacionCaravana` la revierte con devolución total (carga Y escolta).
 *
 * `escolta` son los escuadrones YA sacados de la guarnición por el llamador (la validación de propiedad y
 * residencia vive en `session/`); aquí solo se comprueba el cupo del Mercado.
 */
export function prepararCaravanaManual(
  caravana: Caravana,
  origen: Asentamiento,
  destino: Asentamiento,
  carga: Record<string, number>,
  escolta: Escuadron[],
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  instante: Instante
): { caravana: Caravana; asentamiento: Asentamiento } {
  if (caravana.tipo !== 'comercial' || caravana.carros === undefined) {
    throw new CaravanaInvalidaError('Solo se lanzan a mano las caravanas comerciales del revamp.');
  }
  if (caravana.estado !== 'disponible') {
    throw new CaravanaInvalidaError('La caravana no está disponible en su origen.');
  }
  if (caravana.origenAsentamientoId !== origen.id) {
    throw new CaravanaInvalidaError('Ese asentamiento no es el origen de la caravana.');
  }
  const capacidad = capacidadCaravana(caravana) * factorCapacidadCaravana(origen);
  if (capacidad <= 0) {
    throw new CaravanaInvalidaError('La caravana no tiene ningún carro con animal: no puede viajar.');
  }
  const totalCarga = Object.values(carga).reduce((a, b) => a + b, 0);
  if (totalCarga <= 0) throw new CaravanaInvalidaError('Hay que cargar algo en la caravana.');
  if (totalCarga > capacidad + 1e-6) {
    throw new CaravanaInvalidaError(`La carga (${totalCarga.toFixed(0)}) supera la capacidad de la caravana (${capacidad.toFixed(0)}).`);
  }
  for (const [recurso, cantidad] of Object.entries(carga)) {
    if (cantidad < 0) throw new CaravanaInvalidaError('Las cantidades de carga no pueden ser negativas.');
    if (cantidadDisponible(origen.almacen, recurso) < cantidad) {
      throw new CaravanaInvalidaError(`No hay ${recurso} suficiente en el almacén de ${origen.id}.`);
    }
  }
  const cupo = cupoEscolta(origen);
  if (escolta.length > cupo) {
    throw new CaravanaInvalidaError(`La escolta (${escolta.length}) supera el cupo del Mercado (${cupo}).`);
  }
  const ruta = calcularRutaComercial(mapa, caminos, origen, destino);
  if (!ruta) throw new CaravanaInvalidaError('No hay ruta por tierra hasta el destino (el agua es infranqueable).');

  const prepTicks = CARAVANA_PREPARACION.kPorCarro * Math.max(0, caravana.carros.length - 1);
  const contenido = Object.fromEntries(Object.entries(carga).filter(([, c]) => c > 0));

  return {
    asentamiento: { ...origen, almacen: descontarRecursos(origen.almacen, carga) },
    caravana: {
      ...caravana,
      estado: prepTicks > 0 ? 'preparando' : 'en_transito',
      destinoAsentamientoId: destino.id,
      contenido,
      posicionActual: origen.posicion,
      progreso: 0,
      ruta,
      preparaHasta: prepTicks > 0 ? sumar(instante, minutos(prepTicks)) : undefined,
      escolta: escolta.length > 0 ? escolta : undefined,
    },
  };
}

/**
 * Saca de la guarnición los escuadrones que un jugador cede como escolta (Doc 3.13.4): valida que son suyos,
 * que están en este asentamiento y que no están aniquilados. Devuelve la escolta y el asentamiento SIN ella.
 */
export function seleccionarEscoltaCaravana(
  asentamiento: Asentamiento,
  jugadorId: string,
  escuadronIds: readonly string[]
): { escolta: Escuadron[]; asentamiento: Asentamiento } {
  const escolta: Escuadron[] = [];
  for (const id of escuadronIds) {
    const e = asentamiento.escuadrones.find((s) => s.id === id);
    if (!e) throw new CaravanaInvalidaError(`El escuadrón ${id} no está en ${asentamiento.id}.`);
    if (e.jugadorId !== jugadorId) throw new CaravanaInvalidaError(`El escuadrón ${id} es de otro jugador.`);
    if (e.cantidad <= 0) throw new CaravanaInvalidaError(`El escuadrón ${id} está aniquilado.`);
    escolta.push(e);
  }
  const ids = new Set(escuadronIds);
  return { escolta, asentamiento: { ...asentamiento, escuadrones: asentamiento.escuadrones.filter((s) => !ids.has(s.id)) } };
}

/** Escuadrones que un jugador tiene YA cedidos como escolta, sumando todas las caravanas (Doc 3.13.4) — para
 * el tope de Liderazgo al ceder más. */
export function escoltaDeJugador(caravanas: readonly Caravana[], jugadorId: string): Escuadron[] {
  return caravanas.flatMap((c) => c.escolta ?? []).filter((e) => e.jugadorId === jugadorId);
}

/** Cancela la preparación de una caravana: devuelve la carga al almacén y la escolta a la guarnición (Doc 3.13.3). */
export function cancelarPreparacionCaravana(
  caravana: Caravana,
  origen: Asentamiento
): { caravana: Caravana; asentamiento: Asentamiento } {
  if (caravana.estado !== 'preparando') throw new CaravanaInvalidaError('La caravana no se está preparando.');
  let almacen = origen.almacen;
  for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
    almacen = agregarRecurso(almacen, recurso, cantidad);
  }
  const escuadrones = caravana.escolta ? devolverEscoltaAGuarnicion(origen.escuadrones, caravana.escolta) : origen.escuadrones;
  return {
    asentamiento: { ...origen, almacen, escuadrones },
    caravana: {
      ...caravana,
      estado: 'disponible',
      destinoAsentamientoId: undefined,
      contenido: {},
      ruta: undefined,
      progreso: 0,
      preparaHasta: undefined,
      escolta: undefined,
    },
  };
}

/**
 * Intercambia carga entre el carro de una caravana APARCADA (Ocupacion §2.3d) y el almacén de la plaza que la
 * hospeda. Espeja `cargarCaravanaAdjunta` (engine/ejercitos.ts) sin los checks de ejército: la caravana está
 * en la plaza por definición.
 *
 * `'cargar'` = almacén → carro (topado por `capacidadCaravana` y por lo que haya en el almacén).
 * `'descargar'` = carro → almacén (topado por lo que quepa en el almacén — nunca se pierde carga que no
 * cabe, igual que el mostrador `comerciarEnPlaza`).
 */
export function moverCargaCarroAparcada(
  caravana: Caravana,
  plaza: Asentamiento,
  recurso: string,
  cantidad: number,
  sentido: 'cargar' | 'descargar'
): { caravana: Caravana; plaza: Asentamiento } {
  if (caravana.estado !== 'aparcada') throw new CaravanaInvalidaError('Esta caravana no está aparcada en una plaza.');
  if (cantidad <= 0) throw new CaravanaInvalidaError('La cantidad tiene que ser positiva.');
  const enElCarro = caravana.contenido[recurso] ?? 0;

  if (sentido === 'cargar') {
    const yaCargado = Object.values(caravana.contenido).reduce((a, b) => a + b, 0);
    const espacio = capacidadCaravana(caravana) - yaCargado;
    const movido = Math.min(cantidad, espacio, cantidadDisponible(plaza.almacen, recurso));
    if (movido <= 0) throw new CaravanaInvalidaError(`No se puede cargar: sin ${recurso} en el almacén de ${plaza.id}, o la caravana va llena.`);
    return {
      caravana: { ...caravana, contenido: { ...caravana.contenido, [recurso]: enElCarro + movido } },
      plaza: { ...plaza, almacen: descontarRecursos(plaza.almacen, { [recurso]: movido }) },
    };
  }

  const item = plaza.almacen[recurso];
  const hueco = item ? Math.max(0, item.capacidad - item.cantidad) : 0;
  const movido = Math.min(cantidad, enElCarro, hueco);
  if (movido <= 0) throw new CaravanaInvalidaError(`No se puede descargar: la caravana no lleva ${recurso}, o el almacén de ${plaza.id} no tiene sitio.`);
  const contenido = { ...caravana.contenido };
  if (enElCarro - movido > 0) contenido[recurso] = enElCarro - movido;
  else delete contenido[recurso];
  return {
    caravana: { ...caravana, contenido },
    plaza: { ...plaza, almacen: agregarRecurso(plaza.almacen, recurso, movido) },
  };
}

/**
 * Envía una caravana APARCADA (Ocupacion §2.3d) de vuelta a su origen. Vacía: aparece en el origen al
 * instante (no hay carga que teletransportar). Con carga: pasa a `'retornando'` y recorre el mapa de vuelta,
 * volcando lo que lleve en el almacén del origen al llegar (`avanzarCaravanas`, rama `retornando`).
 */
export function enviarCaravanaAlOrigen(
  caravana: Caravana,
  anfitriona: Asentamiento,
  origen: Asentamiento,
  mapa: Mapa
): { caravana: Caravana } {
  if (caravana.estado !== 'aparcada') throw new CaravanaInvalidaError('Solo se envía al origen una caravana aparcada.');
  const carga = Object.values(caravana.contenido).reduce((a, b) => a + b, 0);
  if (carga <= 0) {
    return {
      caravana: { ...caravana, estado: 'disponible', posicionActual: origen.posicion, ruta: undefined, destinoAsentamientoId: undefined },
    };
  }
  const ruta = calcularRuta(mapa, anfitriona.posicion, origen.posicion);
  if (!ruta) throw new CaravanaInvalidaError('No hay ruta por tierra de vuelta al origen desde aquí.');
  return {
    caravana: { ...caravana, estado: 'retornando', destinoAsentamientoId: anfitriona.id, ruta, progreso: 0, posicionActual: anfitriona.posicion },
  };
}

/**
 * Mueve un carro (con su animal) de una caravana disponible a otra del mismo asentamiento (Doc 3.13.5).
 * Sin coste ni tiempo — es mantenimiento de flota, no una mecánica.
 */
export function moverCarroEntreCaravanas(
  desde: Caravana,
  hacia: Caravana,
  carroIndice: number
): { desde: Caravana; hacia: Caravana } {
  for (const c of [desde, hacia]) {
    if (c.tipo !== 'comercial' || c.carros === undefined) {
      throw new CaravanaInvalidaError('Las dos tienen que ser caravanas comerciales del revamp.');
    }
    if (c.estado !== 'disponible') {
      throw new CaravanaInvalidaError('Solo se reconfiguran caravanas disponibles en su origen.');
    }
  }
  if (desde.id === hacia.id) throw new CaravanaInvalidaError('Origen y destino son la misma caravana.');
  if (desde.origenAsentamientoId !== hacia.origenAsentamientoId) {
    throw new CaravanaInvalidaError('Las dos caravanas tienen que ser del mismo asentamiento.');
  }
  const carro = desde.carros![carroIndice];
  if (!carro) throw new CaravanaInvalidaError(`La caravana no tiene un carro en la posición ${carroIndice}.`);
  return {
    desde: { ...desde, carros: desde.carros!.filter((_, i) => i !== carroIndice) },
    hacia: { ...hacia, carros: [...hacia.carros!, carro] },
  };
}

/** Tasa base (Doc 3.5); si es externa, el Tesorero del destino (quien cobra la comisión) puede modularla. */
function tasaComision(origen: Asentamiento, destino: Asentamiento): number {
  if (origen.faccionId === destino.faccionId) return COMISION.tasaMismaFaccion;
  return COMISION.tasaExterna * factorComisionExterna(destino);
}

function bonusPorDistancia(dist: number): number {
  const proporcion = Math.min(1, dist / COMISION.distanciaParaBonusMax);
  return 1 + proporcion * (COMISION.bonusPorDistanciaMax - 1);
}

/** Avanza caravanas en tránsito: movimiento y, al llegar, entrega + comisión con bonificación por distancia (Doc 3.8). */
function avanzarCaravanas(
  caravanas: Caravana[],
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  asentamientosPorId: Map<string, Asentamiento>,
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  facciones: Faccion[],
  instante: Instante,
  eventos: EventoCrudo[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const restantes: Caravana[] = [];

  for (const caravana of caravanas) {
    // Revamp (Doc 3.13.3): una caravana lanzada a mano espera en el origen hasta que se cumple su
    // `preparaHasta`, y entonces sale. Mientras tanto no se mueve ni es interceptable (está en su ciudad).
    if (caravana.estado === 'preparando') {
      if (caravana.preparaHasta !== undefined && instante >= caravana.preparaHasta) {
        const destino = asentamientosPorId.get(caravana.destinoAsentamientoId ?? '');
        restantes.push({ ...caravana, estado: 'en_transito', preparaHasta: undefined });
        eventos.push({
          codigo: 'comercio.caravana_sale',
          mensaje: `Caravana comercial de ${caravana.origenAsentamientoId} termina de prepararse y sale hacia ${destino?.id ?? caravana.destinoAsentamientoId}.`,
          payload: {
            origenId: caravana.origenAsentamientoId,
            destinoId: caravana.destinoAsentamientoId ?? '',
            cantidad: Object.values(caravana.contenido).reduce((a, b) => a + b, 0),
            recurso: Object.keys(caravana.contenido)[0] ?? '',
          } satisfies PayloadCaravanaSale,
        });
      } else {
        restantes.push(caravana);
      }
      continue;
    }
    if (!caravana.destinoAsentamientoId) {
      // Sin destino: una Caravana de Fundación (Doc 1.8, la avanza `avanzarCaravanasFundacion`), una comercial
      // propia 'disponible' sin asignar (Doc 3.2), una 'adjunta' (la mueve `avanzarEjercitos`), o una
      // 'aparcada' en una plaza tras guarnecer (Ocupacion §2.3d). Ninguna se mueve aquí: se dejan pasar.
      restantes.push(caravana);
      continue;
    }
    const origen = asentamientosPorId.get(caravana.origenAsentamientoId);
    const destino = asentamientosPorId.get(caravana.destinoAsentamientoId);
    if (!origen || !destino) continue; // asentamiento desaparecido (fuera de alcance de Fase 0 aún)

    // Retorno real tras entregar (a petición del usuario: una caravana NUNCA se teletransporta) — recorre la
    // MISMA ruta que la llevó a `destino`, pero en sentido inverso, de vuelta a `origen` (su asentamiento de
    // origen permanente, ver `estado` en domain/types.ts). `origenAsentamientoId`/`destinoAsentamientoId` NO
    // se tocan durante el retorno (siguen siendo origen real / destino real de la entrega ya hecha) — solo se
    // invierten los puntos de inicio/fin del movimiento de este tick.
    const retornando = caravana.estado === 'retornando';
    const puntoInicio = retornando ? destino.posicion : origen.posicion;
    const puntoFin = retornando ? origen.posicion : destino.posicion;

    // Distancia en línea recta: base de la bonificación por distancia de la comisión (más abajo) — NO de
    // cuántos ticks tarda la caravana, que depende de la longitud real de la polilínea de `ruta` (puede
    // rodear terreno costoso).
    const distanciaTotal = Math.max(1, distancia(puntoInicio, puntoFin));
    // Revamp (Doc 3.13): una comercial con `carros` deriva su velocidad de sus animales; el resto cae al
    // catálogo. Ver `velocidadCaravana`, engine/caravanas.ts.
    const velocidadBase = velocidadCaravana(caravana);
    // "Rutas Rápidas" (Tesorero, ampliación de comercio) solo aplica a la flota comercial propia — no a
    // Caravanas de Fundación ni a los tipos todavía sin uso real (militar/contrabando, Doc 3.6).
    const velocidad = caravana.tipo === 'comercial' ? velocidadBase * factorVelocidadCaravana(origen) : velocidadBase;

    // Avance real por coste de terreno, sobre la longitud de la polilínea calculada al lanzar la caravana
    // (ver `asignarCaravanasATrueque`) — toda caravana en movimiento (con `destinoAsentamientoId`) trae `ruta`.
    // Bonus de Camino Comercial (Doc 1.6): si existe un camino ya construido para este par de asentamientos,
    // la caravana lo está siguiendo (ver `asignarCaravanasATrueque`, que reusa su polilínea como `ruta`) —
    // todo el trayecto cuenta como "sobre el camino". Simétrico en ambos sentidos: el mismo camino sirve para
    // ir y volver.
    const enCaminoComercial = buscarCamino(caminos, origen.id, destino.id) !== undefined;
    const factorCosteExtra = enCaminoComercial ? COSTE_MOVIMIENTO.factorCamino : 1;
    const avance = avanzarPosicionEnRuta(mapa, caravana.ruta!, caravana.progreso, velocidad, factorCosteExtra);
    const progreso = avance.progreso;
    const posicionActual = avance.posicion;

    if (progreso < 1) {
      restantes.push({ ...caravana, progreso, posicionActual });
      continue;
    }

    if (retornando) {
      // Llegada de vuelta a `origen`: disponible de nuevo para un nuevo envío — sin entrega, comisión ni
      // peaje. `ruta` se limpia: la siguiente asignación calcula una nueva desde `origen`. La escolta (Doc
      // 3.13.4) vuelve a la guarnición del origen.
      let origenTrasLlegada = origen;
      if (caravana.escolta && caravana.escolta.length > 0) {
        origenTrasLlegada = { ...origenTrasLlegada, escuadrones: devolverEscoltaAGuarnicion(origenTrasLlegada.escuadrones, caravana.escolta) };
      }
      // El retorno de comercio siempre llega vacío (entregó en destino). Una caravana ENVIADA A CASA desde
      // una plaza anfitriona (Ocupacion §2.3d, `enviarCaravanaAlOrigen`) puede llegar cargada: se vuelca en
      // el almacén del origen antes de volver al pool. Defensivo para el caso de comercio (contenido ya vacío).
      for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
        if (cantidad > 0) origenTrasLlegada = { ...origenTrasLlegada, almacen: agregarRecurso(origenTrasLlegada.almacen, recurso, cantidad) };
      }
      if (origenTrasLlegada !== origen) asentamientosPorId.set(origen.id, origenTrasLlegada);
      restantes.push({
        ...caravana,
        estado: 'disponible',
        contenido: {},
        destinoAsentamientoId: undefined,
        origenAcuerdoId: undefined,
        ladoAcuerdo: undefined,
        progreso: 0,
        posicionActual: origen.posicion,
        ruta: undefined,
        escolta: undefined,
      });
      continue;
    }

    // Llegada: entrega el contenido, cobra comisión de comercio con bonificación por distancia.
    let almacenDestino = destino.almacen;
    let valorTotal = 0;
    for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
      valorTotal += cantidad * calcularPrecioReferencia(recurso, [...asentamientosPorId.values()]);
      almacenDestino = agregarRecurso(almacenDestino, recurso, cantidad);
    }
    const comision = comisionDeEntrega(valorTotal, distanciaTotal, origen, destino, facciones);
    almacenDestino = agregarRecurso(almacenDestino, 'oro', comision);
    asentamientosPorId.set(destino.id, { ...destino, almacen: almacenDestino });

    eventos.push({
      codigo: 'comercio.caravana_llega',
      mensaje: `Caravana ${caravana.tipo} de ${origen.id} llega a ${destino.id}: entrega ${Object.entries(caravana.contenido)
        .map(([r, c]) => `${c.toFixed(0)} ${r}`)
        .join(', ')} (comisión +${comision.toFixed(1)} oro).`,
      payload: {
        caravanaId: caravana.id,
        tipo: caravana.tipo,
        origenId: origen.id,
        destinoId: destino.id,
        contenido: caravana.contenido,
        comision,
      } satisfies PayloadCaravanaLlega,
    });

    if (caravana.origenAcuerdoId && caravana.ladoAcuerdo) {
      const acuerdo = acuerdosPorId.get(caravana.origenAcuerdoId);
      if (acuerdo) {
        const entregado = Object.values(caravana.contenido)[0] ?? 0;
        const aplicado = aplicarEntregaATrueque(acuerdo, caravana.ladoAcuerdo, entregado, asentamientosPorId);
        const actualizado = aplicado.acuerdo;
        eventos.push(...aplicado.eventos);
        ajustesReputacion.push(...aplicado.ajustesReputacion);
        acuerdosPorId.set(acuerdo.id, actualizado);
      }
    }

    if (caravana.tipo === 'comercial') {
      // Flota propia (ampliación de comercio): en vez de desaparecer O de reaparecer instantáneamente en
      // `origen` (bug corregido a petición del usuario), arranca el viaje de vuelta real — misma `ruta`
      // invertida, ver `retornando` arriba — y solo al completarlo vuelve a 'disponible'. Es un activo
      // persistente y con costo (`construirCaravanaComercial`), no un objeto de un solo uso como el resto de
      // tipos de caravana.
      restantes.push({
        ...caravana,
        estado: 'retornando',
        contenido: {},
        origenAcuerdoId: undefined,
        ladoAcuerdo: undefined,
        progreso: 0,
        posicionActual: destino.posicion,
        ruta: [...caravana.ruta!].reverse(),
      });
    }
    // Resto de tipos (militar/contrabando, sin uso real todavía, Doc 3.6; Caravana de Fundación no llega
    // aquí, ver arriba): comportamiento sin cambios, la caravana se pierde al entregar.
  }

  return restantes;
}

interface LadoPendiente {
  acuerdo: AcuerdoTrueque;
  lado: 'A' | 'B';
  origenId: string;
  destinoId: string;
  recurso: string;
  total: number;
  entregado: number;
}

/**
 * Score de prioridad (ampliación de comercio, a petición del usuario) para decidir, cuando hay menos
 * caravanas disponibles que envíos pendientes en un mismo asentamiento, cuál se sirve primero. Tres factores
 * normalizados a 0-100 y ponderados (ver `ASIGNACION_CARAVANA`, constants.ts):
 * - Urgencia por expiración: cuánto del plazo del acuerdo ya se consumió.
 * - Urgencia por volumen: qué fracción del total pactado sigue pendiente.
 * - Cercanía: destinos más cercanos rinden más envíos por caravana disponible (round-trip más corto).
 */
function scoreAsignacion(l: LadoPendiente, origen: Asentamiento, destino: Asentamiento, instante: Instante): number {
  const plazoTotal = l.acuerdo.expiraEn - l.acuerdo.creadoEn;
  const restante = Math.max(0, l.acuerdo.expiraEn - instante);
  const urgenciaExpiracion = plazoTotal > 0 ? 100 * (1 - restante / plazoTotal) : 100;
  const pendiente = l.total - l.entregado;
  const urgenciaVolumen = l.total > 0 ? 100 * Math.min(1, pendiente / l.total) : 0;
  const dist = distancia(origen.posicion, destino.posicion);
  const cercania = 100 * (1 - Math.min(1, dist / ASIGNACION_CARAVANA.distanciaReferencia));
  return (
    urgenciaExpiracion * ASIGNACION_CARAVANA.pesoUrgenciaExpiracion +
    urgenciaVolumen * ASIGNACION_CARAVANA.pesoUrgenciaVolumen +
    cercania * ASIGNACION_CARAVANA.pesoCercania
  );
}

/**
 * Comisión que cobra el destino por una entrega (Doc 3.3 + 2.7). Extraída de `avanzarCaravanas` al aparecer el
 * segundo camino de entrega —la manual desde una caravana escoltada (Doc 5.13.3)—: los dos tienen que cobrar
 * igual, y con la fórmula copiada eso duraría hasta el primer retoque de una de las dos.
 *
 * Incluye la penalización por reputación: tratar con una Facción de origen poco confiable deja peores
 * términos, y el destino extrae más comisión de esa entrega.
 */
export function comisionDeEntrega(
  valorTotal: number,
  distanciaTotal: number,
  origen: Asentamiento,
  destino: Asentamiento,
  facciones: readonly Faccion[]
): number {
  const faccionOrigen = facciones.find((f) => f.id === origen.faccionId);
  const factorReputacion = faccionOrigen ? factorComisionPorReputacion(faccionOrigen) : 1;
  return valorTotal * tasaComision(origen, destino) * bonusPorDistancia(distanciaTotal) * factorReputacion;
}

/**
 * Suma una entrega a un trueque y, si con ella quedan saldados los dos lados, lo cierra (Doc 3.2).
 *
 * También extraída al aparecer la entrega manual: cerrar un trueque otorga reputación a AMBAS Facciones, y
 * esa regla tiene que ser la misma la conduzca el tick o la conduzca un jugador desde su caravana escoltada.
 */
export function aplicarEntregaATrueque(
  acuerdo: AcuerdoTrueque,
  lado: 'A' | 'B',
  cantidad: number,
  asentamientosPorId: Map<string, Asentamiento>
): { acuerdo: AcuerdoTrueque; eventos: EventoCrudo[]; ajustesReputacion: AjusteReputacion[] } {
  const eventos: EventoCrudo[] = [];
  const ajustesReputacion: AjusteReputacion[] = [];
  const actualizado: AcuerdoTrueque =
    lado === 'A'
      ? { ...acuerdo, cantidadEntregadaA: acuerdo.cantidadEntregadaA + cantidad }
      : { ...acuerdo, cantidadEntregadaB: acuerdo.cantidadEntregadaB + cantidad };

  if (actualizado.cantidadEntregadaA >= actualizado.cantidadTotalA && actualizado.cantidadEntregadaB >= actualizado.cantidadTotalB) {
    actualizado.estado = 'cumplido';
    eventos.push({
      codigo: 'comercio.trueque_cumplido',
      mensaje: `Trueque ${acuerdo.id} cumplido entre ${acuerdo.asentamientoAId} y ${acuerdo.asentamientoBId}.`,
      payload: {
        acuerdoId: acuerdo.id,
        asentamientoAId: acuerdo.asentamientoAId,
        asentamientoBId: acuerdo.asentamientoBId,
      } satisfies PayloadTruequeCumplido,
    });
    const faccionA = asentamientosPorId.get(acuerdo.asentamientoAId)?.faccionId;
    const faccionB = asentamientosPorId.get(acuerdo.asentamientoBId)?.faccionId;
    if (faccionA) ajustesReputacion.push({ faccionId: faccionA, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
    if (faccionB) ajustesReputacion.push({ faccionId: faccionB, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
  }
  return { acuerdo: actualizado, eventos, ajustesReputacion };
}

export class EntregaInvalidaError extends Error {}

/**
 * Entrega MANUAL desde una caravana escoltada a un trueque activo (Doc 5.13.3, decisión del usuario 2026-09-04).
 *
 * Es la contrapartida de `asignarCaravanasATrueque`: allí el motor decide qué caravana sirve qué envío por
 * score; aquí lo decide el jugador, que ha cargado lo que quería y lo ha llevado adonde quería. El diseño
 * objetivo del documento de comercio, en fin — el reparto automático siempre fue el sustituto de Fase 0.
 *
 * Condiciones, y cada una responde a una pregunta distinta de la interfaz:
 *  - el trueque está ACTIVO y uno de sus lados lo debe la Facción del ejército (`ladoPendienteParaEjercito`);
 *  - el ejército está al alcance del asentamiento que RECIBE (el otro lado del trueque);
 *  - la caravana lleva el recurso que ese lado debe.
 *
 * Entrega el menor de lo que carga y lo que falta: sobre-entregar un trueque no tendría dónde imputarse.
 * Cobra la misma comisión que una entrega automática (`comisionDeEntrega`) y cierra el trueque por la misma
 * vía (`aplicarEntregaATrueque`), para que el camino manual no sea una puerta trasera con otras reglas.
 */
export function entregarDesdeCaravanaAdjunta(
  caravana: Caravana,
  acuerdo: AcuerdoTrueque,
  lado: 'A' | 'B',
  recurso: string,
  faltante: number,
  destino: Asentamiento,
  origen: Asentamiento,
  asentamientos: readonly Asentamiento[],
  facciones: readonly Faccion[]
): {
  caravana: Caravana;
  destino: Asentamiento;
  acuerdo: AcuerdoTrueque;
  entregado: number;
  comision: number;
  eventos: EventoCrudo[];
  ajustesReputacion: AjusteReputacion[];
} {
  const cargado = caravana.contenido[recurso] ?? 0;
  if (cargado <= 0) throw new EntregaInvalidaError(`La caravana ${caravana.id} no lleva ${recurso}.`);

  const entregado = Math.min(cargado, faltante);
  if (entregado <= 0) throw new EntregaInvalidaError('Ese lado del trueque ya está saldado.');

  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, a]));
  const valorTotal = entregado * calcularPrecioReferencia(recurso, [...asentamientos]);
  const comision = comisionDeEntrega(valorTotal, Math.max(1, distancia(origen.posicion, destino.posicion)), origen, destino, facciones);

  let almacen = agregarRecurso(destino.almacen, recurso, entregado);
  almacen = agregarRecurso(almacen, 'oro', comision);

  const aplicado = aplicarEntregaATrueque(acuerdo, lado, entregado, asentamientosPorId);
  const eventos: EventoCrudo[] = [
    {
      codigo: 'comercio.entrega_escoltada',
      mensaje: `La caravana escoltada ${caravana.id} entrega ${entregado.toFixed(0)} ${recurso} en ${destino.id} (comisión +${comision.toFixed(1)} oro).`,
      payload: { caravanaId: caravana.id, acuerdoId: acuerdo.id, destinoId: destino.id, recurso, entregado, comision } satisfies PayloadEntregaEscoltada,
    },
    ...aplicado.eventos,
  ];

  const restante = cargado - entregado;
  const contenido = { ...caravana.contenido };
  if (restante > 0) contenido[recurso] = restante;
  else delete contenido[recurso];

  return {
    caravana: { ...caravana, contenido },
    destino: { ...destino, almacen },
    acuerdo: aplicado.acuerdo,
    entregado,
    comision,
    eventos,
    ajustesReputacion: aplicado.ajustesReputacion,
  };
}

/**
 * Asigna caravanas 'disponibles' de la flota propia a los lados pendientes de trueque (ampliación de
 * comercio, a petición del usuario — "solo simulación", Doc 3.2: en el diseño objetivo el jugador elige la
 * caravana, la carga y la escolta a mano; esto es el sustituto automático de Fase 0). Ya NO crea caravanas de
 * la nada como antes — si un asentamiento no tiene ninguna disponible este tick, el envío simplemente espera.
 * Cuando compiten varios envíos por menos caravanas de las que hacen falta, se asignan por `scoreAsignacion`
 * descendente, caravana por caravana, hasta agotar el pool disponible de ese asentamiento.
 */
function asignarCaravanasATrueque(
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  asentamientosPorId: Map<string, Asentamiento>,
  caravanas: Caravana[],
  instante: Instante,
  eventos: EventoCrudo[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const pendientesPorOrigen = new Map<string, LadoPendiente[]>();

  for (const acuerdo of acuerdosPorId.values()) {
    // Una propuesta que nadie contesta CADUCA, y sin penalizar a nadie: no hubo promesa que romper. Por eso
    // sale por aqui y no por la rama de abajo, que si ajusta reputacion.
    if (acuerdo.estado === 'propuesto') {
      if (instante >= acuerdo.expiraEn) {
        acuerdosPorId.set(acuerdo.id, { ...acuerdo, estado: 'expirado' });
        eventos.push({
          codigo: 'comercio.trueque_expirado',
          mensaje: `Trueque ${acuerdo.id} caducó sin respuesta.`,
          payload: { acuerdoId: acuerdo.id } satisfies PayloadTruequeExpirado,
        });
      }
      continue;
    }
    if (acuerdo.estado !== 'activo') continue;
    if (instante >= acuerdo.expiraEn) {
      acuerdosPorId.set(acuerdo.id, { ...acuerdo, estado: 'expirado' });
      eventos.push({
        codigo: 'comercio.trueque_expirado',
        mensaje: `Trueque ${acuerdo.id} expiró sin completarse.`,
        payload: { acuerdoId: acuerdo.id } satisfies PayloadTruequeExpirado,
      });
      // Incumplir un acuerdo aceptado penaliza reputación (Doc 2.7) — solo al lado que no entregó su cupo.
      const faccionA = asentamientosPorId.get(acuerdo.asentamientoAId)?.faccionId;
      const faccionB = asentamientosPorId.get(acuerdo.asentamientoBId)?.faccionId;
      if (faccionA && acuerdo.cantidadEntregadaA < acuerdo.cantidadTotalA) {
        ajustesReputacion.push({ faccionId: faccionA, delta: REPUTACION.penalizacionTruequeIncumplido, razon: 'trueque incumplido' });
      }
      if (faccionB && acuerdo.cantidadEntregadaB < acuerdo.cantidadTotalB) {
        ajustesReputacion.push({ faccionId: faccionB, delta: REPUTACION.penalizacionTruequeIncumplido, razon: 'trueque incumplido' });
      }
      continue;
    }

    const lados: LadoPendiente[] = [
      { acuerdo, lado: 'A', origenId: acuerdo.asentamientoAId, destinoId: acuerdo.asentamientoBId, recurso: acuerdo.recursoA, total: acuerdo.cantidadTotalA, entregado: acuerdo.cantidadEntregadaA },
      { acuerdo, lado: 'B', origenId: acuerdo.asentamientoBId, destinoId: acuerdo.asentamientoAId, recurso: acuerdo.recursoB, total: acuerdo.cantidadTotalB, entregado: acuerdo.cantidadEntregadaB },
    ];

    for (const l of lados) {
      if (l.entregado >= l.total) continue;
      const yaEnTransito = caravanas.some((c) => c.origenAcuerdoId === acuerdo.id && c.ladoAcuerdo === l.lado && c.estado === 'en_transito');
      if (yaEnTransito) continue;
      const arr = pendientesPorOrigen.get(l.origenId) ?? [];
      arr.push(l);
      pendientesPorOrigen.set(l.origenId, arr);
    }
  }

  const caravanasPorId = new Map(caravanas.map((c) => [c.id, c]));

  for (const [origenId, pendientes] of pendientesPorOrigen) {
    const origen = asentamientosPorId.get(origenId);
    if (!origen) continue;

    // Revamp (Doc 3.13.5): `reservadaManual` la saca del reparto automático — el jugador la despacha a mano.
    const disponibles = caravanas.filter(
      (c) => c.tipo === 'comercial' && c.estado === 'disponible' && !c.reservadaManual && c.origenAsentamientoId === origenId
    );
    if (disponibles.length === 0) continue;

    const conScore = pendientes
      .map((l) => {
        const destino = asentamientosPorId.get(l.destinoId);
        return destino ? { l, destino, score: scoreAsignacion(l, origen, destino, instante) } : null;
      })
      .filter((x): x is { l: LadoPendiente; destino: Asentamiento; score: number } => x !== null)
      .sort((a, b) => b.score - a.score);

    let idx = 0;
    for (const { l, destino } of conScore) {
      if (idx >= disponibles.length) break;
      const caravana = disponibles[idx]!;

      const disponibleStock = cantidadDisponible(origen.almacen, l.recurso);
      const pendiente = l.total - l.entregado;
      // Revamp (Doc 3.13): capacidad derivada de los carros de ESTA caravana (tras migración, 1 carro básico
      // + 1 buey = 500, idéntico al fijo anterior). "Carga Ampliada" se aplica encima como siempre.
      const capacidad = capacidadCaravana(caravana) * factorCapacidadCaravana(origen);
      const cantidad = Math.min(disponibleStock, pendiente, capacidad);
      if (cantidad <= 0) continue; // sin stock suficiente todavía: la caravana se reintenta el siguiente tick, no consume su turno

      idx++;
      // Ruta calculada al lanzar (Fase 0.3): rodea terreno costoso en vez de ir en línea recta — ver
      // `world/rutas.ts`. Si ya existe un Camino Comercial para este par (Doc 1.6, ver `engine/caminos.ts`),
      // reusa su polilínea en vez de recalcular — es literalmente "seguir el camino ya construido", y es lo
      // que le da el bonus de velocidad en `avanzarCaravanas` (mismo par -> mismo camino encontrado).
      // `buscarCamino` empareja el par en CUALQUIER orden pero devuelve `puntos` siempre en el orden en que
      // se guardó (`asentamientoAId` -> `asentamientoBId`) — bug corregido a petición del usuario: cuando el
      // camino se reutiliza en el sentido CONTRARIO (esta caravana va de B a A), había que invertir la
      // polilínea; si no, `progreso: 0` caía en el extremo del camino más cercano al DESTINO en vez del
      // propio origen, y la caravana "saltaba" allí en su primer paso de movimiento (ver `avanzarPosicionEnRuta`,
      // que siempre mide el progreso desde `ruta[0]` hacia adelante).
      // El agua es infranqueable: sin camino por tierra la caravana NO sale y la carga se queda en el
      // almacén. Comprobado ANTES de descontar recursos. Ver `calcularRutaComercial` (reusa el Camino
      // Comercial del par si existe, orientado origen→destino).
      const ruta = calcularRutaComercial(mapa, caminos, origen, destino);
      if (!ruta) continue;

      asentamientosPorId.set(origen.id, { ...origen, almacen: descontarRecursos(origen.almacen, { [l.recurso]: cantidad }) });
      caravanasPorId.set(caravana.id, {
        ...caravana,
        estado: 'en_transito',
        destinoAsentamientoId: l.destinoId,
        contenido: { [l.recurso]: cantidad },
        origenAcuerdoId: l.acuerdo.id,
        ladoAcuerdo: l.lado,
        posicionActual: origen.posicion,
        progreso: 0,
        ruta,
      });
      eventos.push({
        codigo: 'comercio.caravana_sale',
        mensaje: `Caravana comercial de ${origenId} sale hacia ${destino.id} con ${cantidad.toFixed(0)} ${l.recurso}.`,
        payload: { origenId, destinoId: destino.id, cantidad, recurso: l.recurso } satisfies PayloadCaravanaSale,
      });
    }
  }

  return [...caravanasPorId.values()];
}

export function avanzarComercio(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  caravanas: Caravana[],
  acuerdos: AcuerdoTrueque[],
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  instante: Instante
): { asentamientos: Asentamiento[]; facciones: Faccion[]; caravanas: Caravana[]; acuerdos: AcuerdoTrueque[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const ajustesReputacion: AjusteReputacion[] = [];
  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, { ...a }]));
  const acuerdosPorId = new Map(acuerdos.map((a) => [a.id, a]));

  const trasMovimiento = avanzarCaravanas(caravanas, mapa, caminos, asentamientosPorId, acuerdosPorId, facciones, instante, eventos, ajustesReputacion);
  const trasAsignacion = asignarCaravanasATrueque(mapa, caminos, acuerdosPorId, asentamientosPorId, trasMovimiento, instante, eventos, ajustesReputacion);

  return {
    asentamientos: asentamientos.map((a) => asentamientosPorId.get(a.id)!),
    facciones: aplicarAjustesReputacion(facciones, ajustesReputacion),
    caravanas: trasAsignacion,
    acuerdos: [...acuerdosPorId.values()],
    eventos,
  };
}
