// Comandos de diplomacia entre Facciones: proponer vasallaje/alianza, romper una relación, rebelarse contra
// un señor (la anexión y la fusión, que piden aceptación, viven en `anexion.ts` y `fusion.ts`). Agrupados por la misma razón que `cargos.ts`: comparten forma y audiencia.
//
// La rebelión la narra el MOTOR (`engine/diplomacia.ts`), que ya
// emite `EventoCrudo` con código y payload; aquí solo se les añade el contexto temporal. Proponer y romper los
// narra esta capa, que es donde se sabe qué relación se creó.
import type { RecursoTipo } from '../../domain/types';
import {
  proponerVasallaje as proponerVasallajeEngine,
  proponerAlianza as proponerAlianzaEngine,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
  declararGuerra as declararGuerraEngine,
  proponerPaz as proponerPazEngine,
  type PayloadGuerraDeclarada,
} from '../../engine/diplomacia';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { desdeCrudos, evento } from './eventos';

export interface PayloadRelacionPropuesta {
  relacionId: string;
  tipo: 'vasallaje' | 'alianza';
  faccionAId: string;
  faccionBId: string;
}
export interface PayloadRelacionRota {
  relacionId: string;
  iniciadorFaccionId: string;
}

export interface ParamsProponerRelacion {
  tipo: 'vasallaje' | 'alianza';
  faccionAId: string;
  faccionBId: string;
  /** Solo para vasallaje; ignorado en alianzas. */
  tributoRecurso?: RecursoTipo;
  tributoCantidad?: number;
}

export const proponerRelacion = comando<ParamsProponerRelacion, { relacionId: string }>((estado, _mapa, ctx, params) => {
  const nueva =
    params.tipo === 'vasallaje'
      ? proponerVasallajeEngine(
          estado.facciones,
          estado.relaciones,
          params.faccionAId,
          params.faccionBId,
          params.tributoRecurso ?? 'trigo',
          params.tributoCantidad ?? 0,
          ctx.instante,
          ctx.ids.siguiente()
        )
      : proponerAlianzaEngine(estado.facciones, estado.relaciones, params.faccionAId, params.faccionBId, ctx.instante, ctx.ids.siguiente());

  const siguiente: GameSessionState = { ...estado, relaciones: [...estado.relaciones, nueva] };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'diplomacia.relacion_propuesta',
        mensaje: `Relación propuesta: ${nueva.id}.`,
        payload: {
          relacionId: nueva.id,
          tipo: params.tipo,
          faccionAId: params.faccionAId,
          faccionBId: params.faccionBId,
        } satisfies PayloadRelacionPropuesta,
      }),
    ],
    { relacionId: nueva.id }
  );
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
