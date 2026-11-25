// Experimento: lo cerrado de los meandros publicados, en anchos del río (sin generar el mundo entero).
import { crearRelieveRegional } from '../src/regional';
import { generarHidrografiaRegional } from '../src/hidrografiaRegional';
import { evaluarElevacion, type CampoElevacion } from '../src/elevacion';
const semilla = Number(process.argv[2] ?? 42);
const t0 = Date.now();
const regional = crearRelieveRegional(semilla, { ancho: 32004, alto: 24000 });
const campo: CampoElevacion = { regional };
const h = generarHidrografiaRegional(semilla, campo);
const pct = (xs: number[], p: number) => [...xs].sort((a, b) => a - b)[Math.floor(p * (xs.length - 1))]!;
const cuellos: number[] = [], ondas: number[] = [], sinuosidades: number[] = [];
let anchos = 0;
for (const r of h.rios) {
  if (r.id.includes('trenza') || r.puntos.length < 4) continue;
  const w = r.anchos.reduce((a, b) => a + b, 0) / r.anchos.length;
  if (w < 20) continue;
  anchos++;
  const ps = r.puntos, s = [0];
  for (let i = 1; i < ps.length; i++) s.push(s[i - 1]! + Math.hypot(ps[i]!.x - ps[i - 1]!.x, ps[i]!.y - ps[i - 1]!.y));
  // el cuello: lo más cerca que pasan dos puntos del río separados por más de media vuelta (3π anchos de recorrido)
  let cuello = Infinity;
  for (let i = 0; i < ps.length; i++) for (let j = i + 1; j < ps.length; j++)
    if (s[j]! - s[i]! > 3 * Math.PI * w) cuello = Math.min(cuello, Math.hypot(ps[j]!.x - ps[i]!.x, ps[j]!.y - ps[i]!.y));
  if (cuello < Infinity) cuellos.push(cuello / w);
  const mio = { cuello: cuello / w } as Record<string, number>;
  // la onda: dos cambios del sentido de giro por onda
  let cambios = 0, antes = 0;
  for (let i = 1; i + 1 < ps.length; i++) {
    const a = ps[i - 1]!, b = ps[i]!, c = ps[i + 1]!, g = Math.sign((b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x));
    if (g && antes && g !== antes) cambios++;
    if (g) antes = g;
  }
  if (cambios >= 2) ondas.push((2 * s.at(-1)!) / cambios / w);
  mio.onda = (2 * s.at(-1)!) / Math.max(1, cambios) / w; mio.sinuosidad = s.at(-1)! / Math.hypot(ps.at(-1)!.x - ps[0]!.x, ps.at(-1)!.y - ps[0]!.y);
  console.log(r.id, 'ancho', w.toFixed(0), Object.entries(mio).map(([k, v]) => k + ' ' + v.toFixed(1)).join(' · '));
  sinuosidades.push(s.at(-1)! / Math.hypot(ps.at(-1)!.x - ps[0]!.x, ps.at(-1)!.y - ps[0]!.y));
}
const fila = (xs: number[]) => [0.1, 0.25, 0.5, 0.9].map(p => pct(xs, p).toFixed(1)).join(' / ');
console.log('ríos de 20 m o más', anchos, 'ms', Date.now() - t0);
console.log('cuello (anchos) p10/p25/p50/p90', fila(cuellos), '· bajo 2,5 anchos:', cuellos.filter(c => c < 2.5).length);
console.log('onda (anchos) p10/p25/p50/p90', fila(ondas));
console.log('sinuosidad p10/p25/p50/p90', fila(sinuosidades));
const sinLagos = { ...campo, lagos: [] };
let malos = 0, total = 0;
for (const r of h.rios) for (let i = 1; i < r.puntos.length; i++) { total++; if (!(evaluarElevacion(sinLagos, r.puntos[i]!) < evaluarElevacion(sinLagos, r.puntos[i - 1]!))) malos++; }
console.log('bajada: vértices que no bajan', malos, 'de', total, '· brazos muertos', h.lagos.filter(l => l.id.includes('brazo')).length);
// el de la mesa: (2400, 1724) u
const sitio = { x: 2400 * 12, y: 1724 * 12 };
for (const r of h.rios) {
  const d = Math.min(...r.puntos.map(p => Math.hypot(p.x - sitio.x, p.y - sitio.y)));
  if (d > 600) continue;
  const w = r.anchos.reduce((a, b) => a + b, 0) / r.anchos.length;
  console.log('cerca de la mesa:', r.id, 'a', d.toFixed(0), 'm · ancho', w.toFixed(1), 'm · puntos', r.puntos.length,
    '·', r.puntos.slice(0, 30).map(p => `(${(p.x / 12).toFixed(1)},${(p.y / 12).toFixed(1)})`).join(' '));
}
const subidas: number[] = [];
for (const r of h.rios) for (let i = 1; i < r.puntos.length; i++) { const d = evaluarElevacion(sinLagos, r.puntos[i]!) - evaluarElevacion(sinLagos, r.puntos[i - 1]!); if (d >= 0) subidas.push(d * 1000); }
if (subidas.length) console.log('subidas (m) p50/p90/p99/max', [0.5, 0.9, 0.99, 1].map(p => pct(subidas, p).toFixed(3)).join(' / '));
const extremos = new Set(h.rios.flatMap(r => [r.puntos[0]!, r.puntos.at(-1)!].map(p => `${p.x},${p.y}`)));
for (const r of h.rios) for (let i = 1; i < r.puntos.length; i++) {
  const d = evaluarElevacion(sinLagos, r.puntos[i]!) - evaluarElevacion(sinLagos, r.puntos[i - 1]!);
  if (d < 0) continue;
  const p = r.puntos[i]!, junta = [...extremos].map(k => k.split(',').map(Number)).reduce((m, [x, y]) => Math.min(m, Math.hypot(x! - p.x, y! - p.y)), Infinity);
  console.log('no baja', r.id, i, 'de', r.puntos.length, 'sube', (d * 1000).toFixed(3), 'm · a', junta.toFixed(0), 'm de una junta');
}
