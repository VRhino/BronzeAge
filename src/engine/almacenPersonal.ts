// Almacén personal del héroe (Doc 2.5, 2026-10-02): lo que guarda para sí, hasta `ALMACEN_PERSONAL.capacidad` en total y de cualquier
// recurso. Se llena y se vacía con el carro de su columna, que es donde lleva lo que gana (botín, compras): nunca toca el almacén de
// una plaza, así que no abre un agujero en la economía de la Facción.
//
// **Vive donde el héroe reside** (2026-10-11): nada se mueve de él a otro espacio por arte de magia. Solo se toca desde la propia residencia o con
// el carro propio del héroe en su base, es decir, con su columna a la puerta de la plaza o del campamento donde reside.
import type { Asentamiento, CampamentoMercenarios, Ejercito, Heroe } from '../domain/types';
import { ALMACEN_PERSONAL } from '../constants';
import { enLaPuertaDe, enLaPuertaDelCampamento } from './ejercitos';
import { HeroeInvalidoError } from './heroe';
import { esResidente } from './pertenencia';

const suma = (r: Record<string, number> | undefined): number => Object.values(r ?? {}).reduce((a, b) => a + b, 0);

export const totalAlmacenPersonal = (heroe: Heroe): number => suma(heroe.almacenPersonal);

/** Carga de un carro: es un carro, no una estantería con un cajón por material. */
const cargaDelCarro = (ejercito: Ejercito): number => suma(ejercito.suministro);

function exigirLider(ejercito: Ejercito, heroe: Heroe): void {
  // El carro es COMÚN (Doc 5.13.2): sin esta condición cualquiera que se uniera en campo vaciaría el de todos.
  if (ejercito.liderId !== heroe.id) throw new HeroeInvalidoError('Solo el Líder de la columna mueve lo de su carro.');
}

/** Dónde vive su residencia: la plaza o el campamento donde reside el héroe, con la que se comprueba su base. */
export interface LugaresDeResidencia {
  asentamientos: readonly Asentamiento[];
  campamentos: readonly CampamentoMercenarios[];
}

/** ¿Está la columna a la puerta de la residencia del héroe? Es la única situación en que el carro toca su almacén personal. */
export function enSuBase(heroe: Heroe, columna: Ejercito, lugares: LugaresDeResidencia): boolean {
  const plaza = lugares.asentamientos.find((a) => esResidente(a, heroe.id));
  if (plaza) return enLaPuertaDe(columna, plaza);
  const campamento = lugares.campamentos.find((c) => c.residentesIds.includes(heroe.id));
  return campamento !== undefined && enLaPuertaDelCampamento(columna, campamento);
}

function exigirEnSuBase(heroe: Heroe, columna: Ejercito, lugares: LugaresDeResidencia): void {
  if (!enSuBase(heroe, columna, lugares)) {
    throw new HeroeInvalidoError('El almacén personal está en tu residencia: solo se toca con tu columna a su puerta.');
  }
}

function exigirCantidad(cantidad: number): void {
  if (!(cantidad > 0)) throw new HeroeInvalidoError('La cantidad tiene que ser positiva.');
}

/** Con `resto` quitado el recurso que quede en cero, para no arrastrar claves vacías. */
function sin0(r: Record<string, number>, recurso: string, valor: number): Record<string, number> {
  const salida = { ...r, [recurso]: valor };
  if (salida[recurso]! <= 0) delete salida[recurso];
  return salida;
}

/** Del carro al almacén personal: lo que cabe hasta el tope. El Líder de la columna, con lo que lleva. */
export function guardarEnAlmacenPersonal(
  heroe: Heroe,
  ejercito: Ejercito,
  recurso: string,
  cantidad: number,
  lugares: LugaresDeResidencia
): { heroe: Heroe; ejercito: Ejercito; movido: number } {
  exigirLider(ejercito, heroe);
  exigirEnSuBase(heroe, ejercito, lugares);
  exigirCantidad(cantidad);
  const enElCarro = ejercito.suministro[recurso] ?? 0;
  const movido = Math.min(cantidad, enElCarro, ALMACEN_PERSONAL.capacidad - totalAlmacenPersonal(heroe));
  if (movido <= 0) throw new HeroeInvalidoError(enElCarro <= 0 ? `El carro no lleva ${recurso}.` : 'El almacén personal está lleno.');
  return {
    heroe: { ...heroe, almacenPersonal: sin0(heroe.almacenPersonal ?? {}, recurso, (heroe.almacenPersonal?.[recurso] ?? 0) + movido) },
    ejercito: { ...ejercito, suministro: sin0(ejercito.suministro, recurso, enElCarro - movido) },
    movido,
  };
}

/** Del almacén personal al carro: lo que cabe en el carro. `capacidadCarga` es `capacidadCargaDe(ejercito, caravanas)`. */
export function sacarDelAlmacenPersonal(
  heroe: Heroe,
  ejercito: Ejercito,
  recurso: string,
  cantidad: number,
  capacidadCarga: number,
  lugares: LugaresDeResidencia
): { heroe: Heroe; ejercito: Ejercito; movido: number } {
  exigirLider(ejercito, heroe);
  exigirEnSuBase(heroe, ejercito, lugares);
  exigirCantidad(cantidad);
  const guardado = heroe.almacenPersonal?.[recurso] ?? 0;
  const movido = Math.min(cantidad, guardado, Math.max(0, capacidadCarga - cargaDelCarro(ejercito)));
  if (movido <= 0) throw new HeroeInvalidoError(guardado <= 0 ? `No hay ${recurso} en el almacén personal.` : 'El carro está lleno.');
  return {
    heroe: { ...heroe, almacenPersonal: sin0(heroe.almacenPersonal ?? {}, recurso, guardado - movido) },
    ejercito: { ...ejercito, suministro: sin0(ejercito.suministro, recurso, (ejercito.suministro[recurso] ?? 0) + movido) },
    movido,
  };
}
