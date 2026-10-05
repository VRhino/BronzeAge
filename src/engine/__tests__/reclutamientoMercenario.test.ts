// Reclutar en un campamento de mercenarios (Doc 1.9b, paso 3): qué ofrece, con qué tecnología, a qué precio y gastando qué población.
import { describe, expect, it } from 'vitest';
import type { CampamentoMercenarios, Ejercito, EstadoTecnologia, Faccion, Heroe, TecnologiaId } from '../../domain/types';
import { MERCENARIOS, RECLUTAMIENTO_ORO_POR_ESCALON, ORO_POR_CABALLO, TROPAS_RECLUTABLES } from '../../constants';
import { minutos } from '../../domain/tiempo';
import { MercenariosInvalidoError } from '../mercenarios';
import { poblacionActual, precioPorSoldado, reclutarEnCampamento, tecnologiasDelCampamento, topePoblacion, tropasDelCampamento } from '../reclutamientoMercenario';
import { costeLiderazgo } from '../liderazgo';
import { escuadronDePrueba, heroeDePrueba, instanteDeTest } from './fixtures';

const T0 = instanteDeTest(0);
const tropa = (id: string) => TROPAS_RECLUTABLES.find((t) => t.id === id)!;
const LANCEROS = tropa('lanceros_mimbre'); // Barracón, nivel 1, equipo: 1 armaMadera
const ADOPTADAS: TecnologiaId[] = [LANCEROS.tecnologia];

const campamento = (extra: Partial<CampamentoMercenarios> = {}): CampamentoMercenarios =>
  ({
    id: 'merc-1',
    posicion: { x: 500, y: 500 },
    origen: 0,
    edificios: [...MERCENARIOS.edificiosFijos, 'barracon'],
    residentesIds: ['h1'],
    poblacion: 100,
    poblacionEn: T0,
    creadoEn: T0,
    ...extra,
  }) as CampamentoMercenarios;

const heroe = (extra: Partial<Heroe> = {}): Heroe => heroeDePrueba('h1', { tipo: 'asentamiento', asentamientoId: 'x' }, extra);
const faccion = (extra: Partial<Faccion> = {}): Faccion => ({ id: 'f', reputacion: 0, ciudadanosIds: ['h1'], ...extra }) as unknown as Faccion;
const columna = (extra: Partial<Ejercito> = {}): Ejercito =>
  ({ id: 'e1', liderId: 'h1', posicionActual: { x: 510, y: 500 }, suministro: {}, escuadronIds: [], ...extra }) as unknown as Ejercito;

/** Precio del escuadrón entero de lanceros para una Facción con plazas y sin recargo por reputación. */
const PRECIO = Math.ceil(precioPorSoldado(LANCEROS, faccion(), true) * LANCEROS.unidadesPorDefecto);

function recluta(opciones: Partial<{ campamentos: CampamentoMercenarios[]; heroes: Heroe[]; ejercitos: Ejercito[]; faccion: Faccion | undefined; conPlazas: boolean; pagarCon: 'almacenPersonal' | 'carro'; adoptadas: TecnologiaId[]; tropaId: string }> = {}) {
  return reclutarEnCampamento(
    opciones.campamentos ?? [campamento()],
    opciones.heroes ?? [heroe({ almacenPersonal: { oro: PRECIO } })],
    opciones.ejercitos ?? [],
    'h1',
    opciones.tropaId ?? LANCEROS.id,
    opciones.adoptadas ?? ADOPTADAS,
    'faccion' in opciones ? opciones.faccion : faccion(),
    opciones.conPlazas ?? true,
    opciones.pagarCon ?? 'almacenPersonal',
    T0,
    0
  );
}

describe('población del campamento', () => {
  it('el tope son sus viviendas por lo que da cada una, y recupera por hora hasta ese tope', () => {
    const c = campamento({ poblacion: 20 });
    expect(topePoblacion(c)).toBe(10 * MERCENARIOS.poblacionPorVivienda);
    expect(poblacionActual(c, T0)).toBe(20);
    expect(poblacionActual(c, (T0 + minutos(60) * 3) as typeof T0)).toBe(20 + 3 * MERCENARIOS.poblacionPorHora);
    expect(poblacionActual(c, (T0 + minutos(60) * 1000) as typeof T0), 'no pasa del tope').toBe(topePoblacion(c));
  });

  it('un campamento con más viviendas tiene más tope sin tocar nada más', () => {
    expect(topePoblacion(campamento({ edificios: ['vivienda', 'vivienda', 'vivienda', 'barracon'] }))).toBe(3 * MERCENARIOS.poblacionPorVivienda);
  });
});

