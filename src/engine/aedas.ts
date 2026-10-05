// Aedas itinerantes (Doc 6.7): NPCs neutrales que recorren los asentamientos por la red de caminos, se detienen unas horas
// en cada uno y, mientras están, le revelan a su Facción lo que conocen (`revelarTecnologias`, `engine/tecnologia.ts`) y
// le venden tecnología (`venderTecnologia`). Sin RNG: el reparto inicial y el siguiente destino salen del orden de ids y de
// la distancia, así que añadirlos no cambia ninguna otra tirada del mundo.
import { AEDAS, PUERTA } from '../constants';
import type { AedaItinerante, Asentamiento, Point, RedCaminos } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { EstadoTecnologia } from '../domain/types';
import { distancia } from '../world/geometria';
import { calcularRuta } from '../world/rutas';
import { avanzarPosicionEnRuta } from './movimiento';
import { aristasDeRed, pesosDeRed } from './redCaminos';
import { revelarTecnologias, type ContextoTecnologia } from './tecnologia';

export type ContextoAedas = ContextoTecnologia & { red: RedCaminos };

const { estanciaMinutos, velocidad, memoriaVisitas } = AEDAS.itinerantes;

/** Cuántos itinerantes hay: uno por cada tanto de Facciones vivas, con un mínimo (Doc 6.7). */
export function itinerantesObjetivo(asentamientos: readonly Asentamiento[]): number {
  if (asentamientos.length === 0) return 0;
  const vivas = new Set(asentamientos.map((a) => a.faccionId)).size;
  return Math.max(AEDAS.itinerantes.minimo, Math.ceil(vivas / AEDAS.itinerantes.faccionesPorAeda));
}

/** El Aeda detenido en esa plaza, si lo hay: el único al que se le puede comprar. */
export function aedaEn(aedas: readonly AedaItinerante[], asentamientoId: string): AedaItinerante | undefined {
  return aedas.find((a) => a.enAsentamientoId === asentamientoId);
}

/** Una plaza los deja pasar salvo que su Facción les haya cerrado la puerta con el grupo `aedas` (Doc 2.8). El cierre por defecto no los incluye. */
function recibeAedas(plaza: Asentamiento): boolean {
  return !(plaza.puertaCerradaA ?? PUERTA.cerradaAPorDefecto).includes('aedas');
}

/** Detenido en la plaza, el tiempo de su estancia. */
function llegado(aeda: AedaItinerante, plaza: Asentamiento, instante: Instante): AedaItinerante {
  return {
    id: aeda.id,
    posicion: plaza.posicion,
    enAsentamientoId: plaza.id,
    hasta: sumar(instante, minutos(estanciaMinutos)),
    progreso: 0,
    recientes: [...aeda.recientes.filter((id) => id !== plaza.id), plaza.id].slice(-memoriaVisitas),
  };
}

/** Los más cercanos que no ha visitado hace poco (o, si no queda ninguno, cualquiera distinto del actual), de cerca a lejos. */
function candidatos(aeda: AedaItinerante, plazas: readonly Asentamiento[]): Asentamiento[] {
  const aceptan = plazas.filter((p) => p.id !== aeda.enAsentamientoId && recibeAedas(p));
  const frescas = aceptan.filter((p) => !aeda.recientes.includes(p.id));
  return (frescas.length > 0 ? frescas : aceptan).sort((a, b) => distancia(aeda.posicion, a.posicion) - distancia(aeda.posicion, b.posicion) || (a.id < b.id ? -1 : 1));
}

/** Sale hacia el candidato más cercano al que haya camino; sin ninguno, se queda esperando otra estancia. */
function partir(aeda: AedaItinerante, plazas: readonly Asentamiento[], ctx: ContextoAedas): AedaItinerante {
  const opciones = { pesos: pesosDeRed(ctx.red), pasosRio: plazas.map((p): Point => p.posicion) };
  for (const destino of candidatos(aeda, plazas)) {
    const ruta = calcularRuta(ctx.mapa, aeda.posicion, destino.posicion, opciones);
    if (ruta) return { id: aeda.id, posicion: aeda.posicion, destinoId: destino.id, ruta, progreso: 0, recientes: aeda.recientes };
  }
  return { ...aeda, hasta: sumar(ctx.instante, minutos(estanciaMinutos)) };
}

