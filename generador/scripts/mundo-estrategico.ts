// Genera el mundo estratégico (campamentos, anillos, estaño y oro) sobre el relieve regional y lo dibuja en SVG.
// Uso: npx tsx generador/scripts/mundo-estrategico.ts [seed] [salida.svg]
import { writeFileSync } from 'node:fs';
import { generarRelieve } from '../src/relieve';
import { generarMundoEstrategico } from '../src/mundoEstrategico';
import { evaluarTerreno } from '../src/elevacion';
import { ESTRATEGICAS, FACTOR } from '../src/campamentos';

const seed = Number(process.argv[2] ?? 42), salida = process.argv[3] ?? `mundo-estrategico-${seed}.svg`;
const ANCHO = 2667 * FACTOR + 0, ALTO = 2000 * FACTOR;
let t = Date.now();
const regional = generarRelieve(seed, ANCHO, ALTO);
console.log(`relieve ${(Date.now() - t) / 1000} s`);
t = Date.now();
const mundo = generarMundoEstrategico({ ...regional, conexion: regional.conexion });
console.log(`estratégico ${(Date.now() - t) / 1000} s; sitios de metal ${mundo.sitios}; campamentos ${mundo.campamentos.length}; vetas ${mundo.vetas.length}`);
const cuenta: Record<string, number> = {};
for (const n of mundo.nodos) cuenta[`${n.tipo}/${n.anillo}`] = (cuenta[`${n.tipo}/${n.anillo}`] ?? 0) + 1;
console.log(cuenta);
const porAnillo: Record<string, number> = {};
for (const q of mundo.parcelas) porAnillo[q.anillo] = (porAnillo[q.anillo] ?? 0) + 1;
const tam = mundo.masas.map(m => m.parcelasIds.length).sort((a, b) => a - b);
console.log(`bosques ${mundo.masas.length}, parcelas ${mundo.parcelas.length}`, porAnillo, `parcelas por masa: min ${tam[0]}, mediana ${tam[Math.floor(tam.length / 2)]}, max ${tam[tam.length - 1]}`);

const E = 1 / FACTOR, celda = 200, col: Record<string, string> = { agua: '#9ec3d6', costa: '#d8d0a0', llano: '#c9d9a0', colina: '#b0a878', montana: '#8a8478', cima: '#e8e8e8' };
let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${ANCHO * E} ${ALTO * E}" width="1600" style="background:#fff">`;
for (let y = 0; y < ALTO; y += celda) for (let x = 0; x < ANCHO; x += celda)
  svg += `<rect x="${x * E}" y="${y * E}" width="${celda * E}" height="${celda * E}" fill="${col[evaluarTerreno(regional.elevacion, { x: x + celda / 2, y: y + celda / 2 })] ?? '#ccc'}"/>`;
for (const q of mundo.parcelas) svg += `<circle cx="${q.centro.x * E}" cy="${q.centro.y * E}" r="2.5" fill="#1b5e20" opacity=".75"/>`;
for (const c of mundo.campamentos) {
  const { x, y } = c.posicion;
  svg += `<circle cx="${x * E}" cy="${y * E}" r="${ESTRATEGICAS.fronteraMax}" fill="none" stroke="#c07000" stroke-width="2" stroke-dasharray="8 6"/>`
    + `<circle cx="${x * E}" cy="${y * E}" r="${ESTRATEGICAS.focoMax}" fill="rgba(60,140,60,.12)" stroke="#2a7a2a" stroke-width="2.5"/>`
    + `<rect x="${x * E - 7}" y="${y * E - 7}" width="14" height="14" fill="#222"/>`;
}
for (const v of mundo.vetas) svg += `<circle cx="${v.centro.x * E}" cy="${v.centro.y * E}" r="30" fill="rgba(120,0,160,.10)" stroke="#7a00a0" stroke-width="1.5"/>`;
for (const n of mundo.nodos) {
  const r = n.anillo === 'foco' ? 6 : 4.5;
  svg += n.tipo === 'estano'
    ? `<circle cx="${n.posicion.x * E}" cy="${n.posicion.y * E}" r="${r}" fill="#1f5fbf" stroke="#fff" stroke-width="1"/>`
    : `<rect x="${n.posicion.x * E - r}" y="${n.posicion.y * E - r}" width="${r * 2}" height="${r * 2}" fill="#e0a800" stroke="#fff" stroke-width="1"/>`;
}
svg += '</svg>';
writeFileSync(salida, svg);
console.log(`escrito ${salida}`);
