import type { Mapa } from '../../world/mapa';
import { crearFaccion as crearFaccionEngine } from '../../engine/faccion';
import { eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, type ContextoComando, type TransicionComando } from './tipos';

export interface ParamsCrearFaccion {
  nombre: string;
}

/**
 * Crea una Facción nueva.
 *
 * A diferencia de la mayoría de comandos, las dos validaciones (nombre vacío, nombre duplicado) viven AQUÍ y
 * no en el motor: `crearFaccion` de `engine/faccion.ts` no las hace, porque son reglas de la partida —qué
 * nombres se admiten en ESTA sesión— y no del modelo de juego. De ahí que sus códigos de error no salgan de
 * `erroresDeDominio.ts` (que solo mapea excepciones del motor) sino que se declaren en el propio comando.
 *
 * Autorización (doc 5, pendiente de Fase C): rol `jugador`. Queda abierto en ese documento si un jugador que
 * ya tiene Facción puede crear otra.
 */
export function crearFaccion(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsCrearFaccion
): TransicionComando<{ faccionId: string }> {
  const nombre = params.nombre.trim();
  if (!nombre) return rechazo(estado, 'faccion.nombre_vacio');
  if (estado.facciones.some((f) => f.nombre.toLowerCase() === nombre.toLowerCase())) {
    return rechazo(estado, 'faccion.nombre_duplicado');
  }

  const nueva = crearFaccionEngine(`faccion-custom-${ctx.ids.siguiente()}`, nombre);
  const siguiente: GameSessionState = { ...estado, facciones: [...estado.facciones, nueva] };
  const evento = eventoLegado(ctx.momento, estado.tick, `Se crea la Facción "${nueva.nombre}".`);

  return exito(siguiente, [evento], { faccionId: nueva.id });
}
