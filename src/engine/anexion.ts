// Anexión (Doc 2.6, opción 1; decidido el 2026-10-06): A absorbe a B SOLO si el Rey de B lo acepta. La propone el Rey o el Embajador de A
// (`PropuestaAnexion`, pendiente y con caducidad) y la contesta el Rey de B. Aquí viven las reglas: quién puede proponer a quién y qué pasa
// con todo lo que colgaba de B (el traslado en sí, compartido con la fusión, vive en `trasladoDeFaccion.ts`). Quién es quién lo comprueba la capa de comandos (`session/comandos/anexion.ts`).
import { ANEXION } from '../constants';
import { sumar, dias, type Instante } from '../domain/tiempo';
import type { Faccion, PropuestaAnexion, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { ReglaInvalidaError } from './errores';
import { propuestasVigentes, senorAjeno, trasladarFaccion, type MundoDeFacciones } from './trasladoDeFaccion';

export class AnexionInvalidaError extends ReglaInvalidaError {}

/** Payloads de este subsistema: la anexión hace DESAPARECER una Facción, así que un consumidor necesita los ids. */
export interface PayloadAnexionPropuesta {
  propuestaId: string;
  absorbenteId: string;
  absorbidaId: string;
}
export interface PayloadAnexionRespondida extends PayloadAnexionPropuesta {
  aceptada: boolean;
}
export interface PayloadAnexion {
  faccionAbsorbenteId: string;
  faccionAbsorbidaId: string;
}

/** Lo que impide que A absorba a B ahora mismo, o nada. Se mira al proponer y otra vez al aceptar: el mundo cambia mientras espera. */
export function exigirAnexionPosible(facciones: readonly Faccion[], relaciones: readonly RelacionPolitica[], absorbenteId: string, absorbidaId: string): void {
  if (absorbenteId === absorbidaId) throw new AnexionInvalidaError('Una Facción no puede anexionarse a sí misma.');
  const absorbida = facciones.find((f) => f.id === absorbidaId);
  if (!facciones.some((f) => f.id === absorbenteId) || !absorbida) throw new AnexionInvalidaError('Alguna de las Facciones no existe.');
  if (absorbida.reyId === null) throw new AnexionInvalidaError(`${absorbida.nombre} no tiene Rey que pueda aceptar.`);
  if (senorAjeno(relaciones, absorbidaId, absorbenteId)) {
    throw new AnexionInvalidaError(`${absorbida.nombre} es vasalla de otra Facción: su señor tiene que liberarla antes.`);
  }
}

/** Una propuesta nueva de `absorbenteId` a `absorbidaId`, o el motivo por el que no cabe. Una sola entre cada par de Facciones, en cualquier sentido. */
export function proponerAnexion(
  facciones: readonly Faccion[],
  relaciones: readonly RelacionPolitica[],
  propuestas: readonly PropuestaAnexion[] | undefined,
  absorbenteId: string,
  absorbidaId: string,
  propuestaPor: string,
  ahora: Instante,
  id: string
): PropuestaAnexion {
  exigirAnexionPosible(facciones, relaciones, absorbenteId, absorbidaId);
  const yaPropuesta = propuestasVigentes(propuestas, ahora).some(
    (p) => (p.absorbenteId === absorbenteId && p.absorbidaId === absorbidaId) || (p.absorbenteId === absorbidaId && p.absorbidaId === absorbenteId)
  );
  if (yaPropuesta) throw new AnexionInvalidaError('Ya hay una propuesta de anexión pendiente entre estas Facciones.');
  return { id, absorbenteId, absorbidaId, propuestaPor, creadaEn: ahora, expiraEn: sumar(ahora, dias(ANEXION.caducidadDias)) };
}

/**
 * Ejecuta la anexión ya aceptada. A conserva nombre, Rey, Embajador, sigilo, reputación, experiencia y tecnología; de B pasa a A todo lo
 * demás (`trasladarFaccion`). Los cargos de Facción de B, su sigilo, su reputación y su tecnología desaparecen con ella.
 */
export function anexionar(mundo: MundoDeFacciones, absorbenteId: string, absorbidaId: string): { mundo: MundoDeFacciones; eventos: EventoCrudo[] } {
  const a = mundo.facciones.find((f) => f.id === absorbenteId);
  const b = mundo.facciones.find((f) => f.id === absorbidaId);
  if (!a || !b || a === b) throw new AnexionInvalidaError('Alguna de las Facciones no existe.');
  return {
    mundo: trasladarFaccion(mundo, b.id, a.id),
    eventos: [
      {
        codigo: 'diplomacia.anexion',
        mensaje: `${a.nombre} anexiona a ${b.nombre}.`,
        payload: { faccionAbsorbenteId: a.id, faccionAbsorbidaId: b.id } satisfies PayloadAnexion,
      },
    ],
  };
}
