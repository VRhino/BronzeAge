// Comandos de ejércitos (Doc 5.12): componer una fuerza y sacarla al mapa.
//
// Estos comandos NO mueven nada — eso es `avanzarEjercitos` en el tick. Aquí solo se decide quién sale, con
// qué, y hacia dónde; el ejército queda creado, en `marchando`, con su ruta ya calculada.
//
// Sustituyen a los comandos de combate como superficie de jugador (Doc 5.12.3): el jugador deja de "atacar a
// X" y pasa a "mandar un ejército a X"; el combate lo dispara la llegada o la proximidad, dentro del tick.
// `combateCampoAbierto` e `interceptarCaravana` YA se retiraron (Paso 11, 2026-09-04); `iniciarAsedio` sigue
// vivo como vía directa entre dos asentamientos vecinos, que no exige movilizar.
import { LOGISTICA } from '../../constants';
import { distancia as distanciaEntre } from '../../world/geometria';
import { aplicarAjustesReputacion } from '../../engine/reputacion';
import { EntregaInvalidaError, entregarDesdeCaravanaAdjunta } from '../../engine/trade';
import {
  cargarCaravanaAdjunta as cargarCaravanaEngine,
  ladoPendienteParaEjercito,
  movilizarEjercito as movilizarEngine,
  replegarEjercito as replegarEngine,
  estacionarEjercito as estacionarEngine,
  unirseAEjercito as unirseEngine,
  adjuntarCaravana as adjuntarCaravanaEngine,
  soltarCaravana as soltarCaravanaEngine,
  type ObjetivoEjercito,
} from '../../engine/ejercitos';
import { liderazgoComprometido } from '../../engine/liderazgo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito, sinCambios } from './tipos';
import { comando, exigirAsentamiento, exigirCaravana, exigirEjercito, conAsentamiento } from './ayudas';
import { evento, eventos } from './eventos';

function conEjercito(estado: GameSessionState, actualizado: GameSessionState['ejercitos'][number]): GameSessionState {
  return { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === actualizado.id ? actualizado : e)) };
}

function conCaravana(estado: GameSessionState, actualizada: GameSessionState['caravanas'][number]): GameSessionState {
  return { ...estado, caravanas: estado.caravanas.map((c) => (c.id === actualizada.id ? actualizada : c)) };
}

function jugadorDe(estado: GameSessionState, jugadorId: string) {
  return estado.jugadores.find((j) => j.id === jugadorId);
}

export interface PayloadEjercitoMovilizado {
  ejercitoId: string;
  origenAsentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
  liderazgoUsado: number;
  objetivo: ObjetivoEjercito;
  /** Trigo que el carro se llevó del almacén (Doc 5.13). Puede ser 0: la ciudad iba justa y se sale sin
   * autonomía, que es lo que el diseño manda en vez de bloquear la salida. */
  trigoCargado: number;
}

export interface ParamsMovilizarEjercito {
  asentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
  objetivo: ObjetivoEjercito;
}

export const movilizarEjercito = comando<ParamsMovilizarEjercito, { ejercitoId: string }>((estado, mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const ejercitoId = ctx.ids.siguiente();

  const { asentamiento: origen, ejercito, trigoCargado } = movilizarEngine(
    asentamiento,
    jugadorDe(estado, params.jugadorId),
    params.jugadorId,
    params.escuadronIds,
    params.objetivo,
    estado.asentamientos,
    mapa,
    `ejercito-${ejercitoId}`,
    ctx.instante
  );

  const destino = params.objetivo.tipo === 'asentamiento' ? params.objetivo.id : 'un punto del mapa';
  // El carro vacío no es un detalle de contabilidad: es la diferencia entre una campaña y una marcha que se
  // deshace por hambre a los pocos ticks, así que se dice en el propio mensaje y no solo en el payload.
  const conElCarro =
    trigoCargado > 0 ? `con ${Math.floor(trigoCargado)} de trigo en el carro` : 'CON EL CARRO VACÍO (el almacén no da más sin dejar la ciudad en riesgo)';
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, origen),
    ejercitos: [...estado.ejercitos, ejercito],
  };

  return exito(
    conHistorialDeJugador(siguiente, params.jugadorId, `Sale de campaña desde ${asentamiento.id} hacia ${destino}.`),
    [
      evento(ctx, {
        codigo: 'ejercito.movilizado',
        mensaje: `${params.jugadorId} sale de ${asentamiento.id} con ${ejercito.escuadrones.length} escuadrón(es) hacia ${destino}, ${conElCarro}.`,
        payload: {
          ejercitoId: ejercito.id,
          origenAsentamientoId: asentamiento.id,
          jugadorId: params.jugadorId,
          escuadronIds: [...params.escuadronIds],
          liderazgoUsado: liderazgoComprometido(ejercito.escuadrones),
          objetivo: params.objetivo,
          trigoCargado,
        } satisfies PayloadEjercitoMovilizado,
        asentamientoId: asentamiento.id,
      }),
    ],
    { ejercitoId: ejercito.id }
  );
});

