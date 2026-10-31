import { createHash } from 'node:crypto';
import { zstdCompressSync, zstdDecompressSync, constants } from 'node:zlib';
import { create, toBinary } from '@bufbuild/protobuf';
import { FACTOR } from './campamentos';
import { evaluarElevacion, evaluarTerreno, type CampoElevacion } from './elevacion';
import { PaqueteRelieveSchema, TeselaRelieveSchema, type TeselaRelieve } from './gen/bronzeage/v1/world_pb';

/**
 * Paquete de relieve (`PaqueteRelieve`, world.proto; contrato 06 §8): la altura fina de todo el mapa para los clientes 3D, la que
 * enseña la mesa de escalas 3D. Muestras en los vértices de una rejilla de 0,5 u, en teselas de 64 u que se abren por separado.
 *
 * Convenciones (las asume igual el validador en Rust; no cambiarlas):
 *  - muestra (i, j) en x = i · paso, y = j · paso (unidades estratégicas), fila 0 al norte; fuera del mapa, la del borde;
 *  - altura u16 = round(clamp(e, 0, 1) · 65535), como el raster `altura` del artefacto; agua = mar y lagos (`evaluarTerreno`);
 *  - cada tesela guarda (lado + 1)² muestras y comparte su última fila y columna con la vecina;
 *  - la altura va como diferencia (módulo 2^16) con la predicción izquierda + arriba − diagonal, y comprimida con zstd;
 *  - hash = sha256 (hex minúsculas) de la serialización con `hash = ""`.
 */
export const RELIEVE_FINO = { paso: 0.5, ladoTesela: 128, nivelZstd: 19 } as const;

export interface DimensionesRelieve {
  /** Tamaño del mapa en unidades estratégicas. */
  ancho: number; alto: number;
  paso: number; lado: number;
  columnas: number; filas: number; teselasX: number; teselasY: number;
}

export function dimensionesRelieve(ancho: number, alto: number, paso: number = RELIEVE_FINO.paso, lado: number = RELIEVE_FINO.ladoTesela): DimensionesRelieve {
  const columnas = Math.ceil(ancho / paso) + 1, filas = Math.ceil(alto / paso) + 1;
  return { ancho, alto, paso, lado, columnas, filas, teselasX: Math.ceil((columnas - 1) / lado), teselasY: Math.ceil((filas - 1) / lado) };
}

/** Las (lado + 1)² muestras de la tesela (tx, ty): altura u16 y agua (1 = agua). */
export function muestrearTesela(campo: CampoElevacion, d: DimensionesRelieve, tx: number, ty: number): { altura: Uint16Array; agua: Uint8Array } {
  const n = d.lado + 1, altura = new Uint16Array(n * n), agua = new Uint8Array(n * n);
  for (let j = 0; j < n; j++) {
    const y = Math.min(d.filas - 1, ty * d.lado + j) * d.paso;
    for (let i = 0; i < n; i++) {
      const x = Math.min(d.columnas - 1, tx * d.lado + i) * d.paso;
      // la última muestra cae justo en el borde (o pasa de él si el tamaño no es múltiplo del paso)
      const p = { x: Math.min(x, d.ancho) * FACTOR, y: Math.min(y, d.alto) * FACTOR }, e = evaluarElevacion(campo, p);
      altura[j * n + i] = Math.round(Math.min(1, Math.max(0, e)) * 65535);
      agua[j * n + i] = evaluarTerreno(campo, p, e) === 'agua' ? 1 : 0;
    }
  }
  return { altura, agua };
}

const prediccion = (v: Uint16Array, n: number, k: number): number => {
  const i = k % n, j = (k - i) / n;
  if (i && j) return v[k - 1]! + v[k - n]! - v[k - n - 1]!;
  return i ? v[k - 1]! : j ? v[k - n]! : 0;
};

const zstd = (b: Uint8Array) => new Uint8Array(zstdCompressSync(b, { params: { [constants.ZSTD_c_compressionLevel]: RELIEVE_FINO.nivelZstd } }));

export function codificarTesela(altura: Uint16Array, agua: Uint8Array, lado: number): TeselaRelieve {
  const n = lado + 1, res = new Uint16Array(n * n);
  for (let k = 0; k < res.length; k++) res[k] = (altura[k]! - prediccion(altura, n, k)) & 0xffff;
  const bytes = new Uint8Array(res.length * 2);
  for (let k = 0; k < res.length; k++) { bytes[2 * k] = res[k]! & 0xff; bytes[2 * k + 1] = res[k]! >> 8; }
  let hayAgua = false;
  const bits = new Uint8Array(Math.ceil(n * n / 8));
  for (let k = 0; k < agua.length; k++) if (agua[k]) { bits[k >> 3]! |= 1 << (k & 7); hayAgua = true; }
  return create(TeselaRelieveSchema, { altura: zstd(bytes), agua: hayAgua ? zstd(bits) : new Uint8Array(0) });
}

/** Lo contrario de `codificarTesela`: lo que hará un cliente con cada tesela. */
export function decodificarTesela(t: TeselaRelieve, lado: number): { altura: Uint16Array; agua: Uint8Array } {
  const n = lado + 1, bytes = zstdDecompressSync(t.altura), altura = new Uint16Array(n * n), agua = new Uint8Array(n * n);
  if (bytes.length !== n * n * 2) throw new Error(`tesela de ${bytes.length} bytes; se esperaban ${n * n * 2}`);
  for (let k = 0; k < altura.length; k++) altura[k] = (bytes[2 * k]! | bytes[2 * k + 1]! << 8) + prediccion(altura, n, k) & 0xffff;
  if (t.agua.length) { const bits = zstdDecompressSync(t.agua); for (let k = 0; k < agua.length; k++) agua[k] = bits[k >> 3]! >> (k & 7) & 1; }
  return { altura, agua };
}

/** El paquete entero, con su hash; `teselas` por filas de teselas desde el noroeste. */
export function construirPaqueteRelieve(d: DimensionesRelieve, teselas: TeselaRelieve[], hashArtefacto: string): { bytes: Uint8Array; hash: string } {
  if (teselas.length !== d.teselasX * d.teselasY) throw new Error(`${teselas.length} teselas; se esperaban ${d.teselasX * d.teselasY}`);
  const p = create(PaqueteRelieveSchema, {
    hash: '', hashArtefacto, paso: Math.round(d.paso * 1000), ladoTesela: d.lado,
    columnas: d.columnas, filas: d.filas, teselasX: d.teselasX, teselasY: d.teselasY, teselas,
  });
  p.hash = createHash('sha256').update(toBinary(PaqueteRelieveSchema, p)).digest('hex');
  return { bytes: toBinary(PaqueteRelieveSchema, p), hash: p.hash };
}
