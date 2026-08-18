import type { Asentamiento, Faccion } from '../domain/types';

export class FusionInvalidaError extends Error {}

function requerirFacciones(facciones: Faccion[], aId: string, bId: string): [Faccion, Faccion] {
  if (aId === bId) throw new FusionInvalidaError('Una Facción no puede fusionarse/anexionar consigo misma.');
  const a = facciones.find((f) => f.id === aId);
  const b = facciones.find((f) => f.id === bId);
  if (!a || !b) throw new FusionInvalidaError('Alguna de las Facciones no existe.');
  return [a, b];
}

function unionCiudadanos(a: Faccion, b: Faccion): string[] {
  return [...new Set([...a.ciudadanosIds, ...b.ciudadanosIds])];
}

/**
 * Anexión (Doc 2.6, opción 1): A absorbe a B. A mantiene nombre/Rey/Embajador. Los cargos de Facción de B se
 * disuelven (B desaparece); los cargos LOCALES de los asentamientos de B se mantienen intactos (no se tocan).
 */
export function anexionar(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  faccionAId: string,
  faccionBId: string
): { facciones: Faccion[]; asentamientos: Asentamiento[]; eventos: string[] } {
  const [a, b] = requerirFacciones(facciones, faccionAId, faccionBId);
  const aFusionada: Faccion = { ...a, ciudadanosIds: unionCiudadanos(a, b) };

  return {
    facciones: facciones.filter((f) => f.id !== faccionBId).map((f) => (f.id === faccionAId ? aFusionada : f)),
    asentamientos: asentamientos.map((asent) => (asent.faccionId === faccionBId ? { ...asent, faccionId: faccionAId } : asent)),
    eventos: [`${a.nombre} anexiona a ${b.nombre}.`],
  };
}

/**
 * Fusión (Doc 2.6, opción 2): A y B se disuelven, nace C. Se "vota" Rey de C (Fase 0: sin votación real
 * interactiva, el llamador indica el Rey resultante). Cargos de Facción anteriores se disuelven y re-designan
 * (Embajador de C queda vacante hasta que el nuevo Rey lo designe); cargos locales se mantienen intactos.
 */
export function fusionar(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  faccionAId: string,
  faccionBId: string,
  nuevoNombre: string,
  nuevoReyId: string,
  tickActual: number
): { facciones: Faccion[]; asentamientos: Asentamiento[]; eventos: string[] } {
  const [a, b] = requerirFacciones(facciones, faccionAId, faccionBId);
  const ciudadanosC = unionCiudadanos(a, b);
  if (!ciudadanosC.includes(nuevoReyId)) {
    throw new FusionInvalidaError('El nuevo Rey debe ser ciudadano de alguna de las dos Facciones fusionadas.');
  }

  const nuevaFaccion: Faccion = {
    id: `faccion-fusion-${tickActual}-${faccionAId}-${faccionBId}`,
    nombre: nuevoNombre,
    reyId: nuevoReyId,
    embajadorId: null,
    nivel: 1,
    // Igual criterio que reputación (Doc 2.7, ver abajo): la fusión da a luz una Facción nueva, sin
    // historial propio de actividad todavía (Doc Fase_0_5 §8) — no hereda la XP de ninguna de las dos.
    experiencia: 0,
    ciudadanosIds: ciudadanosC,
    // Reputación (Doc 2.7) arranca en 0: la fusión da a luz una Facción nueva, sin historial propio todavía.
    reputacion: 0,
  };

  return {
    facciones: [...facciones.filter((f) => f.id !== faccionAId && f.id !== faccionBId), nuevaFaccion],
    asentamientos: asentamientos.map((asent) =>
      asent.faccionId === faccionAId || asent.faccionId === faccionBId ? { ...asent, faccionId: nuevaFaccion.id } : asent
    ),
    eventos: [`${a.nombre} y ${b.nombre} se fusionan en ${nuevoNombre}.`],
  };
}
