// La potencia del generador sin `Math.pow` ni `**`: su resultado con exponente no entero cambia en el último bit de una versión de V8 a otra
// (Node 22 y Node 24 dan alturas distintas con la misma semilla, 2026-11-26), y el mundo con él. `Math.exp` y `Math.log` son el port de fdlibm de V8
// y dan lo mismo en las dos (lo mide la sonda de `__tests__/determinismo.test.ts`); los exponentes enteros van por multiplicaciones, exactas en IEEE.

/** `b` elevado a `e`, para `b >= 0`, igual en cualquier versión de Node. */
export function potencia(b: number, e: number): number {
  if (Number.isInteger(e) && Math.abs(e) <= 64) {
    let r = 1, x = b, n = Math.abs(e);
    while (n > 0) {
      if (n & 1) r *= x;
      x *= x;
      n >>= 1;
    }
    return e < 0 ? 1 / r : r;
  }
  if (b < 0 || Number.isNaN(b)) return NaN;
  if (b === 0) return e > 0 ? 0 : Infinity;
  if (b === 1) return 1;
  return Math.exp(e * Math.log(b));
}
