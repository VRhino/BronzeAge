// El mercado de un campamento de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40, paso 4): SOLO VENDE, con stock limitado, y
// el oro que cobra se destruye (sumidero). Su stock sigue el pulso de la economía real: cada trueque cerrado entre Facciones y cada venta
// en el mostrador suma lo intercambiado a un contador global por bien, y cada `MERCENARIOS.mercado.cadaHoras` cada campamento repone una
// fracción de esos contadores, que vuelven a cero. Un bien que nadie comercia no se repone.
import type { CampamentoMercenarios, Asentamiento, Ejercito, Heroe, MercadoMercenario } from '../domain/types';
import { capacidadCargaDe } from './ejercitos';
import type { EventoCrudo } from '../domain/eventos';
import { MERCENARIOS, PRECIO_BASE, ALMACEN_PERSONAL } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { calcularPrecioReferencia } from './market';
import { totalAlmacenPersonal } from './almacenPersonal';
import { MercenariosInvalidoError } from './mercenarios';

/** Lo que se vende desde el primer día: los bienes con precio de referencia (hoy, los recursos en bruto). */
export const BIENES_EN_VENTA: readonly string[] = Object.keys(PRECIO_BASE);

/** El mercado con el que nace un campamento: stock inicial de los bienes en venta. */
export const mercadoInicial = (): Record<string, number> => ({
  ...Object.fromEntries(BIENES_EN_VENTA.map((b) => [b, MERCENARIOS.mercado.stockInicial])),
  ...MERCENARIOS.mercado.pilas,
});

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
  const aportes = Object.entries(mercado.contadores).filter(([bien, n]) => n > 0 && !(bien in MERCENARIOS.mercado.pilas));
  return {
    campamentos: campamentos.map((c) => {
      // Las pilas (D41) vuelven a su tope pase lo que pase en el mundo.
      const stock = { ...c.mercado, ...MERCENARIOS.mercado.pilas };
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

/** El día de mundo de un instante: el cupo de compra (D41) se cuenta por día. */
const diaDe = (instante: Instante): number => Math.floor(instante / 86_400_000);

/**
 * Comprar en el mercado de un campamento (Doc 1.9b), pagando con el oro del almacén personal. Sirve lo que puede en vez de fallar cuando
 * se pide de más —el tope sale del stock, del cupo del día, del oro y del sitio donde va—, pero falla si no puede servir nada. El oro
 * cobrado se destruye. Unidades enteras.
 *
 * - **Quien reside en él** compra cualquier bien en venta, a su almacén personal, con cupo diario en madera y piedra (D41).
 * - **Cualquier otro héroe** solo compra trigo, para repostar, y le va al carro de su columna (D44).
 */
export function comprarEnCampamento(
  campamento: CampamentoMercenarios,
  heroe: Heroe,
  /** Su columna en la puerta (o aparcada dentro): adonde va el trigo de quien no reside. */
  columna: Ejercito | undefined,
  asentamientos: Asentamiento[],
  recurso: string,
  cantidadPedida: number,
  instante: Instante
): { campamento: CampamentoMercenarios; heroe: Heroe; columna: Ejercito | undefined; cantidad: number; oro: number } {
  if (!(cantidadPedida >= 1)) throw new MercenariosInvalidoError('Hay que pedir al menos una unidad.');
  if (recurso === 'oro') throw new MercenariosInvalidoError('El oro no se vende: es con lo que se paga.');
  const reside = campamento.residentesIds.includes(heroe.id);
  if (!reside && recurso !== 'trigo') throw new MercenariosInvalidoError('Quien no reside aquí solo compra trigo para repostar.');
  if (!reside && !columna) throw new MercenariosInvalidoError('Hace falta la columna para llevarse el trigo.');
  const precio = precioDeVenta(recurso, asentamientos);
  if (precio <= 0) throw new MercenariosInvalidoError('Ese bien no se vende en el campamento.');
  const coste = (n: number) => Math.ceil(n * precio);

  const dia = diaDe(instante);
  const comprado = heroe.cupoCampamento?.dia === dia ? heroe.cupoCampamento.comprado : {};
  const cupo = reside && recurso in MERCENARIOS.mercado.cupoDiario ? MERCENARIOS.mercado.cupoDiario[recurso]! - (comprado[recurso] ?? 0) : Infinity;
  const stock = Math.floor(campamento.mercado[recurso] ?? 0);
  // Se paga primero con el oro de botín (D27: el mercado del campamento es uno de sus dos destinos), luego con el del almacén.
  const botin = heroe.oroDeBotin ?? 0;
  const guardadoOro = botin + (heroe.almacenPersonal?.['oro'] ?? 0);
  const delAlmacen = (n: number) => Math.max(0, coste(n) - botin);
  // Sitio: en el almacén cada unidad ocupa 1 y libera lo que cuesta en oro; en el carro, lo que quede libre.
  const enCarro = !reside;
  const libreCarro = columna ? capacidadCargaDe(columna) - Object.values(columna.suministro).reduce((a, b) => a + b, 0) : 0;
  const libre = enCarro ? libreCarro : ALMACEN_PERSONAL.capacidad - totalAlmacenPersonal(heroe);
  let cantidad = Math.min(Math.floor(cantidadPedida), stock, cupo, Math.floor(guardadoOro / precio));
  while (cantidad > 0 && (enCarro ? cantidad : cantidad - delAlmacen(cantidad)) > libre) cantidad--;
  while (cantidad > 0 && coste(cantidad) > guardadoOro) cantidad--;
  if (cantidad <= 0) {
    throw new MercenariosInvalidoError(
      stock <= 0
        ? `El campamento no tiene ${recurso} en venta.`
        : cupo <= 0
          ? `Ya has comprado hoy todo el ${recurso} que se te vende.`
          : guardadoOro < precio
            ? 'No hay oro suficiente en el almacén personal.'
            : enCarro
              ? 'El carro está lleno.'
              : 'El almacén personal está lleno.'
    );
  }

  const oro = coste(cantidad);
  const almacenPersonal: Record<string, number> = { ...heroe.almacenPersonal, oro: (heroe.almacenPersonal?.['oro'] ?? 0) - delAlmacen(cantidad) };
  if (!enCarro) almacenPersonal[recurso] = (almacenPersonal[recurso] ?? 0) + cantidad;
  if (almacenPersonal['oro'] === 0) delete almacenPersonal['oro'];
  const cupoCampamento = reside && recurso in MERCENARIOS.mercado.cupoDiario ? { dia, comprado: { ...comprado, [recurso]: (comprado[recurso] ?? 0) + cantidad } } : heroe.cupoCampamento;
  return {
    campamento: { ...campamento, mercado: { ...campamento.mercado, [recurso]: (campamento.mercado[recurso] ?? 0) - cantidad } },
    heroe: { ...heroe, almacenPersonal, oroDeBotin: Math.max(0, botin - oro), ...(cupoCampamento ? { cupoCampamento } : {}) },
    columna: enCarro && columna ? { ...columna, suministro: { ...columna.suministro, [recurso]: (columna.suministro[recurso] ?? 0) + cantidad } } : columna,
    cantidad,
    oro,
  };
}
