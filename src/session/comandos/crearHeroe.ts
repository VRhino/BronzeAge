import { LIDERAZGO } from '../../constants';
import type { Heroe } from '../../domain/types';
import { progresionInicial } from '../../engine/heroe';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface ParamsCrearHeroe {
  displayName: string;
  /** El campamento de mercenarios donde nace (D3/D74): dentro, como residente. */
  campamentoId: string;
  classDefinitionId: string;
  genero: Heroe['genero'];
  avatar: Heroe['avatar'];
  /** Lo pone el SERVIDOR, nunca el cliente (el esquema HTTP no lo admite): `'bot'` si la cuenta es de bot (doc 12 §8.3). */
  controlador?: Heroe['controlador'];
}

/**
 * Crea el héroe del jugador DENTRO del campamento de mercenarios que elige, como residente (Doc 1.3, D3/D74), sin
 * columna en el mapa: sale cuando quiera, como de su casa. Cuenta para el contador de la elección (D79). Es lo primero que hace un
 * jugador en una partida: sin héroe, la superficie de jugador no le deja hacer nada más (doc 02 §4.2).
 *
 * Aquí `ctx.actor` es el `jugadorId` de la membresía, porque todavía no hay héroe al que atribuir la acción.
 * Si ya lo tiene, el actor que llega es su héroe: por eso se comprueban las dos cosas.
 */
export const crearHeroe = comando<ParamsCrearHeroe, { heroeId: string }>((estado, _mapa, ctx, params) => {
  const displayName = params.displayName.trim();
  if (!displayName) rechazar(CODIGOS_ERROR.heroeNombreVacio);
  if (estado.heroes.some((h) => h.jugadorId === ctx.actor || h.id === ctx.actor)) rechazar(CODIGOS_ERROR.heroeYaExiste);

  const campamento = estado.campamentosMercenarios.find((c) => c.id === params.campamentoId);
  if (!campamento) rechazar(CODIGOS_ERROR.heroeCampamentoDesconocido);

  const heroeId = `heroe-${ctx.ids.siguiente()}`;
  const heroe: Heroe = {
    id: heroeId,
    jugadorId: ctx.actor,
    controlador: params.controlador ?? 'humano',
    displayName,
    classDefinitionId: params.classDefinitionId,
    genero: params.genero,
    avatar: params.avatar,
    liderazgoBase: LIDERAZGO.base,
    ubicacion: { tipo: 'mercenarios', campamentoId: campamento.id },
    escuadrones: [],
    ...progresionInicial(heroeId),
  };
  return exito(
    {
      ...estado,
      heroes: [...estado.heroes, heroe],
      campamentosMercenarios: estado.campamentosMercenarios.map((c) =>
        c.id === campamento.id ? { ...c, residentesIds: [...c.residentesIds, heroeId], eligieronComoInicial: c.eligieronComoInicial + 1 } : c
      ),
    },
    [evento(ctx, { codigo: 'heroe.creado', mensaje: `${displayName} llega a ${campamento.id}.`, payload: { heroeId, campamentoId: campamento.id } })],
    { heroeId }
  );
});
