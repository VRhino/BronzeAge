// Víveres del héroe (Doc 5.13, 2026-10-08): del carro sí, al carro nunca; comer del común descuenta a prorrata.
import { describe, expect, it } from 'vitest';
import type { Ejercito } from '../../domain/types';
import { LOGISTICA } from '../../constants';
import { descontarViveres, pasarAViveres, repartirEnViveres, ViveresInvalidosError } from '../viveres';
import { heroeDePrueba } from './fixtures';

const heroe = (id: string, viveres?: number) => ({ ...heroeDePrueba(id, { tipo: 'desconectado', punto: { x: 0, y: 0 } }), ...(viveres !== undefined ? { viveres } : {}) });
const columna = (suministro: Record<string, number>, liderId = 'h1') => ({ id: 'c', liderId, suministro }) as Ejercito;

describe('pasarAViveres', () => {
  it('mueve del carro lo que cabe, y solo el Líder', () => {
    const r = pasarAViveres(heroe('h1', LOGISTICA.capacidadViveresPorHeroe - 30), columna({ trigo: 100, madera: 5 }), 50);
    expect(r.movido).toBe(30);
    expect(r.heroe.viveres).toBe(LOGISTICA.capacidadViveresPorHeroe);
    expect(r.ejercito.suministro).toEqual({ trigo: 70, madera: 5 });
    expect(() => pasarAViveres(heroe('h2'), columna({ trigo: 100 }), 10)).toThrow(ViveresInvalidosError);
    expect(() => pasarAViveres(heroe('h1'), columna({ madera: 5 }), 10)).toThrow(ViveresInvalidosError);
  });
});

describe('el montón común de una columna', () => {
  it('comer descuenta a prorrata de lo que lleva cada uno', () => {
    const tras = descontarViveres([heroe('a', 300), heroe('b', 100), heroe('fuera', 50)], ['a', 'b'], 40);
    expect(tras.map((h) => h.viveres)).toEqual([270, 90, 50]);
  });

  it('repostar llena a cada uno hasta su tope', () => {
    const r = repartirEnViveres([heroe('a', LOGISTICA.capacidadViveresPorHeroe - 10), heroe('b')], ['a', 'b'], 100);
    expect(r.repartido).toBe(100);
    expect(r.heroes.map((h) => h.viveres)).toEqual([LOGISTICA.capacidadViveresPorHeroe, 90]);
  });
});
