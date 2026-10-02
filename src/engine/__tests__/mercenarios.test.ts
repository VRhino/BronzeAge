// Aparición de campamentos de mercenarios (Doc 1.9b): el del día 1 en el centro, y los demás donde se juntan las zonas de
// varias Facciones. Mapa sintético: lo que se mide es la regla de dónde y cuándo, no el terreno real.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, CampamentoMercenarios, Point, ZonaBosque, ZonaInfluencia } from '../../domain/types';
import { MERCENARIOS } from '../../constants';
import type { Mapa } from '../../world/mapa';
import { avanzarAparicionMercenarios } from '../mercenarios';
import { instanteDeTest } from './fixtures';

function mapaDe(opciones: { agua?: (p: Point) => boolean; bosques?: ZonaBosque[] } = {}): Mapa {
  const limites = { ancho: 2000, alto: 2000 };
  return {
    limites,
    dentroDelMapa: (p: Point) => p.x >= 0 && p.y >= 0 && p.x <= limites.ancho && p.y <= limites.alto,
    esTransitable: () => true,
    terrenoEn: (p: Point) => (opciones.agua?.(p) ? 'agua' : 'llano'),
    listarBosques: () => opciones.bosques ?? [],
  } as unknown as Mapa;
}

/** Plaza mínima: solo lo que lee la aparición. */
const plaza = (id: string, faccionId: string, x: number, y: number, radioPotencial = 60): Asentamiento =>
  ({ id, faccionId, posicion: { x, y }, radioPotencial }) as unknown as Asentamiento;

/** Zona circular (octógono) de una plaza: lo que `computeTodasLasZonas` daría sin vecinos. */
const zonaDe = (a: Asentamiento): ZonaInfluencia => ({
  asentamientoId: a.id,
  poligono: Array.from({ length: 16 }, (_, i) => ({
    x: a.posicion.x + a.radioPotencial * Math.cos((2 * Math.PI * i) / 16),
    y: a.posicion.y + a.radioPotencial * Math.sin((2 * Math.PI * i) / 16),
  })),
});

const T0 = instanteDeTest(0);
const sinZonas: ZonaInfluencia[] = [];
const distancia = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

describe('el campamento del día 1', () => {
  it('nace en el centro del mapa cuando no hay ninguno, con el layout fijo y un edificio militar', () => {
    const r = avanzarAparicionMercenarios([], [], sinZonas, mapaDe(), T0);

    expect(r.campamentos).toHaveLength(1);
    const c = r.campamentos[0]!;
    expect(distancia(c.posicion, { x: 1000, y: 1000 })).toBe(0);
    expect(c.origen).toBeGreaterThanOrEqual(0);
    expect(c.origen).toBeLessThan(MERCENARIOS.origenes);
    expect(c.edificios.slice(0, MERCENARIOS.edificiosFijos.length)).toEqual([...MERCENARIOS.edificiosFijos]);
    expect(MERCENARIOS.edificiosMilitares).toContain(c.edificios.at(-1));
    expect(r.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['mercenarios.campamento_aparece']);
  });

  it('se pega al borde de un bosque cercano, y es determinista', () => {
    const bosque: ZonaBosque = { id: 'b', centro: { x: 1000, y: 1060 }, radio: 40, densidad: 1 };
    const a = avanzarAparicionMercenarios([], [], sinZonas, mapaDe({ bosques: [bosque] }), T0);
    const b = avanzarAparicionMercenarios([], [], sinZonas, mapaDe({ bosques: [bosque] }), T0);

    expect(a.campamentos).toEqual(b.campamentos);
    // El centro (1000,1000) queda a 60 del centro del bosque: a 20 de su borde, dentro del margen.
    expect(distancia(a.campamentos[0]!.posicion, bosque.centro)).toBeCloseTo(bosque.radio + MERCENARIOS.pegadoAlBorde);
  });

  it('busca tierra firme si el centro es agua', () => {
    const lago = (p: Point) => distancia(p, { x: 1000, y: 1000 }) < 100;
    const c = avanzarAparicionMercenarios([], [], sinZonas, mapaDe({ agua: lago }), T0).campamentos[0]!;
    expect(lago(c.posicion)).toBe(false);
    expect(distancia(c.posicion, { x: 1000, y: 1000 })).toBeLessThanOrEqual(MERCENARIOS.radioBusqueda);
  });
});

