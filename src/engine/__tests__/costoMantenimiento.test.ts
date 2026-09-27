// El mantenimiento es de los edificios y se cobra por el nivel efectivo (Doc 4.5, decisión del usuario 2026-09-27).
// Medido en la Era I: cuatro capitales de nivel 2 caían a nivel efectivo 1 sin dejar de pagar piedra, y acababan en
// ruinas unas tres horas después de quedarse sin ella.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import { calcularCostoMantenimiento } from '../mantenimiento';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const plaza = (): Asentamiento => fundarAsentamientoDeTest(crearMapaDeterminista(7), crearFacciones(), 'faccion-1', []).asentamiento;
const conEdificiosExtra = (a: Asentamiento, n: number): Asentamiento => ({
  ...a,
  edificios: [...a.edificios, ...Array.from({ length: n }, (_, i) => ({ ...a.edificios[0]!, id: `extra-${i}`, tipo: 'vivienda' as const, estado: 'activo' as const }))],
});

describe('coste de mantenimiento', () => {
  it('escala con los edificios activos, no con la población', () => {
    const base = plaza();
    const madera = (a: Asentamiento) => calcularCostoMantenimiento(a, a).madera!;
    expect(madera({ ...base, poblacion: { ...base.poblacion, pesants: base.poblacion.pesants * 10 } })).toBeCloseTo(madera(base));
    expect(madera(conEdificiosExtra(base, MANTENIMIENTO.edificiosReferencia)) - madera(base)).toBeCloseTo(MANTENIMIENTO.costoBase.madera);
  });

  it('cobra por el nivel efectivo: una plaza de nivel 2 caída a 1 deja de pagar piedra', () => {
    const base = plaza();
    expect(calcularCostoMantenimiento({ ...base, nivel: 2, nivelActual: 2 }, base).piedra).toBeGreaterThan(0);
    expect(calcularCostoMantenimiento({ ...base, nivel: 2, nivelActual: 1 }, base).piedra).toBeUndefined();
  });
});
