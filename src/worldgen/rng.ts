// PRNG determinista (mulberry32) — mundo reproducible a partir de un seed.
// Vive en `worldgen/` porque la generación es su consumidor principal y la razón de que exista, pero es
// una utilidad genérica: los tests del motor también lo usan para sustituir `Math.random` y poder comparar
// dos corridas (ver `__tests__/fixtures.ts`).

/**
 * `estado()` expone el contador interno de 32 bits para poder guardarlo y retomar EXACTAMENTE la misma
 * secuencia con `restaurarRng` — necesario desde que `GameSession.exportar`/`importar` dejaron de reiniciar
 * el RNG desde la seed del mundo en cada carga (Fase B3, ver `session/gameSession.ts`). No cambia el
 * contrato de llamada: sigue siendo una función que se invoca como `rng()`, esto es solo una propiedad extra
 * sobre esa misma función.
 */
export interface RandomFn {
  (): number;
  estado(): number;
}

function crearDesdeContador(contadorInicial: number): RandomFn {
  let a = contadorInicial;
  const rng = (() => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }) as RandomFn;
  rng.estado = () => a >>> 0;
  return rng;
}

export function createRng(seed: number): RandomFn {
  return crearDesdeContador(seed >>> 0);
}

/** Reconstruye un RNG que continúa exactamente donde `estado()` lo dejó — a diferencia de `createRng(seed)`,
 * que siempre vuelve a la posición inicial de la secuencia de esa seed. */
export function restaurarRng(estado: number): RandomFn {
  return crearDesdeContador(estado >>> 0);
}

export function randRange(rng: RandomFn, min: number, max: number): number {
  return min + rng() * (max - min);
}

export function randInt(rng: RandomFn, min: number, max: number): number {
  return Math.floor(randRange(rng, min, max + 1));
}
