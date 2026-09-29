import type { EstadoTecnologia, TecnologiaId, TecnologiasFaccion } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { TECNOLOGIAS } from '../constants';

/** Las tecnologías con las que nace toda Facción (Doc 6.2). */
export const TECNOLOGIAS_DE_ARRANQUE: readonly TecnologiaId[] = (Object.keys(TECNOLOGIAS) as TecnologiaId[]).filter(
  (id) => TECNOLOGIAS[id].deArranque
);

/** El servidor arranca en la Era I, sin logros (Doc 6.2). */
export function estadoTecnologiaInicial(desde: Instante): EstadoTecnologia {
  return { era: 'reinos_palaciales', eraDesde: desde, contadores: {}, logros: {}, primeros: {}, porFaccion: {} };
}

/** Tecnologías de una Facción. Una Facción que aún no tiene entrada solo tiene las de arranque. */
export function tecnologiasDe(estado: EstadoTecnologia, faccionId: string): TecnologiasFaccion {
  return estado.porFaccion[faccionId] ?? { aparecidas: [...TECNOLOGIAS_DE_ARRANQUE], adoptadas: [...TECNOLOGIAS_DE_ARRANQUE] };
}

export function tieneTecnologia(estado: EstadoTecnologia, faccionId: string, id: TecnologiaId): boolean {
  return tecnologiasDe(estado, faccionId).adoptadas.includes(id);
}
