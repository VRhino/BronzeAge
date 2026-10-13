// Dibuja un artefacto de mundo en SVG leyendo solo el archivo (sin regenerar el relieve).
// Uso: npx tsx scripts/ver-artefacto.ts mundo-<hash>.bin salida.svg
import { readFileSync, writeFileSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';
import { fromBinary } from '@bufbuild/protobuf';
import { ArtefactoDeMundoSchema, ClaseTerreno } from '../src/gen/bronzeage/v1/world_pb';

const [ruta, salida] = [process.argv[2]!, process.argv[3] ?? 'artefacto.svg'];
const a = fromBinary(ArtefactoDeMundoSchema, readFileSync(ruta));
const t = a.terreno!, col = t.columnas, fil = t.filas;
const terreno = zstdDecompressSync(t.datos);
const E = 1 / 1000, paso = 2, celda = t.tamanoCelda * E;
const color: Record<number, string> = {
  [ClaseTerreno.TERRENO_AGUA]: '#9ec3d6', [ClaseTerreno.TERRENO_COSTA]: '#d8d0a0', [ClaseTerreno.TERRENO_LLANO]: '#c9d9a0',
  [ClaseTerreno.TERRENO_COLINA]: '#b0a878', [ClaseTerreno.TERRENO_MONTANA]: '#8a8478', [ClaseTerreno.TERRENO_CIMA]: '#e8e8e8',
};
let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${Number(a.manifiesto!.tamano!.x) * E} ${Number(a.manifiesto!.tamano!.y) * E}" width="1600" style="background:#fff">`;
for (let y = 0; y < fil; y += paso) for (let x = 0; x < col; x += paso)
  svg += `<rect x="${x * celda}" y="${y * celda}" width="${celda * paso}" height="${celda * paso}" fill="${color[terreno[y * col + x]!] ?? '#ccc'}"/>`;
const P = (p?: { x: bigint; y: bigint }) => ({ x: Number(p?.x ?? 0n) * E, y: Number(p?.y ?? 0n) * E });
for (const q of a.parcelas) { const c = P(q.centro); svg += `<circle cx="${c.x}" cy="${c.y}" r="2" fill="#1b5e20" opacity=".6"/>`; }
for (const m of a.campamentos) {
  const c = P(m.posicion);
  svg += `<circle cx="${c.x}" cy="${c.y}" r="250" fill="none" stroke="#c07000" stroke-width="2" stroke-dasharray="8 6"/><circle cx="${c.x}" cy="${c.y}" r="150" fill="rgba(60,140,60,.12)" stroke="#2a7a2a" stroke-width="2.5"/><rect x="${c.x - 7}" y="${c.y - 7}" width="14" height="14" fill="#222"/>`;
}
for (const v of a.vetas) { const c = P(v.centro); svg += `<circle cx="${c.x}" cy="${c.y}" r="30" fill="rgba(120,0,160,.10)" stroke="#7a00a0" stroke-width="1.5"/>`; }
const fill: Record<string, string> = { estano: '#1f5fbf', oro: '#e0a800', cobre: '#c8602a', hierro: '#555', piedra: '#999', livestock: '#e8e8e8' };
for (const n of a.nodos) { const c = P(n.posicion); svg += `<circle cx="${c.x}" cy="${c.y}" r="${n.tipo === 'estano' || n.tipo === 'oro' ? 5 : 3}" fill="${fill[n.tipo] ?? '#000'}" stroke="#fff" stroke-width=".8"/>`; }
for (const b of a.bandidosIniciales) { const c = P(b.posicion); svg += `<path d="M${c.x - 5} ${c.y - 5}L${c.x + 5} ${c.y + 5}M${c.x + 5} ${c.y - 5}L${c.x - 5} ${c.y + 5}" stroke="#b00020" stroke-width="2.5"/>`; }
for (const al of a.alijos) { const c = P(al.posicion); svg += `<path d="M${c.x} ${c.y - 6}L${c.x + 6} ${c.y}L${c.x} ${c.y + 6}L${c.x - 6} ${c.y}Z" fill="#fff" stroke="#e0a800" stroke-width="2"/>`; }
svg += '</svg>';
writeFileSync(salida, svg);
console.log(`hash ${a.manifiesto!.hash}; escrito ${salida}`);
