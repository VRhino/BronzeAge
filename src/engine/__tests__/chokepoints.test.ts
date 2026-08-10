// Control de chokepoints y peaje (Fase 0.3, Doc 1.5, ver `engine/chokepoints.ts`). Fixtures mínimas: solo
// los campos que el código bajo prueba realmente lee (`id`/`faccionId` de Asentamiento, `poligono` de
// ZonaInfluencia) — cast a `as unknown as Asentamiento`, mismo patrón que los `Mapa` sintéticos de
// `world/__tests__/rutas.test.ts` / `engine/__tests__/movimiento.test.ts`.

import { describe, expect, it } from 'vitest';
import type { Asentamiento, Chokepoint, Point, ZonaInfluencia } from '../../domain/types';
import { chokepointsDePeajeEnRuta, controladorDeChokepoint } from '../chokepoints';

function cuadrado(cx: number, cy: number, lado: number): Point[] {
  const r = lado / 2;
  return [
    { x: cx - r, y: cy - r },
    { x: cx + r, y: cy - r },
    { x: cx + r, y: cy + r },
    { x: cx - r, y: cy + r },
  ];
}

function asentamiento(id: string, faccionId: string): Asentamiento {
  return { id, faccionId } as unknown as Asentamiento;
}

function chokepoint(id: string, posicion: Point, radio = 40): Chokepoint {
  return { id, posicion, radio };
}

describe('controladorDeChokepoint', () => {
  const zonas: ZonaInfluencia[] = [
    { asentamientoId: 'a1', poligono: cuadrado(100, 100, 60) },
    { asentamientoId: 'a2', poligono: cuadrado(500, 500, 60) },
  ];

  it('devuelve el asentamiento cuya zona cubre el chokepoint', () => {
    expect(controladorDeChokepoint(chokepoint('c1', { x: 100, y: 100 }), zonas)).toBe('a1');
    expect(controladorDeChokepoint(chokepoint('c2', { x: 500, y: 500 }), zonas)).toBe('a2');
  });

  it('devuelve null si ninguna zona lo cubre', () => {
    expect(controladorDeChokepoint(chokepoint('c3', { x: 1000, y: 1000 }), zonas)).toBeNull();
  });
});

describe('chokepointsDePeajeEnRuta', () => {
  const ruta: Point[] = [
    { x: 0, y: 100 },
    { x: 1000, y: 100 },
  ];
  const asentamientosPorId = new Map<string, Asentamiento>([
    ['propio', asentamiento('propio', 'faccion-origen')],
    ['rival', asentamiento('rival', 'faccion-rival')],
    ['aliado-misma-faccion', asentamiento('aliado-misma-faccion', 'faccion-origen')],
  ]);

  it('cobra por un chokepoint cerca de la ruta controlado por una Facción distinta a la de origen', () => {
    const zonas: ZonaInfluencia[] = [{ asentamientoId: 'rival', poligono: cuadrado(500, 100, 60) }];
    const chokepoints = [chokepoint('c1', { x: 500, y: 100 })];
    const peajes = chokepointsDePeajeEnRuta(chokepoints, zonas, ruta, 'faccion-origen', asentamientosPorId);
    expect(peajes).toEqual([{ chokepoint: chokepoints[0], controladorId: 'rival' }]);
  });

  it('NO cobra si el chokepoint está controlado por un asentamiento de la MISMA Facción que origen', () => {
    const zonas: ZonaInfluencia[] = [{ asentamientoId: 'aliado-misma-faccion', poligono: cuadrado(500, 100, 60) }];
    const chokepoints = [chokepoint('c1', { x: 500, y: 100 })];
    expect(chokepointsDePeajeEnRuta(chokepoints, zonas, ruta, 'faccion-origen', asentamientosPorId)).toEqual([]);
  });

  it('NO cobra si ningún asentamiento controla el chokepoint (sin dueño)', () => {
    const chokepoints = [chokepoint('c1', { x: 500, y: 100 })];
    expect(chokepointsDePeajeEnRuta(chokepoints, [], ruta, 'faccion-origen', asentamientosPorId)).toEqual([]);
  });

  it('NO cobra si el chokepoint está más lejos de la ruta que su propio radio', () => {
    const zonas: ZonaInfluencia[] = [{ asentamientoId: 'rival', poligono: cuadrado(500, 500, 60) }];
    const chokepoints = [chokepoint('c1', { x: 500, y: 500 }, 40)]; // ruta pasa por y=100, a 400 de distancia
    expect(chokepointsDePeajeEnRuta(chokepoints, zonas, ruta, 'faccion-origen', asentamientosPorId)).toEqual([]);
  });

  it('devuelve varios chokepoints de peaje si la ruta atraviesa más de uno', () => {
    const zonas: ZonaInfluencia[] = [
      { asentamientoId: 'rival', poligono: cuadrado(300, 100, 60) },
      { asentamientoId: 'rival', poligono: cuadrado(700, 100, 60) },
    ];
    const chokepoints = [chokepoint('c1', { x: 300, y: 100 }), chokepoint('c2', { x: 700, y: 100 })];
    const peajes = chokepointsDePeajeEnRuta(chokepoints, zonas, ruta, 'faccion-origen', asentamientosPorId);
    expect(peajes.map((p) => p.chokepoint.id).sort()).toEqual(['c1', 'c2']);
  });
});
