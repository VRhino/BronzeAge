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
  inspeccionarCaravana as inspeccionarCaravanaEngine,
  inspeccionarColumna,
  type ComposicionColumna,
  type ContenidoCaravana,
} from '../../engine/ejercitos';
import { conHistorialDeJugador } from '../estado';
import { exito } from './tipos';
import { comando, exigirCaravana, exigirColumnaDe, exigirEjercito } from './ayudas';
import { evento } from './eventos';

/** A qué se puede apuntar desde el menú de interacción: una columna o una caravana. Es la misma forma que usa
 * la persecución, y no por casualidad — se persigue lo que se puede mirar. */
export type ObjetivoDeInteraccion = { tipo: 'ejercito'; id: string } | { tipo: 'caravana'; id: string };

export interface ParamsInspeccionar {
  jugadorId: string;
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
  const observador = exigirColumnaDe(estado, params.jugadorId);

  if (params.objetivo.tipo === 'ejercito') {
    const objetivo = exigirEjercito(estado, params.objetivo.id);
    const composicion = inspeccionarColumna(observador, objetivo);

    return exito(
      conHistorialDeJugador(estado, params.jugadorId, `Inspecciona la columna ${objetivo.id}.`),
      [
        evento(ctx, {
          codigo: 'columna.observada',
          mensaje: `Alguien se ha acercado a mirar la columna ${objetivo.id}.`,
          payload: { observadorId: params.jugadorId, objetivo: params.objetivo } satisfies PayloadObservado,
          // Atribuido al OBSERVADO, no al que mira: el aviso es para quien lo sufre.
          asentamientoId: objetivo.origenAsentamientoId,
        }),
      ],
      composicion
    );
  }

  const caravana = exigirCaravana(estado, params.objetivo.id);
  const escoltada = estado.ejercitos.some((e) => e.caravanasAdjuntasIds.includes(caravana.id));
  const contenido = inspeccionarCaravanaEngine(observador, caravana, escoltada);

  return exito(
    conHistorialDeJugador(estado, params.jugadorId, `Inspecciona la caravana ${caravana.id}.`),
    [
      evento(ctx, {
        codigo: 'caravana.observada',
        mensaje: `Alguien se ha acercado a mirar la caravana ${caravana.id}.`,
        payload: { observadorId: params.jugadorId, objetivo: params.objetivo } satisfies PayloadObservado,
        asentamientoId: caravana.origenAsentamientoId,
      }),
    ],
    contenido
  );
});
