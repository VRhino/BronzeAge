// Refundar desde un campamento de mercenarios (Doc 1.9b, paso 5): el fondo de la Facción, la Caravana de Fundación al 75 % y su viaje.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Caravana, Ejercito, Faccion, Heroe } from '../../domain/types';
import { FUNDACION, MERCENARIOS } from '../../constants';
import { costoCaravanaFundacion } from '../expansion';
import { MercenariosInvalidoError } from '../mercenarios';
import { aportarARefundacion, comprarCaravanaDeRefundacion, costoRefundacion, fondoDeFaccion, retirarDeRefundacion } from '../refundacion';
import { crearFacciones, heroeDePrueba, instanteDeTest } from './fixtures';


const campamento = (extra: Partial<CampamentoMercenarios> = {}): CampamentoMercenarios =>
  ({
    id: 'merc-1',
    posicion: { x: 400, y: 400 },
    origen: 0,
    edificios: ['vivienda'],
    residentesIds: ['h1', 'h2'],
    poblacion: 0,
    poblacionEn: 0,
    mercado: {},
    fondos: {},
    creadoEn: 0,
    ...extra,
  }) as unknown as CampamentoMercenarios;
const faccion = (extra: Partial<Faccion> = {}): Faccion => ({ ...crearFacciones()[0]!, id: 'f', ciudadanosIds: ['h1', 'h2'], nivel: 1, ...extra });
const heroe = (id: string, almacenPersonal: Record<string, number>): Heroe => heroeDePrueba(id, { tipo: 'asentamiento', asentamientoId: 'x' }, { almacenPersonal });
const plaza = (faccionId: string): Asentamiento => ({ id: 'a', faccionId, posicion: { x: 1500, y: 1500 }, radioPotencial: 30 }) as unknown as Asentamiento;

describe('coste', () => {
  it('es el 75 % de una Caravana de Fundación normal, por recurso', () => {
    const normal = costoCaravanaFundacion();
    const r = costoRefundacion();
    for (const [recurso, n] of Object.entries(normal)) {
      if ((n ?? 0) <= 0) continue;
      expect(r[recurso]).toBe(Math.ceil((n ?? 0) * MERCENARIOS.refundacion.porcentajeCoste));
    }
  });
});

describe('fondo de refundación (D39)', () => {
  const columna = (suministro: Record<string, number>, extra: Partial<Ejercito> = {}) =>
    ({ id: 'col', liderId: 'h1', participantes: [{ heroeId: 'h1', unidoEn: 0 }], escuadronIds: [], suministro, caravanasAdjuntasIds: [], ...extra }) as unknown as Ejercito;

  it('desde el almacén personal en el campamento donde reside, y lo suyo se retira', () => {
    const a = aportarARefundacion(campamento(), heroe('h1', { madera: 100 }), undefined, faccion(), [], 'madera', 60, 'almacen');
    expect(a.movido).toBe(60);
    expect(a.heroe.almacenPersonal).toEqual({ madera: 40 });
    expect(a.campamento.fondos).toEqual({ h1: { madera: 60 } });

    const r = retirarDeRefundacion(a.campamento, a.heroe, undefined, 'madera', 25, 'almacen');
    expect(r.movido).toBe(25);
    expect(r.campamento.fondos['h1']).toEqual({ madera: 35 });
    expect(r.heroe.almacenPersonal).toEqual({ madera: 65 });
  });

  it('desde el carro en otro campamento: los materiales no viajan solos', () => {
    const ajeno = campamento({ residentesIds: [] });
    expect(() => aportarARefundacion(ajeno, heroe('h1', { madera: 100 }), undefined, faccion(), [], 'madera', 10, 'almacen')).toThrow(MercenariosInvalidoError);
    const a = aportarARefundacion(ajeno, heroe('h1', {}), columna({ madera: 50, trigo: 70 }), faccion(), [], 'madera', 30, 'carro');
    expect(a.columna!.suministro).toEqual({ madera: 20, trigo: 70 });
    expect(a.campamento.fondos).toEqual({ h1: { madera: 30 } });
  });

  it('el oro retirado vuelve como oro de botín (D27)', () => {
    const r = retirarDeRefundacion(campamento({ fondos: { h1: { oro: 20 } } }), heroe('h1', {}), undefined, 'oro', 20, 'almacen');
    expect(r.heroe.oroDeBotin).toBe(20);
    expect(r.heroe.almacenPersonal).toEqual({});
  });

  it('el fondo de la Facción suma a sus ciudadanos y no a otros', () => {
    const c = campamento({ fondos: { h1: { madera: 10 }, h2: { madera: 5 }, intruso: { madera: 999 } } });
    expect(fondoDeFaccion(c, faccion())).toEqual({ madera: 15 });
  });

  it('rechaza: solo una Facción sin asentamientos, solo ciudadanos, y más de lo que hay', () => {
    const h = heroe('h1', { madera: 10 });
    expect(() => aportarARefundacion(campamento(), h, undefined, faccion(), [plaza('f')], 'madera', 5, 'almacen')).toThrow(MercenariosInvalidoError);
    expect(() => aportarARefundacion(campamento(), h, undefined, faccion({ ciudadanosIds: [] }), [], 'madera', 5, 'almacen')).toThrow(MercenariosInvalidoError);
    expect(() => aportarARefundacion(campamento(), h, undefined, faccion(), [], 'piedra', 5, 'almacen')).toThrow(MercenariosInvalidoError);
    expect(aportarARefundacion(campamento(), h, undefined, faccion(), [], 'madera', 500, 'almacen').movido, 'solo lo que tiene').toBe(10);
  });
});

