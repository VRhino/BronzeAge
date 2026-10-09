// Comandos de diplomacia entre Facciones: proponer vasallaje/alianza, romper una relación, rebelarse contra
// un señor (la anexión y la fusión, que piden aceptación, viven en `anexion.ts` y `fusion.ts`). Agrupados por la misma razón que `cargos.ts`: comparten forma y audiencia.
//
// La rebelión la narra el MOTOR (`engine/diplomacia.ts`), que ya
// emite `EventoCrudo` con código y payload; aquí solo se les añade el contexto temporal. Proponer y romper los
// narra esta capa, que es donde se sabe qué relación se creó.
import type { Instante } from '../../domain/tiempo';
import type { PropuestaRelacion, RecursoTipo } from '../../domain/types';
import {
  DiplomaciaInvalidaError,
  exigirRelacionPosible,
  proponerRelacionPendiente,
  relacionDePropuesta,
  type PayloadRelacionPropuestaPendiente,
  type PayloadRelacionRespondida,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
  declararGuerra as declararGuerraEngine,
  proponerPaz as proponerPazEngine,
  type PayloadGuerraDeclarada,
} from '../../engine/diplomacia';
import { propuestasVigentes } from '../../engine/trasladoDeFaccion';
import { faccionesConBatallaAbierta } from '../batallas';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, exigirFaccion, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { desdeCrudos, evento } from './eventos';

export interface PayloadRelacionRota {
  relacionId: string;
  iniciadorFaccionId: string;
}

export interface ParamsProponerRelacion {
  tipo: 'vasallaje' | 'alianza';
  /** Quien propone: en un vasallaje, la señora. La otra tiene que aceptar (2026-10-09). */
  faccionAId: string;
  faccionBId: string;
  /** Solo para vasallaje; ignorado en alianzas. */
  tributoRecurso?: RecursoTipo;
  tributoCantidad?: number;
}

export interface ParamsResponderRelacion {
  propuestaId: string;
  aceptar: boolean;
}

export interface ParamsRetirarRelacion {
  propuestaId: string;
}

function pendiente(estado: GameSessionState, propuestaId: string, ahora: Instante): PropuestaRelacion {
  const propuesta = (estado.propuestasRelacion ?? []).find((p) => p.id === propuestaId);
  if (!propuesta) rechazar(CODIGOS_ERROR.relacionPropuestaNoExiste);
  if (propuestasVigentes([propuesta], ahora).length === 0) rechazar(CODIGOS_ERROR.relacionPropuestaCaducada);
  return propuesta;
}

const carga = (p: PropuestaRelacion): PayloadRelacionPropuestaPendiente => ({ propuestaId: p.id, tipo: p.tipo, faccionAId: p.faccionAId, faccionBId: p.faccionBId });
const sin = (e: GameSessionState, propuestaId: string): GameSessionState => ({ ...e, propuestasRelacion: (e.propuestasRelacion ?? []).filter((p) => p.id !== propuestaId) });

/** Alianza o vasallaje (Doc 2.3 y 2.4): deja una propuesta pendiente que solo el Rey de la otra Facción acepta (2026-10-09), como la anexión. */
export const proponerRelacion = comando<ParamsProponerRelacion, { propuestaId: string }>((estado, _mapa, ctx, params) => {
  const a = exigirFaccion(estado, params.faccionAId);
  const b = exigirFaccion(estado, params.faccionBId);
  const nueva = proponerRelacionPendiente(
    estado.facciones,
    estado.relaciones,
    estado.propuestasRelacion,
    params.tipo,
    params.faccionAId,
    params.faccionBId,
    params.tipo === 'vasallaje' ? { recurso: params.tributoRecurso ?? 'trigo', cantidadPorMinuto: params.tributoCantidad ?? 0 } : undefined,
    ctx.actor,
    ctx.instante,
    `relacion-${params.faccionAId}-${params.faccionBId}-${ctx.ids.siguiente()}`
  );
  return exito(
    { ...estado, propuestasRelacion: [...propuestasVigentes(estado.propuestasRelacion, ctx.instante), nueva] },
    [
      evento(ctx, {
        codigo: 'diplomacia.relacion_propuesta',
        mensaje: `${a.nombre} propone ${params.tipo === 'alianza' ? 'una alianza' : 'un vasallaje'} a ${b.nombre}.`,
        payload: { ...carga(nueva), expiraEn: nueva.expiraEn },
      }),
    ],
    { propuestaId: nueva.id }
  );
});

