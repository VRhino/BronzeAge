import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { zstdDecompressSync } from 'node:zlib';
import { fromBinary, toBinary } from '@bufbuild/protobuf';
import { describe, expect, it } from 'vitest';
import { construirArtefacto, escribirArtefacto, mil } from '../artefacto';
import { FACTOR } from '../campamentos';
import type { MundoGenerado } from '../mundoTipos';
import { Anillo, ArtefactoDeMundoSchema, ClaseTerreno, EncodingRaster } from '../gen/bronzeage/v1/world_pb';

/** Mundo sintético de 480×360 unidades del generador (40×30 estratégicas): 10×8 celdas de 48. */
function mundoSintetico(): MundoGenerado {
  return {
    seed: 7,
    ancho: 480,
    alto: 360,
    generador: 'sintetico-0.1',
    campamentos: [
      { id: 'campamento-0', posicion: { x: 96, y: 96 } },
      { id: 'campamento-10', posicion: { x: 400, y: 300 } },
      { id: 'campamento-2', posicion: { x: 240, y: 180 } },
    ],
    recursos: [
      { id: 'nodo-1', tipo: 'piedra', posicion: { x: 100, y: 100 }, anillo: 'foco', cantidadInicial: 500 },
      { id: 'nodo-2', tipo: 'estano', posicion: { x: 300, y: 200 }, anillo: 'frontera', cantidadInicial: 80, vetaId: 'veta-0' },
      { id: 'nodo-3', tipo: 'ganaderia', posicion: { x: 50, y: 300 }, anillo: 'lejana', cantidadInicial: 40 },
      { id: 'nodo-4', tipo: 'cobre', posicion: { x: 60, y: 60 }, anillo: 'foco', cantidadInicial: 120 },
    ],
    vetas: [{ id: 'veta-0', centro: { x: 300, y: 200 }, nodosIds: ['nodo-2'], campamentoMasCercano: 'campamento-2' }],
    masas: [{ id: 'bosque-0', centro: { x: 120, y: 240 }, radio: 60, densidad: 0.5, area: 12 * 12 * 4, anillo: 'foco', parcelasIds: ['parcela-0', 'parcela-1'] }],
    parcelas: [
      { id: 'parcela-0', masaId: 'bosque-0', centro: { x: 110, y: 230 }, densidad: 0.62, anillo: 'foco' },
      { id: 'parcela-1', masaId: 'bosque-0', centro: { x: 130, y: 250 }, densidad: 0.38, anillo: 'frontera' },
    ],
    bandidos: [{ id: 'bandidos-0', posicion: { x: 310, y: 210 }, nivel: 2, vetaId: 'veta-0', anillo: 'frontera' }],
    alijos: [{ id: 'alijo-0', posicion: { x: 450, y: 40 }, recurso: 'piedra', cantidad: 15, anillo: 'lejana' }],
    rios: [{ id: 'rio-0', puntos: [{ x: 0, y: 120 }, { x: 240, y: 130 }], terminaEnLago: true, navegable: false, vados: [{ x: 120, y: 125 }] }],
    grafo: {
      espaciado: 96,
      nodos: [{ id: 1, x: 0, y: 0 }, { id: 0, x: 96, y: 0 }, { id: 2, x: 96, y: 96 }],
      aristas: [{ a: 0, b: 1, costeMilesimas: 1000, vado: false }, { a: 0, b: 2, costeMilesimas: 1500, vado: true }],
    },
    muestras: {
      altura: p => p.x / 480,
      terreno: p => (p.x < 100 ? 'agua' : p.y > 300 ? 'montana' : 'llano'),
      fertilidad: p => p.y / 360,
      humedad: () => 0.5,
      densidadBosque: p => (p.x > 240 ? 1 : 0),
      transitable: p => p.x >= 100 && (p.x + p.y) % 3 < 2,
    },
  };
}

const sha = (b: Uint8Array) => createHash('sha256').update(b).digest('hex');

