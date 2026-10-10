import { describe, expect, it } from 'vitest';
import { SIMULACION } from '../../../constants';
import type { Foto } from '../diff';
import { anotarMovimiento, ticksEntre } from '../movimiento';

const T = SIMULACION.duracionTickMs;
const ruta = [{ x: 0, y: 0 }, { x: 100, y: 0 }];
const MOVILES = { propios: ['ejercitos'], ajenos: ['ejercitosAvistados'] };
const foto = (instante: number, progreso: number, x: number, extra: Partial<Record<string, unknown>> = {}): Foto => ({
  instante,
  ejercitos: [{ id: 'e1', estado: 'marchando', ruta, progreso, posicionActual: { x: progreso * 100, y: 0 }, ...extra }],
  ejercitosAvistados: [{ id: 'x1', posicionActual: { x, y: 0 } }],
});

describe('anotarMovimiento', () => {
  it('sin foto anterior no anota nada', () => {
    const f = foto(T, 0.1, 5);
    expect(anotarMovimiento(f, undefined, MOVILES)).toBe(f);
  });

  it('el avance de un tick es la diferencia de progreso, y el desplazamiento de un ajeno la de posición', () => {
    const r = anotarMovimiento(foto(2 * T, 0.2, 12), foto(T, 0.1, 5), MOVILES);
    expect((r.ejercitos as { avancePorTick: number }[])[0]!.avancePorTick).toBeCloseTo(0.1);
    expect((r.ejercitosAvistados as { desplazamientoPorTick: { x: number } }[])[0]!.desplazamientoPorTick.x).toBeCloseTo(7);
  });

  it('con varios ticks de por medio lo reparte por tick', () => {
    const r = anotarMovimiento(foto(3 * T, 0.3, 0), foto(T, 0.1, 0), MOVILES);
    expect((r.ejercitos as { avancePorTick: number }[])[0]!.avancePorTick).toBeCloseTo(0.1);
  });

  it('un comando entre dos ticks (mismo instante) conserva lo anotado en vez de poner cero', () => {
    const previa = anotarMovimiento(foto(2 * T, 0.2, 12), foto(T, 0.1, 5), MOVILES);
    const r = anotarMovimiento(foto(2 * T, 0.2, 12), previa, MOVILES);
    expect((r.ejercitos as { avancePorTick: number }[])[0]!.avancePorTick).toBeCloseTo(0.1);
  });

  it('estacionado avanza 0; con otra ruta no se inventa el avance', () => {
    const quieto = anotarMovimiento(foto(2 * T, 0.1, 0, { estado: 'estacionado' }), foto(T, 0.1, 0), MOVILES);
    expect((quieto.ejercitos as { avancePorTick: number }[])[0]!.avancePorTick).toBe(0);
    const otraRuta = anotarMovimiento(foto(2 * T, 0, 0, { ruta: [{ x: 0, y: 0 }, { x: 0, y: 50 }] }), foto(T, 0.9, 0), MOVILES);
    expect((otraRuta.ejercitos as Record<string, unknown>[])[0]).not.toHaveProperty('avancePorTick');
  });

  it('no muta las entidades de la foto de entrada', () => {
    const nueva = foto(2 * T, 0.2, 12);
    anotarMovimiento(nueva, foto(T, 0.1, 5), MOVILES);
    expect((nueva.ejercitos as Record<string, unknown>[])[0]).not.toHaveProperty('avancePorTick');
  });
});

describe('ticksEntre', () => {
  it('0 si el reloj no avanzó', () => {
    expect(ticksEntre(T, T)).toBe(0);
    expect(ticksEntre(T, 3 * T)).toBe(2);
  });
});