export interface PayloadEjercitoRefuerzo {
  ejercitoId: string;
  asentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
  /** Trigo que el que se une aporta al carro común, tomado de SU asentamiento (Doc 5.13). */
  trigoCargado: number;
}

export interface ParamsUnirseAEjercito {
  ejercitoId: string;
  asentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
}

export const unirseAEjercito = comando<ParamsUnirseAEjercito, void>((estado, _mapa, ctx, params) => {
  const ejercitoActual = exigirEjercito(estado, params.ejercitoId);
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const { asentamiento: origen, ejercito, trigoCargado } = unirseEngine(
    ejercitoActual,
    asentamiento,
    jugadorDe(estado, params.jugadorId),
    params.jugadorId,
    params.escuadronIds,
    ctx.instante,
    estado.caravanas
  );

  const siguiente = conEjercito(conAsentamiento(estado, origen), ejercito);
  return exito(
    conHistorialDeJugador(siguiente, params.jugadorId, `Se une al ejército ${ejercito.id} desde ${asentamiento.id}.`),
    [
      evento(ctx, {
        codigo: 'ejercito.refuerzo',
        mensaje: `${params.jugadorId} refuerza el ejército ${ejercito.id} con ${params.escuadronIds.length} escuadrón(es) y ${Math.floor(trigoCargado)} de trigo.`,
        payload: {
          ejercitoId: ejercito.id,
          asentamientoId: asentamiento.id,
          jugadorId: params.jugadorId,
          escuadronIds: [...params.escuadronIds],
          trigoCargado,
        } satisfies PayloadEjercitoRefuerzo,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});

export interface PayloadEjercitoRepliegue {
  ejercitoId: string;
  /** Desde qué estado se ordenó: `marchando` es cancelar la marcha a media ruta, `estacionado` es levantar
   * el campamento. Misma operación, distinta historia (Doc 5.12.6). */
  desdeEstado: 'marchando' | 'estacionado';
}

export interface ParamsReplegarEjercito {
  ejercitoId: string;
}

/** Replegar y "cancelar la marcha" son la MISMA operación desde estados distintos (Doc 5.12.6), así que hay
 * un solo comando: marchando da media vuelta, estacionado calcula ruta nueva a casa. */
export const replegarEjercito = comando<ParamsReplegarEjercito, void>((estado, mapa, ctx, params) => {
  const ejercitoActual = exigirEjercito(estado, params.ejercitoId);
  const origen = estado.asentamientos.find((a) => a.id === ejercitoActual.origenAsentamientoId);
  const desdeEstado = ejercitoActual.estado;

  const ejercito = replegarEngine(ejercitoActual, origen, mapa);

  return exito(conEjercito(estado, ejercito), [
    evento(ctx, {
      codigo: 'ejercito.repliegue',
      mensaje:
        desdeEstado === 'marchando'
          ? `El ejército ${ejercito.id} cancela la marcha y vuelve a ${ejercito.origenAsentamientoId}.`
          : `El ejército ${ejercito.id} levanta el campamento y vuelve a ${ejercito.origenAsentamientoId}.`,
      payload: { ejercitoId: ejercito.id, desdeEstado: desdeEstado as 'marchando' | 'estacionado' } satisfies PayloadEjercitoRepliegue,
    }),
  ]);
});

export interface PayloadEjercitoEstacionado {
  ejercitoId: string;
}

export interface ParamsEstacionarEjercito {
  ejercitoId: string;
}

export const estacionarEjercito = comando<ParamsEstacionarEjercito, void>((estado, _mapa, ctx, params) => {
  const ejercito = estacionarEngine(exigirEjercito(estado, params.ejercitoId));

  return exito(conEjercito(estado, ejercito), [
    evento(ctx, {
      codigo: 'ejercito.estacionado',
      mensaje: `El ejército ${ejercito.id} acampa y pasa a consumo reducido.`,
      payload: { ejercitoId: ejercito.id } satisfies PayloadEjercitoEstacionado,
    }),
  ]);
});

export interface PayloadReabastecerAliados {
  asentamientoId: string;
  permitido: boolean;
}

export interface ParamsAlternarReabastecerAliados {
  asentamientoId: string;
  /** `true` abre el almacén a los ejércitos aliados; `false` lo cierra. */
  permitido: boolean;
}

/**
 * Abre o cierra el almacén de un asentamiento a los ejércitos de sus ALIADOS (Doc 5.13, Paso 8).
 *
 * Es una decisión de la plaza que da, no del que pasa: repostar le cuesta stock real, y por eso los ejércitos
 * propios entran siempre y los aliados solo con esto activo. Cerrar no tiene efecto retroactivo — lo ya
 * repuesto está repuesto; a partir del siguiente tick la puerta está cerrada.
 */
export const alternarReabastecerAliados = comando<ParamsAlternarReabastecerAliados, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  if ((asentamiento.permiteReabastecerAliados ?? false) === params.permitido) return sinCambios(estado);

  const actualizado = { ...asentamiento, permiteReabastecerAliados: params.permitido };
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'logistica.reabastecer_aliados',
      mensaje: `${asentamiento.id} ${params.permitido ? 'abre' : 'cierra'} su almacén a los ejércitos aliados.`,
      payload: { asentamientoId: asentamiento.id, permitido: params.permitido } satisfies PayloadReabastecerAliados,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface PayloadCaravanaAdjunta {
  ejercitoId: string;
  caravanaId: string;
  jugadorId: string;
}

export interface ParamsAdjuntarCaravana {
  ejercitoId: string;
  caravanaId: string;
  jugadorId: string;
}

/** Engancha una caravana propia al ejército como tren de suministros (Doc 5.13.2). Las condiciones —misma
 * Facción, disponible, al alcance— viven en el motor (`adjuntarCaravana`); aquí solo se resuelve y se narra. */
export const adjuntarCaravana = comando<ParamsAdjuntarCaravana, void>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);
  const caravana = exigirCaravana(estado, params.caravanaId);
  const origen = estado.asentamientos.find((a) => a.id === caravana.origenAsentamientoId);

  const r = adjuntarCaravanaEngine(ejercito, caravana, origen);
  return exito(conCaravana(conEjercito(estado, r.ejercito), r.caravana), [
    evento(ctx, {
      codigo: 'ejercito.caravana_adjuntada',
      mensaje: `La caravana ${caravana.id} se engancha al ejército ${ejercito.id}.`,
      payload: { ejercitoId: ejercito.id, caravanaId: caravana.id, jugadorId: params.jugadorId } satisfies PayloadCaravanaAdjunta,
      asentamientoId: ejercito.origenAsentamientoId,
    }),
  ]);
});

export interface ParamsSoltarCaravana {
  ejercitoId: string;
  caravanaId: string;
  jugadorId: string;
}

/** Suelta una caravana del ejército; se queda donde esté la columna (Doc 5.13.2). */
export const soltarCaravana = comando<ParamsSoltarCaravana, void>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);

  const caravana = exigirCaravana(estado, params.caravanaId);
  const r = soltarCaravanaEngine(ejercito, caravana);
  return exito(conCaravana(conEjercito(estado, r.ejercito), r.caravana), [
    evento(ctx, {
      codigo: 'ejercito.caravana_soltada',
      mensaje: `La caravana ${params.caravanaId} se desengancha del ejército ${ejercito.id}.`,
      payload: { ejercitoId: ejercito.id, caravanaId: params.caravanaId, jugadorId: params.jugadorId } satisfies PayloadCaravanaAdjunta,
      asentamientoId: ejercito.origenAsentamientoId,
    }),
  ]);
});

