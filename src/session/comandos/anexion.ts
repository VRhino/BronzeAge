// Anexión entre Facciones (Doc 2.6, decidido el 2026-10-06): la propone el Rey o el Embajador de la absorbente, y solo la ejecuta el Rey de la
// absorbida al aceptarla. Pendiente en `estado.propuestasAnexion` hasta que se contesta, se retira o caduca. Las reglas están en `engine/anexion.ts`.
import {
  AnexionInvalidaError,
  anexionar as anexionarEngine,
  exigirAnexionPosible,
  propuestasVigentes,
  proponerAnexion as proponerAnexionEngine,
  type PayloadAnexionPropuesta,
  type PayloadAnexionRespondida,
} from '../../engine/anexion';
import type { Instante } from '../../domain/tiempo';
import type { PropuestaAnexion } from '../../domain/types';
import { batallasActivas } from '../batallas';
import type { GameSessionState } from '../estado';
import { exigirFaccion, comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { desdeCrudos, evento } from './eventos';
import { exito } from './tipos';

export interface ParamsProponerAnexion {
  /** La absorbente: quien propone. */
  faccionAId: string;
  faccionBId: string;
}

export interface ParamsResponderAnexion {
  propuestaId: string;
  aceptar: boolean;
}

export interface ParamsRetirarAnexion {
  propuestaId: string;
}

function pendiente(estado: GameSessionState, propuestaId: string, ahora: Instante): PropuestaAnexion {
  const propuesta = (estado.propuestasAnexion ?? []).find((p) => p.id === propuestaId);
  if (!propuesta) rechazar(CODIGOS_ERROR.anexionNoExiste);
  if (propuestasVigentes([propuesta], ahora).length === 0) rechazar(CODIGOS_ERROR.anexionCaducada);
  return propuesta;
}

const carga = (p: PropuestaAnexion): PayloadAnexionPropuesta => ({ propuestaId: p.id, absorbenteId: p.absorbenteId, absorbidaId: p.absorbidaId });

export const proponerAnexion = comando<ParamsProponerAnexion, { propuestaId: string }>((estado, _mapa, ctx, params) => {
  const absorbente = exigirFaccion(estado, params.faccionAId);
  const absorbida = exigirFaccion(estado, params.faccionBId);
  const nueva = proponerAnexionEngine(
    estado.facciones,
    estado.relaciones,
    estado.propuestasAnexion,
    params.faccionAId,
    params.faccionBId,
    ctx.actor,
    ctx.instante,
    `anexion-${params.faccionAId}-${params.faccionBId}-${ctx.ids.siguiente()}`
  );
  return exito(
    { ...estado, propuestasAnexion: [...propuestasVigentes(estado.propuestasAnexion, ctx.instante), nueva] },
    [
      evento(ctx, {
        codigo: 'diplomacia.anexion_propuesta',
        mensaje: `${absorbente.nombre} propone anexionar a ${absorbida.nombre}.`,
        payload: carga(nueva) satisfies PayloadAnexionPropuesta,
      }),
    ],
    { propuestaId: nueva.id }
  );
});

/** El Rey de la absorbida acepta (la anexión se ejecuta en el acto) o rechaza. */
export const responderAnexion = comando<ParamsResponderAnexion, void>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);
  const sin = (e: GameSessionState): GameSessionState => ({ ...e, propuestasAnexion: (e.propuestasAnexion ?? []).filter((p) => p.id !== propuesta.id) });

  if (!params.aceptar) {
    const absorbente = exigirFaccion(estado, propuesta.absorbenteId);
    const absorbida = exigirFaccion(estado, propuesta.absorbidaId);
    return exito(sin(estado), [
      evento(ctx, {
        codigo: 'diplomacia.anexion_rechazada',
        mensaje: `${absorbida.nombre} rechaza ser anexionada por ${absorbente.nombre}.`,
        payload: { ...carga(propuesta), aceptada: false } satisfies PayloadAnexionRespondida,
      }),
    ]);
  }

  // El mundo cambió mientras esperaba: lo que valía al proponer se vuelve a mirar, y una batalla abierta de cualquiera de las dos no se mueve de bando.
  exigirAnexionPosible(estado.facciones, estado.relaciones, propuesta.absorbenteId, propuesta.absorbidaId);
  const de = new Set([propuesta.absorbenteId, propuesta.absorbidaId]);
  const plazas = new Set(estado.asentamientos.filter((a) => de.has(a.faccionId)).map((a) => a.id));
  const ejercitos = new Set(estado.ejercitos.filter((e) => de.has(e.faccionId)).map((e) => e.id));
  const enBatalla = batallasActivas(estado, ctx.instante).some(
    (b) => b.bloqueo.ejercitoIds.some((id) => ejercitos.has(id)) || (b.bloqueo.asentamientoId !== undefined && plazas.has(b.bloqueo.asentamientoId))
  );
  if (enBatalla) throw new AnexionInvalidaError('Alguna de las dos Facciones tiene una batalla en curso: espera a que termine.');

  const resultado = anexionarEngine(estado, propuesta.absorbenteId, propuesta.absorbidaId);
  return exito({ ...estado, ...resultado.mundo }, desdeCrudos(ctx, resultado.eventos));
});

/** La absorbente retira su propuesta mientras nadie la ha contestado. */
export const retirarAnexion = comando<ParamsRetirarAnexion, void>((estado, _mapa, ctx, params) => {
  const propuesta = pendiente(estado, params.propuestaId, ctx.instante);
  return exito({ ...estado, propuestasAnexion: (estado.propuestasAnexion ?? []).filter((p) => p.id !== propuesta.id) }, [
    evento(ctx, {
      codigo: 'diplomacia.anexion_retirada',
      mensaje: `${propuesta.absorbenteId} retira su propuesta de anexión a ${propuesta.absorbidaId}.`,
      payload: carga(propuesta) satisfies PayloadAnexionPropuesta,
    }),
  ]);
});
