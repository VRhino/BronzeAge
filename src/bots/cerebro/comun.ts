// Lecturas que comparten los cerebros. Todas salen de la vista (la proyección del jugador) y de reglas públicas del
// motor aplicadas a lo que el bot ve: un cliente humano podría hacer exactamente las mismas cuentas.
import type { Asentamiento, Ejercito, Escuadron, Heroe, RecursoTipo } from '../../domain/types';
import { puedeLlevar } from '../../engine/liderazgo';
import { poderEscuadron } from '../../engine/tropa';
import type { Vista } from '../puerto';

export type HeroeVisto = NonNullable<Vista['heroe']>;

/** El propio héroe con la forma que esperan las reglas del motor (`puedeLlevar`, `poderEscuadron`). */
export const comoHeroe = (h: HeroeVisto): Heroe => h as unknown as Heroe;

/** La plaza propia que el héroe pisa, si la pisa (Doc 1.10.1: es la única cuyo interior ve). */
export const plazaDentro = (vista: Vista): Asentamiento | undefined => vista.asentamientos[0];

export const residentesDe = (a: Asentamiento): string[] => [...new Set([...a.heroesFundadoresIds, ...a.casasCompradas])];

/** Su columna, si va en una. */
export const columnaPropia = (vista: Vista, yo: string): Ejercito | undefined =>
  vista.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === yo));

export const estaHerido = (vista: Vista): boolean => (vista.heroe?.heridoHasta ?? -Infinity) > vista.instante;

/** Tiene algún cargo en esa plaza o es el Rey: el que manda se queda en casa. */
export function tieneCargo(vista: Vista, yo: string, plaza: Asentamiento): boolean {
  const faccion = vista.facciones.find((f) => f.id === vista.faccionId);
  return faccion?.reyId === yo || Object.values(plaza.cargos).includes(yo);
}

/** Las escuadras de su campamento que no están en la guarnición y tienen soldados. */
export const escuadrasLibres = (h: HeroeVisto): Escuadron[] =>
  h.escuadrones.filter((e) => e.contenedor.tipo === 'campamento' && e.cantidad > 0 && !e.enGuarnicion);

/** De `escuadras`, las que le caben en su Liderazgo (Doc 5.11), las más fuertes primero. */
export function loQueLeCabe(h: HeroeVisto, escuadras: readonly Escuadron[]): Escuadron[] {
  const elegidas: Escuadron[] = [];
  for (const e of [...escuadras].sort((a, b) => poderEscuadron(b) - poderEscuadron(a) || (a.id < b.id ? -1 : 1))) {
    if (puedeLlevar(comoHeroe(h), [...elegidas, e])) elegidas.push(e);
  }
  return elegidas;
}

export function fraccionDe(plaza: Asentamiento, recurso: RecursoTipo): number {
  const item = plaza.almacen[recurso];
  return !item || item.capacidad <= 0 ? 0 : item.cantidad / item.capacidad;
}


/** Las plazas de su Facción que conoce (las que ve y la que pisa): id, posición y cargos. */
export function plazasPropias(vista: Vista): { id: string; posicion: { x: number; y: number }; cargos?: Asentamiento['cargos'] }[] {
  const dentro = plazaDentro(vista);
  const vistas = vista.asentamientosAvistados.filter((a) => a.faccionId === vista.faccionId && a.id !== dentro?.id);
  return [...(dentro ? [dentro] : []), ...vistas];
}

/** Las plazas que conoce —las que ve y las que recuerda—, con la misma forma: id, Facción, posición. Lo visto gana. */
export function plazasConocidas(vista: Vista): { id: string; faccionId: string; posicion: { x: number; y: number } }[] {
  const vistas = vista.asentamientosAvistados.map((a) => ({ id: a.id, faccionId: a.faccionId, posicion: a.posicion }));
  const vistasIds = new Set(vistas.map((a) => a.id));
  const recordadas = vista.asentamientosConocidos
    .filter((f) => !vistasIds.has(f.asentamientoId))
    .map((f) => ({ id: f.asentamientoId, faccionId: f.faccionId, posicion: f.posicion }));
  return [...vistas, ...recordadas];
}
