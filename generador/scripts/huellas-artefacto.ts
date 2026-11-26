// Huellas de cada parte de un artefacto de mundo, para comparar dos generaciones (en dos máquinas) y ver dónde empiezan a diferir.
// Los rásteres se miden descomprimidos: la compresión puede cambiar de una versión de Node a otra sin que cambie el mundo.
// Uso: npx tsx scripts/huellas-artefacto.ts salida/mundo/mundo-<hash>.bin
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';
import { fromBinary, toBinary, type DescMessage } from '@bufbuild/protobuf';
import { ArtefactoDeMundoSchema } from '../src/gen/bronzeage/v1/world_pb';

const h = (b: Uint8Array) => createHash('sha256').update(b).digest('hex').slice(0, 12);
const bruto = readFileSync(process.argv[2]!);
const a = fromBinary(ArtefactoDeMundoSchema, bruto) as unknown as Record<string, unknown>;
const salida: Record<string, unknown> = { node: process.version, plataforma: `${process.platform}-${process.arch}`, archivo: h(bruto) };
for (const f of ArtefactoDeMundoSchema.fields) {
  const v = a[f.localName];
  if (f.fieldKind === 'list' && f.listKind === 'message') {
    const s = f.message as DescMessage, xs = v as never[];
    const hs = xs.map(x => h(toBinary(s, x)));
    salida[f.localName] = { n: xs.length, hash: h(Buffer.from(hs.join(''))), primeros: hs.slice(0, 300) };
  } else if (f.fieldKind === 'message' && v) {
    const s = f.message as DescMessage, o = v as Record<string, unknown>;
    if (s.typeName.endsWith('.Raster')) {
      const datos = o.datos as Uint8Array;
      salida[f.localName] = { datos: h(zstdDecompressSync(datos)), comprimido: h(datos), resto: h(toBinary(s, { ...o, datos: new Uint8Array() } as never)) };
    } else salida[f.localName] = h(toBinary(s, v as never));
  }
}
console.log(JSON.stringify(salida));
