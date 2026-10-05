// La Caravana de Fundación única (Doc 1.8, D30): la de una plaza y la de un campamento son la misma entidad. Se lanza o se compra sin destino,
// parada en su origen con un titular; sin nadie que la lleve vuelve sola, suelta caduca, y al desarmarse o caducar devuelve lo que costó.
// Y funda donde esté su columna (`fundarConCaravana`): nunca encima de un campamento de mercenarios (D16).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Caravana, Ejercito, Faccion, Heroe } from '../../domain/types';
import { CARAVANA_COOLDOWN, FUNDACION, MERCENARIOS, MOVIMIENTO } from '../../constants';
import {
  avanzarCaravanasDeFundacion,
  costoCaravanaFundacion,
  desarmarCaravanaFundacion,
  devolverAportes,
  ExpansionInvalidaError,
  fundarConCaravana,
  lanzarCaravanaFundacion,
} from '../expansion';
import { FundacionInvalidaError } from '../settlement';
import { comprarCaravanaDeRefundacion, costoRefundacion } from '../refundacion';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, heroeDePrueba, instanteDeTest, posicionRecomendable } from './fixtures';

const mapa = crearMapaDeterminista(1);

/** Una Facción de nivel 3 (cap 3) con su primera plaza en nivel 2 y el almacén lleno, y su primer ciudadano como titular. */
function contextoDePlaza() {
  const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 3 } : f));
  const { asentamiento, facciones: trasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const plaza: Asentamiento = {
    ...asentamiento,
    nivel: 2,
    nivelActual: 2,
    almacen: Object.fromEntries(Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: 5000, capacidad: 5000 }])),
  };
  const faccion = trasFundar.find((f) => f.id === 'faccion-1')!;
  return { plaza, faccion, titularId: faccion.ciudadanosIds[0]! };
}

const heroe = (id: string, extra: Partial<Heroe> = {}): Heroe => heroeDePrueba(id, { tipo: 'asentamiento', asentamientoId: 'x' }, extra);

describe('lanzar la Caravana de Fundación desde una plaza', () => {
  it('nace sin destino, parada en la plaza, con su titular, su Facción y su caducidad; cobra el coste y arranca el cooldown', () => {
    const { plaza, faccion, titularId } = contextoDePlaza();
    const r = lanzarCaravanaFundacion(plaza, faccion, titularId, [plaza], [], instanteDeTest(0), 0);

    expect(r.caravana).toMatchObject({
      tipo: 'construccion',
      origenAsentamientoId: plaza.id,
      faccionId: faccion.id,
      estado: 'disponible',
      titularId,
      posicionActual: plaza.posicion,
      caducaEn: instanteDeTest(FUNDACION.caducidadCaravanaHoras * 60),
    });
    expect(r.caravana.origenCampamentoId).toBeUndefined();
    expect(r.caravana.contenido).toEqual(costoCaravanaFundacion());
    for (const [recurso, n] of Object.entries(costoCaravanaFundacion())) {
      expect(r.origenActualizado.almacen[recurso]!.cantidad).toBe(5000 - (n ?? 0));
    }
    expect(r.origenActualizado.ultimaCaravanaCreadaEn).toBe(instanteDeTest(0));
  });

  it('rechaza: no ciudadano, nivel < 2, sin recursos, cooldown y cupo del Cap (que cuenta las caravanas ya lanzadas)', () => {
    const { plaza, faccion, titularId } = contextoDePlaza();
    const lanza = (p = plaza, f = faccion, t = titularId, caravanas: Caravana[] = [], tick = 0) => lanzarCaravanaFundacion(p, f, t, [plaza], caravanas, instanteDeTest(tick), 0);

    expect(() => lanza(plaza, faccion, 'forastero')).toThrow(ExpansionInvalidaError);
    expect(() => lanza({ ...plaza, nivel: 1, nivelActual: 1 })).toThrow(ExpansionInvalidaError);
    expect(() => lanza({ ...plaza, almacen: { ...plaza.almacen, madera: { cantidad: 0, capacidad: 5000 } } })).toThrow(ExpansionInvalidaError);

    const primera = lanza();
    expect(() => lanza(primera.origenActualizado, faccion, titularId, [primera.caravana], 1), 'cooldown').toThrow(ExpansionInvalidaError);
    const segunda = lanza(primera.origenActualizado, faccion, titularId, [primera.caravana], CARAVANA_COOLDOWN.cooldownMinutos);
    // Nivel 3 de Facción = cap 3: la plaza + dos caravanas lo llenan, y una tercera ya no cabe.
    expect(segunda.caravana.id).toBeDefined();
    expect(() => lanza(segunda.origenActualizado, faccion, titularId, [primera.caravana, segunda.caravana], 2 * CARAVANA_COOLDOWN.cooldownMinutos)).toThrow(ExpansionInvalidaError);
  });
});

