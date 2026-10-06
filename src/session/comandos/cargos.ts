// Comandos de cargos y ciudadanía: Rey y Embajador (Facción), cargos locales de asentamiento, residencia
// y activación de políticas. Agrupados en un archivo porque comparten la misma forma —localizar la entidad,
// delegar en el motor, registrar— y separarlos en cinco archivos de 25 líneas sería ruido sin beneficio.
//
// Ninguno de estos comandos produce eventos en el motor (devuelve la entidad actualizada y nada más), así que
// los narra esta capa entera: es la que sabe a quién se nombró y en qué Facción.
import type { CargoTipo } from '../../domain/types';
import { designarCapital as designarCapitalEngine } from '../../engine/capital';
import { acogerEnCampamentoMasCercano, posicionDeHeroe, residirEnCampamento as residirEnCampamentoEngine, salirDeCampamentos } from '../../engine/mercenarios';
import { asignarCargoLocal as asignarCargoLocalEngine, asignarEmbajador as asignarEmbajadorEngine, asignarRey as asignarReyEngine } from '../../engine/cargos';
import {
  cambiarResidencia as cambiarResidenciaEngine,
  dejarResidencia as dejarResidenciaEngine,
  exigirSinCooldownDeResidencia,
} from '../../engine/faccion';
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
export interface PayloadResidenciaCambiada {
  heroeId: string;
  /** Ausente si no residía en ninguna plaza (vivía en un campamento de mercenarios). */
  origenId?: string;
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

export interface ParamsCambiarResidencia {
  destinoId: string;
  heroeId: string;
}

export const cambiarResidencia = comando<ParamsCambiarResidencia, void>((estado, _mapa, ctx, params) => {
  exigirSinCooldownDeResidencia(estado.cambiosResidenciaPorHeroe?.[params.heroeId], ctx.instante);
  const { origen, destino } = cambiarResidenciaEngine(estado.facciones, estado.asentamientos, params.destinoId, params.heroeId);
  const desde = origen ? origen.id : 'su campamento';
  // El campamento se muda con él, pero la guarnición era de la plaza que deja (Doc 5.15.3). Se reside en un solo sitio: deja el
  // campamento de mercenarios donde viviera.
  const siguiente = conHistorialDeJugador(
    {
      ...conAsentamientos(estado, origen ? [origen, destino] : [destino]),
      campamentosMercenarios: salirDeCampamentos(estado.campamentosMercenarios, params.heroeId),
      heroes: sinGuarnicion(estado.heroes, params.heroeId),
      cambiosResidenciaPorHeroe: { ...estado.cambiosResidenciaPorHeroe, [params.heroeId]: ctx.instante },
    },
    params.heroeId,
    `Cambia su residencia de ${desde} a ${destino.id}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'ciudadania.residencia_cambiada',
      mensaje: `${params.heroeId} deja de residir en ${desde} y se muda a ${destino.id}.`,
      payload: { heroeId: params.heroeId, ...(origen ? { origenId: origen.id } : {}), destinoId: destino.id } satisfies PayloadResidenciaCambiada,
      asentamientoId: destino.id,
    }),
  ]);
});

export interface ParamsDejarResidencia {
  heroeId: string;
}

export interface PayloadResidenciaDejada {
  heroeId: string;
  asentamientoId: string;
}

/** Dejar la casa sin dejar la Facción (Doc 2.5): el héroe pasa, en el acto, al campamento de mercenarios más cercano. */
export const dejarResidencia = comando<ParamsDejarResidencia, void>((estado, _mapa, ctx, params) => {
  const origen = dejarResidenciaEngine(estado.asentamientos, params.heroeId);
  // Sin casa no se queda: al campamento de mercenarios más cercano a donde está (o a la plaza que deja, si no se sabe dónde).
  const heroe = estado.heroes.find((h) => h.id === params.heroeId);
  const desde = (heroe && posicionDeHeroe(heroe, estado.asentamientos, estado.ejercitos)) ?? origen.posicion;
  const siguiente = conHistorialDeJugador(
    {
      ...conAsentamiento(estado, origen),
      campamentosMercenarios: acogerEnCampamentoMasCercano(estado.campamentosMercenarios, [params.heroeId], desde),
      heroes: sinGuarnicion(estado.heroes, params.heroeId),
      cambiosResidenciaPorHeroe: { ...estado.cambiosResidenciaPorHeroe, [params.heroeId]: ctx.instante },
    },
    params.heroeId,
    `Deja su residencia en ${origen.id}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'ciudadania.residencia_dejada',
      mensaje: `${params.heroeId} deja de residir en ${origen.id}.`,
      payload: { heroeId: params.heroeId, asentamientoId: origen.id } satisfies PayloadResidenciaDejada,
      asentamientoId: origen.id,
    }),
  ]);
});

