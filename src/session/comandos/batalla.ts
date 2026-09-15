// Los dos comandos que actúan SOBRE una batalla de Unity (doc 02 §3.1), y el candado que lleva puesto el resto del
// registro (Doc 5.15.1). Las reglas viven en `session/batallas.ts`; aquí, resolver la batalla y narrar.
import {
  batallasActivas,
  cancelarBatalla as cancelar,
  eventosDeBatalla,
  tocaLoBloqueado,
  unirseABatalla as unirse,
  type Batalla,
} from '../batallas';
import type { Instante } from '../../domain/tiempo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { CODIGOS_ERROR } from './codigosDeError';
import { comando, rechazar } from './ayudas';
import { evento } from './eventos';
import { exito, rechazo, type ManejadorComando } from './tipos';

export interface ParamsUnirseABatalla {
  heroeId: string;
  battleId: string;
}

export interface ParamsCancelarBatalla {
  battleId: string;
}

function exigirBatalla(estado: GameSessionState, battleId: string, ahora: Instante): Batalla {
  const batalla = batallasActivas(estado, ahora).find((b) => b.id === battleId);
  if (!batalla) rechazar(CODIGOS_ERROR.batallaNoExiste);
  return batalla;
}

/** La columna del héroe entra en el bando de su Facción (Doc 5.15.1). Devuelve las secuencias de sus incorporaciones. */
export const unirseABatalla = comando<ParamsUnirseABatalla, { secuencias: number[] }>((estado, _mapa, ctx, params) => {
  const r = unirse(estado, exigirBatalla(estado, params.battleId, ctx.instante), params.heroeId, ctx.instante);
  const batalla = r.estado.batallas.find((b) => b.id === params.battleId)!;
  return exito(
    conHistorialDeJugador(r.estado, params.heroeId, `Se une a la batalla ${params.battleId}.`),
    eventosDeBatalla(r.estado, batalla, 'batalla.refuerzos', `Llegan refuerzos a la batalla ${batalla.id}.`).map((e) => evento(ctx, e)),
    { secuencias: r.nuevas.map((n) => n.secuencia) }
  );
});

/** Echarse atrás antes de que empiece la partida: nadie pierde nada (doc 01 §15). */
export const cancelarBatalla = comando<ParamsCancelarBatalla, void>((estado, _mapa, ctx, params) => {
  const batalla = exigirBatalla(estado, params.battleId, ctx.instante);
  const siguiente = cancelar(estado, batalla);
  return exito(
    siguiente,
    eventosDeBatalla(siguiente, batalla, 'batalla.cancelada', `Se cancela la batalla ${batalla.id}: nadie pierde nada.`).map((e) => evento(ctx, e))
  );
});

/** Pone el candado de batalla (Doc 5.15.1) a todos los comandos salvo `libres`, que actúan sobre la propia batalla. */
export function conCandadoDeBatalla<T extends Record<string, ManejadorComando<any, any>>>(manejadores: T, libres: readonly (keyof T)[]): T {
  const conCandado =
    (manejador: ManejadorComando<unknown, unknown>): ManejadorComando<unknown, unknown> =>
    (estado, mapa, ctx, params) =>
      tocaLoBloqueado(estado, ctx.instante, ctx.actor, params) ? rechazo(estado, CODIGOS_ERROR.batallaBloqueo) : manejador(estado, mapa, ctx, params);
  return Object.fromEntries(Object.entries(manejadores).map(([tipo, m]) => [tipo, libres.includes(tipo) ? m : conCandado(m)])) as T;
}
