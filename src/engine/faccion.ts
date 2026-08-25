import type { Asentamiento, Faccion } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { CAP_FUNDACION_POR_NIVEL, CIUDADANIA, CUPO_NIVEL_ASENTAMIENTO, NIVEL_FACCION } from '../constants';

/** Fase A5 — payload de `faccion.nivel_subio` (ver `avanzarNivelesFaccion`). */
export interface PayloadFaccionNivelSubio {
  faccionId: string;
  nivelNuevo: number;
}

export class FaccionInvalidaError extends Error {}

export function crearFaccion(id: string, nombre: string): Faccion {
  const nombreLimpio = nombre.trim();
  if (!nombreLimpio) throw new FaccionInvalidaError('El nombre de la Facción no puede estar vacío.');
  return { id, nombre: nombreLimpio, reyId: null, embajadorId: null, nivel: 1, experiencia: 0, ciudadanosIds: [], reputacion: 0 };
}

/**
 * Nivel de Facción por EXPERIENCIA (Doc 1.7, rediseño Fase 0.5): puramente derivado de `faccion.experiencia`
 * (ya monótona por construcción, ver `aplicarAjustesExperiencia`) contra la curva de umbrales acumulados
 * `NIVEL_FACCION.xpParaNivel`.
 */
export function calcularNivelFaccion(faccion: Faccion): number {
  let nivel = 1;
  for (const umbral of NIVEL_FACCION.xpParaNivel) {
    if (faccion.experiencia < umbral) break;
    nivel += 1;
  }
  return Math.min(NIVEL_FACCION.nivelMaximo, nivel);
}

/** Cap de fundación (Doc 1.7): límite duro de asentamientos FUNDADOS (no aplica a conquista/anexión). */
export function calcularCapFundacion(nivel: number): number {
  const idx = Math.min(CAP_FUNDACION_POR_NIVEL.length, Math.max(1, nivel)) - 1;
  return CAP_FUNDACION_POR_NIVEL[idx] ?? CAP_FUNDACION_POR_NIVEL[CAP_FUNDACION_POR_NIVEL.length - 1]!;
}

/** Cupo de asentamientos en nivel 2 o 3 según el nivel de Facción (Doc Fase_0_5 §5). Nivel 1 no tiene cupo. */
export function calcularCupoNivel(nivelFaccion: number, nivelObjetivo: 2 | 3): number {
  const curva = nivelObjetivo === 2 ? CUPO_NIVEL_ASENTAMIENTO.maxNivel2 : CUPO_NIVEL_ASENTAMIENTO.maxNivel3;
  const idx = Math.min(curva.length, Math.max(1, nivelFaccion)) - 1;
  return curva[idx] ?? curva[curva.length - 1]!;
}

export interface AjusteExperiencia {
  faccionId: string;
  delta: number;
  razon: string;
}

/** Aplica una lista de ganancias de experiencia (Doc Fase_0_5 §8: combate/construcción/conquista/caravanas). */
export function aplicarAjustesExperiencia(facciones: Faccion[], ajustes: AjusteExperiencia[]): Faccion[] {
  if (ajustes.length === 0) return facciones;
  const porId = new Map(facciones.map((f) => [f.id, f]));
  for (const { faccionId, delta } of ajustes) {
    const faccion = porId.get(faccionId);
    if (faccion && delta > 0) porId.set(faccionId, { ...faccion, experiencia: faccion.experiencia + delta });
  }
  return facciones.map((f) => porId.get(f.id)!);
}

/** Recalcula el nivel de todas las Facciones a partir de su experiencia actual; devuelve eventos de subida. */
export function avanzarNivelesFaccion(facciones: Faccion[]): { facciones: Faccion[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const actualizadas = facciones.map((f) => {
    const nuevoNivel = calcularNivelFaccion(f);
    if (nuevoNivel > f.nivel) {
      eventos.push({
        codigo: 'faccion.nivel_subio',
        mensaje: `${f.nombre} sube a nivel de Facción ${nuevoNivel}.`,
        payload: { faccionId: f.id, nivelNuevo: nuevoNivel } satisfies PayloadFaccionNivelSubio,
      });
    }
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
 *
 * Un jugador reside en UN solo asentamiento (Doc 2.1, a petición del usuario: es lo que le permite tener como
 * mucho un escuadrón de cada tropa — ver `Escuadron.jugadorId`, domain/types.ts) — por eso necesita la lista
 * COMPLETA de asentamientos, no solo el de destino, para comprobar que el jugador no reside ya en otro.
 */
export function comprarCasa(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  asentamientoId: string,
  jugadorId: string
): { facciones: Faccion[]; asentamiento: Asentamiento } {
  const asentamiento = asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) throw new FaccionInvalidaError('El asentamiento no existe.');
  const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) throw new FaccionInvalidaError('La Facción del asentamiento no existe.');

  const yaCiudadanoDeOtra = facciones.some((f) => f.id !== faccion.id && esCiudadano(f, jugadorId));
  if (yaCiudadanoDeOtra) {
    throw new FaccionInvalidaError('El jugador ya es ciudadano de otra Facción (Doc 0: 1 y solo 1 Facción).');
  }
  if (asentamiento.casasCompradas.includes(jugadorId)) {
    throw new FaccionInvalidaError('El jugador ya tiene casa en este asentamiento.');
  }
  const yaResideEnOtroAsentamiento = asentamientos.some(
    (a) => a.id !== asentamiento.id && (a.jugadoresFundadoresIds.includes(jugadorId) || a.casasCompradas.includes(jugadorId))
  );
  if (yaResideEnOtroAsentamiento) {
    throw new FaccionInvalidaError('El jugador ya reside en otro asentamiento (Doc 2.1: 1 jugador, 1 asentamiento).');
  }
  if (asentamiento.casasCompradas.length >= capacidadCasas(asentamiento)) {
    throw new FaccionInvalidaError('No quedan espacios de vivienda para ciudadanos en este asentamiento.');
  }

  return {
    facciones: facciones.map((f) => (f.id === faccion.id ? otorgarCiudadania(f, jugadorId) : f)),
    asentamiento: { ...asentamiento, casasCompradas: [...asentamiento.casasCompradas, jugadorId] },
  };
}
