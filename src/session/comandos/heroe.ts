// Comandos del héroe sobre sí mismo (doc 02 §4.2). No reciben `heroeId`: el actor es siempre su propio héroe.
import type { AtributosHeroe, Heroe } from '../../domain/types';
import type { GameSessionState } from '../estado';
import {
  asignarGuarnicion as asignarEnHeroe,
  borrarLoadout as borrarDelHeroe,
  guardarLoadout as guardarEnHeroe,
  liderazgoDeLoadout,
  repartirAtributos,
  retirarGuarnicion as retirarEnHeroe,
} from '../../engine/heroe';
import { esResidente } from '../../engine/pertenencia';
import { exito } from './tipos';
import { comando, exigirJugador } from './ayudas';
import { evento } from './eventos';

const conHeroe = (estado: GameSessionState, heroe: Heroe): GameSessionState => ({
  ...estado,
  heroes: estado.heroes.map((h) => (h.id === heroe.id ? heroe : h)),
});

export interface ParamsRepartirPuntos {
  atributos: Partial<AtributosHeroe>;
}

/** Reparte puntos de atributo sin gastar (Doc 5.16.1). Los perks esperan al catálogo de Conquest (CQ-004). */
export const repartirPuntos = comando<ParamsRepartirPuntos, undefined>((estado, _mapa, ctx, params) => {
  const heroe = repartirAtributos(exigirJugador(estado, ctx.actor), params.atributos);
  return exito(conHeroe(estado, heroe), [
    evento(ctx, {
      codigo: 'heroe.puntos_repartidos',
      mensaje: `${heroe.displayName} reparte sus puntos de atributo.`,
      payload: { heroeId: heroe.id, atributos: params.atributos },
    }),
  ]);
});

export interface ParamsGuardarLoadout {
  /** Ausente = uno nuevo. */
  loadoutId?: string;
  displayName: string;
  squadIds: string[];
  perksSeleccionados: number[];
  activo?: boolean;
}

/** Guarda o reescribe un loadout (Doc 5.16.5). Devuelve su id y el Liderazgo que suma. */
export const guardarLoadout = comando<ParamsGuardarLoadout, { loadoutId: string; liderazgoTotal: number }>((estado, _mapa, ctx, params) => {
  const antes = exigirJugador(estado, ctx.actor);
  const heroe = guardarEnHeroe(antes, { ...params, id: params.loadoutId }, () => `loadout-${ctx.ids.siguiente()}`);
  const loadout = params.loadoutId ? heroe.loadouts.find((l) => l.id === params.loadoutId)! : heroe.loadouts[heroe.loadouts.length - 1]!;
  return exito(conHeroe(estado, heroe), [], { loadoutId: loadout.id, liderazgoTotal: liderazgoDeLoadout(heroe, loadout) });
});

export const borrarLoadout = comando<{ loadoutId: string }, undefined>((estado, _mapa, ctx, params) =>
  exito(conHeroe(estado, borrarDelHeroe(exigirJugador(estado, ctx.actor), params.loadoutId)), [])
);

/** Entrega una escuadra del campamento a la guarnición de su residencia (Doc 5.15.3). */
export const asignarGuarnicion = comando<{ squadId: string }, undefined>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const residencia = estado.asentamientos.find((a) => esResidente(a, heroe.id));
  return exito(conHeroe(estado, asignarEnHeroe(heroe, residencia, params.squadId)), []);
});

export const retirarGuarnicion = comando<{ squadId: string }, undefined>((estado, _mapa, ctx, params) =>
  exito(conHeroe(estado, retirarEnHeroe(exigirJugador(estado, ctx.actor), params.squadId)), [])
);
