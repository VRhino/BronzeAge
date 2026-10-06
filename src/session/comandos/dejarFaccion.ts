// Abandonar la propia Facción (a petición del usuario, 2026-08-27). Sin parámetros: el actor solo puede dejar
// SU PROPIA Facción, nunca la de otro (no hay `heroeId` en `params` que falsear).
//
// Estampa `salidasFaccionPorHeroe` para que `crearFaccion` pueda aplicar el cooldown anti-abuso — es el
// ÚNICO comando que lo escribe.
//
// Al irse también deja su residencia y sus cargos LOCALES (Doc 2.5, 2026-10-02): `dejarResidencia`. `quitarCiudadania`
// (`engine/faccion.ts`) libera Rey/Embajador, los cargos de FACCIÓN. Abandonar no cuenta para el cooldown de residencia.
import { dejarResidencia, esCiudadano, quitarCiudadania } from '../../engine/faccion';
import { esResidente } from '../../engine/pertenencia';
import { sinGuarnicion } from '../../engine/tropa';
import { comando, conAsentamiento, conFaccion, conFaccionEnSuColumna, rechazar } from './ayudas';
import { exito } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionAbandonada {
  faccionId: string;
  heroeId: string;
}

export type ParamsDejarFaccion = Record<string, never>;

export const dejarFaccion = comando<ParamsDejarFaccion, void>((estado, _mapa, ctx, _params) => {
  const faccion = estado.facciones.find((f) => esCiudadano(f, ctx.actor));
  if (!faccion) rechazar(CODIGOS_ERROR.faccionNoPerteneces);

  const actualizada = quitarCiudadania(faccion, ctx.actor);
  // Se va con su casa: sin ciudadanía no se reside ni se gobierna (Doc 2.5, 2026-10-02). Sigue siendo suyo lo que lleva.
  const residencia = estado.asentamientos.some((a) => esResidente(a, ctx.actor)) ? dejarResidencia(estado.asentamientos, ctx.actor) : undefined;
  const sinCasa = residencia ? { ...conAsentamiento(estado, residencia), heroes: sinGuarnicion(estado.heroes, ctx.actor) } : estado;
  const siguiente = conFaccionEnSuColumna({
    ...conFaccion(sinCasa, actualizada),
    salidasFaccionPorHeroe: { ...estado.salidasFaccionPorHeroe, [ctx.actor]: ctx.instante },
  }, ctx.actor);
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'faccion.abandonada',
      mensaje: `${ctx.actor} abandona ${faccion.nombre}.`,
      payload: { faccionId: faccion.id, heroeId: ctx.actor } satisfies PayloadFaccionAbandonada,
    }),
  ]);
});
