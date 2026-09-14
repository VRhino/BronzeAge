import type { Asentamiento, CargoTipo, Faccion } from '../domain/types';
import { esCiudadano } from './faccion';
import { cargoOcupado, conCargoLocal } from './pertenencia';

export class CargoInvalidoError extends Error {}

/**
 * Rey (Doc 2.2): por defecto en Liga-por-vasallaje, electo por voto en Liga-por-alianza. Fase 0 no simula
 * una votación real (sin jugadores interactivos) — se modela como designación directa entre ciudadanos.
 */
export function asignarRey(faccion: Faccion, heroeId: string): Faccion {
  if (!esCiudadano(faccion, heroeId)) {
    throw new CargoInvalidoError('Solo un ciudadano de la Facción puede ser Rey.');
  }
  return { ...faccion, reyId: heroeId };
}

/** Embajador (Doc 2.2): designado DIRECTAMENTE por el Rey — requiere que exista un Rey. */
export function asignarEmbajador(faccion: Faccion, heroeId: string): Faccion {
  if (!faccion.reyId) throw new CargoInvalidoError('La Facción necesita un Rey antes de designar Embajador.');
  if (!esCiudadano(faccion, heroeId)) {
    throw new CargoInvalidoError('Solo un ciudadano de la Facción puede ser Embajador.');
  }
  return { ...faccion, embajadorId: heroeId };
}

/**
 * Cargos locales (Doc 2.2): Gobernador se elige entre ciudadanos (Fase 0: designación directa); el resto
 * los designa el Gobernador, por lo que requieren que ya haya un Gobernador en el puesto.
 */
export function asignarCargoLocal(
  asentamiento: Asentamiento,
  faccion: Faccion,
  cargo: CargoTipo,
  heroeId: string
): Asentamiento {
  if (asentamiento.faccionId !== faccion.id) {
    throw new CargoInvalidoError('El asentamiento no pertenece a esta Facción.');
  }
  if (!esCiudadano(faccion, heroeId)) {
    throw new CargoInvalidoError('Solo un ciudadano de la Facción puede ejercer un cargo local.');
  }
  if (cargo !== 'gobernador' && !cargoOcupado(asentamiento, 'gobernador')) {
    throw new CargoInvalidoError('El asentamiento necesita un Gobernador antes de designar el resto de cargos.');
  }

  return conCargoLocal(asentamiento, cargo, heroeId);
}
