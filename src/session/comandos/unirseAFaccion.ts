// Unirse a una Facción EXISTENTE sin pasar por `comprarCasa` (a petición del usuario, 2026-08-27). A
// diferencia de comprar casa, no consume cupo de vivienda ni ata al jugador a un asentamiento concreto: solo
// otorga ciudadanía. Las dos vías conviven — `comprarCasa` sigue siendo la única que además da residencia.
import { esCiudadano, otorgarCiudadania } from '../../engine/faccion';
import { comando, conFaccion, exigirFaccion, rechazar } from './ayudas';
import { exito, sinCambios } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionUnion {
  faccionId: string;
  jugadorId: string;
}

export interface ParamsUnirseAFaccion {
  faccionId: string;
}

export const unirseAFaccion = comando<ParamsUnirseAFaccion, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);

  // Ya es ciudadano de ESTA: idempotente, no un rechazo (mismo criterio que `alternarFaccionNpc` — pedir el
  // estado en el que ya se está es seguro ante un reintento por reconexión, doc 2 punto 10).
  if (esCiudadano(faccion, ctx.actor)) return sinCambios(estado);

  if (estado.facciones.some((f) => f.id !== faccion.id && esCiudadano(f, ctx.actor))) {
    rechazar(CODIGOS_ERROR.faccionYaPerteneces);
  }

  const actualizada = otorgarCiudadania(faccion, ctx.actor);
  const siguiente = conFaccion(estado, actualizada);
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'faccion.ciudadania_union',
      mensaje: `${ctx.actor} se une a ${faccion.nombre}.`,
      payload: { faccionId: faccion.id, jugadorId: ctx.actor } satisfies PayloadFaccionUnion,
    }),
  ]);
});