describe('desarmar a mano', () => {
  const faccion = crearFacciones()[0]!;
  const plaza = (): Asentamiento => contextoDePlaza().plaza;
  const suelta = (p: Asentamiento, extra: Partial<Caravana> = {}): Caravana => ({
    ...lanzarCaravanaFundacion(p, { ...faccion, ciudadanosIds: ['h1'], nivel: 3 }, 'h1', [p], [], instanteDeTest(0), 0).caravana,
    ...extra,
  });

  it('la de una plaza, suelta en su puerta, devuelve el coste íntegro a su almacén', () => {
    const p = plaza();
    const c = suelta(p);
    const sinCoste = { ...p, almacen: { ...p.almacen, madera: { cantidad: 0, capacidad: 5000 } } };
    const r = desarmarCaravanaFundacion(c, p.posicion, { asentamientos: [sinCoste], heroes: [] }, [faccion]);
    expect(r.asentamientos[0]!.almacen['madera']!.cantidad).toBe(costoCaravanaFundacion()['madera']);
  });

  it('la de un campamento devuelve a cada aportante lo suyo', () => {
    const f = { ...faccion, id: 'f', ciudadanosIds: ['h1'], nivel: 1 };
    const campamento = { id: 'merc-1', posicion: { x: 400, y: 400 }, residentesIds: ['h1'], fondos: { h1: costoRefundacion() } } as unknown as CampamentoMercenarios;
    const c = comprarCaravanaDeRefundacion(campamento, f, [], [], 'h1', instanteDeTest(0)).caravana;
    const r = desarmarCaravanaFundacion(c, campamento.posicion, { asentamientos: [], heroes: [heroe('h1')] }, [f]);
    expect(r.heroes[0]!.oroDeBotin).toBe(costoRefundacion()['oro']);
    expect(r.heroes[0]!.almacenPersonal?.['madera']).toBe(costoRefundacion()['madera']);
  });

  it('solo suelta y en la puerta de su origen: ni enganchada, ni lejos (sería teletransportar el coste)', () => {
    const p = plaza();
    const mundo = { asentamientos: [p], heroes: [] };
    expect(() => desarmarCaravanaFundacion(suelta(p, { estado: 'adjunta' }), p.posicion, mundo, [faccion])).toThrow(ExpansionInvalidaError);
    const lejos = { x: p.posicion.x + MOVIMIENTO.radioPuerta + 1, y: p.posicion.y };
    expect(() => desarmarCaravanaFundacion(suelta(p, { posicionActual: lejos }), p.posicion, mundo, [faccion])).toThrow(ExpansionInvalidaError);
    expect(() => desarmarCaravanaFundacion(suelta(p), undefined, mundo, [faccion]), 'sin origen').toThrow(ExpansionInvalidaError);
  });
});

