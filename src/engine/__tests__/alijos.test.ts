// Alijos de exploración (D29, D60-D63): colocados al crear el mundo, por héroe, solo oro de botín, solo Facciones sin asentamiento.
import { describe, expect, it } from 'vitest';
import type { Ejercito } from '../../domain/types';
import { ALIJOS, MERCENARIOS, VISION } from '../../constants';
import { colocarCampamentosIniciales } from '../mercenarios';
import { AlijoInvalidoError, abrirAlijo, alijosALaVista, colocarAlijos } from '../alijos';
import { crearMapaDeterminista, heroeDePrueba, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(42);
const campamentos = colocarCampamentosIniciales(mapa, instanteDeTest(0));
const distancia = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
const columnaEn = (posicionActual: { x: number; y: number }) => ({ id: 'col', posicionActual }) as unknown as Ejercito;

describe('colocarAlijos', () => {
  const alijos = colocarAlijos(mapa, campamentos);

  it('los de cada banda alrededor de cada campamento, con su oro, fuera de la protección y con ese campamento como el más cercano', () => {
    const porCampamento = ALIJOS.bandas.reduce((n, b) => n + b.cuantos, 0);
    expect(alijos.length).toBeGreaterThan(campamentos.length * porCampamento * 0.8);
    for (const a of alijos) {
      const cerca = Math.min(...campamentos.map((c) => distancia(a.posicion, c.posicion)));
      expect(cerca).toBeGreaterThan(MERCENARIOS.radioProteccion);
      const banda = ALIJOS.bandas.find((b) => b.oro === a.oro)!;
      expect(cerca).toBeGreaterThanOrEqual(banda.desde - 1e-6);
      expect(cerca).toBeLessThanOrEqual(banda.hasta + 1e-6);
    }
  });

  it('misma seed, mismos alijos', () => {
    expect(colocarAlijos(mapa, campamentos)).toEqual(alijos);
  });
});

describe('ver y abrir un alijo', () => {
  const alijo = { id: 'alijo-x', posicion: { x: 500, y: 500 }, oro: 5 };
  const heroe = heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'col' });

  it('se ve a la vista del héroe y hasta que lo abre', () => {
    expect(alijosALaVista([alijo], heroe, columnaEn({ x: 500 + VISION.jugadorSolo, y: 500 }))).toEqual([alijo]);
    expect(alijosALaVista([alijo], heroe, columnaEn({ x: 500 + VISION.jugadorSolo + 1, y: 500 }))).toEqual([]);
    expect(alijosALaVista([alijo], { ...heroe, alijosAbiertos: ['alijo-x'] }, columnaEn({ x: 500, y: 500 }))).toEqual([]);
  });

  it('se abre en el sitio una vez por héroe, a su oro de botín; solo quien es de una Facción sin asentamiento', () => {
    const abierto = abrirAlijo(alijo, heroe, columnaEn({ x: 500, y: 505 }), true);
    expect(abierto.oroDeBotin).toBe(5);
    expect(() => abrirAlijo(alijo, abierto, columnaEn({ x: 500, y: 505 }), true)).toThrow(AlijoInvalidoError);
    expect(() => abrirAlijo(alijo, heroe, columnaEn({ x: 600, y: 600 }), true), 'lejos').toThrow(AlijoInvalidoError);
    expect(() => abrirAlijo(alijo, heroe, columnaEn({ x: 500, y: 500 }), false), 'con asentamiento').toThrow(AlijoInvalidoError);
  });
});
