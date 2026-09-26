import { afterEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Point } from '../../domain/types';
import { ZONA_INFLUENCIA } from '../../constants';
import { computeTodasLasZonas, pointInPolygon, posicionLibreParaFundar } from '../zones';

// Solo los campos consumidos por la geometría, como en zonas_fusionadas.test.ts.
function plaza(id: string, faccionId: string, x: number, y: number, radio: number): Asentamiento {
  return { id, faccionId, posicion: { x, y }, radioPotencial: radio } as Asentamiento;
}

const segmentosIniciales = ZONA_INFLUENCIA.segmentosPoligono;
afterEach(() => { ZONA_INFLUENCIA.segmentosPoligono = segmentosIniciales; });

function comprobar(asentamientos: Asentamiento[]) {
  const zonas = computeTodasLasZonas(asentamientos);
  const puntos: Point[] = [];
  for (let x = -20; x <= 180; x += 10) {
    for (let y = -20; y <= 180; y += 10) puntos.push({ x, y });
  }
  // Vértices, bordes y puntos a ambos lados: conservar también la semántica numérica.
  for (const z of zonas) {
    z.poligono.forEach((p, i) => {
      const siguiente = z.poligono[(i + 1) % z.poligono.length]!;
      puntos.push(p, { x: (p.x + siguiente.x) / 2, y: (p.y + siguiente.y) / 2 },
        { x: p.x + 1e-9, y: p.y }, { x: p.x - 1e-9, y: p.y });
    });
  }
  for (const p of puntos) {
    expect(posicionLibreParaFundar(p, asentamientos)).toBe(zonas.every((z) => !pointInPolygon(p, z.poligono)));
  }
}

describe('consulta de fundación con geometría reutilizada', () => {
  it('coincide con el cálculo directo en vacíos, solapes, recortes y copias del estado', () => {
    comprobar([]);
    const asentamientos = [plaza('a', 'roja', 30, 40, 50), plaza('b', 'roja', 60, 50, 40), plaza('c', 'azul', 100, 80, 70)];
    comprobar(asentamientos);
    comprobar(structuredClone(asentamientos));
    comprobar([...asentamientos].reverse());
    comprobar([plaza('otro-mundo', 'verde', 120, 120, 10)]);
    comprobar(asentamientos);
  });

  it('invalida al cambiar radio, posición, dueño, id o población de plazas sobre el mismo array', () => {
    const asentamientos = [plaza('a', 'roja', 30, 40, 50), plaza('b', 'azul', 80, 50, 40)];
    comprobar(asentamientos);
    asentamientos[0]!.radioPotencial = 80;
    comprobar(asentamientos);
    asentamientos[1]!.posicion.x = 150;
    comprobar(asentamientos);
    asentamientos[1]!.posicion.y = 130;
    comprobar(asentamientos);
    asentamientos[1]!.faccionId = 'roja';
    comprobar(asentamientos);
    asentamientos[1]!.faccionId = 'azul';
    asentamientos[1]!.id = 'a'; // La igualdad de id también decide qué recortes se omiten.
    comprobar(asentamientos);
    asentamientos.push(plaza('c', 'verde', 90, 90, 50));
    comprobar(asentamientos);
    asentamientos.splice(0, 1);
    comprobar(asentamientos);
    asentamientos.length = 0;
    comprobar(asentamientos);
  });

  it('invalida cuando cambia la resolución del polígono en caliente', () => {
    const asentamientos = [plaza('a', 'roja', 50, 50, 50)];
    comprobar(asentamientos);
    ZONA_INFLUENCIA.segmentosPoligono = 4;
    comprobar(asentamientos);
    ZONA_INFLUENCIA.segmentosPoligono = segmentosIniciales;
    comprobar(asentamientos);
  });
});
