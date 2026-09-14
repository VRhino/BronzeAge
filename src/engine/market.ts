import type { Asentamiento, Ejercito, OrdenMercado } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';

/**
 * Payload de `mercado.compra` (ver `comerciarEnPlaza`). Ya no son dos plazas: uno de los dos lados es
 * siempre una COLUMNA, que es quien tiene que estar alli.
 */
export interface PayloadMercadoCompra {
  plazaId: string;
  columnaId: string;
  /** El de la ORDEN: `'venta'` = la plaza vende y la columna compra. */
  tipo: 'compra' | 'venta';
  recurso: string;
  cantidad: number;
  /** Lo que cambia de manos por la mercancia, sin la comision. */
  valor: number;
  comision: number;
}
/** Payload de `mercado.orden_expirada` — la oferta se retiró sola sin que nadie la tomara. */
export interface PayloadOrdenExpirada {
  ordenId: string;
  asentamientoId: string;
}
import { COMISION, MERCADO, PRECIO_BASE, PRECIO_REFERENCIA } from '../constants';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { factorComisionExterna } from './politicas';
import { tieneMercadoActivo } from './asentamientoQuery';
import { enLaPuertaDe } from './ejercitos';

export class OrdenInvalidaError extends Error {}

/** Precio de referencia por defecto según escasez/abundancia GLOBAL del recurso (Doc 3.4, sin componente de distancia). */
export function calcularPrecioReferencia(recurso: string, asentamientos: Asentamiento[]): number {
  const base = PRECIO_BASE[recurso];
  if (base === undefined) return 0; // oro no cotiza contra sí mismo
  const stockGlobal = asentamientos.reduce((acc, a) => acc + cantidadDisponible(a.almacen, recurso), 0);
  const factor = PRECIO_REFERENCIA.stockObjetivoGlobal / Math.max(1, stockGlobal);
  const factorClamped = Math.max(PRECIO_REFERENCIA.factorMin, Math.min(PRECIO_REFERENCIA.factorMax, factor));
  return base * factorClamped;
}

/** Coloca una orden de compra/venta (Doc 3.3). Si no se especifica precio, se usa el de referencia vigente. */
export function colocarOrdenMercado(
  asentamientos: Asentamiento[],
  asentamientoId: string,
  tipo: 'compra' | 'venta',
  recurso: string,
  cantidad: number,
  instante: Instante,
  precioUnitario?: number,
  contador = 0
): OrdenMercado {
  const asentamiento = asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) throw new OrdenInvalidaError('El asentamiento no existe.');
  // Ampliación de comercio (a petición del usuario, Doc 3.3): antes cualquier asentamiento podía colocar
  // órdenes desde el tick 0, sin edificio — ahora exige Mercado activo, igual que Trueque exige caravanas
  // propias (ver `construirCaravanaComercial`, engine/trade.ts).
  if (!tieneMercadoActivo(asentamiento)) {
    throw new OrdenInvalidaError('El asentamiento necesita un Mercado activo para colocar órdenes.');
  }
  if (cantidad <= 0) throw new OrdenInvalidaError('La cantidad debe ser mayor que 0.');
  if (tipo === 'venta' && cantidadDisponible(asentamiento.almacen, recurso) < cantidad) {
    throw new OrdenInvalidaError('No hay suficiente stock para vender esa cantidad.');
  }

  return {
    id: `orden-${asentamientoId}-${contador}`,
    asentamientoId,
    tipo,
    recurso,
    cantidad,
    cantidadCumplida: 0,
    precioUnitario: precioUnitario ?? calcularPrecioReferencia(recurso, asentamientos),
    creadoEn: instante,
    expiraEn: sumar(instante, minutos(MERCADO.plazoOrdenMinutos)),
    estado: 'activa',
  };
}

/**
 * Retira las ofertas que nadie tomó (Doc 3.3).
 *
 * Es la contrapartida obligada de haber quitado el emparejamiento automático: desde que una orden solo se
 * cumple en el mostrador, nada más la cierra. Sin esto una plaza acumularía ofertas eternas a precios de hace
 * cien ticks — y en particular una plaza NPC, que solo republica cuando la anterior ya no está activa, se
 * quedaría congelada para siempre en su primer precio.
 */