describe('qué se recluta', () => {
  it('lo que permiten sus edificios y lo que desbloquea su tecnología: sin edificio o sin tecnología no sale', () => {
    expect(tropasDelCampamento(campamento(), ADOPTADAS).map((t) => t.id)).toContain(LANCEROS.id);
    expect(tropasDelCampamento(campamento({ edificios: ['taberna', 'vivienda', 'mercado', 'galeriaDeTiro'] }), ADOPTADAS).map((t) => t.id), 'sin Barracón').not.toContain(LANCEROS.id);
    expect(tropasDelCampamento(campamento(), []).map((t) => t.id), 'sin tecnología').not.toContain(LANCEROS.id);
    // La leva del Centro Urbano no existe aquí.
    expect(tropasDelCampamento(campamento(), ['leva_comunal']).every((t) => t.edificio !== 'centroUrbano')).toBe(true);
  });
});

describe('tecnología propia del campamento', () => {
  const tecnologia = (primeros: EstadoTecnologia['primeros'], adoptaPor: Record<string, TecnologiaId[]> = {}): EstadoTecnologia =>
    ({
      era: 'reinos_palaciales',
      eraDesde: T0,
      contadores: {},
      logros: {},
      primeros,
      porFaccion: Object.fromEntries(Object.entries(adoptaPor).map(([f, adoptadas]) => [f, { aparecidas: adoptadas, adoptadas }])),
    }) as EstadoTecnologia;
  const f = (id: string) => ({ id }) as Faccion;
  const ID: TecnologiaId = 'escudos_ligeros';

  it('desbloquea la última cuando la tiene el X % de las Facciones humanas vivas', () => {
    const t = tecnologia({ [ID]: { faccionId: 'a', en: T0 } }, { a: [ID], b: [ID] });
    expect(tecnologiasDelCampamento(t, [f('a'), f('b'), f('c'), f('d')], T0)).toContain(ID);
    expect(tecnologiasDelCampamento(t, [f('a'), f('b'), f('c'), f('d'), f('e')], T0), '2 de 5 no llega a la mitad').not.toContain(ID);
  });

  it('o a las T horas de que la primera la desbloqueó, lo que llegue antes', () => {
    const t = tecnologia({ [ID]: { faccionId: 'a', en: T0 } }, { a: [ID] });
    const humanas = [f('a'), f('b'), f('c')];
    const casi = (T0 + minutos(60 * MERCENARIOS.tecnologia.horasTrasLaPrimera) - 1) as typeof T0;
    const justo = (T0 + minutos(60 * MERCENARIOS.tecnologia.horasTrasLaPrimera)) as typeof T0;
    expect(tecnologiasDelCampamento(t, humanas, casi)).not.toContain(ID);
    expect(tecnologiasDelCampamento(t, humanas, justo)).toContain(ID);
  });

  it('una que nadie ha desbloqueado no está, y sin Facciones humanas manda solo el plazo', () => {
    expect(tecnologiasDelCampamento(tecnologia({}), [f('a')], T0)).not.toContain(ID);
    const t = tecnologia({ [ID]: { faccionId: 'a', en: T0 } });
    expect(tecnologiasDelCampamento(t, [], T0)).not.toContain(ID);
    expect(tecnologiasDelCampamento(t, [], (T0 + minutos(60 * 100)) as typeof T0)).toContain(ID);
  });
});

