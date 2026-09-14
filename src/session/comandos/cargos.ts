// Comandos de cargos y ciudadanía: Rey y Embajador (Facción), cargos locales de asentamiento, compra de casa
// y activación de políticas. Agrupados en un archivo porque comparten la misma forma —localizar la entidad,
// delegar en el motor, registrar— y separarlos en cinco archivos de 25 líneas sería ruido sin beneficio.
//
// Ninguno de estos comandos produce eventos en el motor (devuelve la entidad actualizada y nada más), así que
// los narra esta capa entera: es la que sabe a quién se nombró y en qué Facción.
import type { CargoTipo } from '../../domain/types';
import { asignarCargoLocal as asignarCargoLocalEngine, asignarEmbajador as asignarEmbajadorEngine, asignarRey as asignarReyEngine } from '../../engine/cargos';
import { comprarCasa as comprarCasaEngine, cambiarResidencia as cambiarResidenciaEngine } from '../../engine/faccion';
import { activarPolitica as activarPoliticaEngine } from '../../engine/politicas';
import { sinGuarnicion } from '../../engine/tropa';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito, type ContextoComando, type TransicionComando } from './tipos';
import { comando, conAsentamiento, conAsentamientos, conFaccion, exigirAsentamiento, exigirFaccion, exigirFaccionDe } from './ayudas';
import { evento } from './eventos';

export interface PayloadCargoFaccion {
  faccionId: string;
  heroeId: string;
  cargo: 'rey' | 'embajador';
}
export interface PayloadCargoLocal {
  asentamientoId: string;
  heroeId: string;
  cargo: CargoTipo;
}
export interface PayloadCasaComprada {
  asentamientoId: string;
  heroeId: string;
}
export interface PayloadResidenciaCambiada {
  heroeId: string;
  origenId: string;
  destinoId: string;
}
export interface PayloadPoliticaActivada {
  asentamientoId: string;
  politicaId: string;
  cargo: CargoTipo;
}

export interface ParamsAsignarCargoFaccion {
  faccionId: string;
  heroeId: string;
}

/** Rey y Embajador comparten todo salvo la función del motor y el nombre del cargo. */
function asignarCargoDeFaccion(
  estado: GameSessionState,
  ctx: ContextoComando,
  params: ParamsAsignarCargoFaccion,
  cargo: 'rey' | 'embajador',
  nombreCargo: string,
  aplicar: (faccion: Parameters<typeof asignarReyEngine>[0], heroeId: string) => ReturnType<typeof asignarReyEngine>
): TransicionComando<void> {
  const faccion = exigirFaccion(estado, params.faccionId);
  const actualizada = aplicar(faccion, params.heroeId);

  const siguiente = conHistorialDeJugador(
    conFaccion(estado, actualizada),
    params.heroeId,
    `Nombrado ${nombreCargo} de ${faccion.nombre}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: `cargo.${cargo}_asignado`,
      mensaje: `${faccion.nombre}: ${params.heroeId} es el nuevo ${nombreCargo}.`,
      payload: { faccionId: faccion.id, heroeId: params.heroeId, cargo } satisfies PayloadCargoFaccion,
    }),
  ]);
}

export const asignarRey = comando<ParamsAsignarCargoFaccion, void>((estado, _mapa, ctx, params) =>
  asignarCargoDeFaccion(estado, ctx, params, 'rey', 'Rey', asignarReyEngine)
);

export const asignarEmbajador = comando<ParamsAsignarCargoFaccion, void>((estado, _mapa, ctx, params) =>
  asignarCargoDeFaccion(estado, ctx, params, 'embajador', 'Embajador', asignarEmbajadorEngine)
);

export interface ParamsAsignarCargoLocal {
  asentamientoId: string;
  cargo: CargoTipo;
  heroeId: string;
}

export const asignarCargoLocal = comando<ParamsAsignarCargoLocal, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const faccion = exigirFaccionDe(estado, asentamiento);

  const actualizado = asignarCargoLocalEngine(asentamiento, faccion, params.cargo, params.heroeId);
  const siguiente = conHistorialDeJugador(
    conAsentamiento(estado, actualizado),
    params.heroeId,
    `Asignado como ${params.cargo} en ${asentamiento.id}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'cargo.local_asignado',
      mensaje: `${params.heroeId} asignado como ${params.cargo}.`,
      payload: { asentamientoId: asentamiento.id, heroeId: params.heroeId, cargo: params.cargo } satisfies PayloadCargoLocal,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsComprarCasa {
  asentamientoId: string;
  heroeId: string;
}

export const comprarCasa = comando<ParamsComprarCasa, void>((estado, _mapa, ctx, params) => {
  // A diferencia del resto, este comando del motor resuelve el asentamiento por su cuenta y lanza
  // `FaccionInvalidaError` si no existe — no hace falta comprobarlo antes.
  const resultado = comprarCasaEngine(estado.facciones, estado.asentamientos, params.asentamientoId, params.heroeId);
  const siguiente = conHistorialDeJugador(
    { ...conAsentamiento(estado, resultado.asentamiento), facciones: resultado.facciones },
    params.heroeId,
    `Compra casa en ${params.asentamientoId} y obtiene ciudadanía.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'ciudadania.casa_comprada',
      mensaje: `${params.heroeId} compra casa en ${params.asentamientoId} y obtiene ciudadanía.`,
      payload: { asentamientoId: resultado.asentamiento.id, heroeId: params.heroeId } satisfies PayloadCasaComprada,
      asentamientoId: resultado.asentamiento.id,
    }),
  ]);
});

export interface ParamsCambiarResidencia {
  destinoId: string;
  heroeId: string;
}

export const cambiarResidencia = comando<ParamsCambiarResidencia, void>((estado, _mapa, ctx, params) => {
  const { origen, destino } = cambiarResidenciaEngine(estado.facciones, estado.asentamientos, params.destinoId, params.heroeId);
  // El campamento se muda con él, pero la guarnición era de la plaza que deja (Doc 5.15.3).
  const siguiente = conHistorialDeJugador(
    { ...conAsentamientos(estado, [origen, destino]), heroes: sinGuarnicion(estado.heroes, params.heroeId) },
    params.heroeId,
    `Cambia su residencia de ${origen.id} a ${destino.id}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'ciudadania.residencia_cambiada',
      mensaje: `${params.heroeId} deja de residir en ${origen.id} y se muda a ${destino.id}.`,
      payload: { heroeId: params.heroeId, origenId: origen.id, destinoId: destino.id } satisfies PayloadResidenciaCambiada,
      asentamientoId: destino.id,
    }),
  ]);
});

export interface ParamsActivarPolitica {
  asentamientoId: string;
  cargo: CargoTipo;
  politicaId: string;
}

export const activarPolitica = comando<ParamsActivarPolitica, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const faccion = exigirFaccionDe(estado, asentamiento);

  const actualizado = activarPoliticaEngine(asentamiento, faccion, params.cargo, params.politicaId, ctx.instante, ctx.ids.siguiente());
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'politica.activada',
      mensaje: `Política "${params.politicaId}" activada por ${params.cargo}.`,
      payload: { asentamientoId: asentamiento.id, politicaId: params.politicaId, cargo: params.cargo } satisfies PayloadPoliticaActivada,
      asentamientoId: asentamiento.id,
    }),
  ]);
});
