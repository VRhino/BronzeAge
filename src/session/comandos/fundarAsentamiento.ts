import type { Point } from '../../domain/types';
import { fundarAsentamiento as fundarAsentamientoEngine } from '../../engine/settlement';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando } from './ayudas';
import { evento } from './eventos';

export interface PayloadAsentamientoFundado {
  asentamientoId: string;
  faccionId: string;
  posicion: Point;
  jugadoresIds: string[];
}

export interface ParamsFundarAsentamiento {
  faccionId: string;
  posicion: Point;
}

/**
 * Funda un asentamiento nuevo para una Facción existente. El fundador es EL ACTOR: recibe casa y, con ella,
 * ciudadanía inmediata de la Facción (Doc 1.2/1.3).
 *
 * Autorización (`comandos/autorizacion.ts`): rol `jugador`, y ser ya ciudadano de esa Facción — salvo que no
 * sea ciudadano de ninguna, porque fundar es una de las dos vías de ENTRAR en una (la otra es `comprarCasa`).
 *
 * **Fundación grupal diferida.** El Doc 1.2/1.3 admite hasta 5 fundadores juntos, y el motor lo soporta
 * (`fundarAsentamiento` de `engine/settlement.ts` recibe una lista). No se expone aquí porque falta lo que la
 * haría legítima: un mecanismo de CONSENTIMIENTO. Aceptar una lista de cofundadores del cliente permitiría
 * meter a cualquier jugador en una Facción sin que él lo pidiera —y, como un jugador solo puede pertenecer a
 * una (Doc 0), dejarlo bloqueado para entrar en la que quería—. Eso es una vía de acoso, no una función.
 *
 * Hasta la Fase C2 este comando fabricaba sus fundadores (`jugador-<faccionId>-<n>`) a partir de un
 * `numJugadores`, herencia de cuando no había identidad real. Con la ciudadanía ya derivada del estado de
 * juego para autorizar (ver `Membresia` en `acceso/tipos.ts`), esos ids ficticios dejaban al jugador real sin
 * ninguna forma de hacerse ciudadano: creaba la Facción, fundaba, y la ciudadanía se la quedaban cinco
 * jugadores que no existían.
 *
 * La gobernanza NPC no pasa por aquí: funda con sus propios ids (`npc-<faccionId>-<n>`, ver
 * `session/npcGobernanza.ts`), que sí son ficticios a propósito porque detrás no hay ninguna persona.
 */
export const fundarAsentamiento = comando<ParamsFundarAsentamiento, { asentamientoId: string }>((estado, mapa, ctx, params) => {
  const jugadoresIds = [ctx.actor];

  const resultado = fundarAsentamientoEngine(
    mapa,
    estado.facciones,
    params.faccionId,
    params.posicion,
    jugadoresIds,
    estado.asentamientos,
    ctx.instante
  );

  let siguiente: GameSessionState = {
    ...estado,
    asentamientos: [...estado.asentamientos, resultado.asentamiento],
    facciones: resultado.facciones,
  };

  const nombreFaccion = resultado.facciones.find((f) => f.id === params.faccionId)?.nombre ?? params.faccionId;
  for (const jugadorId of jugadoresIds) {
    siguiente = conHistorialDeJugador(siguiente, jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
  }

  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'fundacion.asentamiento_fundado',
        mensaje: `${nombreFaccion} funda asentamiento en (${Math.round(params.posicion.x)}, ${Math.round(params.posicion.y)}).`,
        payload: {
          asentamientoId: resultado.asentamiento.id,
          faccionId: params.faccionId,
          posicion: params.posicion,
          jugadoresIds,
        } satisfies PayloadAsentamientoFundado,
        asentamientoId: resultado.asentamiento.id,
      }),
    ],
    { asentamientoId: resultado.asentamiento.id }
  );
});