describe('precio', () => {
  it('base (oro del escalón, caballos y equipo) × recargo; la reputación baja lo encarece; sin asentamientos lo abarata', () => {
    const base = (RECLUTAMIENTO_ORO_POR_ESCALON[LANCEROS.escalon] ?? 0) + 1 * MERCENARIOS.valorEquipoEnOro;
    expect(precioPorSoldado(LANCEROS, faccion(), true)).toBeCloseTo(base * MERCENARIOS.recargo);
    expect(precioPorSoldado(LANCEROS, faccion({ reputacion: -100 }), true)).toBeGreaterThan(precioPorSoldado(LANCEROS, faccion(), true));
    expect(precioPorSoldado(LANCEROS, faccion(), false)).toBeCloseTo(base * MERCENARIOS.recargo * MERCENARIOS.descuentoSinAsentamientos);
    expect(precioPorSoldado(LANCEROS, undefined, false), 'sin Facción no hay descuento').toBeCloseTo(base * MERCENARIOS.recargo);
  });

  it('los caballos cuestan oro', () => {
    const caballeria = TROPAS_RECLUTABLES.find((t) => (t.caballos ?? 0) > 0)!;
    const sinCaballos = { ...caballeria, caballos: 0 };
    expect(precioPorSoldado(caballeria, faccion(), true) - precioPorSoldado(sinCaballos, faccion(), true)).toBeCloseTo((caballeria.caballos ?? 0) * ORO_POR_CABALLO * MERCENARIOS.recargo);
  });
});