export interface PayloadCargaCaravana {
  ejercitoId: string;
  caravanaId: string;
  asentamientoId: string;
  recurso: string;
  cargado: number;
}

export interface ParamsCargarCaravana {
  ejercitoId: string;
  caravanaId: string;
  /** De qué plaza se carga — tiene que estar al alcance y abrirle el almacén al ejército. */
  asentamientoId: string;
  recurso: string;
  cantidad: number;
}

/**
 * Carga mercancía en una caravana escoltada (Doc 5.13.3). El jugador elige QUÉ lleva: una caravana enganchada
 * ya no la reparte el comercio automático.
 */
export const cargarCaravana = comando<ParamsCargarCaravana, { cargado: number }>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);
  const caravana = exigirCaravana(estado, params.caravanaId);
  const plaza = exigirAsentamiento(estado, params.asentamientoId);

  const r = cargarCaravanaEngine(ejercito, caravana, plaza, params.recurso, params.cantidad, estado.relaciones);
  const siguiente = conCaravana(conAsentamiento(estado, r.plaza), r.caravana);
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'ejercito.caravana_cargada',
        mensaje: `La caravana ${caravana.id} carga ${r.cargado.toFixed(0)} ${params.recurso} en ${plaza.id}.`,
        payload: {
          ejercitoId: ejercito.id,
          caravanaId: caravana.id,
          asentamientoId: plaza.id,
          recurso: params.recurso,
          cargado: r.cargado,
        } satisfies PayloadCargaCaravana,
        asentamientoId: plaza.id,
      }),
    ],
    { cargado: r.cargado }
  );
});