describe('artefacto de mundo', () => {
  it('ida y vuelta y determinismo', () => {
    const a = construirArtefacto(mundoSintetico()), b = construirArtefacto(mundoSintetico());
    expect(Buffer.from(a.bytes).equals(Buffer.from(b.bytes))).toBe(true);
    expect(a.hash).toBe(b.hash);
    const msg = fromBinary(ArtefactoDeMundoSchema, a.bytes);
    expect(msg.manifiesto?.hash).toBe(a.hash);
    expect(toBinary(ArtefactoDeMundoSchema, msg)).toEqual(a.bytes);
  });

  it('el hash se recalcula con el campo vacío', () => {
    const { bytes, hash } = construirArtefacto(mundoSintetico());
    const msg = fromBinary(ArtefactoDeMundoSchema, bytes);
    msg.manifiesto!.hash = '';
    expect(sha(toBinary(ArtefactoDeMundoSchema, msg))).toBe(hash);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('cambiar el mundo cambia el hash', () => {
    const m = mundoSintetico();
    const h0 = construirArtefacto(m).hash;
    m.recursos[0]!.cantidadInicial += 1;
    expect(construirArtefacto(m).hash).not.toBe(h0);
  });

  it('manifiesto y capas: tamaños, sha256 de lo guardado', () => {
    const msg = fromBinary(ArtefactoDeMundoSchema, construirArtefacto(mundoSintetico()).bytes);
    const man = msg.manifiesto!;
    expect(man.versionFormato).toBe(1);
    expect(man.generador).toBe('sintetico-0.1');
    expect(man.factorUnity).toBe(12);
    expect(man.tamano).toMatchObject({ x: 40000n, y: 30000n });
    const rasters = { altura: msg.altura, terreno: msg.terreno, fertilidad: msg.fertilidad, humedad: msg.humedad, densidad_bosque: msg.densidadBosque, anillos: msg.anillos, transitable: msg.transitable };
    expect(man.capas.map(c => c.nombre)).toEqual(Object.keys(rasters));
    for (const c of man.capas) {
      const r = rasters[c.nombre as keyof typeof rasters]!;
      expect(c.archivo).toBe('');
      expect(c.sha256).toBe(sha(r.datos));
      expect(c.bytes).toBe(BigInt(r.datos.length));
    }
  });

  it('rasters: dimensiones y longitudes descomprimidas', () => {
    const msg = fromBinary(ArtefactoDeMundoSchema, construirArtefacto(mundoSintetico()).bytes);
    const celdas = 10 * 8;
    const largo = (r: NonNullable<typeof msg.altura>) => {
      expect(r.zstd).toBe(true);
      expect([r.columnas, r.filas, r.tamanoCelda]).toEqual([10, 8, 4000]);
      return zstdDecompressSync(r.datos).length;
    };
    expect(msg.altura!.encoding).toBe(EncodingRaster.ENCODING_U16_LE);
    expect(largo(msg.altura!)).toBe(2 * celdas);
    for (const r of [msg.terreno, msg.fertilidad, msg.humedad, msg.densidadBosque, msg.anillos]) {
      expect(r!.encoding).toBe(EncodingRaster.ENCODING_U8);
      expect(largo(r!)).toBe(celdas);
    }
    expect(msg.transitable!.encoding).toBe(EncodingRaster.ENCODING_BITS);
    expect(largo(msg.transitable!)).toBe(Math.ceil(celdas / 8));
  });

  it('rasters: el centro de cada celda, fila 0 al norte, bits LSB primero', () => {
    const msg = fromBinary(ArtefactoDeMundoSchema, construirArtefacto(mundoSintetico()).bytes);
    const alt = zstdDecompressSync(msg.altura!.datos), ter = zstdDecompressSync(msg.terreno!.datos);
    const anil = zstdDecompressSync(msg.anillos!.datos), bits = zstdDecompressSync(msg.transitable!.datos);
    const m = mundoSintetico();
    // Celda (col 9, fila 7): centro (456, 344) → altura 0.95, montaña (x>=100, y>300).
    const i = 7 * 10 + 9;
    expect(alt[2 * i]! | (alt[2 * i + 1]! << 8)).toBe(Math.round((456 / 480) * 65535));
    expect(ter[i]).toBe(ClaseTerreno.TERRENO_MONTANA);
    expect(ter[0]).toBe(ClaseTerreno.TERRENO_AGUA); // x = 24 < 100
    expect(anil[0]).toBe(Anillo.FOCO);
    for (let k = 0; k < 80; k++) {
      const p = { x: ((k % 10) + 0.5) * 48, y: (Math.floor(k / 10) + 0.5) * 48 };
      expect(((bits[k >> 3]! >> (k & 7)) & 1) === 1).toBe(m.muestras.transitable(p));
    }
  });

  it('vectoriales: milésimas, ordenación por id y asociaciones', () => {
    const msg = fromBinary(ArtefactoDeMundoSchema, construirArtefacto(mundoSintetico()).bytes);
    expect(mil(12)).toBe(1000n);
    expect(mil(96)).toBe(8000n);
    expect(msg.campamentos.map(c => c.id)).toEqual(['campamento-0', 'campamento-2', 'campamento-10']);
    expect(msg.campamentos[0]!.posicion).toMatchObject({ x: 8000n, y: 8000n });
    expect(msg.navegacion!.nodos.map(n => n.id)).toEqual([0, 1, 2]);
    expect(msg.navegacion!.espaciadoMin).toBe(8000);
    expect(msg.navegacion!.aristas[1]).toMatchObject({ a: 0, b: 2, costeMilesimas: 1500, vado: true });
    const por = Object.fromEntries(msg.nodos.map(n => [n.id, n]));
    expect(por['nodo-1']).toMatchObject({ tipo: 'piedra', rareza: 'comun', anillo: 'foco', cantidadInicial: 500_000n });
    expect(por['nodo-2']).toMatchObject({ tipo: 'estano', rareza: 'raro' });
    expect(por['nodo-3']).toMatchObject({ tipo: 'livestock', rareza: 'comun' });
    expect(por['nodo-4']).toMatchObject({ tipo: 'cobre', rareza: 'intermedio' });
    expect(msg.vetas[0]).toMatchObject({ id: 'veta-0', campamentoBandidosId: 'bandidos-0', alijoId: 'alijo-0', nodosIds: ['nodo-2'] });
    expect(msg.bandidosIniciales[0]).toMatchObject({ bosqueId: 'bosque-0', campamentoMercenariosId: 'campamento-2', vetaId: 'veta-0', nivel: 2 });
    expect(msg.bosques[0]).toMatchObject({ id: 'bosque-0', densidadMilesimas: 500, radio: 5000n, area: 4_000_000n });
    expect(msg.parcelas.map(p => [p.bosqueId, p.densidadMilesimas, p.anillo])).toEqual([['bosque-0', 620, 'foco'], ['bosque-0', 380, 'frontera']]);
    expect(msg.alijos[0]).toMatchObject({ recurso: 'piedra', cantidad: 15_000n });
    expect(msg.rios[0]).toMatchObject({ terminaEnLago: true, navegable: false });
    expect(msg.rios[0]!.vados).toHaveLength(1);
    expect(FACTOR).toBe(12);
  });

  it('escribe mundo-<hash>.bin en salida/', () => {
    const { ruta, hash } = escribirArtefacto(mundoSintetico(), join(import.meta.dirname, '..', '..', 'salida'));
    expect(ruta.endsWith(`mundo-${hash}.bin`)).toBe(true);
    expect(existsSync(ruta)).toBe(true);
    expect(sha(fromBinaryHashless(readFileSync(ruta)))).toBe(hash);
  });
});

function fromBinaryHashless(bytes: Uint8Array): Uint8Array {
  const msg = fromBinary(ArtefactoDeMundoSchema, bytes);
  msg.manifiesto!.hash = '';
  return toBinary(ArtefactoDeMundoSchema, msg);
}
