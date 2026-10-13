// Genera un demo.json sintético para el visor (sin el núcleo Rust).
// Uso: node generar-ejemplo.mjs [entidades=50] [salida=ejemplo.json] [semilla=42]
// Con 10000 entidades: node generar-ejemplo.mjs 10000 ejemplo-10000.json   (no se commitea)
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const N = +(process.argv[2] ?? 50);
const salida = process.argv[3] ?? join(dirname(fileURLToPath(import.meta.url)), 'ejemplo.json');
const semilla = +(process.argv[4] ?? 42);

// PRNG determinista (mulberry32)
let a = semilla >>> 0;
const azar = () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };

const ANCHO = 2667000, ALTO = 2000000, COL = 667, FIL = 500, DURACION = 3600000;
const CELDA = ANCHO / COL;

// terreno: suma de ondas aleatorias + caída radial -> una isla grande con costa irregular
const ondas = Array.from({ length: 8 }, (_, k) => ({ fx: (1 + k) * (0.6 + azar()), fy: (1 + k) * (0.6 + azar()), fa: azar() * 6.28, fb: azar() * 6.28, amp: 1 / (1 + k) }));
const terreno = new Uint8Array(COL * FIL);
for (let j = 0; j < FIL; j++) for (let i = 0; i < COL; i++) {
  const u = i / COL, v = j / FIL;
  let h = 0; for (const o of ondas) h += o.amp * Math.sin(o.fx * 6.28 * u + o.fa) * Math.sin(o.fy * 6.28 * v + o.fb);
  const r = Math.hypot((u - 0.5) * 1.3, (v - 0.5) * 1.6);
  h = h * 0.35 - r * 1.5 + 1.0; // >0 tierra
  terreno[j * COL + i] = h < 0 ? 1 : h < 0.06 ? 2 : h < 0.45 ? 3 : h < 0.8 ? 4 : h < 1.05 ? 5 : 6;
}
const celdaDe = (x, y) => terreno[Math.min(FIL - 1, Math.max(0, Math.floor(y / CELDA))) * COL + Math.min(COL - 1, Math.max(0, Math.floor(x / CELDA)))];
const esTierra = (x, y) => { const c = celdaDe(x, y); return c >= 3 && c <= 4; }; // llano o colina

function puntoEnTierra() { for (;;) { const x = azar() * ANCHO, y = azar() * ALTO; if (esTierra(x, y)) return { x, y }; } }
function destinoCerca(p) {
  for (let n = 0; n < 40; n++) {
    const ang = azar() * 6.283, d = 80000 + azar() * 220000;
    const x = p.x + Math.cos(ang) * d, y = p.y + Math.sin(ang) * d;
    if (x > 0 && y > 0 && x < ANCHO && y < ALTO && esTierra(x, y)) return { x, y };
  }
  return puntoEnTierra();
}

const campamentos = Array.from({ length: 6 }, (_, i) => ({ id: 'campamento-' + i, ...puntoEnTierra() }));
const FACCIONES = [['f1', '#3b82f6'], ['f2', '#ef4444'], ['f3', '#22a35a'], ['f4', '#d97706']];

const entidades = [];
for (let i = 0; i < N; i++) {
  const [faccion, color] = FACCIONES[i % FACCIONES.length];
  const base = campamentos[i % campamentos.length];
  let p = { x: base.x, y: base.y }, t = Math.floor(azar() * 120000);
  const plan = [{ x: Math.round(p.x), y: Math.round(p.y), t }];
  const ritmo = 0.4 + azar() * 0.5; // unidades por segundo (= milésimas por ms)
  while (t < DURACION) {
    // 12 % de las veces el siguiente destino es un campamento; si no, un punto cercano
    const q = azar() < 0.12 ? campamentos[Math.floor(azar() * campamentos.length)] : destinoCerca(p);
    const d = Math.hypot(q.x - p.x, q.y - p.y);
    t += Math.max(1000, Math.round(d / ritmo));
    p = { x: q.x, y: q.y };
    plan.push({ x: Math.round(p.x), y: Math.round(p.y), t });
    if (azar() < 0.3) { t += Math.round(60000 + azar() * 240000); plan.push({ x: Math.round(p.x), y: Math.round(p.y), t }); } // alto
  }
  entidades.push({ id: 'ej-' + (i + 1), nombre: 'Ejército ' + (i + 1), faccion, color, plan, radio_vision: 30000 });
}

