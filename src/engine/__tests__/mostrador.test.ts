// El mostrador: comerciar EN una plaza, en persona (`engine/market.ts`, Doc 3.3).
//
// Sustituye a los tests del emparejamiento automático entre plazas, que liquidaba órdenes al instante y sin
// que nada recorriera el mapa. Esa simplificación se retiró el 2026-09-07 a petición del usuario
// (`Consideraciones/Comercio_Fisico_Definicion.md`): los recursos no viajan solos, hay que ir a por ellos.
//
// Lo que estos tests fijan es justamente lo que la simplificación no podía tener: que haya que ESTAR allí,
// que el oro ocupe carro, y que el carro sea un tope real.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from './fixtures';
import type { Asentamiento, Ejercito, OrdenMercado } from '../../domain/types';
import { caducarOrdenes, comerciarEnPlaza, OrdenInvalidaError } from '../market';
import type { PayloadMercadoCompra } from '../market';
import { almacenSintetico } from './tradeFixtures';
import { COMISION, MERCADO } from '../../constants';

const POSICION = { x: 500, y: 500 };

function plazaConMercado(id: string, faccionId: string, recursos: Record<string, number>): Asentamiento {
  return {
    id,
    faccionId,
    posicion: POSICION,
    almacen: almacenSintetico(recursos),
    politicasActivas: [],
    edificios: [{ id: `mercado-${id}`, tipo: 'mercado', posicion: POSICION, estado: 'activo', nivelInterno: 1 }],
  } as unknown as Asentamiento;
}

/** Columna de un solo jugador, plantada en la puerta de la plaza salvo que se diga otra cosa. */
function columna(faccionId: string, suministro: Record<string, number>, opciones: { posicion?: { x: number; y: number } } = {}): Ejercito {
  return {
    id: 'columna-1',
    faccionId,
    liderId: 'jugador-1',
    tipo: 'personal',
    participantes: [{ jugadorId: 'jugador-1', unidoEn: instanteDeTest(0) }],
    escuadrones: [],
    suministro,
    caravanasAdjuntasIds: [],
    posicionActual: opciones.posicion ?? POSICION,
    estado: 'estacionado',
  } as unknown as Ejercito;
}

function orden(tipo: 'compra' | 'venta', recurso: string, cantidad: number, precioUnitario: number): OrdenMercado {
  return {
    id: `orden-${tipo}`,
    asentamientoId: 'plaza',
    tipo,
    recurso,
    cantidad,
    cantidadCumplida: 0,
    precioUnitario,
    creadoEn: instanteDeTest(0),
    expiraEn: instanteDeTest(MERCADO.plazoOrdenMinutos),
    estado: 'activa',
  };
}

describe('el mostrador: la plaza VENDE y el jugador compra', () => {
  it('paga con el oro de su carro, se lleva la mercancía en el carro, y la plaza cobra hasta el último gramo', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', { oro: 1000 });
    const venta = orden('venta', 'madera', 50, 2);

    const r = comerciarEnPlaza(ejercito, 'jugador-1', plaza, venta, 50, 5000, instanteDeTest(1));

    expect(r.cantidad).toBe(50);
    expect(r.valor).toBe(100);
    // Facciones distintas: comisión externa (Doc 3.5), sin política que la module.
    expect(r.comision).toBeCloseTo(100 * COMISION.tasaExterna, 6);

    // El carro: sale el oro (valor + comisión), entra la madera.
    expect(r.ejercito.suministro['madera']).toBe(50);
    expect(r.ejercito.suministro['oro']).toBeCloseTo(1000 - 100 - r.comision, 6);

    // La plaza: sale la madera, entra EXACTAMENTE lo que el jugador pagó. El oro se conserva — el
    // emparejamiento viejo acuñaba la comisión de la nada.
    expect(r.plaza.almacen['madera']!.cantidad).toBe(50);
    expect(r.plaza.almacen['oro']!.cantidad).toBeCloseTo(100 + r.comision, 6);

    expect(r.orden.cantidadCumplida).toBe(50);
    expect(r.orden.estado).toBe('cumplida');

    const evento = r.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba un evento migrado, no una cadena');
    expect(evento.codigo).toBe('mercado.compra');
    const p = evento.payload as PayloadMercadoCompra;
    expect(p.plazaId).toBe('plaza');
    expect(p.columnaId).toBe('columna-1');
    expect(p.tipo).toBe('venta');
  });

  it('sirve lo que puede: sin oro para tanto, compra menos en vez de fallar', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', { oro: 21.6 }); // da para 10 a precio 2 + 8% de comisión
    const venta = orden('venta', 'madera', 50, 2);

    const r = comerciarEnPlaza(ejercito, 'jugador-1', plaza, venta, 50, 5000, instanteDeTest(1));

    expect(r.cantidad).toBeCloseTo(10, 6);
    expect(r.orden.estado).toBe('activa'); // queda pendiente para el siguiente que pase
    expect(r.ejercito.suministro['oro']).toBeUndefined(); // gastó justo lo que llevaba
  });

  it('el carro es un tope real cuando el género pesa más que el oro que lo paga', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 1000, oro: 0 });
    // Precio 0.5: cada unidad comprada mete 1 de madera y saca ~0.54 de oro, así que el carro CRECE.
    const ejercito = columna('faccion-jugador', { oro: 900 });
    const venta = orden('venta', 'madera', 1000, 0.5);

    // Capacidad 1000 con 900 de oro ya dentro: quedan 100 de hueco.
    const r = comerciarEnPlaza(ejercito, 'jugador-1', plaza, venta, 1000, 1000, instanteDeTest(1));

    const cargaFinal = Object.values(r.ejercito.suministro).reduce((a, b) => a + b, 0);
    expect(cargaFinal).toBeLessThanOrEqual(1000 + 1e-9);
    expect(r.cantidad).toBeLessThan(1000);
    expect(r.cantidad).toBeGreaterThan(0);
  });

  it('con el carro seco no compra nada, y lo dice en vez de devolver cero', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', {});
    expect(() => comerciarEnPlaza(ejercito, 'jugador-1', plaza, orden('venta', 'madera', 50, 2), 50, 5000, instanteDeTest(1))).toThrow(
      OrdenInvalidaError
    );
  });
});