export interface ParamsResidirEnCampamento {
  heroeId: string;
  campamentoId: string;
}

export interface PayloadResidenciaEnCampamento {
  heroeId: string;
  campamentoId: string;
}

/**
 * Residir en un campamento de mercenarios (Doc 2.5): cualquier héroe, de cualquier Facción, aunque la suya tenga asentamientos.
 * Deja la casa que tuviera (con sus cargos locales) y suelta la guarnición; cuenta para el cooldown de residencia.
 */
export const residirEnCampamento = comando<ParamsResidirEnCampamento, void>((estado, _mapa, ctx, params) => {
  exigirSinCooldownDeResidencia(estado.cambiosResidenciaPorHeroe?.[params.heroeId], ctx.instante);
  const r = residirEnCampamentoEngine(estado.campamentosMercenarios, estado.asentamientos, params.heroeId, params.campamentoId);
  const siguiente = conHistorialDeJugador(
    {
      ...estado,
      campamentosMercenarios: r.campamentos,
      asentamientos: r.asentamientos,
      heroes: sinGuarnicion(estado.heroes, params.heroeId),
      cambiosResidenciaPorHeroe: { ...estado.cambiosResidenciaPorHeroe, [params.heroeId]: ctx.instante },
    },
    params.heroeId,
    `Pasa a residir en el campamento de mercenarios ${params.campamentoId}.`
  );
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'ciudadania.residencia_en_campamento',
      mensaje: `${params.heroeId} pasa a residir en el campamento de mercenarios ${params.campamentoId}.`,
      payload: { heroeId: params.heroeId, campamentoId: params.campamentoId } satisfies PayloadResidenciaEnCampamento,
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

export interface ParamsDesignarCapital {
  faccionId: string;
  asentamientoId: string;
}

export interface PayloadCapitalDesignada {
  faccionId: string;
  asentamientoId: string;
}

export interface ParamsAdmitirOtrasFacciones {
  faccionId: string;
  admitir: boolean;
}

/** El Rey decide si los ataques de su Facción admiten a héroes de otras (Doc 2.2, 5.15.1b): un ajuste permanente. */
export const admitirOtrasFacciones = comando<ParamsAdmitirOtrasFacciones, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);
  const siguiente = conFaccion(estado, { ...faccion, admiteOtrasEnAtaques: params.admitir });
  const plaza = estado.asentamientos.find((a) => a.faccionId === faccion.id);
  return exito(
    siguiente,
    plaza
      ? [
          evento(ctx, {
            codigo: 'faccion.admision_en_ataques',
            mensaje: `${faccion.nombre} ${params.admitir ? 'admite' : 'ya no admite'} a otras Facciones en sus ataques.`,
            payload: { faccionId: faccion.id, admitir: params.admitir },
            asentamientoId: plaza.id,
          }),
        ]
      : []
  );
});

/** El Rey designa la capital de su Facción (Doc 2.2): Palacio activo y cooldown entre traslados. */
export const designarCapital = comando<ParamsDesignarCapital, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);
  const resultado = designarCapitalEngine(faccion, estado.asentamientos, params.asentamientoId, ctx.instante);
  const siguiente = conAsentamientos(conFaccion(estado, resultado.faccion), resultado.asentamientos);
  return exito(siguiente, [
    evento(ctx, {
      codigo: 'faccion.capital_designada',
      mensaje: `${faccion.nombre} traslada su capital a ${params.asentamientoId}.`,
      payload: { faccionId: faccion.id, asentamientoId: params.asentamientoId } satisfies PayloadCapitalDesignada,
      asentamientoId: params.asentamientoId,
    }),
  ]);
});
