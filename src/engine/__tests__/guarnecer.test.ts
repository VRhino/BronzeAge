// `guarnecer` (Ocupacion_Post_Conquista_Definicion.md §2.3): un ejército marcha a una plaza de su Facción y
// entra y se desarma: cada héroe entra como lo haría solo (2026-10-08).
// Las caravanas adjuntas quedan 'aparcadas' (§2.3d): no las usa la anfitriona, intercambian con su almacén, y
// salen solo enganchadas a un ejército o enviadas a su origen.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Caravana, Ejercito, Escuadron, Point } from '../../domain/types';
import { caravanasAlEntrar, exigirPuedeGuarnecer, MovilizacionInvalidaError, adjuntarCaravana } from '../ejercitos';
import { moverCargaCarroAparcada, enviarCaravanaAlOrigen, CaravanaInvalidaError, avanzarComercio } from '../trade';
import { instante } from '../../domain/tiempo';
import type { EjercitoConTropa } from '../tropa';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';
import { RED_VACIA } from '../redCaminos';

const mapa = crearMapaDeterminista(42);

function plaza(faccionId: string, existentes: Asentamiento[] = []): Asentamiento {
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), faccionId, existentes, 0, posicionRecomendable(mapa, existentes));
  return { ...asentamiento, poblacion: { pesants: 300, artesanos: 0, nobleza: 0 } };
}

const esc = (heroeId: string): Escuadron =>
  escuadronDePrueba(`esc-${heroeId}`, heroeId, 'milicia_lanceros', 20, { contenedor: { tipo: 'ejercito', ejercitoId: 'ej-1' } });

function ejercitoDe(
  faccionId: string,
  posicion: Point,
  escuadrones: Escuadron[],
  extra: Partial<Ejercito> = {}
): EjercitoConTropa {
  return {
    id: 'ej-1',
    faccionId,
    origenAsentamientoId: 'origen',
    participantes: [...new Set(escuadrones.map((e) => e.heroeId))].map((heroeId) => ({ heroeId, unidoEn: instante(0) })),
    tipo: 'ejercito',
    politicaDeUnion: 'rechazar',
    liderId: escuadrones[0]?.heroeId ?? 'j1',
    escuadronIds: escuadrones.map((e) => e.id),
    escuadrones,
    suministro: { trigo: 300 },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: posicion },
    ruta: [],
    progreso: 1,
    posicionActual: posicion,
    estado: 'estacionado',
    ...extra,
  };
}

function caravanaAdjunta(id: string, origenId: string, posicion: Point): Caravana {
  return {
    id,
    tipo: 'comercial',
    origenAsentamientoId: origenId,
    contenido: {},
    posicionActual: posicion,
    progreso: 0,
    estado: 'adjunta',
    carros: [{ tipoCarro: 'basico', animal: 'buey' }],
  };
}

describe('guarnecer — qué se comprueba del ejército (cada héroe entra después como lo haría solo)', () => {
  const b: Asentamiento = { ...plaza('faccion-1'), casasCompradas: ['j1', 'j2'] };

  it('un ejército de su Facción en la puerta puede guarnecer, residan o no los que van dentro', () => {
    expect(() => exigirPuedeGuarnecer(b, ejercitoDe('faccion-1', b.posicion, [esc('j1'), esc('forastero')]))).not.toThrow();
  });

  it('una columna PERSONAL no guarnece', () => {
    expect(() => exigirPuedeGuarnecer(b, ejercitoDe('faccion-1', b.posicion, [], { tipo: 'personal' }))).toThrow(MovilizacionInvalidaError);
  });

  it('no se guarnece una plaza de OTRA Facción', () => {
    expect(() => exigirPuedeGuarnecer(b, ejercitoDe('faccion-2', b.posicion, [esc('j1')]))).toThrow(MovilizacionInvalidaError);
  });

  it('hay que estar EN LA PUERTA', () => {
    expect(() => exigirPuedeGuarnecer(b, ejercitoDe('faccion-1', { x: b.posicion.x + 5000, y: b.posicion.y }, [esc('j1')]))).toThrow(MovilizacionInvalidaError);
  });
});

