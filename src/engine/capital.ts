// La capital de una Facción (Doc 2.2): designarla. Qué asentamiento ES la capital lo decide `encontrarCapital`
// (`mantenimiento.ts`); aquí solo la regla de cambiarla. Quién puede es autorización (`session/comandos/autorizacion.ts`).
import { CAPITAL } from '../constants';
import type { Asentamiento, Faccion } from '../domain/types';
import { dias, sumar } from '../domain/tiempo';
import type { Instante } from '../domain/tiempo';
import { ReglaInvalidaError } from './errores';
import { capitalDesignada, encontrarCapital } from './mantenimiento';

export class CapitalInvalidaError extends ReglaInvalidaError {}

/**
 * El Rey designa su capital: un asentamiento propio con Palacio activo. Trasladarla tiene un cooldown, para que
 * no se reubique tras cada conquista y el factor de distancia del mantenimiento siga mordiendo. El cooldown no
 * cuenta si la capital designada se perdió (no hay a qué ser fiel): se puede designar otra al instante.
 */
export function designarCapital(
  faccion: Faccion,
  asentamientos: readonly Asentamiento[],
  asentamientoId: string,
  instante: Instante
): { faccion: Faccion; asentamientos: Asentamiento[] } {
  const destino = asentamientos.find((a) => a.id === asentamientoId);
  if (!destino || destino.faccionId !== faccion.id) throw new CapitalInvalidaError('La capital tiene que ser un asentamiento de la propia Facción.');
  if (!destino.edificios.some((e) => e.tipo === 'palacio' && e.estado === 'activo')) {
    throw new CapitalInvalidaError('La capital necesita un Palacio construido.');
  }
  if (encontrarCapital(faccion.id, [...asentamientos])?.id === destino.id) throw new CapitalInvalidaError('Ese asentamiento ya es la capital.');
  if (faccion.capitalDesignadaEn !== undefined && capitalDesignada(faccion.id, asentamientos)) {
    const libreEn = sumar(faccion.capitalDesignadaEn, dias(CAPITAL.cooldownDias));
    if (instante < libreEn) throw new CapitalInvalidaError(`La capital se trasladó hace poco: no se puede volver a cambiar hasta ${libreEn}.`);
  }

  return {
    faccion: { ...faccion, capitalDesignadaEn: instante },
    asentamientos: asentamientos.map((a) => {
      if (a.id === destino.id) return { ...a, capitalDeFaccionId: faccion.id };
      return a.capitalDeFaccionId === faccion.id ? { ...a, capitalDeFaccionId: undefined } : a;
    }),
  };
}
