import { createHash } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { potencia } from '../potencia';

// El mundo ha de salir igual con la misma semilla en cualquier máquina y con cualquier versión de Node (Víctor, 2026-11-26: Node 24 en su PC y
// Node 22 en la nube daban mundos distintos por `Math.pow`). Esta sonda congela, bit a bit, lo que dan las funciones matemáticas que usa el
// generador sobre entradas exactas; los valores se midieron iguales en Node 22 (Linux) y Node 24 (Windows). Si una cambia en otra versión, la
// prueba lo dice antes de que cambie el mundo.
const huella = (xs: number[]) => createHash('sha256').update(new Uint8Array(new Float64Array(xs).buffer)).digest('hex').slice(0, 12);

function entradas() {
  let s = 12345n;
  const rnd = () => { s = (s * 6364136223846793005n + 1442695040888963407n) & ((1n << 64n) - 1n); return Number(s >> 11n) / 9007199254740992; };
  const p2 = [1];
  for (let i = 1; i <= 40; i++) p2.push(p2[i - 1]! * 2);
  const esc = () => p2[Math.floor(rnd() * 41)]! / 1048576;
  const n = 200_000, xs = Array.from({ length: n }, () => (rnd() - 0.5) * esc()), ys = Array.from({ length: n }, () => (rnd() - 0.5) * esc());
  return { n, xs, ys, pos: xs.map(Math.abs), peq: xs.map(x => x % 50), base: xs.map(x => Math.abs(x) % 4), expo: ys.map(y => y % 8) };
}

describe('determinismo del generador', () => {
  it('las funciones matemáticas que usa dan lo mismo bit a bit', () => {
    const { n, xs, ys, pos, peq } = entradas();
    expect({
      entrada: huella(xs.concat(ys)),
      sin: huella(xs.map(Math.sin)), cos: huella(xs.map(Math.cos)), exp: huella(peq.map(Math.exp)), tanh: huella(peq.map(Math.tanh)),
      atan2: huella(xs.map((x, i) => Math.atan2(ys[i]!, x))), hypot2: huella(xs.map((x, i) => Math.hypot(x, ys[i]!))),
      hypot3: huella(xs.map((x, i) => Math.hypot(x, ys[i]!, xs[(i + 1) % n]!))), sqrt: huella(pos.map(Math.sqrt)), log: huella(pos.map(Math.log)),
    }).toEqual({
      entrada: 'a709591e011c', sin: '7788bfa9686f', cos: 'ee0c80488728', exp: '82da0aa547e3', tanh: '1cc672063c99', atan2: 'c1e3a852da43',
      hypot2: 'cf85eb44efa1', hypot3: '32055803f6dc', sqrt: 'b83d6e13ec9b', log: '0d3546a1e5ef',
    });
  });

  it('la potencia propia da lo mismo bit a bit y se parece a la de la máquina', () => {
    const { base, expo } = entradas();
    const r = base.map((b, i) => potencia(b, expo[i]! < 0 ? -expo[i]! : expo[i]!));
    expect(huella(r)).toBe('cd7f89741648');
    base.slice(0, 2000).forEach((b, i) => { const e = Math.abs(expo[i]!), m = Math.pow(b, e); expect(Math.abs(r[i]! - m)).toBeLessThanOrEqual(1e-12 * Math.max(1, Math.abs(m))); });
    expect([potencia(2, 10), potencia(3, 4), potencia(0.5, -2), potencia(0, 1.5), potencia(1, 7.3)]).toEqual([1024, 81, 4, 0, 1]);
  });

  it('el código del generador no usa Math.pow ni el operador **', () => {
    const src = path.join(__dirname, '..');
    const ficheros = [...readdirSync(src).filter(f => f.endsWith('.ts')).map(f => path.join(src, f)), path.join(src, '..', 'scripts', 'generar-mundo.ts')];
    const fuera: string[] = [];
    for (const f of ficheros) {
      readFileSync(f, 'utf8').split(/\r?\n/).forEach((linea, i) => {
        const codigo = linea.replace(/\/\/.*$/, '').replace(/^\s*\*.*$/, '');
        if (/Math\.pow\s*\(|[^*/]\*\*[^*/]/.test(codigo)) fuera.push(`${path.basename(f)}:${i + 1}`);
      });
    }
    expect(fuera, 'usa potencia() de src/potencia.ts').toEqual([]);
  });
});