describe('la caravana con el tiempo (D13, D14, D40, D43, D68), de plaza y de campamento', () => {
  const f: Faccion = { ...crearFacciones()[0]!, id: 'f', ciudadanosIds: ['h1'], nivel: 3 };
  const campamentos = [{ id: 'merc-1', posicion: { x: 400, y: 400 }, residentesIds: ['h1'], fondos: {} } as unknown as CampamentoMercenarios];
  const costo = costoRefundacion();

  /** Las dos formas de nacer, con el mundo en el que vive su origen. */
  const origenes = [
    {
      nombre: 'plaza',
      plaza: { id: 'plaza-1', faccionId: 'f', posicion: { x: 400, y: 400 }, almacen: { madera: { cantidad: 0, capacidad: 5000 } } } as unknown as Asentamiento,
      caravana: (): Caravana => ({
        id: 'c1',
        tipo: 'construccion',
        origenAsentamientoId: 'plaza-1',
        faccionId: 'f',
        contenido: { madera: 70 },
        posicionActual: { x: 400, y: 400 },
        progreso: 0,
        estado: 'disponible',
        titularId: 'h1',
        caducaEn: instanteDeTest(FUNDACION.caducidadCaravanaHoras * 60),
      }),
      devuelto: (r: { asentamientos: Asentamiento[]; heroes: Heroe[] }) => r.asentamientos[0]!.almacen['madera']!.cantidad,
      esperado: 70,
    },
    {
      nombre: 'campamento',
      plaza: undefined,
      caravana: (): Caravana =>
        comprarCaravanaDeRefundacion({ ...campamentos[0]!, fondos: { h1: { ...costo } } }, f, [], [], 'h1', instanteDeTest(0)).caravana,
      devuelto: (r: { asentamientos: Asentamiento[]; heroes: Heroe[] }) => r.heroes[0]!.oroDeBotin ?? 0,
      esperado: costo['oro']!,
    },
  ];

  for (const o of origenes) {
    describe(o.nombre, () => {
      const mundo = () => ({ asentamientos: o.plaza ? [o.plaza] : [], heroes: [heroe('h1')] });
      const tick = (cs: Caravana[], ejercitos: Ejercito[], m: { asentamientos: Asentamiento[]; heroes: Heroe[] }, t: number) =>
        avanzarCaravanasDeFundacion(cs, ejercitos, m, [f], campamentos, mapa, instanteDeTest(t));

      it('suelta y sin nadie, caduca y devuelve', () => {
        const c = o.caravana();
        expect(tick([c], [], mundo(), FUNDACION.caducidadCaravanaHoras * 60 - 1).caravanas).toHaveLength(1);
        const despues = tick([c], [], mundo(), FUNDACION.caducidadCaravanaHoras * 60);
        expect(despues.caravanas).toEqual([]);
        expect(o.devuelto(despues)).toBe(o.esperado);
      });

      it('enganchada sin su titular, se suelta y vuelve sola; al llegar se desarma y devuelve', () => {
        const lejos = { x: 600, y: 400 };
        const c = { ...o.caravana(), estado: 'adjunta' as const, posicionActual: lejos, caducaEn: undefined };
        const ajena = { id: 'col', participantes: [{ heroeId: 'otro', unidoEn: 0 }], caravanasAdjuntasIds: [c.id], posicionActual: lejos } as unknown as Ejercito;
        const r = tick([c], [ajena], mundo(), 1);
        expect(r.caravanas[0]).toMatchObject({ estado: 'retornando', progreso: 0 });
        expect(r.ejercitos[0]!.caravanasAdjuntasIds).toEqual([]);

        let cs = r.caravanas;
        let m = { asentamientos: r.asentamientos, heroes: r.heroes };
        for (let t = 2; t < 500 && cs.length > 0; t++) {
          const paso = tick(cs, [], m, t);
          cs = paso.caravanas;
          m = { asentamientos: paso.asentamientos, heroes: paso.heroes };
        }
        expect(cs, 'llegó y se desarmó').toEqual([]);
        expect(o.devuelto(m)).toBe(o.esperado);
      });

      it('enganchada por su titular, sigue; aparcada al guarnecer, cuenta como suelta y caduca desde entonces', () => {
        const c = { ...o.caravana(), estado: 'adjunta' as const, caducaEn: undefined };
        const suya = { id: 'col', participantes: [{ heroeId: 'h1', unidoEn: 0 }], caravanasAdjuntasIds: [c.id], posicionActual: c.posicionActual } as unknown as Ejercito;
        expect(tick([c], [suya], mundo(), 1).caravanas).toEqual([c]);

        const aparcada = { ...c, estado: 'aparcada' as const };
        const suelta = tick([aparcada], [], mundo(), 10).caravanas[0]!;
        expect(suelta).toMatchObject({ estado: 'disponible', caducaEn: instanteDeTest(10 + FUNDACION.caducidadCaravanaHoras * 60) });
      });
    });
  }

  it('devolver a los aportantes: lo suyo, el oro como botín; quien ya no es ciudadano lo pierde', () => {
    const c = { ...origenes[1]!.caravana(), aportes: { h1: { madera: 10, oro: 5 }, ajeno: { madera: 7 } } };
    const [h1, ajeno] = devolverAportes(c, [heroe('h1'), heroe('ajeno')], [f]);
    expect(h1!.almacenPersonal).toEqual({ madera: 10 });
    expect(h1!.oroDeBotin).toBe(5);
    expect(ajeno!.almacenPersonal ?? {}).toEqual({});
  });

  it('si su plaza de origen ya no existe, al caducar se pierde en vez de devolver', () => {
    const c = origenes[0]!.caravana();
    const r = avanzarCaravanasDeFundacion([c], [], { asentamientos: [], heroes: [] }, [f], [], mapa, instanteDeTest(FUNDACION.caducidadCaravanaHoras * 60));
    expect(r.caravanas).toEqual([]);
    expect(r.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['fundacion.caravana_perdida']);
  });

  it('sin origen en pie, la que vuelve sola se pierde', () => {
    const c = { ...origenes[0]!.caravana(), estado: 'adjunta' as const, caducaEn: undefined };
    const sinTitular = { id: 'col', participantes: [], caravanasAdjuntasIds: [c.id], posicionActual: c.posicionActual } as unknown as Ejercito;
    const r = avanzarCaravanasDeFundacion([c], [sinTitular], { asentamientos: [], heroes: [] }, [f], [], mapa, instanteDeTest(1));
    expect(r.caravanas).toEqual([]);
    expect(r.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['fundacion.caravana_perdida']);
  });
});

