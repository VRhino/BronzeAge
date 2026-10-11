// El mercado de un campamento de mercenarios (Doc 1.9b, paso 4): solo vende, se repone con lo que se comercia entre Facciones en el
// mundo, y el oro cobrado se destruye.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Ejercito, Heroe, MercadoMercenario } from '../../domain/types';
import { MERCENARIOS } from '../../constants';
import { minutos } from '../../domain/tiempo';
import { MercenariosInvalidoError } from '../mercenarios';
import {
  BIENES_EN_VENTA,
  comerciadoDeEventos,
  comprarEnCampamento,
  mercadoInicial,
  mercadoMercenarioInicial,
  precioDeVenta,
  reponerMercados,
  sumarComerciado,
} from '../mercadoMercenario';
import { heroeDePrueba, instanteDeTest } from './fixtures';

const T0 = instanteDeTest(0);
const horas = (n: number) => (T0 + minutos(60 * n)) as typeof T0;

const campamento = (mercado: Record<string, number> = { madera: 100 }, id = 'c1'): CampamentoMercenarios =>
  ({ id, posicion: { x: 0, y: 0 }, origen: 0, edificios: ['vivienda'], residentesIds: ['h1'], poblacion: 50, poblacionEn: T0, mercado, creadoEn: T0 }) as unknown as CampamentoMercenarios;
const heroe = (almacenPersonal?: Record<string, number>): Heroe => heroeDePrueba('h1', { tipo: 'asentamiento', asentamientoId: 'x' }, { almacenPersonal });
/** Sin stock en ningún almacén el precio de referencia es el máximo (×3 del base): se comprueba contra `precioDeVenta`, no contra una cifra. */
const mundo: Asentamiento[] = [];

describe('mercado inicial', () => {
  it('nace con stock de los bienes con precio de referencia —las pilas llenas (D41)—, y el oro no se vende', () => {
    const m = mercadoInicial();
    expect(Object.keys(m).sort()).toEqual([...BIENES_EN_VENTA].sort());
    expect(m['cobre']).toBe(MERCENARIOS.mercado.stockInicial);
    for (const [bien, pila] of Object.entries(MERCENARIOS.mercado.pilas)) expect(m[bien]).toBe(pila);
    expect(m['oro']).toBeUndefined();
  });
});

describe('lo comerciado en el mundo', () => {
  const trueque = (entreFacciones: boolean, intercambiado: Record<string, number>) => ({ codigo: 'comercio.trueque_cumplido', mensaje: '', payload: { entreFacciones, intercambiado } });
  const mostrador = (entreFacciones: boolean, recurso: string, cantidad: number) => ({ codigo: 'mercado.comercio_en_plaza', mensaje: '', payload: { entreFacciones, recurso, cantidad } });

  it('suma el trueque cerrado y la venta del mostrador solo si son entre Facciones distintas', () => {
    const eventos = [
      trueque(true, { madera: 100, oro: 5 }),
      trueque(false, { madera: 9999 }), // con uno mismo: no cuenta
      mostrador(true, 'madera', 20),
      mostrador(false, 'piedra', 500),
      'texto sin estructura',
    ];
    expect(comerciadoDeEventos(eventos)).toEqual({ madera: 120, oro: 5 });
  });

  it('sumarComerciado acumula, y sin novedad devuelve el mismo objeto', () => {
    const m: MercadoMercenario = { contadores: { madera: 10 }, reponeEn: horas(3) };
    expect(sumarComerciado(m, { madera: 5, piedra: 1 }).contadores).toEqual({ madera: 15, piedra: 1 });
    expect(sumarComerciado(m, {})).toBe(m);
  });
});

