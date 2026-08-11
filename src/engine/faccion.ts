import type { Asentamiento, Faccion } from '../domain/types';
import { CAP_FUNDACION_POR_NIVEL, CIUDADANIA, NIVEL_FACCION } from '../constants';
import { poblacionTotal } from './asentamientoQuery';

export class FaccionInvalidaError extends Error {}

export function crearFaccion(id: string, nombre: string): Faccion {
  const nombreLimpio = nombre.trim();
  if (!nombreLimpio) throw new FaccionInvalidaError('El nombre de la Facción no puede estar vacío.');
  return { id, nombre: nombreLimpio, reyId: null, embajadorId: null, nivel: 1, ciudadanosIds: [], reputacion: 0 };
}

function asentamientosDe(faccionId: string, asentamientos: Asentamiento[]): Asentamiento[] {
  return asentamientos.filter((a) => a.faccionId === faccionId);
}

/**
 * Nivel de Facción (Doc 1.7): "qué lo hace subir" no está cerrado en el diseño — placeholder que combina
 * nº de asentamientos propios y población NPC total. Recalculado cada tick a partir del estado actual.
 */
export function calcularNivelFaccion(faccion: Faccion, asentamientos: Asentamiento[]): number {
  const propios = asentamientosDe(faccion.id, asentamientos);
  const poblacion = propios.reduce((acc, a) => acc + poblacionTotal(a), 0);
  const puntos = propios.length * NIVEL_FACCION.puntosPorAsentamiento + Math.floor(poblacion / NIVEL_FACCION.poblacionPorPunto);
  return Math.min(NIVEL_FACCION.nivelMaximo, 1 + Math.floor(puntos / NIVEL_FACCION.puntosPorNivel));
}

/** Cap de fundación (Doc 1.7): límite duro de asentamientos FUNDADOS (no aplica a conquista/anexión). */
export function calcularCapFundacion(nivel: number): number {
  const idx = Math.min(CAP_FUNDACION_POR_NIVEL.length, Math.max(1, nivel)) - 1;
  return CAP_FUNDACION_POR_NIVEL[idx] ?? CAP_FUNDACION_POR_NIVEL[CAP_FUNDACION_POR_NIVEL.length - 1]!;
}

/** Recalcula el nivel de todas las Facciones a partir del estado actual de asentamientos; devuelve eventos de subida. */
export function avanzarNivelesFaccion(facciones: Faccion[], asentamientos: Asentamiento[]): { facciones: Faccion[]; eventos: string[] } {
  const eventos: string[] = [];
  const actualizadas = facciones.map((f) => {
    const nuevoNivel = calcularNivelFaccion(f, asentamientos);
    if (nuevoNivel > f.nivel) eventos.push(`${f.nombre} sube a nivel de Facción ${nuevoNivel}.`);
    return nuevoNivel === f.nivel ? f : { ...f, nivel: nuevoNivel };
  });
  return { facciones: actualizadas, eventos };
}

export function esCiudadano(faccion: Faccion, jugadorId: string): boolean {
  return faccion.ciudadanosIds.includes(jugadorId);
}

export function otorgarCiudadania(faccion: Faccion, jugadorId: string): Faccion {
  if (esCiudadano(faccion, jugadorId)) return faccion;
  return { ...faccion, ciudadanosIds: [...faccion.ciudadanosIds, jugadorId] };
}

export function capacidadCasas(asentamiento: Asentamiento): number {
  return CIUDADANIA.casasBasePorAsentamiento + (asentamiento.nivel - 1) * CIUDADANIA.casasPorNivelAdicional;
}

/**
 * Compra de casa (Doc 2.5): segunda vía de ciudadanía, dentro de un asentamiento de la PROPIA Facción del jugador.
 * En Fase 0 no existe todavía un registro global de "a qué Facción pertenece cada jugador" fuera de las listas
 * de ciudadanos, así que la única validación de "un jugador, una Facción" es no estar ya en OTRA lista.
 */
export function comprarCasa(
  facciones: Faccion[],
  asentamiento: Asentamiento,
  jugadorId: string
): { facciones: Faccion[]; asentamiento: Asentamiento } {
  const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) throw new FaccionInvalidaError('La Facción del asentamiento no existe.');

  const yaCiudadanoDeOtra = facciones.some((f) => f.id !== faccion.id && esCiudadano(f, jugadorId));
  if (yaCiudadanoDeOtra) {
    throw new FaccionInvalidaError('El jugador ya es ciudadano de otra Facción (Doc 0: 1 y solo 1 Facción).');
  }
  if (asentamiento.casasCompradas.includes(jugadorId)) {
    throw new FaccionInvalidaError('El jugador ya tiene casa en este asentamiento.');
  }
  if (asentamiento.casasCompradas.length >= capacidadCasas(asentamiento)) {
    throw new FaccionInvalidaError('No quedan espacios de vivienda para ciudadanos en este asentamiento.');
  }

  return {
    facciones: facciones.map((f) => (f.id === faccion.id ? otorgarCiudadania(f, jugadorId) : f)),
    asentamiento: { ...asentamiento, casasCompradas: [...asentamiento.casasCompradas, jugadorId] },
  };
}
