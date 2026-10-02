// El mercado de un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 4): SOLO VENDE, con stock limitado, y
// el oro que cobra se destruye (sumidero). Su stock sigue el pulso de la economía real: cada trueque cerrado entre Facciones y cada venta
// en el mostrador suma lo intercambiado a un contador global por bien, y cada `MERCENARIOS.mercado.cadaHoras` cada campamento repone una
// fracción de esos contadores, que vuelven a cero. Un bien que nadie comercia no se repone.
import type { CampamentoMercenarios, Asentamiento, Heroe, MercadoMercenario } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { MERCENARIOS, PRECIO_BASE, ALMACEN_PERSONAL } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { calcularPrecioReferencia } from './market';
import { totalAlmacenPersonal } from './almacenPersonal';
import { MercenariosInvalidoError, campamentoDeResidente } from './mercenarios';

/** Lo que se vende desde el primer día: los bienes con precio de referencia (hoy, los recursos en bruto). */
export const BIENES_EN_VENTA: readonly string[] = Object.keys(PRECIO_BASE);

/** El mercado con el que nace un campamento: stock inicial de los bienes en venta. */
export const mercadoInicial = (): Record<string, number> => Object.fromEntries(BIENES_EN_VENTA.map((b) => [b, MERCENARIOS.mercado.stockInicial]));

export const mercadoMercenarioInicial = (desde: Instante): MercadoMercenario => ({
  contadores: {},
  reponeEn: sumar(desde, minutos(60 * MERCENARIOS.mercado.cadaHoras)),
});

/**
 * Lo que un hecho del mundo aporta al contador global (Doc 1.9b): el trueque cerrado entre Facciones DISTINTAS (los dos lados) y lo
 * vendido o comprado en el mostrador de una plaza ajena. Comerciar con uno mismo no cuenta: en la Era II medida, 2 910 de 2 919 trueques
 * eran de la misma Facción, y habrían inflado el stock con un comercio que no mueve el mundo. Lee los eventos, que es por donde pasa todo
 * hecho de la partida (`exito`).
 */
export function comerciadoDeEventos(eventos: readonly EventoCrudo[]): Record<string, number> {
  const comerciado: Record<string, number> = {};
  const suma = (recurso: string, n: number) => {
    if (n > 0) comerciado[recurso] = (comerciado[recurso] ?? 0) + n;
  };
  for (const e of eventos) {
    if (typeof e === 'string') continue;
    if (e.codigo === 'comercio.trueque_cumplido') {
      const p = e.payload as { entreFacciones?: boolean; intercambiado?: Record<string, number> };
      if (p.entreFacciones) for (const [r, n] of Object.entries(p.intercambiado ?? {})) suma(r, n);
    } else if (e.codigo === 'mercado.comercio_en_plaza') {
      const p = e.payload as { entreFacciones?: boolean; recurso: string; cantidad: number };
      if (p.entreFacciones) suma(p.recurso, p.cantidad);
    }
  }
  return comerciado;
}

/** El mercado con lo comerciado sumado a sus contadores. Sin novedad devuelve el mismo objeto. */
export function sumarComerciado(mercado: MercadoMercenario, comerciado: Record<string, number>): MercadoMercenario {
  const entradas = Object.entries(comerciado);
  if (entradas.length === 0) return mercado;
  const contadores = { ...mercado.contadores };
  for (const [r, n] of entradas) contadores[r] = (contadores[r] ?? 0) + n;
  return { ...mercado, contadores };
}

/**
 * La reposición periódica: al llegar `reponeEn`, cada campamento recibe `proporcionRepone` de lo comerciado de cada bien, sin pasar de
 * `topePorBien`, y los contadores vuelven a cero. Es una cita agendada, no un barrido: cada tick solo compara un instante.
 */