export interface ParamsEntregarDeCaravana {
  ejercitoId: string;
  caravanaId: string;
  acuerdoId: string;
}

/**
 * Entrega manual desde una caravana escoltada a un trueque activo (Doc 5.13.3).
 *
 * El motor resuelve de qué lado está el ejército y cuánto falta (`ladoPendienteParaEjercito`) — es la misma
 * consulta con la que la interfaz pinta la lista de trueques y su faltante. Aquí solo se comprueba la
 * geografía (estar al alcance del que RECIBE) y se narra.
 */
export const entregarDeCaravana = comando<ParamsEntregarDeCaravana, { entregado: number; comision: number }>(
  (estado, _mapa, ctx, params) => {
    const ejercito = exigirEjercito(estado, params.ejercitoId);
    const caravana = exigirCaravana(estado, params.caravanaId);
    const acuerdo = estado.acuerdos.find((a) => a.id === params.acuerdoId);
    if (!acuerdo) throw new EntregaInvalidaError(`El trueque ${params.acuerdoId} no existe.`);
    if (!ejercito.caravanasAdjuntasIds.includes(caravana.id)) {
      throw new EntregaInvalidaError('Esa caravana no va con este ejército.');
    }

    const pendiente = ladoPendienteParaEjercito(ejercito, acuerdo, estado.asentamientos);
    if (!pendiente) throw new EntregaInvalidaError('Este ejército no tiene nada pendiente en ese trueque.');

    const destino = exigirAsentamiento(estado, pendiente.destinoId);
    if (distanciaEntre(ejercito.posicionActual, destino.posicion) > LOGISTICA.radioReabastecimiento) {
      throw new EntregaInvalidaError(`El ejército está demasiado lejos de ${destino.id} para entregar.`);
    }
    const origen = exigirAsentamiento(estado, pendiente.lado === 'A' ? acuerdo.asentamientoAId : acuerdo.asentamientoBId);

    const r = entregarDesdeCaravanaAdjunta(
      caravana,
      acuerdo,
      pendiente.lado,
      pendiente.recurso,
      pendiente.faltante,
      destino,
      origen,
      estado.asentamientos,
      estado.facciones
    );

    const siguiente: GameSessionState = {
      ...conCaravana(conAsentamiento(estado, r.destino), r.caravana),
      acuerdos: estado.acuerdos.map((a) => (a.id === r.acuerdo.id ? r.acuerdo : a)),
      facciones: aplicarAjustesReputacion(estado.facciones, r.ajustesReputacion),
    };

    return exito(siguiente, eventos(ctx, r.eventos.map((e) => (typeof e === 'string' ? { codigo: 'legado', mensaje: e } : e))), {
      entregado: r.entregado,
      comision: r.comision,
    });
  }
);
