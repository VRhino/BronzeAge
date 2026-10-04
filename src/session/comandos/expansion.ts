// Comandos de expansión: lanzar una Caravana de Fundación hacia un punto del mapa, desarmarla para recuperar su contenido si se
// cambia de idea antes de que llegue, y `fundar` con la de un campamento donde esté su titular (D10, D30).
import type { Point } from '../../domain/types';
import {
  desarmarCaravanaFundacion as desarmarCaravanaFundacionEngine,
  fundarConCaravana,
  lanzarCaravanaFundacion as lanzarCaravanaFundacionEngine,
} from '../../engine/expansion';
import { FUNDACION } from '../../constants';
import { absorberColumna } from '../../engine/ejercitos';
import { esCiudadano } from '../../engine/faccion';
import { salirDeCampamentos } from '../../engine/mercenarios';
import { exigirPuertaDeFundacion } from '../../engine/settlement';
import { conEscuadrones } from '../../engine/tropa';
import { situarHeroes } from '../../engine/ubicacion';
import { conHistorialDeJugador } from '../estado';
import { CODIGOS_ERROR } from './codigosDeError';
import { desdeCrudos } from './eventos';
import { conExploracionFundida, conTropaDe, exigirColumnaDe, exigirJugador, rechazar } from './ayudas';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento, exigirCaravana, exigirFaccionDe } from './ayudas';
import { evento } from './eventos';

export interface PayloadCaravanaFundacionLanzada {
  caravanaId: string;
  origenAsentamientoId: string;
  faccionId: string;
  destino: Point;
  numJugadores: number;
}
export interface PayloadCaravanaFundacionDesarmada {
  caravanaId: string;
  origenAsentamientoId: string;
}

export interface ParamsLanzarCaravanaFundacion {
  origenAsentamientoId: string;
  destino: Point;
  numJugadores: number;
}

export const lanzarCaravanaFundacion = comando<ParamsLanzarCaravanaFundacion, { caravanaId: string }>((estado, mapa, ctx, params) => {
  const origen = exigirAsentamiento(estado, params.origenAsentamientoId);
  const faccion = exigirFaccionDe(estado, origen);

  const resultado = lanzarCaravanaFundacionEngine(
    mapa,
    origen,
    faccion,
    params.destino,
    estado.asentamientos,
    estado.caravanas,
    params.numJugadores,
    ctx.instante,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, resultado.origenActualizado),
    caravanas: [...estado.caravanas, resultado.caravana],
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'expansion.caravana_lanzada',
        mensaje: `Lanza una Caravana de Fundación hacia (${Math.round(params.destino.x)}, ${Math.round(params.destino.y)}).`,
        payload: {
          caravanaId: resultado.caravana.id,
          origenAsentamientoId: origen.id,
          faccionId: faccion.id,
          destino: params.destino,
          numJugadores: params.numJugadores,
        } satisfies PayloadCaravanaFundacionLanzada,
        asentamientoId: origen.id,
      }),
    ],
    { caravanaId: resultado.caravana.id }
  );
});

export interface ParamsDesarmarCaravanaFundacion {
  caravanaId: string;
}

export const desarmarCaravanaFundacion = comando<ParamsDesarmarCaravanaFundacion, void>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId ?? '');

  const actualizado = desarmarCaravanaFundacionEngine(origen, caravana);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, actualizado),
    caravanas: estado.caravanas.filter((c) => c.id !== params.caravanaId),
  };
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'expansion.caravana_desarmada',
      mensaje: `Desarma la Caravana de Fundación ${params.caravanaId} y recupera su contenido.`,
      payload: { caravanaId: params.caravanaId, origenAsentamientoId: origen.id } satisfies PayloadCaravanaFundacionDesarmada,
      asentamientoId: origen.id,
    }),
  ]);
});

export interface PayloadFundado {
  asentamientoId: string;
  faccionId: string;
  caravanaId: string;
  posicion: Point;
  heroesIds: string[];
}

/**
 * Fundar con la Caravana de Fundación de un campamento, donde esté (D10, D30): la lleva enganchada a su columna su titular, y funda él. Los
 * cofundadores son los ciudadanos de su Facción que van en esa columna, hasta `FUNDACION.maxJugadoresFundacionGrupal` (M2: unirse a la
 * columna es el consentimiento, D12). La caravana se gasta y la columna entra en la plaza nueva: todos residen ya en ella.
 */
export const fundar = comando<Record<string, never>, { asentamientoId: string }>((estado, mapa, ctx) => {
  const titular = exigirJugador(estado, ctx.actor);
  const columna = exigirColumnaDe(estado, titular.id);
  const caravana = estado.caravanas.find((c) => columna.caravanasAdjuntasIds.includes(c.id) && c.tipo === 'construccion' && c.titularId === titular.id);
  if (!caravana?.faccionId) rechazar(CODIGOS_ERROR.fundacionSinCaravana);
  const faccionId = caravana.faccionId!;
  const faccion = estado.facciones.find((f) => f.id === faccionId);
  const fundadores = [titular.id, ...columna.participantes.map((p) => p.heroeId).filter((id) => id !== titular.id)]
    .filter((id) => faccion && esCiudadano(faccion, id))
    .slice(0, FUNDACION.maxJugadoresFundacionGrupal);
  exigirPuertaDeFundacion(fundadores, true);

  const r = fundarConCaravana(mapa, estado.facciones, faccionId, columna.posicionActual, fundadores, estado.asentamientos, estado.campamentosMercenarios, ctx.instante);
  // Fundar es ENTRAR en lo que se acaba de levantar (Doc 1.10): la columna se deshace dentro —tropa al campamento, carro al almacén—,
  // sin la caravana, que se gasta en la fundación.
  const sinCaravana = { ...columna, faccionId, caravanasAdjuntasIds: columna.caravanasAdjuntasIds.filter((id) => id !== caravana.id) };
  const dentro = absorberColumna(r.asentamiento, conTropaDe(estado, sinCaravana), true);
  const enLaColumna = columna.participantes.map((p) => p.heroeId);

  let siguiente: GameSessionState = {
    ...estado,
    asentamientos: [...estado.asentamientos, dentro.asentamiento],
    facciones: r.facciones,
    ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id),
    caravanas: estado.caravanas
      .filter((c) => c.id !== caravana.id)
      .map((c) => (sinCaravana.caravanasAdjuntasIds.includes(c.id) ? { ...c, estado: 'aparcada' as const, posicionActual: dentro.asentamiento.posicion } : c)),
    campamentosMercenarios: salirDeCampamentos(estado.campamentosMercenarios, ...fundadores),
    heroes: situarHeroes(conEscuadrones(estado.heroes, dentro.tropa), enLaColumna, { tipo: 'asentamiento', asentamientoId: dentro.asentamiento.id }),
  };
  for (const heroeId of fundadores) {
    siguiente = conExploracionFundida(siguiente, heroeId, faccionId);
    siguiente = conHistorialDeJugador(siguiente, heroeId, `Funda ${dentro.asentamiento.id}.`);
  }

  const posicion = columna.posicionActual;
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'fundacion.asentamiento_fundado',
        mensaje: `${faccion?.nombre ?? faccionId} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`,
        payload: { asentamientoId: dentro.asentamiento.id, faccionId, caravanaId: caravana.id, posicion, heroesIds: fundadores } satisfies PayloadFundado,
        asentamientoId: dentro.asentamiento.id,
      }),
      ...desdeCrudos(ctx, r.eventos),
    ],
    { asentamientoId: dentro.asentamiento.id }
  );
});