export function reponerMercados(
  campamentos: readonly CampamentoMercenarios[],
  mercado: MercadoMercenario,
  instante: Instante
): { campamentos: CampamentoMercenarios[]; mercado: MercadoMercenario } {
  if (instante < mercado.reponeEn) return { campamentos: campamentos as CampamentoMercenarios[], mercado };
  const siguiente = { contadores: {}, reponeEn: sumar(instante, minutos(60 * MERCENARIOS.mercado.cadaHoras)) };
  const aportes = Object.entries(mercado.contadores).filter(([, n]) => n > 0);
  if (aportes.length === 0) return { campamentos: campamentos as CampamentoMercenarios[], mercado: siguiente };
  return {
    campamentos: campamentos.map((c) => {
      const stock = { ...c.mercado };
      for (const [bien, comerciado] of aportes) {
        stock[bien] = Math.min(MERCENARIOS.mercado.topePorBien, (stock[bien] ?? 0) + comerciado * MERCENARIOS.mercado.proporcionRepone);
      }
      return { ...c, mercado: stock };
    }),
    mercado: siguiente,
  };
}

/** Lo que cuesta una unidad: el precio de referencia vigente más el margen. 0 si el bien no tiene precio (no se vende). */
export const precioDeVenta = (recurso: string, asentamientos: Asentamiento[]): number => calcularPrecioReferencia(recurso, asentamientos) * MERCENARIOS.mercado.margen;

/**
 * Comprar en el mercado del campamento donde reside el héroe, pagando con su almacén personal y recibiendo en él (Doc 1.9b). Sirve lo
 * que puede en vez de fallar cuando se pide de más —el tope sale del stock, del oro que lleva y del sitio que le queda en el almacén—, pero
 * falla si no puede servir nada. El oro cobrado se destruye. Unidades enteras.
 */
export function comprarEnCampamento(
  campamentos: readonly CampamentoMercenarios[],
  heroes: readonly Heroe[],
  asentamientos: Asentamiento[],
  heroeId: string,
  recurso: string,
  cantidadPedida: number
): { campamentos: CampamentoMercenarios[]; heroes: Heroe[]; cantidad: number; oro: number } {
  const campamento = campamentoDeResidente(campamentos, heroeId);
  if (!campamento) throw new MercenariosInvalidoError('Solo se compra en el campamento donde se reside.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new MercenariosInvalidoError('Ese héroe no existe.');
  if (!(cantidadPedida >= 1)) throw new MercenariosInvalidoError('Hay que pedir al menos una unidad.');
  if (recurso === 'oro') throw new MercenariosInvalidoError('El oro no se vende: es con lo que se paga.');
  const precio = precioDeVenta(recurso, asentamientos);
  if (precio <= 0) throw new MercenariosInvalidoError('Ese bien no se vende en el campamento.');

  const stock = Math.floor(campamento.mercado[recurso] ?? 0);
  const guardadoOro = heroe.almacenPersonal?.['oro'] ?? 0;
  const libre = ALMACEN_PERSONAL.capacidad - totalAlmacenPersonal(heroe);
  const coste = (n: number) => Math.ceil(n * precio);
  let cantidad = Math.min(Math.floor(cantidadPedida), stock, Math.floor(guardadoOro / precio));
  // Cada unidad ocupa 1 y libera lo que cuesta en oro: se baja hasta que el almacén quepa.
  while (cantidad > 0 && cantidad - coste(cantidad) > libre) cantidad--;
  while (cantidad > 0 && coste(cantidad) > guardadoOro) cantidad--;
  if (cantidad <= 0) {
    throw new MercenariosInvalidoError(
      stock <= 0 ? `El campamento no tiene ${recurso} en venta.` : guardadoOro < precio ? 'No hay oro suficiente en el almacén personal.' : 'El almacén personal está lleno.'
    );
  }

  const oro = coste(cantidad);
  const almacenPersonal: Record<string, number> = { ...heroe.almacenPersonal, oro: guardadoOro - oro, [recurso]: (heroe.almacenPersonal?.[recurso] ?? 0) + cantidad };
  if (almacenPersonal['oro'] === 0) delete almacenPersonal['oro'];
  return {
    campamentos: campamentos.map((c) => (c.id === campamento.id ? { ...c, mercado: { ...c.mercado, [recurso]: (c.mercado[recurso] ?? 0) - cantidad } } : c)),
    heroes: heroes.map((h) => (h.id === heroe.id ? { ...heroe, almacenPersonal } : h)),
    cantidad,
    oro,
  };
}