describe('fundar con la caravana: dónde sí y dónde no', () => {
  const faccion = crearFacciones()[0]!;
  const ciudadano = ['h1'];
  const campamento = (posicion: { x: number; y: number }): CampamentoMercenarios => ({ id: 'merc-1', posicion, residentesIds: [], fondos: {} }) as unknown as CampamentoMercenarios;
  const f = { ...faccion, id: 'f', ciudadanosIds: ciudadano, nivel: 1 };
  const sitio = posicionRecomendable(mapa, []);
  const funda = (cerca: number | undefined) =>
    fundarConCaravana(mapa, [f], 'f', sitio, ['h1'], [], cerca === undefined ? [] : [campamento({ x: sitio.x + cerca, y: sitio.y })], instanteDeTest(0));

  it('a menos de `radioExclusionFundar` de un campamento se rechaza, y a esa distancia o más se funda (D16)', () => {
    expect(() => funda(0), 'encima del campamento').toThrow(FundacionInvalidaError);
    expect(() => funda(MERCENARIOS.radioExclusionFundar - 1)).toThrow(FundacionInvalidaError);
    expect(funda(MERCENARIOS.radioExclusionFundar).asentamiento.faccionId).toBe('f');
    expect(funda(undefined).asentamiento.posicion).toEqual(sitio);
  });

  it('cuenta cualquier campamento, no solo el de origen de la caravana', () => {
    const lejano = campamento({ x: 5, y: 5 });
    const cercano = { ...campamento({ x: sitio.x + 10, y: sitio.y }), id: 'merc-2' };
    expect(() => fundarConCaravana(mapa, [f], 'f', sitio, ['h1'], [], [lejano, cercano], instanteDeTest(0))).toThrow(FundacionInvalidaError);
  });
});
