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
import { esCiudadano } from '../../engine/faccion';
import {
  atacarCampamentoBandidos as atacarCampamentoBandidosEngine,
  iniciarAsedio as iniciarAsedioEngine,
} from '../../engine/combate';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, conAsentamientos, exigirAsentamiento, exigirCampamento } from './ayudas';
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
  const faccionDelJugador = estado.facciones.find((f) => esCiudadano(f, params.jugadorId));
  const actualizado = reclutarTropaEngine(
    asentamiento,
    params.jugadorId,
    faccionDelJugador?.id ?? '',
    params.tropaId,
    params.origen,
    ctx.ids.siguiente()
  );
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

// `combateCampoAbierto` e `interceptarCaravana` VIVÍAN AQUÍ y se retiraron en el Paso 11 del movimiento de
// ejércitos (2026-09-04). No se han perdido: son ahora resoluciones del MOTOR disparadas por la geometría
// (`resolverEncuentros`, engine/ejercitos.ts), y el jugador llega a ellas mandando un ejército en vez de
// declarando un ataque desde el sofá. Ver Doc 5.12.3 y §2.6 del documento de ejecución.
//
// Retirarlos no era solo limpieza: `interceptarCaravana` resolvía contra una defensa base FIJA, que es
// exactamente lo que la escolta (Doc 5.13.3) sustituyó — mantener los dos habría dejado dos reglas distintas
// para el mismo hecho según por dónde se entrara.

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
