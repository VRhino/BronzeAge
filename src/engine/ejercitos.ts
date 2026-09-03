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
import { LOGISTICA } from '../constants';
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
 * El carro de suministros nace VACÍO a propósito: cargarlo del granero es el Paso 6 del plan, aislado para
 * que su impacto económico se pueda medir en batch por separado del resto de la mecánica.
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
): { asentamiento: Asentamiento; ejercito: Ejercito } {
  if (!esResidente(asentamiento, jugadorId)) {
    throw new MovilizacionInvalidaError('Solo un residente puede sacar tropas de este asentamiento.');
  }
  if (objetivo.tipo === 'asentamiento' && objetivo.id === asentamiento.id) {
    throw new MovilizacionInvalidaError('El destino no puede ser el propio asentamiento de origen.');
  }

  const escuadrones = seleccionarParaCampana(asentamiento, jugadorId, escuadronIds);
  exigirLiderazgo(jugador, escuadrones);

  const destino = puntoDeObjetivo(objetivo, asentamientos);
  const ruta = calcularRuta(mapa, asentamiento.posicion, destino);
  const idsFuera = new Set(escuadrones.map((e) => e.id));

  return {
    asentamiento: { ...asentamiento, escuadrones: asentamiento.escuadrones.filter((e) => !idsFuera.has(e.id)) },
    ejercito: {
      id,
      faccionId: asentamiento.faccionId,
      origenAsentamientoId: asentamiento.id,
      escuadrones,
      suministro: {},
      caravanasAdjuntasIds: [],
      objetivo,
      ruta,
      progreso: 0,
      posicionActual: asentamiento.posicion,
      estado: 'marchando',
    },
  };
}

/**
 * Un jugador se suma a un ejército ya en campaña (Doc 5.12.1) con escuadrones de SU asentamiento.
 *
 * Exige proximidad: el ejército tiene que estar pasando por (o parado en) el asentamiento del que se une. Sin
 * eso, unirse sería teletransportar refuerzos al otro extremo del mapa — y como el radio es el mismo que el
 * del reabastecimiento, "por dónde puede pasar a recogerte" y "dónde puede repostar" son la misma geografía.
 */
export function unirseAEjercito(
  ejercito: Ejercito,
  asentamiento: Asentamiento,
  jugador: Jugador | undefined,
  jugadorId: string,
  escuadronIds: readonly string[]
): { asentamiento: Asentamiento; ejercito: Ejercito } {
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
  return {
    asentamiento: { ...asentamiento, escuadrones: asentamiento.escuadrones.filter((e) => !idsFuera.has(e.id)) },
    ejercito: { ...ejercito, escuadrones: [...ejercito.escuadrones, ...escuadrones] },
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

  return {
    ...ejercito,
    estado: 'regresando',
    objetivo,
    ruta: calcularRuta(mapa, ejercito.posicionActual, origen.posicion),
    progreso: 0,
  };
}

/** Planta el ejército donde está (Doc 5.12.3): deja de avanzar y pasa a consumo reducido. Aparcar en un paso
 * de montaña es una jugada legítima, y por eso el consumo baja pero nunca llega a 0. */
export function estacionarEjercito(ejercito: Ejercito): Ejercito {
  if (ejercito.estado === 'estacionado') throw new MovilizacionInvalidaError('El ejército ya está estacionado.');
  return { ...ejercito, estado: 'estacionado', objetivo: { tipo: 'punto', punto: ejercito.posicionActual } };
}
