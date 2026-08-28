import type { GameSessionState } from '../estado';
import { exito, sinCambios } from './tipos';
import { comando, exigirFaccion } from './ayudas';
import { evento } from './eventos';

export interface PayloadFaccionNpc {
  faccionId: string;
  activo: boolean;
}

export interface ParamsAlternarFaccionNpc {
  faccionId: string;
  /** `true` cede la Facción al NPC de gobernanza; `false` la devuelve a control manual. */
  activo: boolean;
}

/**
 * Cede una Facción al NPC de gobernanza, o la recupera.
 *
 * Es un comando **administrativo** (rol técnico, doc 5), no una palanca de juego: el usuario confirmó que en
 * una partida real el valor no cambia en caliente — una Facción declarada IA lo es hasta que se destruye. Se
 * mantiene invocable para corrección y moderación.
 *
 * `faccionesNpcIds` vive en el estado de la partida y no en el runner (doc 7 §7.2) precisamente porque cambia
 * el resultado del tick: si viviera fuera, un reinicio con otra configuración divergiría de lo que el
 * snapshot dice haber pasado.
 *
 * Pedir el estado en el que ya está NO es un error: es idempotente y se resuelve sin mutar ni versionar, que
 * es lo que hace segura una reintentar por reconexión (doc 2, punto 10: idempotencia de comandos).
 */
export const alternarFaccionNpc = comando<ParamsAlternarFaccionNpc, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);

  const yaEsNpc = estado.faccionesNpcIds.includes(params.faccionId);
  if (params.activo === yaEsNpc) return sinCambios(estado);

  const siguiente: GameSessionState = {
    ...estado,
    faccionesNpcIds: params.activo
      ? [...estado.faccionesNpcIds, params.faccionId]
      : estado.faccionesNpcIds.filter((id) => id !== params.faccionId),
  };
  return exito(siguiente, [
    evento(ctx, estado, {
      codigo: params.activo ? 'faccion.cedida_al_npc' : 'faccion.recuperada_del_npc',
      mensaje: params.activo
        ? `${faccion.nombre}: pasa a estar controlada por el NPC de gobernanza (juega sola).`
        : `${faccion.nombre}: vuelve a control manual del jugador.`,
      payload: { faccionId: faccion.id, activo: params.activo } satisfies PayloadFaccionNpc,
    }),
  ]);
});
