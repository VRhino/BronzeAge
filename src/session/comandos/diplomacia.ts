// Comandos de diplomacia entre Facciones: proponer vasallaje/alianza, romper una relación, rebelarse contra
// un señor, anexionar y fusionar. Agrupados por la misma razón que `cargos.ts`: comparten forma y audiencia.
//
// Anexionar, fusionar y la rebelión los narra el MOTOR (`engine/fusion.ts`, `engine/diplomacia.ts`), que ya
// emite `EventoCrudo` con código y payload; aquí solo se les añade el contexto temporal. Proponer y romper los
// narra esta capa, que es donde se sabe qué relación se creó.
import type { RecursoTipo } from '../../domain/types';
import {
  proponerVasallaje as proponerVasallajeEngine,
  proponerAlianza as proponerAlianzaEngine,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
} from '../../engine/diplomacia';
import { anexionar as anexionarEngine, fusionar as fusionarEngine } from '../../engine/fusion';
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

/**
 * Quita de `faccionesNpcIds` las Facciones que ya no existen. Anexionar y fusionar pueden hacer desaparecer
 * una Facción, y dejar su id colgando ahí significaría que si alguien creara después otra con el mismo id,
 * arrancaría gobernada por el NPC sin haberlo pedido. Mismo criterio que `GameStore.sincronizarFaccionesNpc`.
 */
function sincronizarFaccionesNpc(estado: GameSessionState): GameSessionState {
  const vigentes = estado.faccionesNpcIds.filter((id) => estado.facciones.some((f) => f.id === id));
  return vigentes.length === estado.faccionesNpcIds.length ? estado : { ...estado, faccionesNpcIds: vigentes };
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

  const resultado = rebelionVasalloEngine(estado.facciones, estado.relaciones, estado.acuerdos, estado.asentamientos, params.relacionId);
  const siguiente: GameSessionState = {
    ...estado,
    facciones: resultado.facciones,
    relaciones: resultado.relaciones,
    acuerdos: resultado.acuerdos,
  };
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos));
});

export interface ParamsAnexionar {
  faccionAId: string;
  faccionBId: string;
}

export const anexionar = comando<ParamsAnexionar, void>((estado, _mapa, ctx, params) => {
  const resultado = anexionarEngine(estado.facciones, estado.asentamientos, params.faccionAId, params.faccionBId);
  const siguiente = sincronizarFaccionesNpc({
    ...estado,
    facciones: resultado.facciones,
    asentamientos: resultado.asentamientos,
  });
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos));
});

export interface ParamsFusionar {
  faccionAId: string;
  faccionBId: string;
  nuevoNombre: string;
  nuevoReyId: string;
}

export const fusionar = comando<ParamsFusionar, void>((estado, _mapa, ctx, params) => {
  const resultado = fusionarEngine(
    estado.facciones,
    estado.asentamientos,
    params.faccionAId,
    params.faccionBId,
    params.nuevoNombre || 'Facción Fusionada',
    params.nuevoReyId,
    ctx.instante
  );
  const siguiente = sincronizarFaccionesNpc({
    ...estado,
    facciones: resultado.facciones,
    asentamientos: resultado.asentamientos,
  });
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos));
});
