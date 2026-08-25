// Comandos militares: reclutar tropa y las cuatro formas de combate (asedio, campo abierto, intercepción de
// caravana y ataque a campamento de bandidos).
//
// Los cuatro de combate son de los pocos comandos de jugador que consumen aleatoriedad (`ctx.rng` — jitter de
// combate, ver `engine/combate.ts`). Por eso importa que el rng venga del contexto y no de un global: es lo
// que mantiene una partida reproducible aunque un jugador ataque en mitad de ella.
//
// ⚠️ Todos arrastraban el `.find(...)!` de `GameStore` sobre asentamiento/caravana/campamento — un id que no
// existiera producía un TypeError en vez de un rechazo. Corregido aquí.
import type { Mapa } from '../../world/mapa';
import { reclutarTropa as reclutarTropaEngine } from '../../engine/tropas';
import {
  atacarCampamentoBandidos as atacarCampamentoBandidosEngine,
  combateCampoAbierto as combateCampoAbiertoEngine,
  interceptarCaravana as interceptarCaravanaEngine,
  iniciarAsedio as iniciarAsedioEngine,
} from '../../engine/combate';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { conHistorialDeJugador, eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';

const ASENTAMIENTO_NO_EXISTE = CODIGOS_ERROR.asentamientoNoExiste;

export interface ParamsReclutarTropa {
  asentamientoId: string;
  jugadorId: string;
  tropaId: string;
  origen: 'pesants' | 'artesanos';
}

export function reclutarTropa(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsReclutarTropa
): TransicionComando<{ reclutados: number }> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  const cantidadDe = (a: typeof asentamiento): number =>
    a.escuadrones.find((e) => e.jugadorId === params.jugadorId && e.tropaId === params.tropaId)?.cantidad ?? 0;

  try {
    const antes = cantidadDe(asentamiento);
    const actualizado = reclutarTropaEngine(asentamiento, params.jugadorId, params.tropaId, params.origen, estado.tick, ctx.ids.siguiente());
    const reclutados = cantidadDe(actualizado) - antes;

    let siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)),
    };
    siguiente = conHistorialDeJugador(siguiente, params.jugadorId, `Recluta ${reclutados} de "${params.tropaId}" en ${asentamiento.id}.`);
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${asentamiento.id}: ${params.jugadorId} recluta ${reclutados} de la tropa "${params.tropaId}" (${params.origen}).`,
      asentamiento.id
    );
    return exito(siguiente, [evento], { reclutados });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsIniciarAsedio {
  atacanteId: string;
  defensorId: string;
  escuadronIds: string[];
}

export function iniciarAsedio(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsIniciarAsedio
): TransicionComando<{ conquistado: boolean }> {
  const atacante = estado.asentamientos.find((a) => a.id === params.atacanteId);
  const defensor = estado.asentamientos.find((a) => a.id === params.defensorId);
  if (!atacante || !defensor) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  try {
    const resultado = iniciarAsedioEngine(atacante, defensor, params.escuadronIds, estado.facciones, estado.relaciones, estado.tick, ctx.rng);
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => {
        if (a.id === resultado.atacante.id) return resultado.atacante;
        if (a.id === resultado.defensor.id) return resultado.defensor;
        return a;
      }),
      facciones: resultado.facciones,
    };
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje, atacante.id));
    return exito(siguiente, eventos, { conquistado: resultado.conquistado });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsCombateCampoAbierto {
  asentamientoAId: string;
  escuadronIdsA: string[];
  asentamientoBId: string;
  escuadronIdsB: string[];
}

export function combateCampoAbierto(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsCombateCampoAbierto
): TransicionComando<void> {
  const a = estado.asentamientos.find((s) => s.id === params.asentamientoAId);
  const b = estado.asentamientos.find((s) => s.id === params.asentamientoBId);
  if (!a || !b) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  try {
    const resultado = combateCampoAbiertoEngine(a, params.escuadronIdsA, b, params.escuadronIdsB, estado.facciones, estado.relaciones, estado.tick, ctx.rng);
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((s) => {
        if (s.id === resultado.asentamientoA.id) return resultado.asentamientoA;
        if (s.id === resultado.asentamientoB.id) return resultado.asentamientoB;
        return s;
      }),
      facciones: resultado.facciones,
    };
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje));
    return exito(siguiente, eventos);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsInterceptarCaravana {
  atacanteId: string;
  escuadronIds: string[];
  caravanaId: string;
}

export function interceptarCaravana(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsInterceptarCaravana
): TransicionComando<{ capturada: boolean }> {
  const atacante = estado.asentamientos.find((a) => a.id === params.atacanteId);
  if (!atacante) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  const caravana = estado.caravanas.find((c) => c.id === params.caravanaId);
  if (!caravana) return rechazo(estado, CODIGOS_ERROR.caravanaNoExiste);

  try {
    const resultado = interceptarCaravanaEngine(atacante, params.escuadronIds, caravana, estado.tick, estado.facciones, estado.asentamientos, ctx.rng);
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a)),
      facciones: resultado.facciones,
      caravanas: resultado.caravanaCapturada ? estado.caravanas.filter((c) => c.id !== caravana.id) : estado.caravanas,
    };
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje, atacante.id));
    return exito(siguiente, eventos, { capturada: resultado.caravanaCapturada });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsAtacarCampamentoBandidos {
  atacanteId: string;
  escuadronIds: string[];
  campamentoId: string;
}

export function atacarCampamentoBandidos(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsAtacarCampamentoBandidos
): TransicionComando<{ destruido: boolean }> {
  const atacante = estado.asentamientos.find((a) => a.id === params.atacanteId);
  if (!atacante) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  const campamento = estado.campamentosBandidos.find((c) => c.id === params.campamentoId);
  if (!campamento) return rechazo(estado, CODIGOS_ERROR.campamentoNoExiste);

  try {
    const resultado = atacarCampamentoBandidosEngine(atacante, params.escuadronIds, campamento, estado.tick, estado.facciones, ctx.rng);
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a)),
      facciones: resultado.facciones,
      // Al destruirlo se agenda su reaparición; el spawn en sí lo evalúa el tick (`avanzarSpawnBandidos`).
      campamentosBandidos: resultado.campamentoDestruido
        ? estado.campamentosBandidos.filter((c) => c.id !== campamento.id)
        : estado.campamentosBandidos,
      bandidosProximoSpawnTick: resultado.campamentoDestruido
        ? estado.tick + CAMPAMENTOS_BANDIDOS.ticksRespawn
        : estado.bandidosProximoSpawnTick,
    };
    const eventos = resultado.eventos.map((mensaje) => eventoLegado(ctx.momento, estado.tick, mensaje, atacante.id));
    return exito(siguiente, eventos, { destruido: resultado.campamentoDestruido });
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
