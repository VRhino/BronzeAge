// Comandos de diplomacia entre Facciones: proponer vasallaje/alianza, romper una relación, rebelarse contra
// un señor, anexionar y fusionar. Agrupados por la misma razón que `cargos.ts`: comparten forma y audiencia.
//
// ⚠️ Corrección respecto a `GameStore`: allí `romperRelacion` NO tenía try/catch, pero el motor lanza
// `DiplomaciaInvalidaError` si la relación no existe (`engine/diplomacia.ts`) — esa excepción llegaba cruda a
// la interfaz en vez de convertirse en un rechazo. Aquí pasa por `rechazoDesdeError` como el resto.
import type { RecursoTipo } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import {
  proponerVasallaje as proponerVasallajeEngine,
  proponerAlianza as proponerAlianzaEngine,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
} from '../../engine/diplomacia';
import { anexionar as anexionarEngine, fusionar as fusionarEngine } from '../../engine/fusion';
import { eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';

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

export function proponerRelacion(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsProponerRelacion
): TransicionComando<{ relacionId: string }> {
  try {
    const nueva =
      params.tipo === 'vasallaje'
        ? proponerVasallajeEngine(
            estado.facciones,
            estado.relaciones,
            params.faccionAId,
            params.faccionBId,
            params.tributoRecurso ?? 'trigo',
            params.tributoCantidad ?? 0,
            estado.tick,
            ctx.ids.siguiente()
          )
        : proponerAlianzaEngine(estado.facciones, estado.relaciones, params.faccionAId, params.faccionBId, estado.tick, ctx.ids.siguiente());

    const siguiente: GameSessionState = { ...estado, relaciones: [...estado.relaciones, nueva] };
    return exito(siguiente, [eventoLegado(ctx.momento, estado.tick, `Relación propuesta: ${nueva.id}.`)], { relacionId: nueva.id });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsRomperRelacion {
  relacionId: string;
  iniciadorFaccionId: string;
}

export function romperRelacion(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsRomperRelacion
): TransicionComando<void> {
  if (!params.relacionId) return rechazo(estado, CODIGOS_ERROR.diplomaciaRelacionNoIndicada);

  try {
    const resultado = romperRelacionEngine(estado.facciones, estado.relaciones, params.relacionId, params.iniciadorFaccionId);
    const siguiente: GameSessionState = { ...estado, facciones: resultado.facciones, relaciones: resultado.relaciones };
    return exito(siguiente, [eventoLegado(ctx.momento, estado.tick, `Relación ${params.relacionId} rota voluntariamente.`)]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsRebelionVasallo {
  relacionId: string;
}

export function rebelionVasallo(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsRebelionVasallo
): TransicionComando<void> {
  if (!params.relacionId) return rechazo(estado, CODIGOS_ERROR.diplomaciaRelacionNoIndicada);

  try {
    const resultado = rebelionVasalloEngine(estado.facciones, estado.relaciones, estado.acuerdos, estado.asentamientos, params.relacionId);
    const siguiente: GameSessionState = {
      ...estado,
      facciones: resultado.facciones,
      relaciones: resultado.relaciones,
      acuerdos: resultado.acuerdos,
    };
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje));
    return exito(siguiente, eventos);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsAnexionar {
  faccionAId: string;
  faccionBId: string;
}

export function anexionar(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsAnexionar): TransicionComando<void> {
  try {
    const resultado = anexionarEngine(estado.facciones, estado.asentamientos, params.faccionAId, params.faccionBId);
    const siguiente = sincronizarFaccionesNpc({
      ...estado,
      facciones: resultado.facciones,
      asentamientos: resultado.asentamientos,
    });
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje));
    return exito(siguiente, eventos);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsFusionar {
  faccionAId: string;
  faccionBId: string;
  nuevoNombre: string;
  nuevoReyId: string;
}

export function fusionar(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsFusionar): TransicionComando<void> {
  try {
    const resultado = fusionarEngine(
      estado.facciones,
      estado.asentamientos,
      params.faccionAId,
      params.faccionBId,
      params.nuevoNombre || 'Facción Fusionada',
      params.nuevoReyId,
      estado.tick
    );
    const siguiente = sincronizarFaccionesNpc({
      ...estado,
      facciones: resultado.facciones,
      asentamientos: resultado.asentamientos,
    });
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje));
    return exito(siguiente, eventos);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
