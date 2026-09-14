// Las acciones que la geometría OFRECE y el jugador decide (Doc 5.12.3).
//
// Este módulo es el reverso de una decisión de diseño: hasta ahora, acercarse a un enemigo bastaba para que
// el motor resolviera el choque dentro del tick. Ahora la proximidad solo ABRE opciones —mirar, perseguir,
// atacar— y ninguna ocurre sin que alguien la pida. Un encuentro deja de ser un accidente por pasar cerca.
//
// Los anillos, de fuera hacia dentro, y qué habilita cada uno:
//
//   150 / 80  ves que hay algo, y quién es
//    40       puedes INSPECCIONARLO — y él se entera de que lo miras
//    15       puedes atacarlo
//    10       puedes cruzar la puerta de una plaza (eso vive en `presencia.ts`)
//
// El de 40 es el que hace de esto un juego de dos: la telemetría que la proyección niega a distancia se
// consigue acercándose, y acercarse te delata. Nadie audita al rival desde el sofá.
import {
  atacarColumna,
  capacidadCargaDe,
  dejarDePerseguir as dejarDePerseguirEngine,
  inspeccionarCaravana as inspeccionarCaravanaEngine,
  inspeccionarColumna,
  interceptar,
  perseguir as perseguirEngine,
  type ComposicionColumna,
  type ContenidoCaravana,
} from '../../engine/ejercitos';
import { devolverEscoltaAGuarnicion } from '../../engine/caravanas';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, exigirCaravana, exigirColumnaDe, exigirEjercito } from './ayudas';
import { desdeCrudos, evento } from './eventos';

/** A qué se puede apuntar desde el menú de interacción: una columna o una caravana. Es la misma forma que usa
 * la persecución, y no por casualidad — se persigue lo que se puede mirar. */
export type ObjetivoDeInteraccion = { tipo: 'ejercito'; id: string } | { tipo: 'caravana'; id: string };

export interface ParamsInspeccionar {
  heroeId: string;
  objetivo: ObjetivoDeInteraccion;
}

export interface PayloadObservado {
  /** Quién mira. Va en el payload porque el aviso es la mitad de la mecánica: el observado tiene derecho a
   * saber que lo miran, y a poder decidir algo al respecto. */
  observadorId: string;
  objetivo: ObjetivoDeInteraccion;
}

/**
 * Mirar de cerca (Doc 5.12.3): qué tropas lleva esa columna y de quién son, o qué carga esa caravana.
 *
 * **No es gratis, y ahí está el diseño.** Hay que meterse en el anillo de 40 —dentro del alcance de una
 * columna que quiera cazarte— y el observado **recibe aviso**. La información que la proyección no regala a
 * distancia se compra acercándose y delatándose.
 *
 * El aviso se emite atribuido a la plaza de origen del observado, que es como esta partida decide audiencias:
 * le llega a su Facción, no al mundo.
 */
export const inspeccionar = comando<ParamsInspeccionar, ComposicionColumna | ContenidoCaravana>((estado, _mapa, ctx, params) => {
  const observador = exigirColumnaDe(estado, params.heroeId);

  if (params.objetivo.tipo === 'ejercito') {
    const objetivo = exigirEjercito(estado, params.objetivo.id);
    const composicion = inspeccionarColumna(observador, objetivo);

    return exito(
      conHistorialDeJugador(estado, params.heroeId, `Inspecciona la columna ${objetivo.id}.`),
      [
        evento(ctx, {
          codigo: 'columna.observada',
          mensaje: `Alguien se ha acercado a mirar la columna ${objetivo.id}.`,
          payload: { observadorId: params.heroeId, objetivo: params.objetivo } satisfies PayloadObservado,
          // Atribuido al OBSERVADO, no al que mira: el aviso es para quien lo sufre.
          asentamientoId: objetivo.origenAsentamientoId,
        }),
      ],
      composicion
    );
  }

  const caravana = exigirCaravana(estado, params.objetivo.id);
  // Escoltada = por un ejército (Doc 5.13.3) o por escuadrones cedidos sin héroe (Doc 3.13.4).
  const escoltada =
    estado.ejercitos.some((e) => e.caravanasAdjuntasIds.includes(caravana.id)) || (caravana.escolta?.length ?? 0) > 0;
  const contenido = inspeccionarCaravanaEngine(observador, caravana, escoltada);

  return exito(
    conHistorialDeJugador(estado, params.heroeId, `Inspecciona la caravana ${caravana.id}.`),
    [
      evento(ctx, {
        codigo: 'caravana.observada',
        mensaje: `Alguien se ha acercado a mirar la caravana ${caravana.id}.`,
        payload: { observadorId: params.heroeId, objetivo: params.objetivo } satisfies PayloadObservado,
        asentamientoId: caravana.origenAsentamientoId,
      }),
    ],
    contenido
  );
});

export interface ParamsAtacar {
  heroeId: string;
  objetivo: ObjetivoDeInteraccion;
}

export interface ParamsPerseguir {
  heroeId: string;
  objetivo: ObjetivoDeInteraccion;
}

export interface ParamsDejarDePerseguir {
  heroeId: string;
}

export interface PayloadPersecucion {
  ejercitoId: string;
  heroeId: string;
  objetivo?: ObjetivoDeInteraccion;
}