describe('los demás campamentos', () => {
  const inicial = (): CampamentoMercenarios[] => avanzarAparicionMercenarios([], [], sinZonas, mapaDe(), T0).campamentos;
  // A la hora en punto: la búsqueda solo se hace cada `cadaMinutos`.
  const enPunto = instanteDeTest(MERCENARIOS.cadaMinutos * 5);

  /** Dos plazas de Facciones distintas con un hueco sin reclamar entre ellas (a `hueco` de cada borde). */
  function frontera(hueco: number, x0 = 300, y = 300) {
    const radio = 60;
    const a = plaza('a', 'f1', x0, y, radio);
    const b = plaza('b', 'f2', x0 + 2 * (radio + hueco), y, radio);
    return { asentamientos: [a, b], zonas: [zonaDe(a), zonaDe(b)] };
  }

  it('aparece en el hueco entre dos Facciones cuyas zonas están a menos de R', () => {
    const { asentamientos, zonas } = frontera(100);
    const r = avanzarAparicionMercenarios(inicial(), asentamientos, zonas, mapaDe(), enPunto);

    expect(r.campamentos).toHaveLength(2);
    expect(r.campamentos[1]!.id).toBe('mercenarios-1');
    expect(r.campamentos[1]!.posicion).toEqual({ x: 300 + 160, y: 300 });
  });

  it('no aparece si las zonas están más lejos que R, ni con una sola Facción, ni si el punto está reclamado', () => {
    const lejos = frontera(MERCENARIOS.radioZonas + 50);
    expect(avanzarAparicionMercenarios(inicial(), lejos.asentamientos, lejos.zonas, mapaDe(), enPunto).campamentos).toHaveLength(1);

    const { asentamientos, zonas } = frontera(100);
    const mismaFaccion = asentamientos.map((a) => ({ ...a, faccionId: 'f1' }) as Asentamiento);
    expect(avanzarAparicionMercenarios(inicial(), mismaFaccion, zonas, mapaDe(), enPunto).campamentos).toHaveLength(1);

    // Zonas tan grandes que cubren el punto medio: está reclamado.
    const grandes = asentamientos.map((a) => ({ ...a, radioPotencial: 200 }) as Asentamiento);
    expect(avanzarAparicionMercenarios(inicial(), grandes, grandes.map(zonaDe), mapaDe(), enPunto).campamentos).toHaveLength(1);
  });

  it('respeta la distancia mínima entre campamentos y el tope por servidor', () => {
    // El hueco cae a menos de la distancia mínima del campamento del centro.
    const cerca = frontera(100, 940, 940);
    expect(avanzarAparicionMercenarios(inicial(), cerca.asentamientos, cerca.zonas, mapaDe(), enPunto).campamentos).toHaveLength(1);

    const llenos = Array.from({ length: MERCENARIOS.topePorServidor }, (_, i) => ({ ...inicial()[0]!, id: `mercenarios-${i}`, posicion: { x: 1500 + i * 5, y: 1500 } }));
    const { asentamientos, zonas } = frontera(100);
    expect(avanzarAparicionMercenarios(llenos, asentamientos, zonas, mapaDe(), enPunto).campamentos).toHaveLength(MERCENARIOS.topePorServidor);
  });

  it('solo busca cada `cadaMinutos`', () => {
    const { asentamientos, zonas } = frontera(100);
    const fuera = instanteDeTest(MERCENARIOS.cadaMinutos * 5 + 1);
    expect(avanzarAparicionMercenarios(inicial(), asentamientos, zonas, mapaDe(), fuera).campamentos).toHaveLength(1);
  });
});
