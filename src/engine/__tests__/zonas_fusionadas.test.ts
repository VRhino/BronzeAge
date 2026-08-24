import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../zones';
import { areaFirmada } from '../../world/poligonos';

/** Asentamiento mínimo: la fusión solo mira facción, posición y radio potencial. */
function asentamiento(id: string, faccionId: string, x: number, y: number, radioPotencial: number): Asentamiento {
  return { id, faccionId, posicion: { x, y }, radioPotencial } as unknown as Asentamiento;
}

function fusionar(asentamientos: Asentamiento[]) {
  return computeZonasFusionadasPorFaccion(computeTodasLasZonas(asentamientos), asentamientos);
}

/** Área que cubre una silueta: los lazos exteriores suman y los huecos restan (orientación opuesta). */
function areaCubierta(contornos: { x: number; y: number }[][]): number {
  return Math.abs(contornos.reduce((total, lazo) => total + areaFirmada(lazo), 0));
}

describe('computeZonasFusionadasPorFaccion', () => {
  it('sin asentamientos no hay siluetas', () => {
    expect(fusionar([])).toEqual([]);
  });

  it('un asentamiento solo da una silueta con un lazo', () => {
    const fusionadas = fusionar([asentamiento('a', 'roja', 500, 500, 100)]);
    expect(fusionadas).toHaveLength(1);
    expect(fusionadas[0]!.faccionId).toBe('roja');
    expect(fusionadas[0]!.contornos).toHaveLength(1);
  });

  it('dos asentamientos HERMANOS que se solapan dan UN solo contorno', () => {
    // El caso que motivó el cambio: la misma facción no se recorta a sí misma (Doc 1.2), así que antes se
    // pintaban dos discos apilados con una frontera interna que no existe.
    const fusionadas = fusionar([
      asentamiento('a', 'roja', 500, 500, 120),
      asentamiento('b', 'roja', 600, 500, 120),
    ]);
    expect(fusionadas).toHaveLength(1);
    expect(fusionadas[0]!.contornos).toHaveLength(1);

    // Y el área es la de la unión, no la suma de los dos discos (que contaría el solape dos veces).
    const areaUnDisco = Math.PI * 120 * 120;
    const cubierta = areaCubierta(fusionadas[0]!.contornos);
    expect(cubierta).toBeGreaterThan(areaUnDisco);
    expect(cubierta).toBeLessThan(areaUnDisco * 2);
  });

  it('dos asentamientos hermanos LEJANOS siguen siendo dos manchas de la misma facción', () => {
    const fusionadas = fusionar([
      asentamiento('a', 'roja', 200, 200, 80),
      asentamiento('b', 'roja', 1500, 1500, 80),
    ]);
    expect(fusionadas).toHaveLength(1);
    expect(fusionadas[0]!.contornos).toHaveLength(2);
  });

  it('facciones distintas nunca se fusionan entre sí', () => {
    const fusionadas = fusionar([
      asentamiento('a', 'roja', 500, 500, 120),
      asentamiento('b', 'azul', 600, 500, 120),
    ]);
    expect(fusionadas.map((z) => z.faccionId).sort()).toEqual(['azul', 'roja']);
    expect(fusionadas).toHaveLength(2);
  });

  it('la frontera con una facción rival se respeta: la silueta fusionada no invade al vecino', () => {
    // Roja tiene dos asentamientos pegados y azul uno enfrente. La frontera cae a mitad de camino entre
    // el asentamiento rojo más cercano y el azul (radios iguales), así que ningún punto de la silueta roja
    // puede pasar de ahí.
    const fusionadas = fusionar([
      asentamiento('r1', 'roja', 400, 500, 150),
      asentamiento('r2', 'roja', 300, 500, 150),
      asentamiento('z1', 'azul', 700, 500, 150),
    ]);
    const roja = fusionadas.find((z) => z.faccionId === 'roja')!;
    const fronteraX = (400 + 700) / 2;
    const maxX = Math.max(...roja.contornos.flat().map((p) => p.x));
    expect(maxX).toBeLessThanOrEqual(fronteraX + 1);
  });

  it('un anillo de asentamientos hermanos deja el claro central como hueco recortable', () => {
    const radioCorona = 220;
    const anillo = Array.from({ length: 8 }, (_, i) => {
      const a = (i / 8) * Math.PI * 2;
      return asentamiento(
        `a${i}`,
        'roja',
        1000 + Math.cos(a) * radioCorona,
        1000 + Math.sin(a) * radioCorona,
        100
      );
    });
    const contornos = fusionar(anillo)[0]!.contornos;
    expect(contornos).toHaveLength(2);
    // Signos opuestos = el relleno `nonzero` del canvas vacía el hueco en vez de taparlo.
    const areas = contornos.map(areaFirmada);
    expect(Math.sign(areas[0]!)).toBe(-Math.sign(areas[1]!));
  });

  it('una zona sin polígono no hace aparecer a su facción con una silueta vacía', () => {
    // `computeZonaInfluencia` devuelve polígono vacío cuando los recortes con rivales no dejan nada
    // (ver el `break` al quedarse sin puntos). Esa zona no debe aportar contorno ni facción al resultado.
    const asentamientos = [asentamiento('a', 'roja', 500, 500, 100), asentamiento('b', 'verde', 900, 900, 100)];
    const zonas = [
      { asentamientoId: 'a', poligono: computeTodasLasZonas(asentamientos)[0]!.poligono },
      { asentamientoId: 'b', poligono: [] },
    ];

    const fusionadas = computeZonasFusionadasPorFaccion(zonas, asentamientos);
    expect(fusionadas.map((z) => z.faccionId)).toEqual(['roja']);
  });

  it('ignora zonas de asentamientos que ya no existen', () => {
    const asentamientos = [asentamiento('a', 'roja', 500, 500, 100)];
    const zonas = [
      ...computeTodasLasZonas(asentamientos),
      { asentamientoId: 'fantasma', poligono: computeTodasLasZonas(asentamientos)[0]!.poligono },
    ];
    expect(computeZonasFusionadasPorFaccion(zonas, asentamientos)).toHaveLength(1);
  });
});