/**
 * Atacar lo que tienes delante, a distancia de choque (Doc 5.12.3). Sustituye al combate que el tick
 * resolvía solo por geometría: acercarse ya no basta.
 *
 * Al derrotado le cae la **tregua** y pierde la mitad de su carro, sea viajero o Ejército. Las dos mitades de la
 * tregua se comprueban en el motor: ni se ataca estando en ella, ni se ataca a quien la tiene.
 */
export const atacar = comando<ParamsAtacar, void>((estado, _mapa, ctx, params) => {
  const atacante = exigirColumnaDe(estado, params.heroeId);

  if (params.objetivo.tipo === 'ejercito') {
    const defensor = exigirEjercito(estado, params.objetivo.id);
    const choque = atacarColumna(
      atacante,
      defensor,
      [...estado.facciones],
      estado.relaciones,
      estado.caravanas,
      ctx.instante,
      ctx.rng
    );

    const porId = new Map([
      [choque.atacante.id, choque.atacante],
      [choque.defensor.id, choque.defensor],
    ]);
    const siguiente: GameSessionState = {
      ...estado,
      ejercitos: estado.ejercitos.map((e) => porId.get(e.id) ?? e),
      facciones: choque.facciones,
    };

    return exito(
      conHistorialDeJugador(siguiente, params.heroeId, `Ataca a la columna ${defensor.id}.`),
      // El combate se narra a los DOS hogares: el que lo sufre tiene tanto derecho a saberlo como el que lo
      // ordena, y sin la segunda atribución el atacado se enteraría por las bajas.
      [
        ...desdeCrudos(ctx, choque.eventos, atacante.origenAsentamientoId),
        ...desdeCrudos(ctx, choque.eventos, defensor.origenAsentamientoId),
      ]
    );
  }

  const caravana = exigirCaravana(estado, params.objetivo.id);
  const emboscada = interceptar(atacante, caravana, capacidadCargaDe(atacante, estado.caravanas), ctx.instante, ctx.rng);
  // Escolta sin héroe (Doc 3.13.4) que vuelve a la guarnición del origen tras perder la caravana.
  const asentamientos =
    emboscada.escoltaDevuelta.length > 0
      ? estado.asentamientos.map((a) =>
          a.id === caravana.origenAsentamientoId
            ? { ...a, escuadrones: devolverEscoltaAGuarnicion(a.escuadrones, emboscada.escoltaDevuelta) }
            : a
        )
      : estado.asentamientos;
  const siguiente: GameSessionState = {
    ...estado,
    asentamientos,
    ejercitos: estado.ejercitos.map((e) => (e.id === emboscada.ejercito.id ? emboscada.ejercito : e)),
    caravanas: emboscada.caravana
      ? estado.caravanas.map((c) => (c.id === caravana.id ? emboscada.caravana! : c))
      : estado.caravanas.filter((c) => c.id !== caravana.id),
  };

  return exito(
    conHistorialDeJugador(siguiente, params.heroeId, `Intercepta la caravana ${caravana.id}.`),
    [
      ...desdeCrudos(ctx, emboscada.eventos, atacante.origenAsentamientoId),
      ...desdeCrudos(ctx, emboscada.eventos, caravana.origenAsentamientoId),
    ]
  );
});

/**
 * Ir a por alguien (Doc 5.12.3). No es un destino sino un objetivo que se mueve: la ruta se recalcula cada
 * tick hacia donde esté, y al alcanzarlo hay combate — porque perseguir ES elegir el combate.
 *
 * Termina de cuatro formas: alcanzándolo, rectificando el rumbo con `marcharA`, soltándolo, o si el objetivo
 * entra en tregua.
 */
export const perseguir = comando<ParamsPerseguir, void>((estado, _mapa, ctx, params) => {
  const columna = exigirColumnaDe(estado, params.heroeId);
  // Que el objetivo exista lo comprueba aquí y no el motor: es una entidad que buscar, no una regla.
  if (params.objetivo.tipo === 'ejercito') exigirEjercito(estado, params.objetivo.id);
  else exigirCaravana(estado, params.objetivo.id);

  const cazando = perseguirEngine(columna, params.objetivo, ctx.instante);

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === cazando.id ? cazando : e)) },
      params.heroeId,
      `Sale en persecución de ${params.objetivo.id}.`
    ),
    [
      evento(ctx, {
        codigo: 'columna.persecucion_iniciada',
        mensaje: `Una columna sale en persecución de ${params.objetivo.id}.`,
        payload: { ejercitoId: cazando.id, heroeId: params.heroeId, objetivo: params.objetivo } satisfies PayloadPersecucion,
        asentamientoId: cazando.origenAsentamientoId,
      }),
    ]
  );
});

/** Soltar la presa. Lo hace también cualquier `marcharA`: elegir destino nuevo es dejar de ir detrás. */
export const dejarDePerseguir = comando<ParamsDejarDePerseguir, void>((estado, _mapa, ctx, params) => {
  const columna = exigirColumnaDe(estado, params.heroeId);
  const suelta = dejarDePerseguirEngine(columna);

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === suelta.id ? suelta : e)) },
      params.heroeId,
      'Abandona la persecución.'
    ),
    [
      evento(ctx, {
        codigo: 'columna.persecucion_abandonada',
        mensaje: 'Una columna abandona la persecución.',
        payload: { ejercitoId: suelta.id, heroeId: params.heroeId } satisfies PayloadPersecucion,
        asentamientoId: suelta.origenAsentamientoId,
      }),
    ]
  );
});
