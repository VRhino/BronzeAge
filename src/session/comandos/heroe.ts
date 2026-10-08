// Comandos del héroe sobre sí mismo (doc 02 §4.2). No reciben `heroeId`: el actor es siempre su propio héroe.
import type { AtributosHeroe, Heroe } from '../../domain/types';
import type { GameSessionState } from '../estado';
import {
  asignarGuarnicion as asignarEnHeroe,
  borrarLoadout as borrarDelHeroe,
  guardarLoadout as guardarEnHeroe,
  liderazgoDeLoadout,
  ordenarEscuadras as ordenarEnHeroe,
  repartirAtributos,
  retirarGuarnicion as retirarEnHeroe,
} from '../../engine/heroe';
import { esResidente } from '../../engine/pertenencia';
import { guardarEnAlmacenPersonal as guardarEngine, sacarDelAlmacenPersonal as sacarEngine } from '../../engine/almacenPersonal';
import { capacidadCargaDe, columnaDe } from '../../engine/ejercitos';
import { pasarAViveres as pasarAViveresEngine } from '../../engine/viveres';
import { exito } from './tipos';
import { comando, exigirColumnaDe, exigirJugador } from './ayudas';
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

export interface ParamsAlmacenPersonal {
  recurso: string;
  cantidad: number;
}

export interface PayloadAlmacenPersonal {
  heroeId: string;
  recurso: string;
  cantidad: number;
  sentido: 'guarda' | 'saca';
}

/** Del carro de la columna al almacén personal (Doc 2.5): lo que cabe hasta el tope. Solo el Líder de la columna. */
export const guardarEnAlmacenPersonal = comando<ParamsAlmacenPersonal, { movido: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const columna = exigirColumnaDe(estado, heroe.id);
  const r = guardarEngine(heroe, columna, params.recurso, params.cantidad);
  const siguiente: GameSessionState = {
    ...conHeroe(estado, r.heroe),
    ejercitos: estado.ejercitos.map((e) => (e.id === r.ejercito.id ? r.ejercito : e)),
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'heroe.almacen_personal',
        mensaje: `${heroe.displayName} guarda ${r.movido.toFixed(0)} ${params.recurso} en su almacén personal.`,
        payload: { heroeId: heroe.id, recurso: params.recurso, cantidad: r.movido, sentido: 'guarda' } satisfies PayloadAlmacenPersonal,
      }),
    ],
    { movido: r.movido }
  );
});

/** Del almacén personal al carro de la columna (Doc 2.5): lo que cabe en el carro. Solo el Líder de la columna. */
export const sacarDelAlmacenPersonal = comando<ParamsAlmacenPersonal, { movido: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const columna = exigirColumnaDe(estado, heroe.id);
  const r = sacarEngine(heroe, columna, params.recurso, params.cantidad, capacidadCargaDe(columna, estado.caravanas));
  const siguiente: GameSessionState = {
    ...conHeroe(estado, r.heroe),
    ejercitos: estado.ejercitos.map((e) => (e.id === r.ejercito.id ? r.ejercito : e)),
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'heroe.almacen_personal',
        mensaje: `${heroe.displayName} saca ${r.movido.toFixed(0)} ${params.recurso} de su almacén personal.`,
        payload: { heroeId: heroe.id, recurso: params.recurso, cantidad: r.movido, sentido: 'saca' } satisfies PayloadAlmacenPersonal,
      }),
    ],
    { movido: r.movido }
  );
});

export interface PayloadViveres {
  heroeId: string;
  cantidad: number;
}

/** Del carro de la columna a los víveres del héroe (Doc 5.13): lo que cabe. Nunca al revés. Solo el Líder de la columna. */
export const pasarAViveres = comando<{ cantidad: number }, { movido: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const r = pasarAViveresEngine(heroe, exigirColumnaDe(estado, heroe.id), params.cantidad);
  const siguiente: GameSessionState = {
    ...conHeroe(estado, r.heroe),
    ejercitos: estado.ejercitos.map((e) => (e.id === r.ejercito.id ? r.ejercito : e)),
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'heroe.viveres',
        mensaje: `${heroe.displayName} pasa ${r.movido.toFixed(0)} de trigo del carro a sus víveres.`,
        payload: { heroeId: heroe.id, cantidad: r.movido } satisfies PayloadViveres,
      }),
    ],
    { movido: r.movido }
  );
});

export interface ParamsOrdenarEscuadras {
  escuadronIds: string[];
}

/** Ordena las escuadras propias (`ordenarEscuadras` del motor): el orden en que entran en combate, también en su columna si va en una. */
export const ordenarEscuadras = comando<ParamsOrdenarEscuadras, undefined>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const r = ordenarEnHeroe(heroe, columnaDe(estado.ejercitos, heroe.id), params.escuadronIds);
  const columna = r.columna;
  return exito({ ...conHeroe(estado, r.heroe), ejercitos: columna ? estado.ejercitos.map((e) => (e.id === columna.id ? columna : e)) : estado.ejercitos }, []);
});