describe('comprar la Caravana de Fundación', () => {
  const costo = costoRefundacion();
  /** Un fondo que cubre el coste, repartido entre los dos héroes, con un sobrante de h2 que debe quedarse. */
  const fondoCompleto = (): CampamentoMercenarios => {
    const h1: Record<string, number> = {};
    const h2: Record<string, number> = {};
    for (const [r, n] of Object.entries(costo)) {
      h1[r] = Math.ceil(n / 2);
      h2[r] = n - Math.ceil(n / 2) + 3;
    }
    return campamento({ fondos: { h1, h2 } });
  };
  const compra = (c: CampamentoMercenarios, extra: { f?: Faccion; asentamientos?: Asentamiento[]; caravanas?: Caravana[] } = {}) =>
    comprarCaravanaDeRefundacion(c, extra.f ?? faccion(), extra.asentamientos ?? [], extra.caravanas ?? [], 'h1', instanteDeTest(0), 0);

  it('nace parada en el campamento, sin destino, con su titular, su Facción y quién aportó qué; deja el sobrante (D10, D11, D34)', () => {
    const r = compra(fondoCompleto());
    const c = r.caravana;

    expect(c.tipo).toBe('construccion');
    expect(c.origenCampamentoId).toBe('merc-1');
    expect(c.faccionId).toBe('f');
    expect(c.estado).toBe('disponible');
    expect(c.titularId).toBe('h1');
    expect(c.posicionActual).toEqual({ x: 400, y: 400 });
    expect(c.caducaEn).toBe(instanteDeTest(FUNDACION.caducidadCaravanaHoras * 60));
    // Lo gastado, por aportante: entre los dos, el coste exacto.
    for (const [recurso, n] of Object.entries(costo)) expect((c.aportes!['h1']?.[recurso] ?? 0) + (c.aportes!['h2']?.[recurso] ?? 0)).toBe(n);
    const sobra = Object.values(r.campamento.fondos).flatMap((f) => Object.values(f)).reduce((a, b) => a + b, 0);
    expect(sobra, 'solo queda lo que sobraba').toBe(3 * Object.keys(costo).length);
  });

  it('gasta lo mismo que cuenta: el aporte de quien se mudó se descuenta, y el de otra Facción no se toca', () => {
    const c = fondoCompleto();
    // h2 aportó y ya no reside aquí; un residente de otra Facción tiene un aporte que va primero en la lista.
    const r = compra({ ...c, residentesIds: ['ajeno', 'h1'], fondos: { ...c.fondos, ajeno: { ...costo } } });
    const fondos = r.campamento.fondos;
    expect(fondos['ajeno'], 'lo ajeno, intacto').toEqual(costo);
    const sobra = Object.values(fondos['h1'] ?? {}).concat(Object.values(fondos['h2'] ?? {})).reduce((a, b) => a + b, 0);
    expect(sobra, 'de la Facción solo queda el sobrante').toBe(3 * Object.keys(costo).length);
  });

  it('rechaza si el fondo no cubre el coste, si la Facción tiene plaza o si ya tiene una Caravana de Fundación', () => {
    expect(() => compra(campamento({ fondos: { h1: { madera: 1 } } }))).toThrow(MercenariosInvalidoError);
    expect(() => compra(fondoCompleto(), { asentamientos: [plaza('f')] })).toThrow(MercenariosInvalidoError);
    const enCamino = { tipo: 'construccion', faccionId: 'f' } as unknown as Caravana;
    expect(() => compra(fondoCompleto(), { caravanas: [enCamino] })).toThrow(MercenariosInvalidoError);
  });
});