describe('reposición', () => {
  const mercado = (contadores: Record<string, number>, reponeEn = horas(3)): MercadoMercenario => ({ contadores, reponeEn });

  it('antes de la hora no cambia nada', () => {
    const cs = [campamento()];
    const r = reponerMercados(cs, mercado({ madera: 1000 }), horas(2));
    expect(r.campamentos).toBe(cs);
    expect(r.mercado.contadores).toEqual({ madera: 1000 });
  });

  it('a la hora, cada campamento recibe la proporción de lo comerciado, con tope, y los contadores vuelven a cero', () => {
    const r = reponerMercados([campamento({ cobre: 100 }), campamento({ cobre: 290 }, 'c2')], mercado({ cobre: 1000, estano: 200 }), horas(3));
    expect(r.campamentos[0]!.mercado['cobre']).toBe(100 + 1000 * MERCENARIOS.mercado.proporcionRepone);
    expect(r.campamentos[0]!.mercado['estano']).toBe(200 * MERCENARIOS.mercado.proporcionRepone);
    expect(r.campamentos[1]!.mercado['cobre'], 'topado').toBe(MERCENARIOS.mercado.topePorBien);
    expect(r.mercado.contadores).toEqual({});
    expect(r.mercado.reponeEn).toBe(horas(3 + MERCENARIOS.mercado.cadaHoras));
  });

  it('un bien que nadie ha comerciado no se repone; las pilas (D41) vuelven a su tope igual', () => {
    const cs = [campamento({ cobre: 10, madera: 3 })];
    const r = reponerMercados(cs, mercado({}), horas(5));
    expect(r.campamentos[0]!.mercado['cobre']).toBe(10);
    expect(r.campamentos[0]!.mercado['madera']).toBe(MERCENARIOS.mercado.pilas['madera']);
    expect(r.mercado.reponeEn).toBe(horas(5 + MERCENARIOS.mercado.cadaHoras));
  });

  it('el mercado inicial agenda la primera reposición K horas después', () => {
    expect(mercadoMercenarioInicial(T0).reponeEn).toBe(horas(MERCENARIOS.mercado.cadaHoras));
  });
});

