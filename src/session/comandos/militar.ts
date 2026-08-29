// Comandos militares: reclutar tropa y las cuatro formas de combate (asedio, campo abierto, intercepción de
// caravana y ataque a campamento de bandidos).
//
// Los cuatro de combate son de los pocos comandos de jugador que consumen aleatoriedad (`ctx.rng` — jitter de
// combate, ver `engine/combate.ts`). Por eso importa que el rng venga del contexto y no de un global: es lo
// que mantiene una partida reproducible aunque un jugador ataque en mitad de ella.
//
// Sus eventos son los MÁS sensibles a visibilidad de toda la capa de comandos —quién atacó a quién— así que
// son los que más ganan con `codigo`/`payload` estructurados: las proyecciones por audiencia de Fase C
// filtran sobre eso. Los payloads de combate los declara `engine/combate.ts`, que es quien resuelve.
import { reclutarTropa as reclutarTropaEngine } from '../../engine/tropas';
import {
  atacarCampamentoBandidos as atacarCampamentoBandidosEngine,
  combateCampoAbierto as combateCampoAbiertoEngine,
  interceptarCaravana as interceptarCaravanaEngine,
  iniciarAsedio as iniciarAsedioEngine,
} from '../../engine/combate';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, conAsentamientos, exigirAsentamiento, exigirCampamento, exigirCaravana } from './ayudas';
import { desdeCrudos, evento } from './eventos';

/** Reclutamiento: lo narra esta capa (el motor devuelve el asentamiento actualizado, sin eventos). */
export interface PayloadReclutamiento {
  asentamientoId: string;
  jugadorId: string;
  tropaId: string;
  origen: 'pesants' | 'artesanos';
  reclutados: number;
}

export interface ParamsReclutarTropa {
  asentamientoId: string;
  jugadorId: string;
  tropaId: string;
  origen: 'pesants' | 'artesanos';
}

export const reclutarTropa = comando<ParamsReclutarTropa, { reclutados: number }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const cantidadDe = (a: typeof asentamiento): number =>
    a.escuadrones.find((e) => e.jugadorId === params.jugadorId && e.tropaId === params.tropaId)?.cantidad ?? 0;

  const antes = cantidadDe(asentamiento);
  const actualizado = reclutarTropaEngine(asentamiento, params.jugadorId, params.tropaId, params.origen, ctx.ids.siguiente());
  const reclutados = cantidadDe(actualizado) - antes;

  const siguiente = conHistorialDeJugador(
    conAsentamiento(estado, actualizado),
    params.jugadorId,
    `Recluta ${reclutados} de "${params.tropaId}" en ${asentamiento.id}.`
  );
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'tropas.reclutadas',
        mensaje: `${params.jugadorId} recluta ${reclutados} de la tropa "${params.tropaId}" (${params.origen}).`,
        payload: { ...params, reclutados } satisfies PayloadReclutamiento,
        asentamientoId: asentamiento.id,
      }),
    ],
    { reclutados }
  );
});

export interface ParamsIniciarAsedio {
  atacanteId: string;
  defensorId: string;
  escuadronIds: string[];
}

export const iniciarAsedio = comando<ParamsIniciarAsedio, { conquistado: boolean }>((estado, _mapa, ctx, params) => {
  const atacante = exigirAsentamiento(estado, params.atacanteId);
  const defensor = exigirAsentamiento(estado, params.defensorId);

  const resultado = iniciarAsedioEngine(atacante, defensor, params.escuadronIds, estado.facciones, estado.relaciones, ctx.instante, ctx.rng);
  const siguiente: GameSessionState = {
    ...conAsentamientos(estado, [resultado.atacante, resultado.defensor]),
    facciones: resultado.facciones,
  };
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos, atacante.id), { conquistado: resultado.conquistado });
});

export interface ParamsCombateCampoAbierto {
  asentamientoAId: string;
  escuadronIdsA: string[];
  asentamientoBId: string;
  escuadronIdsB: string[];
}

export const combateCampoAbierto = comando<ParamsCombateCampoAbierto, void>((estado, _mapa, ctx, params) => {
  const a = exigirAsentamiento(estado, params.asentamientoAId);
  const b = exigirAsentamiento(estado, params.asentamientoBId);

  const resultado = combateCampoAbiertoEngine(a, params.escuadronIdsA, b, params.escuadronIdsB, estado.facciones, estado.relaciones, ctx.instante, ctx.rng);
  const siguiente: GameSessionState = {
    ...conAsentamientos(estado, [resultado.asentamientoA, resultado.asentamientoB]),
    facciones: resultado.facciones,
  };
  // Sin `asentamientoId`: el choque es entre DOS asentamientos, atribuirlo a uno sería arbitrario — los ids de
  // ambos bandos van en el `payload` de `combate.resuelto`.
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos));
});

export interface ParamsInterceptarCaravana {
  atacanteId: string;
  escuadronIds: string[];
  caravanaId: string;
}

export const interceptarCaravana = comando<ParamsInterceptarCaravana, { capturada: boolean }>((estado, _mapa, ctx, params) => {
  const atacante = exigirAsentamiento(estado, params.atacanteId);
  const caravana = exigirCaravana(estado, params.caravanaId);

  const resultado = interceptarCaravanaEngine(atacante, params.escuadronIds, caravana, ctx.instante, estado.facciones, estado.asentamientos, ctx.rng);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, resultado.atacante),
    facciones: resultado.facciones,
    caravanas: resultado.caravanaCapturada ? estado.caravanas.filter((c) => c.id !== caravana.id) : estado.caravanas,
  };
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos, atacante.id), { capturada: resultado.caravanaCapturada });
});

export interface ParamsAtacarCampamentoBandidos {
  atacanteId: string;
  escuadronIds: string[];
  campamentoId: string;
}

export const atacarCampamentoBandidos = comando<ParamsAtacarCampamentoBandidos, { destruido: boolean }>((estado, _mapa, ctx, params) => {
  const atacante = exigirAsentamiento(estado, params.atacanteId);
  const campamento = exigirCampamento(estado, params.campamentoId);

  const resultado = atacarCampamentoBandidosEngine(atacante, params.escuadronIds, campamento, ctx.instante, estado.facciones, ctx.rng);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, resultado.atacante),
    facciones: resultado.facciones,
    // Al destruirlo se agenda su reaparición; el spawn en sí lo evalúa el tick (`avanzarSpawnBandidos`).
    campamentosBandidos: resultado.campamentoDestruido
      ? estado.campamentosBandidos.filter((c) => c.id !== campamento.id)
      : estado.campamentosBandidos,
    bandidosProximoSpawnEn: resultado.campamentoDestruido
      ? sumar(ctx.instante, minutos(CAMPAMENTOS_BANDIDOS.respawnMinutos))
      : estado.bandidosProximoSpawnEn,
  };
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos, atacante.id), { destruido: resultado.campamentoDestruido });
});
