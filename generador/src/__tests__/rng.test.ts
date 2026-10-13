// Contrato de `estado()`/`restaurarRng` (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3): la razón de
// ser de estas dos funciones es que `GameSession.importar` pueda CONTINUAR la secuencia de un RNG en vez de
// reiniciarla desde la seed — necesario para reconstruir un bug de producción a partir de un snapshot.
import { describe, expect, it } from 'vitest';
import { createRng, derivarSeed, restaurarRng } from '../rng';

describe('subsemillas de worldgen', () => {
  it('congela los valores de referencia y separa las etapas', () => {
    expect(derivarSeed(42, 'elevacion-v1')).toBe(687880233);
    expect(derivarSeed(42, 'rios-v1')).toBe(522548578);
    expect(derivarSeed(1, 'estructura-v1')).toBe(1313885234);
    expect(derivarSeed(0, '')).toBe(2166136261);
    expect(derivarSeed(42, 'elevacion-v1')).not.toBe(derivarSeed(42, 'rios-v1'));
  });
});

describe('RandomFn — estado() y restaurarRng()', () => {
  it('restaurarRng(estado()) continúa exactamente la misma secuencia', () => {
    const rng = createRng(12345);
    for (let i = 0; i < 7; i++) rng(); // avanza a un punto cualquiera, no el inicial

    const punto = rng.estado();
    const siguientesDelOriginal = Array.from({ length: 5 }, () => rng());

    const continuado = restaurarRng(punto);
    const siguientesDelRestaurado = Array.from({ length: 5 }, () => continuado());

    expect(siguientesDelRestaurado).toEqual(siguientesDelOriginal);
  });

  it('restaurarRng NO es lo mismo que createRng con la misma seed: reanuda, no reinicia', () => {
    const rng = createRng(999);
    for (let i = 0; i < 20; i++) rng();

    const reanudado = restaurarRng(rng.estado())();
    const reiniciado = createRng(999)();

    expect(reanudado).not.toBe(reiniciado);
  });

  it('estado() no se mueve por leerlo, solo por invocar el RNG', () => {
    const rng = createRng(1);
    rng();
    rng();
    const a = rng.estado();
    const b = rng.estado();
    expect(a).toBe(b);
  });
});
