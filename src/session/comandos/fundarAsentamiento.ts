import type { Point } from '../../domain/types';
import { exigirPuertaDeFundacion, fundarAsentamiento as fundarAsentamientoEngine } from '../../engine/settlement';
import { esCiudadano } from '../../engine/faccion';
import { cruzarLaPuerta, puntoDeFundacionDe, situarJugadores } from '../../engine/ubicacion';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conExploracionFundida, exigirJugador } from './ayudas';
import { evento } from './eventos';

export interface PayloadAsentamientoFundado {
  asentamientoId: string;
  faccionId: string;
  posicion: Point;
  jugadoresIds: string[];
}

export interface ParamsFundarAsentamiento {
  faccionId: string;
}

/**
 * Funda un asentamiento nuevo para una Facción existente. El fundador es EL ACTOR: recibe casa y, con ella,
 * ciudadanía inmediata de la Facción (Doc 1.2/1.3).
 *
 * **Se funda DONDE SE ESTA** (Doc 1.3): la posición no la elige el cliente, sale de la columna del fundador
 * (`puntoDeFundacionDe`) — hay que estar en campo abierto, ni dentro de una plaza ni desconectado. Fundar es
 * ENTRAR en lo que se acaba de levantar, así que esa columna se deshace dentro (`cruzarLaPuerta`), con sus
 * tropas y su carga si llevaba alguna: casi nunca, porque la columna con la que se aparece nace vacía, pero
 * un ciudadano que funda de campaña con su propia columna sí puede llegar con algo.
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

  // La puerta de entrada al mundo (`Consideraciones/Entrada_Al_Mundo_Definicion.md`): cuánta gente hace falta
  // para fundar, y si hay que haber sido ciudadano antes. Hoy las dos palancas están abiertas para las
  // primeras pruebas; lo que se decida después se enchufa en `exigirPuertaDeFundacion` y no aquí.
  //
  // "Ya fue ciudadano" se resuelve con `salidasFaccionPorJugador`, que es el registro de quien ALGUNA VEZ
  // dejó una Facción, más la ciudadanía vigente. Un jugador que nunca ha estado en ninguna no aparece en
  // ninguno de los dos.
  const yaFueCiudadano =
    estado.salidasFaccionPorJugador[ctx.actor] !== undefined || estado.facciones.some((f) => esCiudadano(f, ctx.actor));
  exigirPuertaDeFundacion(jugadoresIds, yaFueCiudadano);

  const fundador = exigirJugador(estado, ctx.actor);
  const { posicion, columna } = puntoDeFundacionDe(fundador, estado.ejercitos);

  const resultado = fundarAsentamientoEngine(
    mapa,
    estado.facciones,
    params.faccionId,
    posicion,
    jugadoresIds,
    estado.asentamientos,
    ctx.instante
  );

  // Fundar es ENTRAR en lo que se acaba de levantar (Doc 1.10): el fundador ya es residente
  // (`fundarAsentamientoEngine` lo puso en `jugadoresFundadoresIds`), así que la columna con la que llegó se
  // deshace dentro — sus tropas a la guarnición, su carro al almacén — igual que al cruzar la puerta de
  // cualquier otra residencia. Reutiliza `cruzarLaPuerta` en vez de repetir la regla: es la MISMA entrada,
  // solo que a una plaza que nace en este mismo instante.
  const cruce = cruzarLaPuerta(columna, resultado.asentamiento, ctx.actor, estado.relaciones);

  let siguiente: GameSessionState = {
    ...estado,
    asentamientos: [...estado.asentamientos, cruce.asentamiento],
    facciones: resultado.facciones,
    ejercitos: cruce.disuelveColumna ? estado.ejercitos.filter((e) => e.id !== columna.id) : estado.ejercitos,
    // Única colocación que hace este comando, y hace falta porque `cruzarLaPuerta` no toca `Jugador.ubicacion`
    // — solo fusiona tropas y carga en el asentamiento.
    jugadores: situarJugadores(estado.jugadores, jugadoresIds, { tipo: 'asentamiento', asentamientoId: resultado.asentamiento.id }),
  };

  const nombreFaccion = resultado.facciones.find((f) => f.id === params.faccionId)?.nombre ?? params.faccionId;
  for (const jugadorId of jugadoresIds) {
    // Lo que anduvo sin bandera pasa a ser conocimiento de la Facción que acaba de fundar (Doc 1.3).
    siguiente = conExploracionFundida(siguiente, jugadorId, params.faccionId);
    siguiente = conHistorialDeJugador(siguiente, jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
  }

  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'fundacion.asentamiento_fundado',
        mensaje: `${nombreFaccion} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`,
        payload: {
          asentamientoId: resultado.asentamiento.id,
          faccionId: params.faccionId,
          posicion,
          jugadoresIds,
        } satisfies PayloadAsentamientoFundado,
        asentamientoId: resultado.asentamiento.id,
      }),
    ],
    { asentamientoId: resultado.asentamiento.id }
  );
});
