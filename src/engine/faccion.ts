import type { Asentamiento, Faccion } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { CAP_FUNDACION_POR_NIVEL, CIUDADANIA, CUPO_NIVEL_ASENTAMIENTO, NIVEL_FACCION } from '../constants';
import { CAMPO_CARGO, esResidente, resideEnOtroAsentamiento } from './pertenencia';

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

export function esCiudadano(faccion: Faccion, heroeId: string): boolean {
  return faccion.ciudadanosIds.includes(heroeId);
}

export function otorgarCiudadania(faccion: Faccion, heroeId: string): Faccion {
  if (esCiudadano(faccion, heroeId)) return faccion;
  return { ...faccion, ciudadanosIds: [...faccion.ciudadanosIds, heroeId] };
}

/**
 * Retira la ciudadanía (comando `dejarFaccion`, a petición del usuario).
 *
 * **Sucesión del trono (a petición del usuario, 2026-09-10): una Facción SIEMPRE tiene Rey mientras le quede
 * al menos un ciudadano.** Si se va el Rey, el trono pasa al siguiente de `ciudadanosIds` (orden de ingreso —
 * el fundador primero). Solo queda `reyId: null` si la Facción se queda sin nadie. Si el heredero ocupaba la
 * embajada, esta se vacía (la re-designa el nuevo Rey). El Embajador que se va también libera su cargo.
 *
 * NO toca residencia (`Asentamiento.casasCompradas`/`heroesFundadoresIds`) ni cargos LOCALES (Gobernador,
 * etc.): Doc 2.5 no define qué pasa con la vivienda al abandonar la Facción, y no existe todavía un comando
 * "dejar residencia"/"vender casa" que lo resuelva — limitación documentada, no un olvido (ver `dejarFaccion.ts`).
 */
export function quitarCiudadania(faccion: Faccion, heroeId: string): Faccion {
  if (!esCiudadano(faccion, heroeId)) return faccion;
  const ciudadanosIds = faccion.ciudadanosIds.filter((id) => id !== heroeId);
  const reyId = faccion.reyId === heroeId ? (ciudadanosIds[0] ?? null) : faccion.reyId;
  return {
    ...faccion,
    ciudadanosIds,
    reyId,
    embajadorId: faccion.embajadorId === heroeId || faccion.embajadorId === reyId ? null : faccion.embajadorId,
  };
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
 * mucho un escuadrón de cada tropa — ver `Escuadron.heroeId`, domain/types.ts) — por eso necesita la lista
 * COMPLETA de asentamientos, no solo el de destino, para comprobar que el jugador no reside ya en otro.
 */
export function comprarCasa(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  asentamientoId: string,
  heroeId: string
): { facciones: Faccion[]; asentamiento: Asentamiento } {
  const asentamiento = asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) throw new FaccionInvalidaError('El asentamiento no existe.');
  const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) throw new FaccionInvalidaError('La Facción del asentamiento no existe.');

  const yaCiudadanoDeOtra = facciones.some((f) => f.id !== faccion.id && esCiudadano(f, heroeId));
  if (yaCiudadanoDeOtra) {
    throw new FaccionInvalidaError('El jugador ya es ciudadano de otra Facción (Doc 0: 1 y solo 1 Facción).');
  }
  if (asentamiento.casasCompradas.includes(heroeId)) {
    throw new FaccionInvalidaError('El jugador ya tiene casa en este asentamiento.');
  }
  if (resideEnOtroAsentamiento(asentamientos, asentamiento.id, heroeId)) {
    throw new FaccionInvalidaError('El jugador ya reside en otro asentamiento (Doc 2.1: 1 jugador, 1 asentamiento).');
  }
  if (asentamiento.casasCompradas.length >= capacidadCasas(asentamiento)) {
    throw new FaccionInvalidaError('No quedan espacios de vivienda para ciudadanos en este asentamiento.');
  }

  return {
    facciones: facciones.map((f) => (f.id === faccion.id ? otorgarCiudadania(f, heroeId) : f)),
    asentamiento: { ...asentamiento, casasCompradas: [...asentamiento.casasCompradas, heroeId] },
  };
}

/**
 * Cambiar de residencia (Doc 2.5/2.6, comando nuevo 2026-09-08 — cierra la limitación conocida "no hay comando
 * vender casa / dejar residencia"): atómico, deja la residencia actual y toma otra plaza de la MISMA Facción.
 *
 * Deja la vieja: fuera de `casasCompradas` Y `heroesFundadoresIds` (ya no reside por ninguna vía), y sus
 * cargos LOCALES ahí se vacían (no se gobierna donde no se vive — misma regla que la conquista). Los
 * escuadrones que tuviera POSADOS en la guarnición vieja NO se tocan — pasan a ser guarnición de no-residente
 * (Doc 5.4, revisión 2026-09-08): defiende, la repone y la re-moviliza igual.
 *
 * La ciudadanía de Facción no cambia (es la misma Facción). Un HUÉRFANO —sin residencia de la que salir— usa
 * `comprarCasa`/`unirseAFaccion`, no este comando.
 *
 * `ponytail:` sin cooldown ni coste todavía — el abuso "mudarse en cada conquista para exprimir el impuesto"
 * necesita un `Jugador.ultimoCambioResidenciaEn` y jugadores reales (§13b, sin implementar). Añadir cuando
 * muerda de verdad — `CIUDADANIA.cooldownCambioResidenciaDias` está reservado en el doc.
 */
export function cambiarResidencia(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  destinoId: string,
  heroeId: string
): { origen: Asentamiento; destino: Asentamiento } {
  const destino = asentamientos.find((a) => a.id === destinoId);
  if (!destino) throw new FaccionInvalidaError('El asentamiento de destino no existe.');
  const faccion = facciones.find((f) => f.id === destino.faccionId);
  if (!faccion || !esCiudadano(faccion, heroeId)) {
    throw new FaccionInvalidaError('Solo se reside en un asentamiento de la propia Facción.');
  }
  const origen = asentamientos.find((a) => a.id !== destinoId && esResidente(a, heroeId));
  if (!origen) {
    throw new FaccionInvalidaError('El jugador no reside en ningún asentamiento: usa comprarCasa, no cambiarResidencia.');
  }
  if (destino.casasCompradas.includes(heroeId) || destino.heroesFundadoresIds.includes(heroeId)) {
    throw new FaccionInvalidaError('El jugador ya reside en el destino.');
  }
  if (destino.vetadosIds?.includes(heroeId) || destino.politicaDeAcceso === 'cerrado') {
    throw new FaccionInvalidaError('El asentamiento de destino no admite nuevos residentes ahora mismo.');
  }
  if (destino.casasCompradas.length + destino.heroesFundadoresIds.length >= capacidadCasas(destino)) {
    throw new FaccionInvalidaError('No quedan espacios de vivienda en el destino.');
  }

  const cargosOrigen = { ...origen.cargos };
  for (const campo of Object.values(CAMPO_CARGO)) {
    if (cargosOrigen[campo] === heroeId) cargosOrigen[campo] = null;
  }
  return {
    origen: {
      ...origen,
      heroesFundadoresIds: origen.heroesFundadoresIds.filter((id) => id !== heroeId),
      casasCompradas: origen.casasCompradas.filter((id) => id !== heroeId),
      cargos: cargosOrigen,
    },
    destino: { ...destino, casasCompradas: [...destino.casasCompradas, heroeId] },
  };
}
