// Quién va dentro de una columna y quién la manda (Doc 5.14): unirse en campo, separarse, y el Líder.
//
// Va aparte de `presencia.ts` porque responde a otra pregunta. Aquélla mueve a UN jugador entre los tres
// sitios donde puede estar; ésta cambia la COMPOSICIÓN de una columna compartida, que es una decisión con
// más de un dueño: el que pide, el que manda, y los que ya van dentro.
//
// El eje que lo ordena todo es el precio del paso 4: un viajero solo rectifica su rumbo cuando quiere, y
// unirse a un ejército lo cambia por un destino que ya no puede tocar. Separarse se lo devuelve al instante.
// Ninguna regla extra impone ese coste — sale de que el ejército tiene el rumbo fijo (Doc 5.12.1).
import { MOVIMIENTO } from '../../constants';
import { segundos, sumar } from '../../domain/tiempo';
import {
  anotarPeticionDeUnion,
  cederLiderazgo as cederEngine,
  retirarPeticionDeUnion,
  separarseDelEjercito as separarseEngine,
  unirseEnCampo as unirseEnCampoEngine,
} from '../../engine/ejercitos';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, exigirColumnaDe, exigirEjercito } from './ayudas';
import { evento } from './eventos';

export interface ParamsUnirseEnCampo {
  ejercitoId: string;
  jugadorId: string;
}

export interface ParamsResponderPeticion {
  ejercitoId: string;
  jugadorId: string;
  solicitanteId: string;
  aceptar: boolean;
}

export interface ParamsSepararse {
  jugadorId: string;
}

export interface ParamsCederLiderazgo {
  ejercitoId: string;
  jugadorId: string;
  sucesorId: string;
}

export interface PayloadComposicionColumna {
  ejercitoId: string;
  jugadorId: string;
}

export interface PayloadPeticionDeUnion {
  ejercitoId: string;
  jugadorId: string;
  /** Hasta cuándo sirve. Va en el payload para que el cliente pueda mostrar la cuenta atrás sin inventársela. */
  expiraEn: number;
}

export interface PayloadSeparacion {
  ejercitoId: string;
  columnaId: string;
  jugadorId: string;
}

export interface PayloadLiderazgoCedido {
  ejercitoId: string;
  anteriorId: string;
  sucesorId: string;
}

function conEjercitos(estado: GameSessionState, cambiados: GameSessionState['ejercitos']): GameSessionState {
  const porId = new Map(cambiados.map((e) => [e.id, e]));
  return { ...estado, ejercitos: estado.ejercitos.map((e) => porId.get(e.id) ?? e) };
}

/**
 * Unirse en campo a un Ejército que tienes delante (Doc 5.14.1), aportando lo que ya llevas encima.
 *
 * Lo que hace o no hace lo decide la política que su Líder fijó al parir la columna:
 *
 * - **aceptar**: entras.
 * - **rechazar**: no, y no hay nada que negociar.
 * - **preguntar**: se le deja al Líder una petición que **vive 10 segundos**. Si no contesta, caduca — y el
 *   silencio cuenta como un no.
 */
export const unirseEnCampo = comando<ParamsUnirseEnCampo, { unido: boolean }>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);
  const columna = exigirColumnaDe(estado, params.jugadorId);

  if (ejercito.politicaDeUnion === 'preguntar') {
    const expiraEn = sumar(ctx.instante, segundos(MOVIMIENTO.vidaPeticionUnionSegundos));
    const conPeticion = anotarPeticionDeUnion(ejercito, columna, ctx.instante, expiraEn);

    return exito(
      conHistorialDeJugador(conEjercitos(estado, [conPeticion]), params.jugadorId, `Pide unirse al ejército ${ejercito.id}.`),
      [
        evento(ctx, {
          codigo: 'columna.union_pedida',
          mensaje: `Un jugador pide unirse al ejército ${ejercito.id}.`,
          payload: { ejercitoId: ejercito.id, jugadorId: params.jugadorId, expiraEn } satisfies PayloadPeticionDeUnion,
          asentamientoId: ejercito.origenAsentamientoId,
        }),
      ],
      { unido: false }
    );
  }

  const fundido = unirseEnCampoEngine(ejercito, columna, ctx.instante);
  const siguiente: GameSessionState = {
    ...conEjercitos(estado, [fundido]),
    ejercitos: conEjercitos(estado, [fundido]).ejercitos.filter((e) => e.id !== columna.id),
    jugadores: estado.jugadores.map((j) =>
      j.id === params.jugadorId ? { ...j, ubicacion: { tipo: 'columna' as const, ejercitoId: fundido.id } } : j
    ),
  };

  return exito(
    conHistorialDeJugador(siguiente, params.jugadorId, `Se une en campo al ejército ${ejercito.id} y adopta su destino.`),
    [
      evento(ctx, {
        codigo: 'columna.union_en_campo',
        mensaje: `Un jugador se une en campo al ejército ${ejercito.id}.`,
        payload: { ejercitoId: ejercito.id, jugadorId: params.jugadorId } satisfies PayloadComposicionColumna,
        asentamientoId: ejercito.origenAsentamientoId,
      }),
    ],
    { unido: true }
  );
});

