// Decisiones del usuario sobre el comercio (2026-10-11, `Docs/Mechanics/Comercio_Caravanas.md`): la comisión solo se acuña entre
// asentamientos distintos, la mala reputación se paga de más del propio almacén, la reputación es entre Facciones, el reparto
// automático no se atasca y «Carga Ampliada» vale para toda caravana propia.
import { describe, expect, it } from 'vitest';
import type { AcuerdoTrueque, Asentamiento, Caravana, Faccion, Point } from '../../domain/types';
import { REPUTACION } from '../../constants';
import { capacidadEfectiva } from '../caravanas';
import { RED_VACIA } from '../redCaminos';
import { aceptarTrueque, aplicarEntregaATrueque, avanzarComercio, construirCaravanaComercial, moverCargaCarroAparcada, prepararCaravanaManual, proponerTrueque } from '../trade';
import { instanteDeTest } from './fixtures';
import { almacenSintetico, caravanaComercialCasiLlegando, mapaSintetico } from './tradeFixtures';

function plaza(id: string, faccionId: string, posicion: Point, recursos: Record<string, number>, politicas: string[] = []): Asentamiento {
  return {
    id,
    faccionId,
    posicion,
    almacen: almacenSintetico(recursos),
    politicasActivas: politicas.map((politicaId) => ({ id: `p-${politicaId}`, politicaId, cargo: 'tesorero', activadaEn: 0, expiraEn: 1e12 })),
    edificios: [{ id: `m-${id}`, tipo: 'mercado', posicion, estado: 'activo', nivelInterno: 1 }],
  } as unknown as Asentamiento;
}
const faccion = (id: string, reputacion = 0): Faccion => ({ id, nombre: id, reputacion }) as unknown as Faccion;

describe('la comisión solo se acuña entre asentamientos distintos', () => {
  it('una caravana manual al propio origen vuelve sin acuñar nada', () => {
    const o = plaza('o', 'f-1', { x: 100, y: 100 }, { madera: 1000, oro: 0 });
    const c: Caravana = { id: 'c', tipo: 'comercial', origenAsentamientoId: 'o', contenido: {}, posicionActual: o.posicion, progreso: 0, estado: 'disponible', carros: [{ tipoCarro: 'basico', animal: 'buey' }] };
    const r = prepararCaravanaManual(c, o, o, { madera: 400 }, [], [], mapaSintetico(), { red: RED_VACIA, asentamientos: [o], zonas: [] }, instanteDeTest(0));
    let asentamientos = [r.asentamiento];
    let caravanas = [r.caravana];
    for (let t = 1; t <= 4; t++) {
      const x = avanzarComercio(asentamientos, [faccion('f-1')], caravanas, [], mapaSintetico(), RED_VACIA, [], instanteDeTest(t));
      asentamientos = x.asentamientos;
      caravanas = x.caravanas;
    }
    expect(asentamientos[0]!.almacen['madera']!.cantidad).toBe(1000);
    expect(asentamientos[0]!.almacen['oro']!.cantidad).toBe(0);
    expect(caravanas[0]!.estado).toBe('disponible');
  });
});

