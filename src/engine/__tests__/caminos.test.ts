// Caminos comerciales (Fase 0.3, Doc 1.6, ver `engine/caminos.ts`).

import { describe, expect, it } from 'vitest';
import type { Asentamiento, CaminoComercial, Point } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { asegurarCaminoComercial, buscarCamino } from '../caminos';

function mapaSintetico(): Mapa {
  // `esTransitable: () => true` = todo tierra: este fixture mide el trazado del camino, no la
  // infranqueabilidad del agua (para eso, `world/__tests__/rutas.test.ts`).
  return { limites: { ancho: 2000, alto: 2000 }, costeEnPunto: () => 1, esTransitable: () => true } as unknown as Mapa;
}

function asentamiento(id: string, posicion: Point): Asentamiento {
  return { id, posicion } as unknown as Asentamiento;
}

describe('buscarCamino', () => {
  const caminos: CaminoComercial[] = [{ id: 'camino-a-b', asentamientoAId: 'a', asentamientoBId: 'b', puntos: [] }];

  it('encuentra el camino sin importar el orden A/B con el que se pregunte', () => {
    expect(buscarCamino(caminos, 'a', 'b')).toBe(caminos[0]);
    expect(buscarCamino(caminos, 'b', 'a')).toBe(caminos[0]);
  });

  it('devuelve undefined si no hay camino para ese par', () => {
    expect(buscarCamino(caminos, 'a', 'c')).toBeUndefined();
  });
});

describe('asegurarCaminoComercial', () => {
  const mapa = mapaSintetico();
  const a = asentamiento('a', { x: 100, y: 100 });
  const b = asentamiento('b', { x: 900, y: 900 });

  it('crea un camino nuevo si no existe uno para el par', () => {
    const resultado = asegurarCaminoComercial([], mapa, a, b);
    expect(resultado).toHaveLength(1);
    expect(resultado[0]!.puntos[0]).toEqual(a.posicion);
    expect(resultado[0]!.puntos[resultado[0]!.puntos.length - 1]).toEqual(b.posicion);
  });

  it('NO duplica si ya existe un camino para el par (en cualquier orden)', () => {
    const existente: CaminoComercial = { id: 'camino-existente', asentamientoAId: 'b', asentamientoBId: 'a', puntos: [b.posicion, a.posicion] };
    const resultado = asegurarCaminoComercial([existente], mapa, a, b);
    expect(resultado).toEqual([existente]);
  });
});
