// Comandos de expansión: la Caravana de Fundación es una sola (Doc 1.8, D30): se lanza desde una plaza (o se compra en un campamento, ver
// `comandos/mercenarios`), nace sin destino con un titular, se desarma a mano en la puerta de su origen, y `fundar` funda con ella donde esté
// la columna de su titular.
import {
  desarmarCaravanaFundacion as desarmarCaravanaFundacionEngine,
  ExpansionInvalidaError,
  fundarConCaravana,
  lanzarCaravanaFundacion as lanzarCaravanaFundacionEngine,
  type PayloadFundado,
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
import { nombreDeCiudadLibre } from '../../engine/nombresDeCiudades';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento, exigirCaravana, exigirFaccionDe } from './ayudas';
import { evento } from './eventos';

export interface PayloadCaravanaFundacionLanzada {
  caravanaId: string;
  origenAsentamientoId: string;
  faccionId: string;
  titularId: string;
}
export interface PayloadCaravanaFundacionDesarmada {
  caravanaId: string;
  faccionId: string;
}

export interface ParamsLanzarCaravanaFundacion {
  origenAsentamientoId: string;
}

/** Quien lanza es el titular (Doc 1.8): la caravana nace parada en su plaza, y él la engancha a su columna y funda donde llegue. */
export const lanzarCaravanaFundacion = comando<ParamsLanzarCaravanaFundacion, { caravanaId: string }>((estado, _mapa, ctx, params) => {
  const origen = exigirAsentamiento(estado, params.origenAsentamientoId);
  const faccion = exigirFaccionDe(estado, origen);
  const titular = exigirJugador(estado, ctx.actor);

  const resultado = lanzarCaravanaFundacionEngine(origen, faccion, titular.id, estado.asentamientos, estado.caravanas, ctx.instante, ctx.ids.siguiente());
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, resultado.origenActualizado),
    caravanas: [...estado.caravanas, resultado.caravana],
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'expansion.caravana_lanzada',
        mensaje: `${titular.displayName} prepara una Caravana de Fundación en ${origen.id}.`,
        payload: { caravanaId: resultado.caravana.id, origenAsentamientoId: origen.id, faccionId: faccion.id, titularId: titular.id } satisfies PayloadCaravanaFundacionLanzada,
        asentamientoId: origen.id,
      }),
    ],
    { caravanaId: resultado.caravana.id }
  );
});

export interface ParamsDesarmarCaravanaFundacion {
  caravanaId: string;
}

/** El titular desarma su Caravana de Fundación suelta, en la puerta de su origen, y recupera lo que costó (a la plaza, o a quien aportó). */
export const desarmarCaravanaFundacion = comando<ParamsDesarmarCaravanaFundacion, void>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  if (caravana.titularId !== ctx.actor) throw new ExpansionInvalidaError('Solo su titular desarma la Caravana de Fundación.');
  const origen = caravana.origenCampamentoId
    ? estado.campamentosMercenarios.find((c) => c.id === caravana.origenCampamentoId)
    : estado.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);

  const devuelto = desarmarCaravanaFundacionEngine(caravana, origen?.posicion, { asentamientos: estado.asentamientos, heroes: estado.heroes }, estado.facciones);
  const siguiente: GameSessionState = {
    ...estado,
    ...devuelto,
    caravanas: estado.caravanas.filter((c) => c.id !== params.caravanaId),
  };
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'expansion.caravana_desarmada',
      mensaje: `Desarma la Caravana de Fundación ${params.caravanaId} y recupera lo que costó.`,
      payload: { caravanaId: params.caravanaId, faccionId: caravana.faccionId ?? '' } satisfies PayloadCaravanaFundacionDesarmada,
      ...(origen && !caravana.origenCampamentoId ? { asentamientoId: origen.id } : {}),
    }),
  ]);
});

/**
 * Fundar con la Caravana de Fundación, la de un campamento o la de una plaza, donde esté (D10, D30): la lleva enganchada a su columna su titular, y funda él. Los
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
  // Recibe el nombre de una ciudad de la época que ninguna plaza de la partida lleve ya (se puede renombrar después).
  const nombre = nombreDeCiudadLibre(estado.asentamientos.map((a) => a.nombre), r.asentamiento.id);
  const nueva = nombre ? { ...r.asentamiento, nombre } : r.asentamiento;
  // Fundar es ENTRAR en lo que se acaba de levantar (Doc 1.10): la columna se deshace dentro —tropa al campamento, carro al almacén—,
  // sin la caravana, que se gasta en la fundación.
  const sinCaravana = { ...columna, faccionId, caravanasAdjuntasIds: columna.caravanasAdjuntasIds.filter((id) => id !== caravana.id) };
  const dentro = absorberColumna(nueva, conTropaDe(estado, sinCaravana), true);
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
        mensaje: `${faccion?.nombre ?? faccionId} funda ${nombre ?? 'un asentamiento'} en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`,
        payload: { asentamientoId: dentro.asentamiento.id, faccionId, caravanaId: caravana.id, posicion, heroesIds: fundadores, ...(nombre ? { nombre } : {}) } satisfies PayloadFundado,
        asentamientoId: dentro.asentamiento.id,
      }),
      ...desdeCrudos(ctx, r.eventos),
    ],
    { asentamientoId: dentro.asentamiento.id }
  );
});
