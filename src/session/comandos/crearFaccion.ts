import { crearFaccion as crearFaccionEngine } from '../../engine/faccion';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

export interface PayloadFaccionCreada {
  faccionId: string;
  nombre: string;
}

export interface ParamsCrearFaccion {
  nombre: string;
}

/**
 * Crea una Facción nueva.
 *
 * A diferencia de la mayoría de comandos, las dos validaciones (nombre vacío, nombre duplicado) viven AQUÍ y
 * no en el motor: `crearFaccion` de `engine/faccion.ts` no las hace, porque son reglas de la partida —qué
 * nombres se admiten en ESTA sesión— y no del modelo de juego.
 *
 * Autorización (doc 5, pendiente de Fase C): rol `jugador`. Queda abierto en ese documento si un jugador que
 * ya tiene Facción puede crear otra.
 */
export const crearFaccion = comando<ParamsCrearFaccion, { faccionId: string }>((estado, _mapa, ctx, params) => {
  const nombre = params.nombre.trim();
  if (!nombre) rechazar(CODIGOS_ERROR.faccionNombreVacio);
  if (estado.facciones.some((f) => f.nombre.toLowerCase() === nombre.toLowerCase())) {
    rechazar(CODIGOS_ERROR.faccionNombreDuplicado);
  }

  const nueva = crearFaccionEngine(`faccion-custom-${ctx.ids.siguiente()}`, nombre);
  const siguiente: GameSessionState = { ...estado, facciones: [...estado.facciones, nueva] };
  return exito(
    siguiente,
    [
      evento(ctx, estado, {
        codigo: 'faccion.creada',
        mensaje: `Se crea la Facción "${nueva.nombre}".`,
        payload: { faccionId: nueva.id, nombre: nueva.nombre } satisfies PayloadFaccionCreada,
      }),
    ],
    { faccionId: nueva.id }
  );
});
