// Experimento: la hidrografía (caudal, ancho, lagos, bajada) sin generar el mundo entero.
import { crearRelieveRegional } from '../src/regional';
import { generarHidrografiaRegional } from '../src/hidrografiaRegional';
import { evaluarElevacion, type CampoElevacion } from '../src/elevacion';
const semilla = Number(process.argv[2] ?? 42);
const t0 = Date.now();
const regional = crearRelieveRegional(semilla, { ancho: 32004, alto: 24000 });
const campo: CampoElevacion = { regional };
const h = generarHidrografiaRegional(semilla, campo);
campo.lagos = h.lagos;
const qs = h.rios.flatMap(r => r.caudal).sort((a, b) => a - b), as = h.rios.flatMap(r => r.anchos).sort((a, b) => a - b);
const pct = (xs: number[], p: number) => xs[Math.floor(p * (xs.length - 1))]!;
console.log('tramos', h.rios.length, 'puntos', qs.length, 'lagos', h.lagos.length, 'ms', Date.now() - t0);
console.log('trenzas', h.rios.filter(r => r.id.includes('trenza')).length, 'brazos muertos', h.lagos.filter(l => l.id.includes('brazo')).map(l => `${l.id} (${(l.centro.x / 12).toFixed(0)},${(l.centro.y / 12).toFixed(0)}) r${l.radio.toFixed(0)}`).join(' '));
console.log('caudal ha p10/50/90/max', [0.1, 0.5, 0.9, 1].map(p => pct(qs, p).toFixed(1)).join(' '));
console.log('ancho u p10/50/90/max', [0.1, 0.5, 0.9, 1].map(p => (pct(as, p) / 12).toFixed(2)).join(' '));
const inicio = new Map<string, number>(), llegan = new Map<string, number>();
const clave = (p: { x: number; y: number }) => `${p.x},${p.y}`;
for (const r of h.rios) { inicio.set(clave(r.puntos[0]!), r.caudal[0]!); const f = clave(r.puntos.at(-1)!); llegan.set(f, (llegan.get(f) ?? 0) + r.caudal.at(-1)!); }
for (const [p, q] of llegan) if (inicio.has(p) && inicio.get(p)! + 1e-6 < q) console.log('confluencia pierde', p, inicio.get(p), q);
const sinLagos = { ...campo, lagos: [] };
let malos = 0, total = 0;
for (const r of h.rios) for (let i = 1; i < r.puntos.length; i++) { total++; if (!(evaluarElevacion(sinLagos, r.puntos[i]!) < evaluarElevacion(sinLagos, r.puntos[i - 1]!))) malos++; }
console.log('bajada: vértices que no bajan', malos, 'de', total);
for (const l of h.lagos) console.log(l.id, 'radio', l.radio.toFixed(0), 'nivel', (l.elevacionCentro * 1000).toFixed(1));
for (const r of h.rios) for (let i = 1; i < r.puntos.length; i++) {
  const a = evaluarElevacion(sinLagos, r.puntos[i - 1]!), b = evaluarElevacion(sinLagos, r.puntos[i]!);
  if (!(b < a)) console.log(r.id, 'i', i, 'de', r.puntos.length, 'fin', r.fin, 'sube', ((b - a) * 1000).toFixed(4), 'm', 'h', (b * 1000).toFixed(1));
}