describe('comprarEnCampamento', () => {
  const precio = precioDeVenta('madera', mundo);
  const compra = (c: CampamentoMercenarios, h: Heroe, recurso: string, n: number, columna?: Ejercito, instante = T0, capacidad?: number) =>
    comprarEnCampamento(c, h, columna, mundo, recurso, n, instante, capacidad);

  it('el precio es el de referencia más el margen', () => {
    expect(precio).toBeGreaterThan(0);
    expect(precioDeVenta('oro', mundo), 'el oro no cotiza').toBe(0);
  });

  it('cobra del almacén personal, entrega en él y resta stock: el oro se destruye', () => {
    const r = compra(campamento(), heroe({ oro: 500 }), 'madera', 10);
    expect(r.cantidad).toBe(10);
    expect(r.oro).toBe(Math.ceil(10 * precio));
    expect(r.heroe.almacenPersonal).toEqual({ oro: 500 - r.oro, madera: 10 });
    expect(r.campamento.mercado['madera']).toBe(90);
  });

  it('sirve lo que puede cuando se pide de más: por stock, por oro y por sitio en el almacén', () => {
    expect(compra(campamento({ madera: 4 }), heroe({ oro: 500 }), 'madera', 50).cantidad).toBe(4);
    expect(compra(campamento(), heroe({ oro: Math.ceil(precio * 3) }), 'madera', 50).cantidad).toBe(3);
    const lleno = compra(campamento(), heroe({ piedra: 990, oro: 10 }), 'madera', 50);
    expect(lleno.cantidad).toBeGreaterThan(0);
    expect(Object.values(lleno.heroe.almacenPersonal!).reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(1000);
  });

  it('cupo diario por héroe en madera y piedra (D41): se agota y vuelve al día siguiente; el trigo no tiene', () => {
    const tope = MERCENARIOS.mercado.cupoDiario['madera']!;
    const primera = compra(campamento({ madera: 300 }), heroe({ oro: 900 }), 'madera', 1000);
    expect(primera.cantidad).toBe(tope);
    expect(() => compra(primera.campamento, primera.heroe, 'madera', 1)).toThrow(MercenariosInvalidoError);
    const manana = (T0 + 86_400_000) as typeof T0;
    expect(compra(primera.campamento, primera.heroe, 'madera', 1, undefined, manana).cantidad).toBe(1);
    expect(compra(campamento({ trigo: 300 }), heroe({ oro: 900 }), 'trigo', 70).cantidad, 'más que el cupo de madera').toBe(70);
  });

  describe('quien no reside, desde la puerta (2026-10-11)', () => {
    const forastero = () => ({ ...campamento({ trigo: 300, madera: 300, cobre: 100 }), residentesIds: [] });
    const columna = (suministro: Record<string, number>) =>
      ({ id: 'col', participantes: [{ heroeId: 'h1', unidoEn: T0 }], escuadronIds: [], suministro, caravanasAdjuntasIds: [] }) as unknown as Ejercito;

    it('compra cualquier bien en venta —también cobre o madera—, paga con el oro del carro y se lo lleva al carro', () => {
      const precioCobre = precioDeVenta('cobre', mundo);
      const r = compra(forastero(), heroe({ oro: 999 }), 'cobre', 20, columna({ oro: 500 }));
      expect(r.cantidad).toBe(20);
      expect(r.oro).toBe(Math.ceil(20 * precioCobre));
      expect(r.columna!.suministro).toEqual({ oro: 500 - r.oro, cobre: 20 });
      expect(r.campamento.mercado['cobre']).toBe(80);
      expect(r.heroe, 'su almacén personal ni se toca ni paga').toEqual(heroe({ oro: 999 }));
      expect(compra(forastero(), heroe({ oro: 0 }), 'trigo', 5, columna({ oro: 50 })).columna!.suministro['trigo']).toBe(5);
    });

    it('sin cupo diario: compra más madera que el tope del residente, y cuantas veces quiera', () => {
      const tope = MERCENARIOS.mercado.cupoDiario['madera']!;
      const r = compra(forastero(), heroe({ oro: 0 }), 'madera', tope + 40, columna({ oro: 400 }));
      expect(r.cantidad).toBe(tope + 40);
      expect(compra(r.campamento, r.heroe, 'madera', 1, r.columna).cantidad).toBe(1);
    });

    it('sirve lo que puede: por stock, por oro del carro y por sitio en el carro', () => {
      expect(compra({ ...forastero(), mercado: { cobre: 4 } }, heroe(), 'cobre', 50, columna({ oro: 500 })).cantidad).toBe(4);
      const precioCobre = precioDeVenta('cobre', mundo);
      expect(compra(forastero(), heroe(), 'cobre', 50, columna({ oro: Math.ceil(precioCobre * 3) })).cantidad).toBe(3);
      // Un carro lleno de otra cosa solo deja sitio para lo que el oro gastado libera.
      const lleno = compra(forastero(), heroe(), 'cobre', 500, columna({ oro: 400, piedra: 100 }), T0, 500);
      expect(Object.values(lleno.columna!.suministro).reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(500);
    });

    it('rechaza: sin columna, sin oro en el carro, sin stock, el oro y los bienes sin precio', () => {
      expect(() => compra(forastero(), heroe({ oro: 999 }), 'trigo', 5)).toThrow(MercenariosInvalidoError);
      expect(() => compra(forastero(), heroe({ oro: 999 }), 'cobre', 5, columna({}))).toThrow(MercenariosInvalidoError);
      expect(() => compra({ ...forastero(), mercado: { cobre: 0 } }, heroe(), 'cobre', 5, columna({ oro: 500 }))).toThrow(MercenariosInvalidoError);
      expect(() => compra(forastero(), heroe(), 'oro', 5, columna({ oro: 500 }))).toThrow(MercenariosInvalidoError);
      expect(() => compra({ ...forastero(), mercado: { armaBronce: 50 } }, heroe(), 'armaBronce', 5, columna({ oro: 500 }))).toThrow(MercenariosInvalidoError);
    });
  });

  it('paga primero con el oro de botín (D27), que no ocupa sitio en el almacén', () => {
    const r = compra(campamento(), { ...heroe({ oro: 100 }), oroDeBotin: 5 }, 'madera', 10);
    expect(r.heroe.oroDeBotin).toBe(Math.max(0, 5 - r.oro));
    expect(r.heroe.almacenPersonal!['oro']).toBe(100 - Math.max(0, r.oro - 5));
  });

  it('rechaza: sin stock, sin oro, el oro, bienes sin precio y cantidades menores que 1', () => {
    expect(() => compra(campamento({ madera: 0 }), heroe({ oro: 500 }), 'madera', 5)).toThrow(MercenariosInvalidoError);
    expect(() => compra(campamento(), heroe({ oro: 0 }), 'madera', 5)).toThrow(MercenariosInvalidoError);
    expect(() => compra(campamento(), heroe({ oro: 500 }), 'oro', 5)).toThrow(MercenariosInvalidoError);
    expect(() => compra(campamento({ armaBronce: 50 }), heroe({ oro: 500 }), 'armaBronce', 5)).toThrow(MercenariosInvalidoError);
    expect(() => compra(campamento(), heroe({ oro: 500 }), 'madera', 0.5)).toThrow(MercenariosInvalidoError);
  });
});