/**
 * El Líder contesta una petición (Doc 5.14.1). Comprueba la caducidad **al leer** —nada se disparó a los 10
 * segundos— y revalida la geometría: el que entra tiene que estar junto a la columna AHORA, no donde estaba
 * cuando lo pidió.
 */
export const responderPeticionDeUnion = comando<ParamsResponderPeticion, { unido: boolean }>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);
  const sinLaPeticion = retirarPeticionDeUnion(ejercito, params.jugadorId, params.solicitanteId, ctx.instante);

  if (!params.aceptar) {
    return exito(
      conHistorialDeJugador(conEjercitos(estado, [sinLaPeticion]), params.jugadorId, `Rechaza a un jugador en ${ejercito.id}.`),
      [
        evento(ctx, {
          codigo: 'columna.union_rechazada',
          mensaje: `El Líder de ${ejercito.id} rechaza a un jugador.`,
          payload: { ejercitoId: ejercito.id, jugadorId: params.solicitanteId } satisfies PayloadComposicionColumna,
          asentamientoId: ejercito.origenAsentamientoId,
        }),
      ],
      { unido: false }
    );
  }

  const columna = exigirColumnaDe(estado, params.solicitanteId);
  const fundido = unirseEnCampoEngine(sinLaPeticion, columna, ctx.instante);

  const siguiente: GameSessionState = {
    ...conEjercitos(estado, [fundido]),
    ejercitos: conEjercitos(estado, [fundido]).ejercitos.filter((e) => e.id !== columna.id),
    jugadores: estado.jugadores.map((j) =>
      j.id === params.solicitanteId ? { ...j, ubicacion: { tipo: 'columna' as const, ejercitoId: fundido.id } } : j
    ),
  };

  return exito(
    conHistorialDeJugador(siguiente, params.solicitanteId, `Es aceptado en el ejército ${ejercito.id}.`),
    [
      evento(ctx, {
        codigo: 'columna.union_en_campo',
        mensaje: `El Líder de ${ejercito.id} acepta a un jugador.`,
        payload: { ejercitoId: ejercito.id, jugadorId: params.solicitanteId } satisfies PayloadComposicionColumna,
        asentamientoId: ejercito.origenAsentamientoId,
      }),
    ],
    { unido: true }
  );
});

/**
 * Separarse de un Ejército (Doc 5.14.2): sales con lo tuyo y naces como columna personal donde estabas,
 * recuperando la libertad de movimiento al instante.
 *
 * Ni el Líder ni el último que queda pueden hacerlo. Las dos prohibiciones sostienen la misma regla —una
 * columna nunca se queda vacía en campo abierto— y por eso el motor las comprueba por separado.
 */
export const separarseDelEjercito = comando<ParamsSepararse, { columnaId: string }>((estado, _mapa, ctx, params) => {
  const ejercito = exigirColumnaDe(estado, params.jugadorId);

  const columnaId = `ejercito-${ctx.ids.siguiente()}`;
  const separado = separarseEngine(ejercito, params.jugadorId, columnaId);

  const siguiente: GameSessionState = {
    ...conEjercitos(estado, [separado.ejercito]),
    ejercitos: [...conEjercitos(estado, [separado.ejercito]).ejercitos, separado.columna],
    jugadores: estado.jugadores.map((j) =>
      j.id === params.jugadorId ? { ...j, ubicacion: { tipo: 'columna' as const, ejercitoId: separado.columna.id } } : j
    ),
  };

  return exito(
    conHistorialDeJugador(siguiente, params.jugadorId, `Se separa del ejército ${ejercito.id} y sigue por libre.`),
    [
      evento(ctx, {
        codigo: 'columna.separacion',
        mensaje: `Un jugador se separa del ejército ${ejercito.id}.`,
        payload: {
          ejercitoId: ejercito.id,
          columnaId: separado.columna.id,
          jugadorId: params.jugadorId,
        } satisfies PayloadSeparacion,
        asentamientoId: ejercito.origenAsentamientoId,
      }),
    ],
    { columnaId: separado.columna.id }
  );
});

/**
 * Elevar a otro integrante a Líder (Doc 5.14.3). Es el único camino para que el Líder pueda irse: primero
 * cede, después ya puede separarse como cualquiera.
 */
export const cederLiderazgo = comando<ParamsCederLiderazgo, void>((estado, _mapa, ctx, params) => {
  const ejercito = exigirEjercito(estado, params.ejercitoId);
  const cedido = cederEngine(ejercito, params.jugadorId, params.sucesorId);

  return exito(
    conHistorialDeJugador(conEjercitos(estado, [cedido]), params.jugadorId, `Cede el mando de ${ejercito.id}.`),
    [
      evento(ctx, {
        codigo: 'columna.liderazgo_cedido',
        mensaje: `El mando del ejército ${ejercito.id} cambia de manos.`,
        payload: {
          ejercitoId: ejercito.id,
          anteriorId: params.jugadorId,
          sucesorId: params.sucesorId,
        } satisfies PayloadLiderazgoCedido,
        asentamientoId: ejercito.origenAsentamientoId,
      }),
    ]
  );
});
