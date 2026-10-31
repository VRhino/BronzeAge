import { describe, expect, it } from 'vitest';
import { createRng, derivarSeed } from '../rng';

describe('subsemillas de worldgen', () => {
  it('congela los valores de referencia y separa las etapas', () => {
    expect(derivarSeed(42, 'elevacion-v1')).toBe(687880233);
    expect(derivarSeed(42, 'rios-v1')).toBe(522548578);
    expect(derivarSeed(1, 'estructura-v1')).toBe(1313885234);
    expect(derivarSeed(0, '')).toBe(2166136261);
    expect(derivarSeed(42, 'elevacion-v1')).not.toBe(derivarSeed(42, 'rios-v1'));
  });
});

describe('createRng', () => {
  it('misma semilla, misma secuencia; otra semilla, otra', () => {
    const secuencia = (seed: number) => { const rng = createRng(seed); return Array.from({ length: 8 }, () => rng()); };
    expect(secuencia(12345)).toEqual(secuencia(12345));
    expect(secuencia(12346)).not.toEqual(secuencia(12345));
  });
});
