// PRNG determinista (mulberry32) — mundo reproducible a partir de un seed.

export type RandomFn = () => number;

export function createRng(seed: number): RandomFn {
  let a = seed >>> 0;
  return function rng() {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function randRange(rng: RandomFn, min: number, max: number): number {
  return min + rng() * (max - min);
}

export function randInt(rng: RandomFn, min: number, max: number): number {
  return Math.floor(randRange(rng, min, max + 1));
}