/** El Rey de la otra Facción acepta (la relación nace en el acto) o rechaza. Al aceptar se revalida todo: el mundo cambia mientras espera. */
export const responderRelacion = comando<ParamsResponderRelacion, { relacionId?: string }>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);
  const a = exigirFaccion(estado, propuesta.faccionAId);
  const b = exigirFaccion(estado, propuesta.faccionBId);

  if (!params.aceptar) {
    return exito(sin(estado, propuesta.id), [
      evento(ctx, {
        codigo: 'diplomacia.relacion_rechazada',
        mensaje: `${b.nombre} rechaza ${propuesta.tipo === 'alianza' ? 'la alianza' : 'el vasallaje'} de ${a.nombre}.`,
        payload: { ...carga(propuesta), aceptada: false } satisfies PayloadRelacionRespondida,
      }),
    ]);
  }

  exigirRelacionPosible(estado.facciones, estado.relaciones, propuesta.tipo, propuesta.faccionAId, propuesta.faccionBId);
  if (faccionesConBatallaAbierta(estado, [propuesta.faccionAId, propuesta.faccionBId], ctx.instante)) {
    throw new DiplomaciaInvalidaError('Alguna de las dos Facciones tiene una batalla en curso: espera a que termine.');
  }
  const nueva = relacionDePropuesta(estado.facciones, estado.relaciones, propuesta, ctx.instante, ctx.ids.siguiente());
  return exito(
    { ...sin(estado, propuesta.id), relaciones: [...estado.relaciones, nueva] },
    [
      evento(ctx, {
        codigo: 'diplomacia.relacion_aceptada',
        mensaje: `${b.nombre} acepta ${propuesta.tipo === 'alianza' ? 'la alianza' : 'el vasallaje'} de ${a.nombre}.`,
        payload: { ...carga(propuesta), aceptada: true, relacionId: nueva.id } satisfies PayloadRelacionRespondida,
      }),
    ],
    { relacionId: nueva.id }
  );
});

/** Quien propuso la retira mientras nadie la ha contestado. */
export const retirarRelacion = comando<ParamsRetirarRelacion, void>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);
  return exito(sin(estado, propuesta.id), [
    evento(ctx, {
      codigo: 'diplomacia.relacion_retirada',
      mensaje: `${propuesta.faccionAId} retira su propuesta de ${propuesta.tipo} a ${propuesta.faccionBId}.`,
      payload: carga(propuesta),
    }),
  ]);
});

export interface ParamsRomperRelacion {
  relacionId: string;
  iniciadorFaccionId: string;
}

export const romperRelacion = comando<ParamsRomperRelacion, void>((estado, _mapa, ctx, params) => {
  if (!params.relacionId) rechazar(CODIGOS_ERROR.diplomaciaRelacionNoIndicada);

  const resultado = romperRelacionEngine(estado.facciones, estado.relaciones, params.relacionId, params.iniciadorFaccionId);
  const siguiente: GameSessionState = { ...estado, facciones: resultado.facciones, relaciones: resultado.relaciones };
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'diplomacia.relacion_rota',
      mensaje: `Relación ${params.relacionId} rota voluntariamente.`,
      payload: { relacionId: params.relacionId, iniciadorFaccionId: params.iniciadorFaccionId } satisfies PayloadRelacionRota,
    }),
  ]);
});

export interface ParamsRebelionVasallo {
  relacionId: string;
}

export const rebelionVasallo = comando<ParamsRebelionVasallo, void>((estado, _mapa, ctx, params) => {
  if (!params.relacionId) rechazar(CODIGOS_ERROR.diplomaciaRelacionNoIndicada);

  const resultado = rebelionVasalloEngine(
    estado.facciones,
    estado.relaciones,
    estado.acuerdos,
    estado.asentamientos,
    params.relacionId,
    ctx.instante,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = {
    ...estado,
    facciones: resultado.facciones,
    relaciones: resultado.relaciones,
    acuerdos: resultado.acuerdos,
  };
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos));
});

export interface ParamsDeclararGuerra {
  /** Quien declara. */
  faccionAId: string;
  faccionBId: string;
}

export const declararGuerra = comando<ParamsDeclararGuerra, { relacionIds: string[] }>((estado, _mapa, ctx, params) => {
  const guerras = declararGuerraEngine(estado.facciones, estado.relaciones, params.faccionAId, params.faccionBId, ctx.instante, ctx.ids.siguiente());
  const siguiente: GameSessionState = { ...estado, relaciones: [...estado.relaciones, ...guerras] };
  const relacionIds = guerras.map((g) => g.id);
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'diplomacia.guerra_declarada',
        mensaje: `${params.faccionAId} declara la guerra a ${guerras.map((g) => g.faccionBId).join(', ')}.`,
        payload: { relacionIds, faccionAId: params.faccionAId, faccionesEnemigasIds: guerras.map((g) => g.faccionBId) } satisfies PayloadGuerraDeclarada,
      }),
    ],
    { relacionIds }
  );
});

export interface ParamsProponerPaz {
  relacionId: string;
  /** Quien ofrece la paz: una de las dos Facciones de la guerra. */
  faccionId: string;
}

export interface PayloadPaz {
  relacionId: string;
  faccionId: string;
  firmada: boolean;
}

export const proponerPaz = comando<ParamsProponerPaz, { firmada: boolean }>((estado, _mapa, ctx, params) => {
  if (!params.relacionId) rechazar(CODIGOS_ERROR.diplomaciaRelacionNoIndicada);

  const { relaciones, firmada } = proponerPazEngine(estado.relaciones, params.relacionId, params.faccionId);
  return exito(
    { ...estado, relaciones },
    [
      evento(ctx, {
        codigo: firmada ? 'diplomacia.paz_firmada' : 'diplomacia.paz_propuesta',
        mensaje: firmada ? `La guerra ${params.relacionId} termina en paz.` : `${params.faccionId} ofrece la paz en la guerra ${params.relacionId}.`,
        payload: { relacionId: params.relacionId, faccionId: params.faccionId, firmada } satisfies PayloadPaz,
      }),
    ],
    { firmada }
  );
});
