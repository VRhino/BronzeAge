// Abandonar la propia Facción (a petición del usuario, 2026-08-27). Sin parámetros: el actor solo puede dejar
// SU PROPIA Facción, nunca la de otro (no hay `jugadorId` en `params` que falsear).
//
// Estampa `salidasFaccionPorJugador` para que `crearFaccion` pueda aplicar el cooldown anti-abuso — es el
// ÚNICO comando que lo escribe.
//
// LIMITACIÓN DOCUMENTADA, no un olvido: no toca residencia (`Asentamiento.casasCompradas`/
// `jugadoresFundadoresIds`) ni cargos LOCALES (Gobernador, etc.) en asentamientos de la Facción abandonada —
// Doc 2.5 no define qué pasa con la vivienda al abandonar, y no existe todavía un comando "dejar
// residencia"/"vender casa" que lo resuelva. Un jugador puede quedar sin ciudadanía y seguir figurando como
// residente/cargo local de un asentamiento de la Facción que dejó. `quitarCiudadania` (`engine/faccion.ts`) sí
// libera Rey/Embajador (cargos de FACCIÓN, no locales) — ver el comentario ahí.
import { esCiudadano, quitarCiudadania } from '../../engine/faccion';
import { comando, conFaccion, rechazar } from './ayudas';
import { exito } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionAbandonada {
  faccionId: string;
  jugadorId: string;
}

export type ParamsDejarFaccion = Record<string, never>;

export const dejarFaccion = comando<ParamsDejarFaccion, void>((estado, _mapa, ctx, _params) => {
  const faccion = estado.facciones.find((f) => esCiudadano(f, ctx.actor));
  if (!faccion) rechazar(CODIGOS_ERROR.faccionNoPerteneces);

  const actualizada = quitarCiudadania(faccion, ctx.actor);
  const siguiente = {
    ...conFaccion(estado, actualizada),
    salidasFaccionPorJugador: { ...estado.salidasFaccionPorJugador, [ctx.actor]: ctx.momento },
  };
  return exito(siguiente, [
    evento(ctx, estado, {
      codigo: 'faccion.abandonada',
      mensaje: `${ctx.actor} abandona ${faccion.nombre}.`,
      payload: { faccionId: faccion.id, jugadorId: ctx.actor } satisfies PayloadFaccionAbandonada,
    }),
  ]);
});
