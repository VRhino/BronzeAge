// Seguir a la presa de una persecución (Doc 5.12.3): la ruta del perseguidor se recalcula hacia donde esté, pero no en
// cada tick. Cerrarla al alcanzarla (a 15) es de `cerrarPersecuciones`, en `ejercitos.ts`; aquí está lo que ocurre
// mientras tanto: recalcular la ruta cuando la presa se ha movido de verdad, y soltarla si se pierde de vista o se
// pone a cubierto.
import type { Asentamiento, Ejercito, Point } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { PERSECUCION } from '../constants';
import { calcularRuta } from '../world/rutas';
import { distancia } from '../world/geometria';
import type { Mapa } from '../world/mapa';

/** Payload de `columna.presa_perdida` y `columna.presa_a_cubierto`: la persecución se soltó sin llegar a 15. */
export interface PayloadPresaSoltada {
  ejercitoId: string;
  objetivo: { tipo: 'ejercito' | 'caravana'; id: string };
}

export interface SeguimientoDePresa<E extends Ejercito> {
  ejercito: E;
  evento?: EventoCrudo;
}

/**
 * Un tick de seguimiento. `presa` es donde está al empezar el tick, o `undefined` si ya no está en el mapa (entró en
 * una plaza o campamento, se disolvió, la capturaron, va adjunta a un ejército o está en una batalla). `vista` es hasta
 * dónde ve el perseguidor (`alcanceDeVista`).
 *
 * - Sin presa en el mapa: la persecución se suelta (`columna.presa_a_cubierto`) y la columna termina el rumbo que
 *   llevaba, que acaba en la puerta de donde se metió; allí acampa.
 * - Presa fuera de vista: se suelta (`columna.presa_perdida`), igual. Si no, la trayectoria del perseguidor
 *   delataría dónde está la presa bajo la niebla.
 * - Si no, se recalcula la ruta solo cuando la presa se ha movido más de `PERSECUCION.umbralRecalculo` desde el
 *   último destino calculado, o cuando el perseguidor llegó a él sin alcanzarla. En el resto de ticks el coste es
 *   una comparación de distancias.
 */
export function seguirPresa<E extends Ejercito>(
  ejercito: E,
  presa: Point | undefined,
  vista: number,
  mapa: Mapa,
  asentamientos: readonly Asentamiento[]
): SeguimientoDePresa<E> {
  const fijada = ejercito.persiguiendo;
  if (!fijada) return { ejercito };

  const soltar = (codigo: 'columna.presa_perdida' | 'columna.presa_a_cubierto', mensaje: string): SeguimientoDePresa<E> => ({
    // Sin un destino calculado nunca llegó a ponerse en marcha: se queda donde está.
    ejercito: { ...ejercito, persiguiendo: undefined, ...(fijada.destino ? {} : { estado: 'estacionado' as const, objetivo: { tipo: 'punto' as const, punto: ejercito.posicionActual } }) },
    evento: {
      codigo,
      mensaje,
      payload: { ejercitoId: ejercito.id, objetivo: { tipo: fijada.tipo, id: fijada.id } } satisfies PayloadPresaSoltada,
      asentamientoId: ejercito.origenAsentamientoId,
    },
  });

  if (!presa) return soltar('columna.presa_a_cubierto', `La columna ${ejercito.id} pierde a ${fijada.id}: se ha puesto a cubierto.`);
  if (distancia(presa, ejercito.posicionActual) > vista) {
    return soltar('columna.presa_perdida', `La columna ${ejercito.id} pierde de vista a ${fijada.id}.`);
  }

  const alDia = fijada.destino !== undefined && ejercito.progreso < 1 && distancia(presa, fijada.destino) <= PERSECUCION.umbralRecalculo;
  if (alDia) return { ejercito };

  const ruta = calcularRuta(mapa, ejercito.posicionActual, presa, { pasosRio: asentamientos.map((a) => a.posicion) });
  // Sin camino por tierra hasta ella (isla, agua) no se la puede seguir.
  if (!ruta) return soltar('columna.presa_perdida', `La columna ${ejercito.id} no encuentra camino hasta ${fijada.id}.`);
  return {
    ejercito: { ...ejercito, ruta, progreso: 0, estado: 'marchando', objetivo: { tipo: 'punto', punto: presa }, persiguiendo: { ...fijada, destino: presa } },
  };
}