// posición en t (mismo cálculo que el visor)
function posEn(pl, t) {
  if (t < pl[0].t || t > pl[pl.length - 1].t) return null;
  let lo = 0, hi = pl.length - 2;
  while (lo < hi) { const m = (lo + hi + 1) >> 1; if (pl[m].t <= t) lo = m; else hi = m - 1; }
  const f = (t - pl[lo].t) / (pl[lo + 1].t - pl[lo].t);
  return { x: pl[lo].x + (pl[lo + 1].x - pl[lo].x) * f, y: pl[lo].y + (pl[lo + 1].y - pl[lo].y) * f };
}

// eventos: encuentro = dos entidades de distinta Facción a menos de UMBRAL (muestreo cada 30 s, rejilla hash)
const UMBRAL = 40000, PASO = 30000, ENFRIAR = 600000, TOPE = 2000;
const eventos = [], ultimo = new Map();
for (let t = 0; t <= DURACION && eventos.length < TOPE; t += PASO) {
  const rej = new Map(), pos = [];
  entidades.forEach((e, i) => {
    const p = posEn(e.plan, t); if (!p) return;
    pos[i] = p;
    const k = Math.floor(p.x / UMBRAL) + ',' + Math.floor(p.y / UMBRAL);
    (rej.get(k) ?? rej.set(k, []).get(k)).push(i);
  });
  for (const [k, lista] of rej) {
    const [cx, cy] = k.split(',').map(Number);
    for (const [dx, dy] of [[0, 0], [1, -1], [1, 0], [1, 1], [0, 1]]) { // semivecindad: cada par se mira una vez
      const otra = rej.get((cx + dx) + ',' + (cy + dy)); if (!otra) continue;
      for (const i of lista) for (const j of otra) {
        if (dx === 0 && dy === 0 && j <= i) continue;
        if (eventos.length >= TOPE || entidades[i].faccion === entidades[j].faccion) continue;
        const pi = pos[i], pj = pos[j];
        if (Math.hypot(pi.x - pj.x, pi.y - pj.y) > UMBRAL) continue;
        const par = Math.min(i, j) + '-' + Math.max(i, j);
        if (t - (ultimo.get(par) ?? -Infinity) < ENFRIAR) continue;
        ultimo.set(par, t);
        eventos.push({ t, tipo: 'encuentro', texto: `${entidades[i].nombre} y ${entidades[j].nombre} se encuentran`,
          x: Math.round((pi.x + pj.x) / 2), y: Math.round((pi.y + pj.y) / 2), entidades: [entidades[i].id, entidades[j].id] });
      }
    }
  }
}
// llegadas a campamento (vértice que coincide con un campamento)
for (const e of entidades) for (const [k, v] of e.plan.entries()) {
  if (k === 0 || v.t > DURACION || (v.x === e.plan[k - 1].x && v.y === e.plan[k - 1].y)) continue; // ni salida ni altos
  const c = campamentos.find(c => Math.abs(c.x - v.x) < 1 && Math.abs(c.y - v.y) < 1);
  if (c && eventos.length < TOPE * 1.5) eventos.push({ t: v.t, tipo: 'llegada', texto: `${e.nombre} llega a ${c.id}`, x: v.x, y: v.y, entidades: [e.id] });
}
eventos.sort((x, y) => x.t - y.t);

const demo = {
  titulo: `Ejemplo sintético (${N} entidades)`, semilla, duracion_ms: DURACION,
  mundo: { ancho: ANCHO, alto: ALTO, terreno: { columnas: COL, filas: FIL, base64: Buffer.from(terreno).toString('base64') } },
  campamentos, entidades, eventos,
};
writeFileSync(salida, JSON.stringify(demo));
console.log(`${salida}: ${N} entidades, ${entidades.reduce((s, e) => s + e.plan.length, 0)} vértices, ${eventos.length} eventos`);
