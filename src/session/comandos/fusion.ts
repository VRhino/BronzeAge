// Fusión entre Facciones (Doc 2.6, decidido el 2026-10-06): la propone el Rey de A fijando el nombre y el Rey de la Facción nueva, y solo la
// ejecuta el Rey de B al aceptarla. Pendiente en `estado.propuestasFusion` hasta que se contesta, se retira o caduca. Reglas: `engine/fusion.ts`.
import {
  exigirFusionPosible,
  FusionInvalidaError,
  fusionar,
  proponerFusion as proponerFusionEngine,
  type PayloadFusionPropuesta,
  type PayloadFusionRespondida,
} from '../../engine/fusion';
import { propuestasVigentes } from '../../engine/trasladoDeFaccion';
import type { Instante } from '../../domain/tiempo';
import type { PropuestaFusion } from '../../domain/types';
import { faccionesConBatallaAbierta } from '../batallas';
import type { GameSessionState } from '../estado';
import { exigirFaccion, comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { desdeCrudos, evento } from './eventos';
import { exito } from './tipos';

export interface ParamsProponerFusion {
  /** La que propone: su Rey la hace. */
  faccionAId: string;
  faccionBId: string;
  nuevoNombre: string;
  /** El Rey de la Facción nueva: el Rey de A o el de B. */
  nuevoReyId: string;
}

export interface ParamsResponderFusion {
  propuestaId: string;
  aceptar: boolean;
}

export interface ParamsRetirarFusion {
  propuestaId: string;
}

function pendiente(estado: GameSessionState, propuestaId: string, ahora: Instante): PropuestaFusion {
  const propuesta = (estado.propuestasFusion ?? []).find((p) => p.id === propuestaId);
  if (!propuesta) rechazar(CODIGOS_ERROR.fusionNoExiste);
  if (propuestasVigentes([propuesta], ahora).length === 0) rechazar(CODIGOS_ERROR.fusionCaducada);
  return propuesta;
}

const carga = (p: PropuestaFusion): PayloadFusionPropuesta => ({ propuestaId: p.id, faccionAId: p.faccionAId, faccionBId: p.faccionBId });
const sin = (e: GameSessionState, propuestaId: string): GameSessionState => ({ ...e, propuestasFusion: (e.propuestasFusion ?? []).filter((p) => p.id !== propuestaId) });

export const proponerFusion = comando<ParamsProponerFusion, { propuestaId: string }>((estado, _mapa, ctx, params) => {
  const a = exigirFaccion(estado, params.faccionAId);
  const b = exigirFaccion(estado, params.faccionBId);
  const nueva = proponerFusionEngine(
    estado.facciones,
    estado.relaciones,
    estado.propuestasFusion,
    params.faccionAId,
    params.faccionBId,
    params.nuevoNombre,
    params.nuevoReyId,
    ctx.instante,
    `fusion-${params.faccionAId}-${params.faccionBId}-${ctx.ids.siguiente()}`
  );
  return exito(
    { ...estado, propuestasFusion: [...propuestasVigentes(estado.propuestasFusion, ctx.instante), nueva] },
    [
      evento(ctx, {
        codigo: 'diplomacia.fusion_propuesta',
        mensaje: `${a.nombre} propone fusionarse con ${b.nombre} en ${nueva.nuevoNombre}.`,
        payload: carga(nueva) satisfies PayloadFusionPropuesta,
      }),
    ],
    { propuestaId: nueva.id }
  );
});

/** El Rey de B acepta (la fusión se ejecuta en el acto) o rechaza. */
export const responderFusion = comando<ParamsResponderFusion, void>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);

  if (!params.aceptar) {
    const a = exigirFaccion(estado, propuesta.faccionAId);
    const b = exigirFaccion(estado, propuesta.faccionBId);
    return exito(sin(estado, propuesta.id), [
      evento(ctx, {
        codigo: 'diplomacia.fusion_rechazada',
        mensaje: `${b.nombre} rechaza fusionarse con ${a.nombre}.`,
        payload: { ...carga(propuesta), aceptada: false } satisfies PayloadFusionRespondida,
      }),
    ]);
  }

  // El mundo cambió mientras esperaba: lo que valía al proponer se vuelve a mirar. Si el Rey de A ya no es quien la hizo, su consentimiento no vale.
  const { a } = exigirFusionPosible(estado.facciones, estado.relaciones, propuesta.faccionAId, propuesta.faccionBId, propuesta.nuevoNombre, propuesta.nuevoReyId);
  if (a.reyId !== propuesta.propuestaPor) throw new FusionInvalidaError(`${a.nombre} ya no tiene el Rey que la propuso.`);
  if (faccionesConBatallaAbierta(estado, [propuesta.faccionAId, propuesta.faccionBId], ctx.instante)) {
    throw new FusionInvalidaError('Alguna de las dos Facciones tiene una batalla en curso: espera a que termine.');
  }

  const resultado = fusionar(estado, propuesta.faccionAId, propuesta.faccionBId, propuesta.nuevoNombre, propuesta.nuevoReyId, ctx.instante);
  return exito({ ...estado, ...resultado.mundo }, desdeCrudos(ctx, resultado.eventos));
});

/** El Rey de A retira su propuesta mientras nadie la ha contestado. */
export const retirarFusion = comando<ParamsRetirarFusion, void>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);
  const a = exigirFaccion(estado, propuesta.faccionAId);
  const b = exigirFaccion(estado, propuesta.faccionBId);
  return exito(sin(estado, propuesta.id), [
    evento(ctx, {
      codigo: 'diplomacia.fusion_retirada',
      mensaje: `${a.nombre} retira su propuesta de fusión con ${b.nombre}.`,
      payload: carga(propuesta) satisfies PayloadFusionPropuesta,
    }),
  ]);
});