describe('el mostrador: la plaza COMPRA y el jugador vende', () => {
  it('descarga el género, cobra el oro en el carro, y la plaza paga de su almacén', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { piedra: 0, oro: 1000 });
    const ejercito = columna('faccion-jugador', { piedra: 40 });
    const compra = orden('compra', 'piedra', 40, 3);

    const r = comerciarEnPlaza(ejercito, 'jugador-1', plaza, compra, 40, 5000, instanteDeTest(1));

    expect(r.cantidad).toBe(40);
    expect(r.valor).toBe(120);
    // Vendiendo, la comisión se la queda la plaza: el jugador cobra de menos, no de más.
    expect(r.ejercito.suministro['piedra']).toBeUndefined();
    expect(r.ejercito.suministro['oro']).toBeCloseTo(120 - r.comision, 6);
    expect(r.plaza.almacen['piedra']!.cantidad).toBe(40);
    expect(r.plaza.almacen['oro']!.cantidad).toBeCloseTo(1000 - 120 + r.comision, 6);
  });

  it('no vende lo que no lleva encima', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { piedra: 0, oro: 1000 });
    const ejercito = columna('faccion-jugador', { madera: 40 });
    expect(() => comerciarEnPlaza(ejercito, 'jugador-1', plaza, orden('compra', 'piedra', 40, 3), 40, 5000, instanteDeTest(1))).toThrow(
      OrdenInvalidaError
    );
  });
});

describe('el mostrador: hay que estar allí, y ser quien manda el carro', () => {
  it('desde lejos no se comercia: es la diferencia entera con el emparejamiento que esto sustituye', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const lejos = columna('faccion-jugador', { oro: 1000 }, { posicion: { x: 1500, y: 1500 } });
    expect(() => comerciarEnPlaza(lejos, 'jugador-1', plaza, orden('venta', 'madera', 50, 2), 50, 5000, instanteDeTest(1))).toThrow(
      OrdenInvalidaError
    );
  });

  it('quien se unió a la columna no gasta el oro de todos: el carro es del Líder', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', { oro: 1000 });
    expect(() => comerciarEnPlaza(ejercito, 'jugador-2', plaza, orden('venta', 'madera', 50, 2), 50, 5000, instanteDeTest(1))).toThrow(
      OrdenInvalidaError
    );
  });

  it('una orden sin precio no se liquida: sería género gratis', () => {
    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', { oro: 1000 });
    expect(() => comerciarEnPlaza(ejercito, 'jugador-1', plaza, orden('venta', 'madera', 50, 0), 50, 5000, instanteDeTest(1))).toThrow(
      OrdenInvalidaError
    );
  });
});

describe('las ofertas que nadie toma se retiran solas', () => {
  it('caducan al cumplirse su plazo, y entonces ya no se pueden tomar', () => {
    const activa = orden('venta', 'madera', 50, 2);

    const antes = caducarOrdenes([activa], instanteDeTest(MERCADO.plazoOrdenMinutos - 1));
    expect(antes.ordenes[0]!.estado).toBe('activa');
    expect(antes.eventos).toHaveLength(0);

    const despues = caducarOrdenes([activa], instanteDeTest(MERCADO.plazoOrdenMinutos));
    expect(despues.ordenes[0]!.estado).toBe('expirada');
    const expirada = despues.eventos[0]!;
    if (typeof expirada === 'string') throw new Error('esperaba un evento migrado, no una cadena');
    expect(expirada.codigo).toBe('mercado.orden_expirada');

    const plaza = plazaConMercado('plaza', 'faccion-plaza', { madera: 100, oro: 0 });
    const ejercito = columna('faccion-jugador', { oro: 1000 });
    expect(() =>
      comerciarEnPlaza(ejercito, 'jugador-1', plaza, activa, 50, 5000, instanteDeTest(MERCADO.plazoOrdenMinutos))
    ).toThrow(OrdenInvalidaError);
  });

  it('una orden ya cumplida no se vuelve a caducar ni genera evento', () => {
    const cumplida: OrdenMercado = { ...orden('venta', 'madera', 50, 2), estado: 'cumplida' };
    const r = caducarOrdenes([cumplida], instanteDeTest(MERCADO.plazoOrdenMinutos * 10));
    expect(r.ordenes[0]!.estado).toBe('cumplida');
    expect(r.eventos).toHaveLength(0);
  });
});
