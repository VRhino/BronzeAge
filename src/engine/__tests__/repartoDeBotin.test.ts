// Repartir un botín entre las columnas del bando ganador (Doc 5.12.3): a partes iguales, lo que no cabe pasa a las
// demás y lo que no cabe en ninguna se pierde.
import { describe, expect, it } from 'vitest';
import type { Ejercito } from '../../domain/types';
import { instante } from '../../domain/tiempo';
import { repartirBotin } from '../repartoDeBotin';

const columna = (id: string, suministro: Record<string, number> = {}): Ejercito => ({
  id,
  faccionId: 'f',
  origenAsentamientoId: 'a',
  participantes: [{ heroeId: `h-${id}`, unidoEn: instante(0) }],
  tipo: 'personal',
  politicaDeUnion: 'rechazar',
  liderId: `h-${id}`,
  escuadronIds: [],
  suministro,
  caravanasAdjuntasIds: [],
  objetivo: { tipo: 'punto', punto: { x: 0, y: 0 } },
  ruta: [],
  progreso: 0,
  posicionActual: { x: 0, y: 0 },
  estado: 'estacionado',
});

const carro = (c: Ejercito | undefined, recurso = 'trigo') => c?.suministro[recurso] ?? 0;
const SIN_LIMITE = () => 10_000;

describe('repartirBotin', () => {
  it('reparte a partes iguales entre las columnas, no todo a la primera', () => {
    const [a, b, c] = repartirBotin([columna('a'), columna('b'), columna('c')], { trigo: 90, madera: 30 }, SIN_LIMITE);

    expect([carro(a), carro(b), carro(c)]).toEqual([30, 30, 30]);
    expect([carro(a, 'madera'), carro(b, 'madera'), carro(c, 'madera')]).toEqual([10, 10, 10]);
  });

  it('lo que no le cabe a una pasa a las que tienen sitio', () => {
    const limites: Record<string, number> = { a: 10, b: 1000, c: 1000 };
    const [a, b, c] = repartirBotin([columna('a'), columna('b'), columna('c')], { trigo: 90 }, (x) => limites[x.id]!);

    expect(carro(a)).toBe(10);
    expect(carro(b) + carro(c)).toBeCloseTo(80);
    expect(carro(b)).toBeCloseTo(carro(c));
  });

  it('lo que no cabe en ninguna se pierde, y la capacidad cuenta lo que ya llevaban', () => {
    const [a, b] = repartirBotin([columna('a', { trigo: 40 }), columna('b')], { oro: 500 }, () => 50);

    expect(a!.suministro['oro']).toBe(10);
    expect(b!.suministro['oro']).toBe(50);
  });

  it('con una sola columna es lo de siempre: todo a ella, hasta donde le quepa', () => {
    const [a] = repartirBotin([columna('a')], { trigo: 70 }, () => 50);

    expect(carro(a)).toBe(50);
  });

  it('sin columnas no hay a quién dárselo', () => {
    expect(repartirBotin([], { trigo: 70 }, SIN_LIMITE)).toEqual([]);
  });
});
