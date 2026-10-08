// Víveres del héroe (Doc 5.13, decisión del usuario 2026-10-08): el trigo que come su columna. Ni carro (carga) ni almacén
// personal (lo que guarda): van siempre con él, tienen un tope fijo por héroe y en un ejército se suman. Se llenan al salir y al
// repostar del almacén de una plaza, con la ración de su campamento de mercenarios y del carro, nunca al revés.
import type { Asentamiento, Ejercito, Heroe } from '../domain/types';
import { LOGISTICA } from '../constants';
import { cantidadDisponible, descontarRecursos } from './almacen';
import { reservaDeTrigo } from './tropas';
import { ReglaInvalidaError } from './errores';

export class ViveresInvalidosError extends ReglaInvalidaError {}

export const viveresDe = (heroe: Pick<Heroe, 'viveres'> | undefined): number => heroe?.viveres ?? 0;

export const huecoEnViveres = (heroe: Pick<Heroe, 'viveres'> | undefined): number => Math.max(0, LOGISTICA.capacidadViveresPorHeroe - viveresDe(heroe));

/**
 * Trigo que una plaza suelta hasta `espacio`, sin bajar de su `reservaDeTrigo` — el mismo margen que frena a la auto-construcción y
 * al reclutamiento: sacar una columna cuesta stock real, pero no puede dejar la ciudad en hambruna. Si no llega, se sale con menos.
 */
export function sacarTrigoDe(
  asentamiento: Asentamiento,
  /** Ración de lo que se queda en el campamento de la plaza: es lo que protege la reserva. */
  consumoTropas: number,
  espacio: number
): { asentamiento: Asentamiento; cargado: number } {
  const disponible = Math.max(0, cantidadDisponible(asentamiento.almacen, 'trigo') - reservaDeTrigo(asentamiento, consumoTropas));
  const cargado = Math.min(Math.max(0, espacio), disponible);
  if (cargado <= 0) return { asentamiento, cargado: 0 };
  return { asentamiento: { ...asentamiento, almacen: descontarRecursos(asentamiento.almacen, { trigo: cargado }) }, cargado };
}

/** Llena los víveres del héroe del almacén de la plaza, hasta su tope y lo que la plaza pueda soltar (`sacarTrigoDe`). */
export function llenarViveres(asentamiento: Asentamiento, consumoTropas: number, heroe: Heroe): { asentamiento: Asentamiento; heroe: Heroe; cargado: number } {
  const r = sacarTrigoDe(asentamiento, consumoTropas, huecoEnViveres(heroe));
  return { asentamiento: r.asentamiento, heroe: r.cargado > 0 ? { ...heroe, viveres: viveresDe(heroe) + r.cargado } : heroe, cargado: r.cargado };
}

/** Los víveres que suman los héroes de una columna: en un ejército son comunes mientras marchan juntos. */
export function viveresDeColumna(heroes: readonly Heroe[], heroeIds: readonly string[]): number {
  const ids = new Set(heroeIds);
  return heroes.reduce((suma, h) => (ids.has(h.id) ? suma + viveresDe(h) : suma), 0);
}

/** Descuenta `cantidad` de los víveres de esos héroes, a prorrata de lo que lleva cada uno: comer del montón común no vacía primero
 * los de uno, así que separarse se lleva su parte sin reparto que hacer. */
export function descontarViveres(heroes: readonly Heroe[], heroeIds: readonly string[], cantidad: number): Heroe[] {
  const total = viveresDeColumna(heroes, heroeIds);
  if (cantidad <= 0 || total <= 0) return [...heroes];
  const fraccion = Math.min(1, cantidad / total);
  const ids = new Set(heroeIds);
  return heroes.map((h) => (ids.has(h.id) && viveresDe(h) > 0 ? { ...h, viveres: viveresDe(h) * (1 - fraccion) } : h));
}

/** Reparte `cantidad` entre los víveres de esos héroes, llenando a cada uno hasta su tope en el orden dado. Devuelve lo repartido. */
export function repartirEnViveres(heroes: readonly Heroe[], heroeIds: readonly string[], cantidad: number): { heroes: Heroe[]; repartido: number } {
  let queda = cantidad;
  const cuanto = new Map<string, number>();
  for (const id of heroeIds) {
    const heroe = heroes.find((h) => h.id === id);
    const mete = Math.min(queda, huecoEnViveres(heroe));
    if (!heroe || mete <= 0) continue;
    cuanto.set(id, mete);
    queda -= mete;
  }
  return { heroes: heroes.map((h) => (cuanto.has(h.id) ? { ...h, viveres: viveresDe(h) + cuanto.get(h.id)! } : h)), repartido: cantidad - queda };
}

/** Hueco que les queda a los víveres de esos héroes, juntos. */
export function huecoDeColumna(heroes: readonly Heroe[], heroeIds: readonly string[]): number {
  return heroeIds.reduce((suma, id) => suma + huecoEnViveres(heroes.find((h) => h.id === id)), 0);
}

/** Del carro de su columna a sus víveres: lo que cabe. Nunca al revés —los víveres no son carga—. El carro es común, así que solo lo
 * toca el Líder, como el almacén personal. */
export function pasarAViveres(heroe: Heroe, ejercito: Ejercito, cantidad: number): { heroe: Heroe; ejercito: Ejercito; movido: number } {
  if (ejercito.liderId !== heroe.id) throw new ViveresInvalidosError('Solo el Líder de la columna mueve lo de su carro.');
  if (!(cantidad > 0)) throw new ViveresInvalidosError('La cantidad tiene que ser positiva.');
  const enElCarro = ejercito.suministro['trigo'] ?? 0;
  const movido = Math.min(cantidad, enElCarro, huecoEnViveres(heroe));
  if (movido <= 0) throw new ViveresInvalidosError(enElCarro <= 0 ? 'El carro no lleva trigo.' : 'Los víveres están llenos.');
  const suministro: Record<string, number> = { ...ejercito.suministro, trigo: enElCarro - movido };
  if (suministro['trigo']! <= 0) delete suministro['trigo'];
  return { heroe: { ...heroe, viveres: viveresDe(heroe) + movido }, ejercito: { ...ejercito, suministro }, movido };
}
