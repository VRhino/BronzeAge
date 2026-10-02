// Refundar desde un campamento de mercenarios (Doc 1.9b, paso 5): el fondo de la Facción, la Caravana de Fundación al 75 % y su viaje.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Caravana, Faccion, Heroe } from '../../domain/types';
import { MERCENARIOS } from '../../constants';
import { avanzarCaravanasFundacion, costoCaravanaFundacion } from '../expansion';
import { MercenariosInvalidoError } from '../mercenarios';
import { aportarARefundacion, comprarCaravanaDeRefundacion, costoRefundacion, fondoDeFaccion, retirarDeRefundacion } from '../refundacion';
import { crearFacciones, crearMapaDeterminista, heroeDePrueba, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(42);
const DESTINO = { x: 430, y: 430 };

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

describe('fondo de refundación', () => {
  it('cada héroe aporta lo que quiere desde su almacén personal, y lo suyo se retira', () => {
    const a = aportarARefundacion([campamento()], [heroe('h1', { madera: 100 })], faccion(), [], 'h1', 'madera', 60);
    expect(a.movido).toBe(60);
    expect(a.heroes[0]!.almacenPersonal).toEqual({ madera: 40 });
    expect(a.campamentos[0]!.fondos).toEqual({ h1: { madera: 60 } });

    const r = retirarDeRefundacion(a.campamentos, a.heroes, 'h1', 'madera', 25, 1000);
    expect(r.movido).toBe(25);
    expect(r.campamentos[0]!.fondos['h1']).toEqual({ madera: 35 });
    expect(r.heroes[0]!.almacenPersonal).toEqual({ madera: 65 });
  });

  it('el fondo de la Facción suma a sus ciudadanos y no a otros', () => {
    const c = campamento({ fondos: { h1: { madera: 10 }, h2: { madera: 5 }, intruso: { madera: 999 } } });
    expect(fondoDeFaccion(c, faccion())).toEqual({ madera: 15 });
  });

  it('rechaza: solo una Facción sin asentamientos, solo ciudadanos, solo en el campamento donde reside, y más de lo que hay', () => {
    const h = [heroe('h1', { madera: 10 })];
    expect(() => aportarARefundacion([campamento()], h, faccion(), [plaza('f')], 'h1', 'madera', 5)).toThrow(MercenariosInvalidoError);
    expect(() => aportarARefundacion([campamento()], h, faccion({ ciudadanosIds: [] }), [], 'h1', 'madera', 5)).toThrow(MercenariosInvalidoError);
    expect(() => aportarARefundacion([campamento({ residentesIds: [] })], h, faccion(), [], 'h1', 'madera', 5)).toThrow(MercenariosInvalidoError);
    expect(() => aportarARefundacion([campamento()], h, faccion(), [], 'h1', 'piedra', 5)).toThrow(MercenariosInvalidoError);
    expect(aportarARefundacion([campamento()], h, faccion(), [], 'h1', 'madera', 500).movido, 'solo lo que tiene').toBe(10);
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
  const compra = (c: CampamentoMercenarios, extra: { f?: Faccion; asentamientos?: Asentamiento[]; caravanas?: Caravana[]; destino?: { x: number; y: number } } = {}) =>
    comprarCaravanaDeRefundacion([c], extra.f ?? faccion(), extra.asentamientos ?? [], extra.caravanas ?? [], mapa, 'h1', extra.destino ?? DESTINO, 0);

  it('sale del campamento con la Facción en sí misma, gasta el fondo y deja el sobrante', () => {
    const r = compra(fondoCompleto());
    const c = r.caravana;

    expect(c.tipo).toBe('construccion');
    expect(c.origenCampamentoId).toBe('merc-1');
    expect(c.faccionId).toBe('f');
    expect(c.destinoPosicion).toEqual(DESTINO);
    expect(c.heroesFundadoresIds).toEqual(['h1']);
    expect(c.posicionActual).toEqual({ x: 400, y: 400 });
    expect(c.ruta?.length).toBeGreaterThan(1);
    const sobra = Object.values(r.campamentos[0]!.fondos).flatMap((f) => Object.values(f)).reduce((a, b) => a + b, 0);
    expect(sobra, 'solo queda lo que sobraba').toBe(3 * Object.keys(costo).length);
  });

  it('rechaza si el fondo no cubre el coste, si la Facción tiene plaza, si ya hay una caravana en camino o el destino está reclamado', () => {
    expect(() => compra(campamento({ fondos: { h1: { madera: 1 } } }))).toThrow(MercenariosInvalidoError);
    expect(() => compra(fondoCompleto(), { asentamientos: [plaza('f')] })).toThrow(MercenariosInvalidoError);
    const enCamino = { tipo: 'construccion', faccionId: 'f' } as unknown as Caravana;
    expect(() => compra(fondoCompleto(), { caravanas: [enCamino] })).toThrow(MercenariosInvalidoError);
    const reclamado = { ...plaza('g'), posicion: DESTINO, radioPotencial: 100 } as Asentamiento;
    expect(() => compra(fondoCompleto(), { asentamientos: [reclamado] })).toThrow(MercenariosInvalidoError);
  });

  it('viaja y funda como cualquier otra: la Facción sale de la propia caravana, sin asentamiento de origen', () => {
    const { caravana } = compra(fondoCompleto());
    let r = { caravanas: [{ ...caravana, progreso: 0.999999 }] as Caravana[], asentamientos: [] as Asentamiento[], facciones: [faccion({ ciudadanosIds: ['h1'] })], eventos: [] as unknown[] };
    for (let t = 1; t < 400 && r.asentamientos.length === 0; t++) {
      r = avanzarCaravanasFundacion(r.caravanas, mapa, r.facciones, r.asentamientos, instanteDeTest(t));
    }
    expect(r.asentamientos).toHaveLength(1);
    expect(r.asentamientos[0]!.faccionId).toBe('f');
    expect(r.asentamientos[0]!.heroesFundadoresIds).toEqual(['h1']);
    expect(r.caravanas).toEqual([]);
  });
});