describe('la mala reputación se paga de más del propio almacén', () => {
  const llegar = (reputacion: number, faccionDestino: string) => {
    const o = plaza('o', 'f-1', { x: 0, y: 0 }, { oro: 100 });
    const d = plaza('d', faccionDestino, { x: 1000, y: 0 }, { oro: 0 });
    const c = caravanaComercialCasiLlegando(o, d, { contenido: { madera: 400 }, origenAsentamientoId: 'o' });
    const facciones = [faccion('f-1', reputacion), ...(faccionDestino === 'f-1' ? [] : [faccion(faccionDestino)])];
    const r = avanzarComercio([o, d], facciones, [c], [], mapaSintetico(), RED_VACIA, [], instanteDeTest(1));
    const oro = (id: string) => r.asentamientos.find((a) => a.id === id)!.almacen['oro']!.cantidad;
    return { destino: oro('d'), origen: oro('o') };
  };

  it('con reputación baja, el destino cobra la comisión acuñada más el recargo, que sale del almacén de origen', () => {
    const normal = llegar(0, 'f-2');
    const mala = llegar(REPUTACION.umbralBajoParaComision, 'f-2');
    const extra = REPUTACION.factorComisionPorReputacionBaja - 1;
    expect(normal.origen).toBe(100);
    expect(mala.origen).toBeCloseTo(100 - normal.destino * extra, 6);
    expect(mala.destino).toBeCloseTo(normal.destino * (1 + extra), 6);
    expect(normal.destino).toBeGreaterThan(0);
  });

  it('paga lo que tenga, sin dejar el almacén en negativo', () => {
    const o = plaza('o', 'f-1', { x: 0, y: 0 }, { oro: 1 });
    const d = plaza('d', 'f-2', { x: 1000, y: 0 }, { oro: 0 });
    const c = caravanaComercialCasiLlegando(o, d, { contenido: { madera: 400 }, origenAsentamientoId: 'o' });
    const r = avanzarComercio([o, d], [faccion('f-1', -100), faccion('f-2')], [c], [], mapaSintetico(), RED_VACIA, [], instanteDeTest(1));
    expect(r.asentamientos.find((a) => a.id === 'o')!.almacen['oro']!.cantidad).toBe(0);
  });

  it('entre plazas de la misma Facción no hay recargo, por mala que sea su reputación', () => {
    const mala = llegar(-100, 'f-1');
    expect(mala.origen).toBe(100);
    // Cobra lo mismo que con buena reputación: el factor solo existe entre Facciones.
    expect(mala.destino).toBeGreaterThan(0);
    expect(mala.destino).toBeCloseTo(llegar(0, 'f-1').destino, 6);
  });
});

describe('la reputación del trueque es entre Facciones', () => {
  const acuerdo = (): AcuerdoTrueque =>
    ({
      id: 'ac', asentamientoAId: 'a', asentamientoBId: 'b', creadoEn: 0, expiraEn: 1e12, estado: 'activo',
      lineasA: [{ recurso: 'madera', cantidadTotal: 10, cantidadEntregada: 0 }],
      lineasB: [{ recurso: 'piedra', cantidadTotal: 10, cantidadEntregada: 10 }],
    }) as AcuerdoTrueque;
  const mundo = (fa: string, fb: string) => new Map([['a', plaza('a', fa, { x: 0, y: 0 }, {})], ['b', plaza('b', fb, { x: 5, y: 0 }, {})]]);

  it('cumplirlo entre Facciones distintas da reputación y experiencia a las dos; consigo misma, a nadie', () => {
    const entre = aplicarEntregaATrueque(acuerdo(), 'A', { madera: 10 }, mundo('f-1', 'f-2'));
    expect(entre.acuerdo.estado).toBe('cumplido');
    expect(entre.ajustesReputacion.map((a) => a.faccionId).sort()).toEqual(['f-1', 'f-2']);
    expect(entre.ajustesExperiencia).toHaveLength(2);

    const consigo = aplicarEntregaATrueque(acuerdo(), 'A', { madera: 10 }, mundo('f-1', 'f-1'));
    expect(consigo.acuerdo.estado).toBe('cumplido');
    expect(consigo.ajustesReputacion).toEqual([]);
    expect(consigo.ajustesExperiencia).toEqual([]);
  });

  it('vencer sin cumplir consigo misma no penaliza; entre Facciones, sí', () => {
    const vencer = (fa: string, fb: string) => {
      const a = plaza('a', fa, { x: 0, y: 0 }, {});
      const b = plaza('b', fb, { x: 5, y: 0 }, {});
      const facciones = fa === fb ? [faccion(fa)] : [faccion(fa), faccion(fb)];
      const vencido = { ...acuerdo(), expiraEn: instanteDeTest(1), lineasB: [{ recurso: 'piedra', cantidadTotal: 10, cantidadEntregada: 0 }] };
      return avanzarComercio([a, b], facciones, [], [vencido], mapaSintetico(), RED_VACIA, [], instanteDeTest(2)).facciones;
    };
    expect(vencer('f-1', 'f-1').map((f) => f.reputacion)).toEqual([0]);
    expect(vencer('f-1', 'f-2').map((f) => f.reputacion)).toEqual([REPUTACION.penalizacionTruequeIncumplido, REPUTACION.penalizacionTruequeIncumplido]);
  });
});

