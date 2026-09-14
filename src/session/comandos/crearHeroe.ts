import { LIDERAZGO } from '../../constants';
import type { Heroe } from '../../domain/types';
import { columnaDeAparicion } from '../../engine/ubicacion';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface ParamsCrearHeroe {
  displayName: string;
  classDefinitionId: string;
  genero: Heroe['genero'];
  avatar: Heroe['avatar'];
}

/**
 * Crea el héroe del jugador y lo hace APARECER en el mundo con su columna (Doc 1.3). Es lo primero que hace un
 * jugador en una partida: sin héroe, la superficie de jugador no le deja hacer nada más (doc 02 §4.2).
 *
 * Aquí `ctx.actor` es el `jugadorId` de la membresía, porque todavía no hay héroe al que atribuir la acción.
 * Si ya lo tiene, el actor que llega es su héroe: por eso se comprueban las dos cosas.
 */
export const crearHeroe = comando<ParamsCrearHeroe, { heroeId: string }>((estado, mapa, ctx, params) => {
  const displayName = params.displayName.trim();
  if (!displayName) rechazar(CODIGOS_ERROR.heroeNombreVacio);
  if (estado.heroes.some((h) => h.jugadorId === ctx.actor || h.id === ctx.actor)) rechazar(CODIGOS_ERROR.heroeYaExiste);

  const heroeId = `heroe-${ctx.ids.siguiente()}`;
  const columna = columnaDeAparicion(`ejercito-${ctx.ids.siguiente()}`, heroeId, mapa, estado.asentamientos, ctx.rng, ctx.instante);
  const heroe: Heroe = {
    id: heroeId,
    jugadorId: ctx.actor,
    controlador: 'humano',
    displayName,
    classDefinitionId: params.classDefinitionId,
    genero: params.genero,
    avatar: params.avatar,
    liderazgoBase: LIDERAZGO.base,
    ubicacion: { tipo: 'columna', ejercitoId: columna.id },
  };
  return exito(
    { ...estado, heroes: [...estado.heroes, heroe], ejercitos: [...estado.ejercitos, columna] },
    [evento(ctx, { codigo: 'heroe.creado', mensaje: `${displayName} aparece en el mundo.`, payload: { heroeId } })],
    { heroeId }
  );
});
