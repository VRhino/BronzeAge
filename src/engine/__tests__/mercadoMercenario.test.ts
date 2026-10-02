// El mercado de un campamento de mercenarios (Doc 1.9b, paso 4): solo vende, se repone con lo que se comercia entre Facciones en el
// mundo, y el oro cobrado se destruye.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Heroe, MercadoMercenario } from '../../domain/types';
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
  it('nace con stock de los bienes con precio de referencia, y el oro no se vende', () => {
    const m = mercadoInicial();
    expect(Object.keys(m).sort()).toEqual([...BIENES_EN_VENTA].sort());
    expect(Object.values(m).every((n) => n === MERCENARIOS.mercado.stockInicial)).toBe(true);
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
    const r = reponerMercados([campamento({ madera: 100 }), campamento({ madera: 290 }, 'c2')], mercado({ madera: 1000, piedra: 200 }), horas(3));
    expect(r.campamentos[0]!.mercado['madera']).toBe(100 + 1000 * MERCENARIOS.mercado.proporcionRepone);
    expect(r.campamentos[0]!.mercado['piedra']).toBe(200 * MERCENARIOS.mercado.proporcionRepone);
    expect(r.campamentos[1]!.mercado['madera'], 'topado').toBe(MERCENARIOS.mercado.topePorBien);
    expect(r.mercado.contadores).toEqual({});
    expect(r.mercado.reponeEn).toBe(horas(3 + MERCENARIOS.mercado.cadaHoras));
  });

  it('un bien que nadie ha comerciado no se repone, y sin contadores solo se agenda la siguiente', () => {
    const cs = [campamento({ madera: 10 })];
    const r = reponerMercados(cs, mercado({}), horas(5));
    expect(r.campamentos).toBe(cs);
    expect(r.mercado.reponeEn).toBe(horas(5 + MERCENARIOS.mercado.cadaHoras));
  });

  it('el mercado inicial agenda la primera reposición K horas después', () => {
    expect(mercadoMercenarioInicial(T0).reponeEn).toBe(horas(MERCENARIOS.mercado.cadaHoras));
  });
});

describe('comprarEnCampamento', () => {
  const precio = precioDeVenta('madera', mundo);

  it('el precio es el de referencia más el margen', () => {
    expect(precio).toBeGreaterThan(0);
    expect(precioDeVenta('oro', mundo), 'el oro no cotiza').toBe(0);
  });

  it('cobra del almacén personal, entrega en él y resta stock: el oro se destruye', () => {
    const r = comprarEnCampamento([campamento()], [heroe({ oro: 500 })], mundo, 'h1', 'madera', 10);
    expect(r.cantidad).toBe(10);
    expect(r.oro).toBe(Math.ceil(10 * precio));
    expect(r.heroes[0]!.almacenPersonal).toEqual({ oro: 500 - r.oro, madera: 10 });
    expect(r.campamentos[0]!.mercado['madera']).toBe(90);
  });

  it('sirve lo que puede cuando se pide de más: por stock, por oro y por sitio en el almacén', () => {
    expect(comprarEnCampamento([campamento({ madera: 4 })], [heroe({ oro: 500 })], mundo, 'h1', 'madera', 50).cantidad).toBe(4);
    const porOro = comprarEnCampamento([campamento()], [heroe({ oro: Math.ceil(precio * 3) })], mundo, 'h1', 'madera', 50);
    expect(porOro.cantidad).toBe(3);
    // Almacén casi lleno de otra cosa: caben pocas unidades, y el oro gastado deja sitio.
    const lleno = comprarEnCampamento([campamento()], [heroe({ piedra: 990, oro: 10 })], mundo, 'h1', 'madera', 50);
    expect(lleno.cantidad).toBeGreaterThan(0);
    const total = Object.values(lleno.heroes[0]!.almacenPersonal!).reduce((a, b) => a + b, 0);
    expect(total).toBeLessThanOrEqual(1000);
  });

  it('rechaza: no reside allí, sin stock, sin oro, el oro, bienes sin precio y cantidades menores que 1', () => {
    const compra = (c: CampamentoMercenarios, h: Heroe, recurso = 'madera', n = 5) => () => comprarEnCampamento([c], [h], mundo, 'h1', recurso, n);
    expect(compra({ ...campamento(), residentesIds: [] }, heroe({ oro: 500 }))).toThrow(MercenariosInvalidoError);
    expect(compra(campamento({ madera: 0 }), heroe({ oro: 500 }))).toThrow(MercenariosInvalidoError);
    expect(compra(campamento(), heroe({ oro: 0 }))).toThrow(MercenariosInvalidoError);
    expect(compra(campamento(), heroe({ oro: 500 }), 'oro')).toThrow(MercenariosInvalidoError);
    expect(compra(campamento({ armaBronce: 50 }), heroe({ oro: 500 }), 'armaBronce')).toThrow(MercenariosInvalidoError);
    expect(compra(campamento(), heroe({ oro: 500 }), 'madera', 0.5)).toThrow(MercenariosInvalidoError);
  });
});