/** Un tick de un Aeda: avanza por su ruta, o espera su estancia, o elige el siguiente destino. */
function avanzarAeda(aeda: AedaItinerante, ctx: ContextoAedas): AedaItinerante {
  const plazas = ctx.asentamientos;
  if (aeda.ruta && aeda.destinoId) {
    const destino = plazas.find((p) => p.id === aeda.destinoId);
    if (destino) {
      const avance = avanzarPosicionEnRuta(ctx.mapa, aeda.ruta, aeda.progreso, velocidad, aristasDeRed(ctx.red));
      return avance.progreso >= 1 ? llegado(aeda, destino, ctx.instante) : { ...aeda, posicion: avance.posicion, progreso: avance.progreso };
    }
    // Su destino ya no existe: elige otro desde donde esté.
    return partir({ id: aeda.id, posicion: aeda.posicion, progreso: 0, recientes: aeda.recientes }, plazas, ctx);
  }
  const plazaActual = aeda.enAsentamientoId ? plazas.find((p) => p.id === aeda.enAsentamientoId) : undefined;
  // Detenido en su plaza, o sin plaza ni destino alcanzable (varado): espera lo que dura una estancia antes de volver a buscar ruta.
  if (aeda.hasta !== undefined && ctx.instante < aeda.hasta && (plazaActual || aeda.enAsentamientoId === undefined)) return aeda;
  return partir(plazaActual ? aeda : { id: aeda.id, posicion: aeda.posicion, progreso: 0, recientes: aeda.recientes }, plazas, ctx);
}

/** Aedas que faltan para el objetivo, repartidos por las plazas en orden de id. */
function nuevosItinerantes(existentes: number, plazas: readonly Asentamiento[], instante: Instante): AedaItinerante[] {
  const ordenadas = plazas.filter(recibeAedas).sort((a, b) => (a.id < b.id ? -1 : 1));
  if (ordenadas.length === 0) return [];
  const nuevos: AedaItinerante[] = [];
  for (let i = existentes; i < itinerantesObjetivo(plazas); i++) {
    nuevos.push(llegado({ id: `aeda-${i + 1}`, posicion: ordenadas[0]!.posicion, progreso: 0, recientes: [] }, ordenadas[i % ordenadas.length]!, instante));
  }
  return nuevos;
}

/**
 * Un tick de los Aedas itinerantes: nacen los que falten, se mueven, y cada uno detenido en una plaza revela a su Facción
 * lo que conoce. Devuelve también la tecnología, que es lo que cambia al revelar.
 */
export function avanzarAedas(
  aedas: readonly AedaItinerante[],
  tecnologia: EstadoTecnologia,
  ctx: ContextoAedas
): { aedas: AedaItinerante[]; tecnologia: EstadoTecnologia; eventos: EventoCrudo[] } {
  const actuales = [...aedas.map((a) => avanzarAeda(a, ctx)), ...nuevosItinerantes(aedas.length, ctx.asentamientos, ctx.instante)];
  const eventos: EventoCrudo[] = [];
  let estado = tecnologia;
  for (const aeda of actuales) {
    const plaza = aeda.enAsentamientoId ? ctx.asentamientos.find((p) => p.id === aeda.enAsentamientoId) : undefined;
    const faccion = plaza && ctx.facciones.find((f) => f.id === plaza.faccionId);
    if (!plaza || !faccion) continue;
    const revelado = revelarTecnologias(estado, faccion, plaza.id, ctx);
    estado = revelado.tecnologia;
    eventos.push(...revelado.eventos);
  }
  return { aedas: actuales, tecnologia: estado, eventos };
}
