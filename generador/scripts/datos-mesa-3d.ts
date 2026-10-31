// Datos de la mesa de escalas 3D (Consideraciones/artefactos/mesa_escalas_3d): una ventana de 320 × 240 u del mundo con la
// altura cada 0,5 u, el agua y la densidad de bosque, más los ríos y campamentos del artefacto que caen dentro.
// Uso: npx tsx scripts/datos-mesa-3d.ts <seed> <mundo-<hash>.bin> <salida.json> [x0 y0]   (unos 25 s)
import { readFileSync, writeFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { fromBinary } from '@bufbuild/protobuf';
import { ArtefactoDeMundoSchema } from '../src/gen/bronzeage/v1/world_pb';
import { generarRelieve } from '../src/relieve';
import { evaluarElevacion, evaluarTerreno } from '../src/elevacion';
import { densidadForestal } from '../src/forestacionRegional';

const F = 12, E = 1 / 1000;
const [seed, rutaMundo, salida, ax, ay] = process.argv.slice(2);
if (!seed || !rutaMundo || !salida) throw new Error('uso: datos-mesa-3d.ts <seed> <mundo.bin> <salida.json> [x0 y0]');
// Por defecto, entre la cordillera norte y el campamento-0 de la seed 42.
const x0 = Number(ax ?? 1276), y0 = Number(ay ?? 340), w = 320, h = 240, paso = 0.5, nx = w / paso + 1, ny = h / paso + 1;
const a = fromBinary(ArtefactoDeMundoSchema, readFileSync(rutaMundo));
const tam = a.manifiesto!.tamano!;
const r = generarRelieve(Number(seed), Number(tam.x) * E * F, Number(tam.y) * E * F), e = r.elevacion;
const b64 = (b: Uint8Array) => gzipSync(Buffer.from(b.buffer, b.byteOffset, b.byteLength), { level: 9 }).toString('base64');

// Altura (u16, 0..1 → 0..1000 m de Unity) y agua en los vértices de la malla
const alt = new Uint16Array(nx * ny), agua = new Uint8Array(nx * ny);
for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) {
  const p = { x: (x0 + i * paso) * F, y: (y0 + j * paso) * F }, z = evaluarElevacion(e, p);
  alt[j * nx + i] = Math.round(Math.min(1, Math.max(0, z)) * 65535);
  agua[j * nx + i] = evaluarTerreno(e, p, z) === 'agua' ? 1 : 0;
}
// Densidad de bosque en el centro de cada celda de 1 u
const pb = 1, bx = w / pb, by = h / pb, bosque = new Uint8Array(bx * by);
for (let j = 0; j < by; j++) for (let i = 0; i < bx; i++)
  bosque[j * bx + i] = Math.round(densidadForestal(r.forestacion, e, { x: (x0 + (i + .5) * pb) * F, y: (y0 + (j + .5) * pb) * F }) * 255);

// Ríos (un vértice de cada dos, como la página de escalas 2D) y campamentos, en coordenadas de la ventana
const cerca = (x: number, y: number) => x >= x0 - 5 && x <= x0 + w + 5 && y >= y0 - 5 && y <= y0 + h + 5;
const rios = a.rios
  .map(rr => rr.puntos.filter((_, k, ps) => k % 2 === 0 || k === ps.length - 1).map(p => [+(Number(p.x) * E).toFixed(1), +(Number(p.y) * E).toFixed(1)] as const))
  .filter(rr => rr.some(([x, y]) => cerca(x, y)))
  .map(rr => rr.map(([x, y]) => [+(x - x0).toFixed(2), +(y - y0).toFixed(2)]));
const campamentos = a.campamentos
  .map(c => ({ id: c.id, x: Number(c.posicion!.x) * E, y: Number(c.posicion!.y) * E }))
  .filter(c => c.x >= x0 && c.x <= x0 + w && c.y >= y0 && c.y <= y0 + h)
  .map(c => ({ id: c.id, x: +(c.x - x0).toFixed(2), y: +(c.y - y0).toFixed(2) }));

// Puntos para los botones «Ir a»: la pantalla (15 × 8,4 u) con más desnivel y la más llana del llano del sur, sin agua
let cordillera = [-1, 0, 0], llano = [Infinity, 0, 0];
for (let j = 0; j < ny - 17; j += 4) for (let i = 0; i < nx - 30; i += 4) {
  let mn = Infinity, mx = -Infinity, mojado = false;
  for (let b = 0; b < 17 && !mojado; b++) for (let c = 0; c < 30; c++) {
    const k = (j + b) * nx + i + c; if (agua[k]) { mojado = true; break; }
    const v = alt[k]! / 65535 * 1000; mn = Math.min(mn, v); mx = Math.max(mx, v);
  }
  if (mojado) continue;
  const d = mx - mn, cx = (i + 15) * paso, cy = (j + 8) * paso;
  if (d > cordillera[0]!) cordillera = [d, cx, cy];
  if (cy > 120 && cy < 215 && d < llano[0]!) llano = [d, cx, cy];
}
const puntos = {
  cordillera: [+cordillera[1]!.toFixed(1), +cordillera[2]!.toFixed(1)],
  llano: [+llano[1]!.toFixed(1), +llano[2]!.toFixed(1)],
  campamento: campamentos[0] ? [campamentos[0].x, +(campamentos[0].y - 6).toFixed(2)] : [w / 2, h - 10],
};

writeFileSync(salida, JSON.stringify({
  x0, y0, w, h, paso, nx, ny, altura: b64(new Uint8Array(alt.buffer)), agua: b64(agua),
  pasoBosque: pb, bx, by, bosque: b64(bosque), rios, campamentos, puntos,
}));
console.log(`${salida}: ventana (${x0}, ${y0}) de ${w} × ${h} u, ${rios.length} ríos, ${campamentos.length} campamentos`);
