import type { Asentamiento, Faccion } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { ReglaInvalidaError } from './errores';

export class FusionInvalidaError extends ReglaInvalidaError {}

/** Fase A5 — payload de la fusión (la anexión vive en `anexion.ts`). El evento hace DESAPARECER una Facción, así que un consumidor
 * necesita los ids para actualizar lo que tuviera cacheado, no solo el texto. */
export interface PayloadFusion {
  faccionAId: string;
  faccionBId: string;
  faccionNuevaId: string;
  nuevoReyId: string;
}

function requerirFacciones(facciones: Faccion[], aId: string, bId: string): [Faccion, Faccion] {
  if (aId === bId) throw new FusionInvalidaError('Una Facción no puede fusionarse consigo misma.');
  const a = facciones.find((f) => f.id === aId);
  const b = facciones.find((f) => f.id === bId);
  if (!a || !b) throw new FusionInvalidaError('Alguna de las Facciones no existe.');
  return [a, b];
}

function unionCiudadanos(a: Faccion, b: Faccion): string[] {
  return [...new Set([...a.ciudadanosIds, ...b.ciudadanosIds])];
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
): { facciones: Faccion[]; asentamientos: Asentamiento[]; eventos: EventoCrudo[] } {
  const [a, b] = requerirFacciones(facciones, faccionAId, faccionBId);
  const ciudadanosC = unionCiudadanos(a, b);
  if (!ciudadanosC.includes(nuevoReyId)) {
    throw new FusionInvalidaError('El nuevo Rey debe ser ciudadano de alguna de las dos Facciones fusionadas.');
  }

  const nuevaFaccion: Faccion = {
    id: `faccion-fusion-${tickActual}-${faccionAId}-${faccionBId}`,
    nombre: nuevoNombre,
    // El sigilo es para siempre y no se cambia (Doc 2.8.1): la fusión hereda el de A, que desaparece con ella, así que
    // no choca con ninguna otra Facción; el de B se pierde con B.
    sigilo: a.sigilo,
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
    eventos: [
      {
        codigo: 'diplomacia.fusion',
        mensaje: `${a.nombre} y ${b.nombre} se fusionan en ${nuevoNombre}.`,
        payload: {
          faccionAId: a.id,
          faccionBId: b.id,
          faccionNuevaId: nuevaFaccion.id,
          nuevoReyId,
        } satisfies PayloadFusion,
      },
    ],
  };
}
