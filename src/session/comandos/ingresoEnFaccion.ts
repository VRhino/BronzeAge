// Entrar en una Facción existente (D5, D6, D31, D46): el héroe pide el ingreso y el Rey acepta o deniega desde su lista de
// solicitantes. No hay otra puerta a la ciudadanía que esta, crear la Facción o fundar con ella (D31: sin compra de casa), y en
// una Facción NPC no se entra (D49). Aceptado, solo es ciudadano: para vivir en una plaza de la Facción se muda con
// `cambiarResidencia`.
import { esCiudadano, responderSolicitud as responderEngine, solicitarIngreso as solicitarEngine } from '../../engine/faccion';
import { comando, conExploracionFundida, conFaccionEnSuColumna, conFaccion, exigirFaccion, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { exito } from './tipos';
import { evento } from './eventos';

export interface PayloadSolicitudIngreso {
  faccionId: string;
  heroeId: string;
}

export interface PayloadSolicitudRespondida extends PayloadSolicitudIngreso {
  aceptada: boolean;
}

export interface ParamsSolicitarIngreso {
  faccionId: string;
}

export interface ParamsResponderSolicitud {
  faccionId: string;
  heroeId: string;
  aceptar: boolean;
}

/** Pedir el ingreso: el actor entra en la lista de solicitantes de la Facción. */
export const solicitarIngreso = comando<ParamsSolicitarIngreso, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);
  if (estado.facciones.some((f) => esCiudadano(f, ctx.actor))) rechazar(CODIGOS_ERROR.faccionYaPerteneces);
  const actualizada = solicitarEngine(faccion, estado.facciones, ctx.actor);
  return exito(conFaccion(estado, actualizada), [
    evento(ctx, {
      codigo: 'faccion.solicitud_ingreso',
      mensaje: `${ctx.actor} pide entrar en ${faccion.nombre}.`,
      payload: { faccionId: faccion.id, heroeId: ctx.actor } satisfies PayloadSolicitudIngreso,
    }),
  ]);
});

/** El Rey acepta o deniega una solicitud (D46). Aceptada, lo que el héroe anduvo sin bandera pasa a la Facción (Doc 1.3). */
export const responderSolicitud = comando<ParamsResponderSolicitud, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);
  const facciones = responderEngine(estado.facciones, faccion.id, params.heroeId, params.aceptar);
  const conRespuesta = { ...estado, facciones };
  return exito(params.aceptar ? conFaccionEnSuColumna(conExploracionFundida(conRespuesta, params.heroeId, faccion.id), params.heroeId) : conRespuesta, [
    evento(ctx, {
      codigo: params.aceptar ? 'faccion.ciudadania_union' : 'faccion.solicitud_denegada',
      mensaje: params.aceptar ? `${params.heroeId} entra en ${faccion.nombre}.` : `${faccion.nombre} deniega la entrada a ${params.heroeId}.`,
      payload: { faccionId: faccion.id, heroeId: params.heroeId, aceptada: params.aceptar } satisfies PayloadSolicitudRespondida,
    }),
  ]);
});
