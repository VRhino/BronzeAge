import { describe, expect, it } from 'vitest';
import { generarRelieve } from '../relieve';

describe('regresiones de conexión continental', () => {
  // La seed 2026 quedaba con un área aislada.
  for (const seed of [99, 2026]) it(`seed ${seed}: las 16 áreas quedan conectadas por tierra`, () => {
    const mapa = generarRelieve(seed, 32000, 24000);
    expect(mapa.conexion.conectadas).toBe(16);
  }, 60000);
});