describe('el reparto automático no se atasca', () => {
  it('un casco sin carros primero en la lista no impide que salga la caravana equipada', () => {
    const a = plaza('A', 'f-1', { x: 0, y: 0 }, { madera: 500, oro: 200 });
    const b = plaza('B', 'f-2', { x: 500, y: 0 }, { oro: 100 });
    const { caravana: buena } = construirCaravanaComercial(a, [], instanteDeTest(0), 0);
    const casco: Caravana = { id: 'casco', tipo: 'comercial', origenAsentamientoId: 'A', contenido: {}, posicionActual: a.posicion, progreso: 0, estado: 'disponible', carros: [], reservadaManual: false };
    const ac = aceptarTrueque(proponerTrueque([a, b], 'A', 'B', [{ recurso: 'madera', cantidad: 50 }], [{ recurso: 'oro', cantidad: 10 }], instanteDeTest(0), 0), instanteDeTest(0));
    const r = avanzarComercio([a, b], [], [casco, buena], [ac], mapaSintetico(), RED_VACIA, [], instanteDeTest(1));
    expect(r.caravanas.find((c) => c.id === buena.id)!.estado).toBe('en_transito');
    expect(r.caravanas.find((c) => c.id === 'casco')!.estado).toBe('disponible');
  });

  it('un trueque sin camino por tierra no gasta la caravana: sale el siguiente lado', () => {
    // Todo lo que está a la derecha de x = 400 es agua: B es inalcanzable, C no.
    const base = mapaSintetico();
    const mapa = Object.assign({}, base, { esTransitable: (p: Point) => p.x < 400 });
    (mapa as unknown as { mundo: unknown }).mundo = mapa;
    const a = plaza('A', 'f-1', { x: 0, y: 0 }, { madera: 500 });
    const b = plaza('B', 'f-2', { x: 500, y: 0 }, { oro: 100 });
    const c = plaza('C', 'f-3', { x: 100, y: 0 }, { oro: 100 });
    const { caravana } = construirCaravanaComercial({ ...a, almacen: almacenSintetico({ madera: 500, oro: 200 }) }, [], instanteDeTest(0), 0);
    // El inalcanzable es mucho más antiguo, así que puntúa más alto y se intenta primero.
    const viejo = aceptarTrueque(proponerTrueque([a, b], 'A', 'B', [{ recurso: 'madera', cantidad: 50 }], [{ recurso: 'oro', cantidad: 10 }], instanteDeTest(0), 0), instanteDeTest(0));
    const nuevo = aceptarTrueque(proponerTrueque([a, c], 'A', 'C', [{ recurso: 'madera', cantidad: 50 }], [{ recurso: 'oro', cantidad: 10 }], instanteDeTest(1000), 1), instanteDeTest(1000));
    const r = avanzarComercio([a, b, c], [], [caravana], [viejo, nuevo], mapa, RED_VACIA, [], instanteDeTest(1100));
    expect(r.caravanas[0]!.estado).toBe('en_transito');
    expect(r.caravanas[0]!.destinoAsentamientoId).toBe('C');
  });
});

describe('«Carga Ampliada» vale para toda caravana propia', () => {
  it('capacidadEfectiva aplica la política del origen, y una aparcada carga con ella', () => {
    const origen = plaza('o', 'f-1', { x: 0, y: 0 }, {}, ['carga_ampliada']);
    const anfitriona = plaza('h', 'f-1', { x: 50, y: 0 }, { madera: 5000 });
    const caravana: Caravana = { id: 'c', tipo: 'comercial', origenAsentamientoId: 'o', contenido: {}, posicionActual: anfitriona.posicion, progreso: 0, estado: 'aparcada', carros: [{ tipoCarro: 'basico', animal: 'buey' }] };
    expect(capacidadEfectiva(caravana, [])).toBe(500);
    expect(capacidadEfectiva(caravana, [origen])).toBe(750);
    expect(moverCargaCarroAparcada(caravana, anfitriona, 'madera', 9999, 'cargar', [origen, anfitriona]).caravana.contenido['madera']).toBe(750);
    expect(moverCargaCarroAparcada(caravana, anfitriona, 'madera', 9999, 'cargar').caravana.contenido['madera']).toBe(500);
  });
});