describe('reclutarEnCampamento', () => {
  it('crea el escuadrón en el campamento del héroe, cobra del almacén personal y gasta población', () => {
    const r = recluta();

    expect(r.cantidad).toBe(LANCEROS.unidadesPorDefecto);
    expect(r.oro).toBe(PRECIO);
    const e = r.heroes[0]!.escuadrones.find((x) => x.tropaId === LANCEROS.id)!;
    expect(e.cantidad).toBe(LANCEROS.unidadesPorDefecto);
    expect(e.contenedor).toEqual({ tipo: 'campamento' });
    expect(r.heroes[0]!.almacenPersonal).toEqual({});
    expect(r.campamentos[0]!.poblacion).toBe(100 - LANCEROS.unidadesPorDefecto);
    expect(r.campamentos[0]!.poblacionEn).toBe(T0);
  });

  it('si ya tiene el escuadrón, repone hasta el tope y solo cobra lo que falta', () => {
    const primero = recluta();
    const herido = { ...primero.heroes[0]!, escuadrones: primero.heroes[0]!.escuadrones.map((e) => ({ ...e, cantidad: 10 })), almacenPersonal: { oro: PRECIO } };
    const r = recluta({ heroes: [herido], campamentos: primero.campamentos });

    expect(r.cantidad).toBe(LANCEROS.unidadesPorDefecto - 10);
    expect(r.heroes[0]!.escuadrones.filter((e) => e.tropaId === LANCEROS.id)).toHaveLength(1);
    expect(r.heroes[0]!.escuadrones.find((e) => e.tropaId === LANCEROS.id)!.cantidad).toBe(LANCEROS.unidadesPorDefecto);
  });

  it('paga con el carro si es el Líder de una columna a la puerta', () => {
    const r = recluta({ heroes: [heroe()], ejercitos: [columna({ suministro: { oro: PRECIO + 5 } })], pagarCon: 'carro' });
    expect(r.ejercitos[0]!.suministro).toEqual({ oro: 5 });
    expect(r.heroes[0]!.almacenPersonal ?? {}).toEqual({});
    // Lejos de la puerta o sin ser Líder, no.
    expect(() => recluta({ heroes: [heroe()], ejercitos: [columna({ suministro: { oro: PRECIO }, posicionActual: { x: 900, y: 900 } })], pagarCon: 'carro' })).toThrow(MercenariosInvalidoError);
    expect(() => recluta({ heroes: [heroe()], ejercitos: [columna({ suministro: { oro: PRECIO }, liderId: 'otro' })], pagarCon: 'carro' })).toThrow(MercenariosInvalidoError);
  });

  describe('con la columna a la puerta', () => {
    const conColumna = (extra: Partial<Ejercito> = {}) => [columna(extra)];
    const paga = { almacenPersonal: { oro: PRECIO } };

    it('el escuadrón nuevo se une a ella si le cabe en el Liderazgo', () => {
      const r = recluta({ heroes: [heroe(paga)], ejercitos: conColumna() });

      expect(r.seUne).toBe(true);
      const e = r.heroes[0]!.escuadrones.find((x) => x.tropaId === LANCEROS.id)!;
      expect(e.contenedor).toEqual({ tipo: 'ejercito', ejercitoId: 'e1' });
      expect(r.ejercitos[0]!.escuadronIds).toEqual([e.id]);
    });

    it('si no le cabe, nace en el campamento: el Liderazgo cuenta también lo que ya lleva fuera', () => {
      expect(recluta({ heroes: [heroe({ ...paga, liderazgoBase: 0 })], ejercitos: conColumna() }).seUne, 'sin Liderazgo').toBe(false);

      const fuera = { ...escuadronDePrueba('ya-fuera', 'h1', 'milicia_lanceros', 5), contenedor: { tipo: 'ejercito' as const, ejercitoId: 'e1' } };
      const justo = costeLiderazgo(fuera.tropaId) + costeLiderazgo(LANCEROS.id) - 1;
      const r = recluta({ heroes: [heroe({ ...paga, liderazgoBase: justo, escuadrones: [fuera] })], ejercitos: conColumna({ escuadronIds: ['ya-fuera'] }) });
      expect(r.seUne).toBe(false);
      expect(r.heroes[0]!.escuadrones.find((x) => x.tropaId === LANCEROS.id)!.contenedor).toEqual({ tipo: 'campamento' });
      expect(r.ejercitos[0]!.escuadronIds).toEqual(['ya-fuera']);
    });

    it('solo si es su Líder y la tiene a la puerta; y reponer uno que ya tiene no lo mueve', () => {
      expect(recluta({ heroes: [heroe(paga)], ejercitos: conColumna({ liderId: 'otro' }) }).seUne, 'no es su Líder').toBe(false);
      expect(recluta({ heroes: [heroe(paga)], ejercitos: conColumna({ posicionActual: { x: 900, y: 900 } }) }).seUne, 'lejos de la puerta').toBe(false);

      const primero = recluta();
      const herido = { ...primero.heroes[0]!, escuadrones: primero.heroes[0]!.escuadrones.map((e) => ({ ...e, cantidad: 10 })), almacenPersonal: { oro: PRECIO } };
      const r = recluta({ heroes: [herido], campamentos: primero.campamentos, ejercitos: conColumna() });
      expect(r.seUne).toBe(false);
      expect(r.heroes[0]!.escuadrones.find((e) => e.tropaId === LANCEROS.id)!.contenedor).toEqual({ tipo: 'campamento' });
    });
  });

  it('rechaza: no reside allí, tropa que no ofrece, sin oro, sin reclutas, ya al tope', () => {
    expect(() => recluta({ campamentos: [campamento({ residentesIds: [] })] })).toThrow(MercenariosInvalidoError);
    expect(() => recluta({ tropaId: 'granjeros' })).toThrow(MercenariosInvalidoError);
    expect(() => recluta({ adoptadas: [] })).toThrow(MercenariosInvalidoError);
    expect(() => recluta({ heroes: [heroe({ almacenPersonal: { oro: PRECIO - 1 } })] })).toThrow(MercenariosInvalidoError);
    expect(() => recluta({ campamentos: [campamento({ poblacion: LANCEROS.unidadesPorDefecto - 1 })] })).toThrow(MercenariosInvalidoError);

    const lleno = recluta();
    const alTope = { ...lleno.heroes[0]!, almacenPersonal: { oro: PRECIO } };
    expect(() => recluta({ heroes: [alTope], campamentos: lleno.campamentos })).toThrow(MercenariosInvalidoError);
  });

  it('la población que se gasta se recupera con el tiempo, sin ningún tick', () => {
    const r = recluta();
    const c = r.campamentos[0]!;
    expect(poblacionActual(c, T0)).toBe(100 - LANCEROS.unidadesPorDefecto);
    expect(poblacionActual(c, (T0 + minutos(60)) as typeof T0)).toBe(100 - LANCEROS.unidadesPorDefecto + MERCENARIOS.poblacionPorHora);
    expect(poblacionActual(c, (T0 + minutos(60) * 10) as typeof T0), 'y no pasa del tope').toBe(100);
  });
});