export function caducarOrdenes(ordenes: readonly OrdenMercado[], instante: Instante): { ordenes: OrdenMercado[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const resultantes = ordenes.map((orden) => {
    if (orden.estado !== 'activa' || instante < orden.expiraEn) return orden;
    eventos.push({
      codigo: 'mercado.orden_expirada',
      mensaje: `${orden.asentamientoId} retira su orden de ${orden.tipo} de ${orden.recurso}: nadie la tomó.`,
      payload: { ordenId: orden.id, asentamientoId: orden.asentamientoId } satisfies PayloadOrdenExpirada,
    });
    return { ...orden, estado: 'expirada' as const };
  });
  return { ordenes: resultantes, eventos };
}

/** Espacio libre para un recurso en un almacen. Un recurso sin entrada todavia no cabe: la capacidad la dan
 * los edificios de almacenaje (`ampliarCapacidad`), no aparece sola al recibir mercancia. */
function huecoPara(plaza: Asentamiento, recurso: string): number {
  const item = plaza.almacen[recurso];
  return item ? Math.max(0, item.capacidad - item.cantidad) : 0;
}

/** Lo que ya va en el carro, sumando recursos: es un carro, no una estanteria con un cajon por material. */
function cargaDe(ejercito: Ejercito): number {
  return Object.values(ejercito.suministro).reduce((suma, cantidad) => suma + cantidad, 0);
}

/** Tasa de comision (Doc 3.5) de esta plaza para esta columna; si es externa, su Tesorero la modula via
 * politica (Doc 4.4). La cobra SIEMPRE la plaza, porque es donde ocurre la transaccion. */
function tasaComision(plaza: Asentamiento, columna: Ejercito): number {
  if (plaza.faccionId === columna.faccionId) return COMISION.tasaMismaFaccion;
  return COMISION.tasaExterna * factorComisionExterna(plaza);
}

/**
 * **El mostrador**: un jugador toma en persona una orden de esta plaza (Doc 3.3), estando alli con su columna.
 *
 * Sustituye al emparejamiento automatico entre plazas, que liquidaba en el acto y sin que nada recorriera el
 * mapa (`Consideraciones/Comercio_Fisico_Definicion.md`, decisiones 1 y 2). Ahora la mercancia y el oro solo
 * se mueven de un sitio a otro **dentro de un carro**, y el viaje entero son cuatro actos: cargar en tu plaza,
 * llegar, comerciar aqui, y volver a depositar — este ultimo ya lo hacia `absorberColumna` sin saberlo.
 *
 * Las dos direcciones son espejo:
 *  - **La plaza VENDE**: el jugador paga oro DE SU CARRO y la mercancia sube AL CARRO.
 *  - **La plaza COMPRA**: el jugador descarga mercancia DE SU CARRO y el oro sube AL CARRO.
 *
 * Y en las dos, el oro **pesa y ocupa carro** (Doc 3.1: no es moneda acunada, es metal precioso pesado). Eso
 * pone un techo fisico a cuanto se puede mover de una tacada, que es la clase de limite que este juego quiere:
 * comprar barato lejos y vender caro en casa cuesta viajes, no un clic.
 *
 * **La comision (Doc 3.5) la paga quien toma la orden y se la queda la plaza**, y esto SI cambia respecto al
 * emparejamiento viejo, que la acunaba de la nada y se la regalaba al vendedor. Aqui el oro se conserva: es
 * literalmente lo que cuesta usar el mercado de otro.
 *
 * **Sirve lo que puede** en vez de fallar cuando se pide de mas —el tope real sale de cinco cosas a la vez y
 * ninguna la ve el cliente entera—, pero **falla si no puede servir nada**: devolver "0 comprado" sin decir
 * por que deja a alguien pulsando un boton que no hace nada.
 */
export function comerciarEnPlaza(
  ejercito: Ejercito,
  /** Quien opera. Tiene que ser el Lider de la columna: el carro es COMUN (Doc 5.13.2), y sin esta condicion
   * cualquiera que se uniera en campo podria gastarse el oro de todos. */
  heroeId: string,
  plaza: Asentamiento,
  orden: OrdenMercado,
  cantidadPedida: number,
  /** `capacidadCargaDe(ejercito, caravanas)`: los carros de sus jugadores mas lo que aporten las caravanas
   * adjuntas (Doc 5.13.2). Se pasa ya calculada para no arrastrar aqui la resolucion de las adjuntas. */
  capacidadCarga: number,
  instante: Instante
): { ejercito: Ejercito; plaza: Asentamiento; orden: OrdenMercado; cantidad: number; valor: number; comision: number; eventos: EventoCrudo[] } {
  if (ejercito.liderId !== heroeId) throw new OrdenInvalidaError('Solo el Lider de la columna comercia con su carro.');
  if (orden.asentamientoId !== plaza.id) throw new OrdenInvalidaError('Esa orden no es de esta plaza.');
  if (orden.estado !== 'activa') throw new OrdenInvalidaError('Esa orden ya no esta en pie.');
  if (instante >= orden.expiraEn) throw new OrdenInvalidaError('Esa orden ha caducado.');
  if (!enLaPuertaDe(ejercito, plaza)) throw new OrdenInvalidaError('Hay que estar en la plaza para comerciar en su mercado.');
  if (!tieneMercadoActivo(plaza)) throw new OrdenInvalidaError('La plaza ya no tiene Mercado activo.');
  if (cantidadPedida <= 0) throw new OrdenInvalidaError('La cantidad tiene que ser positiva.');
  // Un precio de 0 dejaria la operacion gratis. Pasa con un recurso sin `PRECIO_BASE` —el oro, que no cotiza
  // contra si mismo—, y de puertas afuera eso es oro infinito. Con el emparejamiento automatico el agujero
  // existia igual, solo que nadie podia pedirlo a mano.
  if (orden.precioUnitario <= 0) throw new OrdenInvalidaError('Esa orden no tiene precio: no se puede liquidar.');

  const { recurso } = orden;
  const tasa = tasaComision(plaza, ejercito);
  const pendiente = orden.cantidad - orden.cantidadCumplida;
  const espacioCarro = Math.max(0, capacidadCarga - cargaDe(ejercito));
  const enElCarro = ejercito.suministro[recurso] ?? 0;
  const oroEnElCarro = ejercito.suministro['oro'] ?? 0;
  // Lo que de verdad cuesta o rinde cada unidad, comision incluida: comprando se paga de mas, vendiendo se
  // cobra de menos, y el resto de topes se miden ya sobre esta cifra y no sobre el precio de escaparate.
  const precioNeto = orden.tipo === 'venta' ? orden.precioUnitario * (1 + tasa) : orden.precioUnitario * (1 - tasa);
  // Cuanto CRECE el carro por unidad: entra mercancia y sale oro, o al reves. Si no crece (o encoge), el carro
  // no es un limite y solo mandan los otros topes.
  const crecimientoPorUnidad = orden.tipo === 'venta' ? 1 - precioNeto : precioNeto - 1;
  const topeCarro = crecimientoPorUnidad > 0 ? espacioCarro / crecimientoPorUnidad : Number.POSITIVE_INFINITY;

  const cantidad =
    orden.tipo === 'venta'
      ? Math.min(cantidadPedida, pendiente, cantidadDisponible(plaza.almacen, recurso), oroEnElCarro / precioNeto, topeCarro)
      : Math.min(cantidadPedida, pendiente, enElCarro, huecoPara(plaza, recurso), cantidadDisponible(plaza.almacen, 'oro') / precioNeto, topeCarro);

  if (!(cantidad > 0)) {
    throw new OrdenInvalidaError(
      orden.tipo === 'venta'
        ? 'No se puede comprar nada: sin oro en el carro, sin sitio, o la plaza ya no tiene ese genero.'
        : 'No se puede vender nada: no llevas ese genero, la plaza no puede pagarlo, o no le cabe.'
    );
  }

  const valor = cantidad * orden.precioUnitario;
  const comision = valor * tasa;
  const oroMovido = cantidad * precioNeto; // valor + comision comprando, valor - comision vendiendo

  const suministro: Record<string, number> = { ...ejercito.suministro };
  let almacen = plaza.almacen;
  if (orden.tipo === 'venta') {
    suministro['oro'] = oroEnElCarro - oroMovido;
    suministro[recurso] = enElCarro + cantidad;
    almacen = agregarRecurso(descontarRecursos(almacen, { [recurso]: cantidad }), 'oro', oroMovido);
  } else {
    suministro[recurso] = enElCarro - cantidad;
    suministro['oro'] = oroEnElCarro + oroMovido;
    almacen = agregarRecurso(descontarRecursos(almacen, { oro: oroMovido }), recurso, cantidad);
  }
  // Un recurso a cero se BORRA del carro en vez de quedarse en 0: `cargaDe` suma valores y no le estorba, pero
  // la proyeccion enseña el carro tal cual y "0 de piedra" es ruido.
  if (suministro[recurso] === 0) delete suministro[recurso];
  if (suministro['oro'] === 0) delete suministro['oro'];

  const cumplida = orden.cantidadCumplida + cantidad;

  return {
    ejercito: { ...ejercito, suministro },
    plaza: { ...plaza, almacen },
    orden: { ...orden, cantidadCumplida: cumplida, estado: cumplida >= orden.cantidad ? 'cumplida' : 'activa' },
    cantidad,
    valor,
    comision,
    eventos: [
      {
        codigo: 'mercado.compra',
        mensaje:
          orden.tipo === 'venta'
            ? `Mercado de ${plaza.id}: una columna compra ${cantidad.toFixed(1)} ${recurso} por ${valor.toFixed(1)} oro (comisión ${comision.toFixed(1)}).`
            : `Mercado de ${plaza.id}: una columna entrega ${cantidad.toFixed(1)} ${recurso} y cobra ${valor.toFixed(1)} oro (comisión ${comision.toFixed(1)}).`,
        payload: {
          plazaId: plaza.id,
          columnaId: ejercito.id,
          tipo: orden.tipo,
          recurso,
          cantidad,
          valor,
          comision,
        } satisfies PayloadMercadoCompra,
      },
    ],
  };
}