describe('las caravanas adjuntas al entrar en un lugar (2026-10-08)', () => {
  const b = plaza('faccion-1');
  const lugar = { tipo: 'asentamiento' as const, id: b.id, posicion: b.posicion };

  it('las de ese lugar quedan APARCADAS en una plaza, no se pierden', () => {
    const car = caravanaAdjunta('car-1', b.id, b.posicion);
    const ej = ejercitoDe('faccion-1', b.posicion, [esc('j1')], { caravanasAdjuntasIds: ['car-1'] });
    const r = caravanasAlEntrar(ej, [car], lugar);
    expect(r).toHaveLength(1);
    expect(r[0]!.estado).toBe('aparcada');
    expect(r[0]!.posicionActual).toEqual(b.posicion);
  });

  it('en un campamento quedan sueltas, como al soltarlas', () => {
    const car = { ...caravanaAdjunta('car-1', '', b.posicion), origenCampamentoId: 'mercenarios-0' };
    const ej = ejercitoDe('faccion-1', b.posicion, [esc('j1')], { caravanasAdjuntasIds: ['car-1'] });
    expect(caravanasAlEntrar(ej, [car], { tipo: 'campamento', id: 'mercenarios-0', posicion: b.posicion })[0]!.estado).toBe('disponible');
  });

  it('si alguna es de OTRO lugar, no se entra', () => {
    const ej = ejercitoDe('faccion-1', b.posicion, [esc('j1')], { caravanasAdjuntasIds: ['car-1'] });
    expect(() => caravanasAlEntrar(ej, [caravanaAdjunta('car-1', 'otra-plaza', b.posicion)], lugar)).toThrow(/no es de este lugar/);
    expect(() => caravanasAlEntrar(ej, [caravanaAdjunta('car-1', b.id, b.posicion)], { tipo: 'campamento', id: 'mercenarios-0', posicion: b.posicion })).toThrow(/no es de este lugar/);
  });
});

describe('la caravana aparcada — intercambio con el almacén y las dos salidas', () => {
  const base = plaza('faccion-1');
  const anfitriona: Asentamiento = {
    ...base,
    almacen: { trigo: { cantidad: 400, capacidad: 1000 }, piedra: { cantidad: 0, capacidad: 500 } },
  };
  // El origen es otra plaza de la misma Facción — se construye a mano (el cap de fundación del fixture es 1).
  const origen: Asentamiento = {
    ...base,
    id: 'origen',
    posicion: posicionRecomendable(mapa, [anfitriona]),
    almacen: { trigo: { cantidad: 0, capacidad: 1000 } },
  };
  const aparcada = (): Caravana => ({
    id: 'car-x',
    tipo: 'comercial',
    origenAsentamientoId: 'origen',
    contenido: {},
    posicionActual: anfitriona.posicion,
    progreso: 0,
    estado: 'aparcada',
    carros: [{ tipoCarro: 'basico', animal: 'buey' }], // capacidad 500
  });

  it('CARGAR mueve del almacén al carro, topado por lo que hay y por la capacidad', () => {
    const r = moverCargaCarroAparcada(aparcada(), anfitriona, 'trigo', 999, 'cargar');
    // pedidos 999, capacidad 500, en el almacén 400 → se mueven 400
    expect(r.caravana.contenido['trigo']).toBe(400);
    expect(r.plaza.almacen['trigo']?.cantidad).toBe(0);
  });

  it('DESCARGAR devuelve del carro al almacén sin perder lo que no cabe', () => {
    const cargada: Caravana = { ...aparcada(), contenido: { trigo: 300 } };
    const r = moverCargaCarroAparcada(cargada, anfitriona, 'trigo', 300, 'descargar');
    expect(r.caravana.contenido['trigo']).toBeUndefined();
    expect(r.plaza.almacen['trigo']?.cantidad).toBe(700);
  });

  it('un recurso que el almacén nunca ha guardado no admite descarga (sin entrada = sin capacidad)', () => {
    const cargada: Caravana = { ...aparcada(), contenido: { madera: 100 } };
    expect(() => moverCargaCarroAparcada(cargada, anfitriona, 'madera', 100, 'descargar')).toThrow(CaravanaInvalidaError);
  });

  it('ENVIAR AL ORIGEN vacía: aparece al instante en el origen', () => {
    const r = enviarCaravanaAlOrigen(aparcada(), anfitriona, origen, mapa);
    expect(r.caravana.estado).toBe('disponible');
    expect(r.caravana.posicionActual).toEqual(origen.posicion);
  });

  it('ENVIAR AL ORIGEN cargada: pasa a retornando y recorre el mapa', () => {
    const cargada: Caravana = { ...aparcada(), contenido: { trigo: 200 } };
    const r = enviarCaravanaAlOrigen(cargada, anfitriona, origen, mapa);
    expect(r.caravana.estado).toBe('retornando');
    expect(r.caravana.destinoAsentamientoId).toBe(anfitriona.id);
    expect(r.caravana.ruta?.length).toBeGreaterThan(1);
  });

  it('una caravana aparcada puede engancharse a CUALQUIER ejército de la Facción', () => {
    const ej = ejercitoDe('faccion-1', anfitriona.posicion, [esc('j9')]);
    const r = adjuntarCaravana(ej, aparcada(), origen.faccionId);
    expect(r.caravana.estado).toBe('adjunta');
    expect(r.ejercito.caravanasAdjuntasIds).toContain('car-x');
  });

  it('el reparto automático de comercio NO toca una caravana aparcada', () => {
    // Un tick de comercio con una aparcada en el estado: sigue igual, no la despacha.
    const antes = aparcada();
    const r = avanzarComercio([anfitriona, origen], [], [antes], [], mapa, RED_VACIA, [], instante(0));
    const despues = r.caravanas.find((c) => c.id === 'car-x')!;
    expect(despues.estado).toBe('aparcada');
    expect(despues.posicionActual).toEqual(anfitriona.posicion);
  });
});
