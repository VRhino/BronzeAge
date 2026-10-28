import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { zstdCompressSync } from 'node:zlib';
import { create, toBinary } from '@bufbuild/protobuf';
import { FACTOR, anilloEn, type Anillo } from './campamentos';
import type { ClaseTerreno as ClaseTerrenoMundo, MundoGenerado, TipoRecurso } from './mundoTipos';
import type { Point } from './tipos';
import {
  Anillo as AnilloPb,
  ArtefactoDeMundoSchema,
  ClaseTerreno as ClaseTerrenoPb,
  EncodingRaster,
  type ArtefactoDeMundo,
  type Raster,
} from './gen/bronzeage/v1/world_pb';

/**
 * Escritor del artefacto de mundo (`ArtefactoDeMundo`, world.proto) a partir del `MundoGenerado` interno.
 *
 * Convenciones (las asume igual el cargador en Rust; no cambiarlas):
 *  - Rasters: celdas de `TAMANO_CELDA` = 4 unidades estratégicas (4000 milésimas), columnas = ceil(ancho/celda), orden por filas,
 *    fila 0 al norte (y creciente = sur en el generador). Se muestrea el centro de cada celda.
 *  - altura u16 LE (0..1 → 0..65535); terreno/anillos u8 con los enums de world.proto; fertilidad, humedad y densidad_bosque u8
 *    (0..1 → 0..255); transitable en bits, celda i → byte i>>3, bit i&7 (LSB primero). Todos comprimidos con zstd.
 *  - Unidades: del generador a milésimas estratégicas, `round(u * 1000 / FACTOR)`.
 *  - hash = sha256 (hex minúsculas) de la serialización del artefacto con `manifiesto.hash = ""`.
 */

export const VERSION_FORMATO = 1;
/** Lado de la celda de los rasters, en milésimas de unidad estratégica. */
export const TAMANO_CELDA = 4000;
const CELDA_GEN = (TAMANO_CELDA / 1000) * FACTOR;

/** Unidades del generador → milésimas estratégicas. */
export const mil = (u: number): bigint => BigInt(Math.round((u * 1000) / FACTOR));
const punto = (p: Point) => ({ x: mil(p.x), y: mil(p.y) });
const milesimas01 = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 1000);
/** unidades del generador² → milésimas estratégicas². */
const milArea = (a: number): bigint => BigInt(Math.round((a * 1_000_000) / (FACTOR * FACTOR)));

const sha256 = (b: Uint8Array) => createHash('sha256').update(b).digest('hex');
const colador = new Intl.Collator('en', { numeric: true });
/** Orden estable por id, numérico dentro del texto («parcela-2» antes que «parcela-10»). */
const porId = <T extends { id: string }>(xs: readonly T[]): T[] => [...xs].sort((a, b) => colador.compare(a.id, b.id));

const TERRENO: Record<ClaseTerrenoMundo, ClaseTerrenoPb> = {
  agua: ClaseTerrenoPb.TERRENO_AGUA,
  costa: ClaseTerrenoPb.TERRENO_COSTA,
  llano: ClaseTerrenoPb.TERRENO_LLANO,
  colina: ClaseTerrenoPb.TERRENO_COLINA,
  montana: ClaseTerrenoPb.TERRENO_MONTANA,
  cima: ClaseTerrenoPb.TERRENO_CIMA,
};
const ANILLO: Record<Anillo, AnilloPb> = { foco: AnilloPb.FOCO, frontera: AnilloPb.FRONTERA, lejana: AnilloPb.LEJANA };

/** Tipo de recurso del motor → (tipo del contrato, rareza). La ganadería es `livestock` en el motor. */
const RECURSO: Record<TipoRecurso, { tipo: string; rareza: 'comun' | 'intermedio' | 'raro' }> = {
  piedra: { tipo: 'piedra', rareza: 'comun' },
  cobre: { tipo: 'cobre', rareza: 'intermedio' },
  hierro: { tipo: 'hierro', rareza: 'intermedio' },
  estano: { tipo: 'estano', rareza: 'raro' },
  oro: { tipo: 'oro', rareza: 'raro' },
  ganaderia: { tipo: 'livestock', rareza: 'comun' },
};

/** Edificios de un campamento de mercenarios (Doc 1.9b): lista fija del contrato. */
const EDIFICIOS_CAMPAMENTO = ['taberna', 'vivienda', 'mercado', 'barracon', 'galeriaDeTiro', 'caballerizas'];

interface Capa { nombre: string; raster: Raster }

function raster(columnas: number, filas: number, encoding: EncodingRaster, datos: Uint8Array): Raster {
  // El Buffer de zlib puede ser una vista sobre un pool compartido: se copia a un Uint8Array propio.
  return { $typeName: 'bronzeage.v1.Raster', columnas, filas, tamanoCelda: TAMANO_CELDA, encoding, datos: new Uint8Array(zstdCompressSync(datos)), zstd: true };
}

/** Rasteriza las capas muestreando el centro de cada celda. */
function rasterizar(m: MundoGenerado): Capa[] {
  const columnas = Math.ceil(m.ancho / CELDA_GEN), filas = Math.ceil(m.alto / CELDA_GEN), n = columnas * filas;
  const altura = new Uint8Array(n * 2), terreno = new Uint8Array(n), fertilidad = new Uint8Array(n), humedad = new Uint8Array(n);
  const bosque = new Uint8Array(n), anillos = new Uint8Array(n), transitable = new Uint8Array(Math.ceil(n / 8));
  const u8 = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255);
  const s = m.muestras;
  for (let f = 0, i = 0; f < filas; f++) {
    for (let c = 0; c < columnas; c++, i++) {
      const p = { x: (c + 0.5) * CELDA_GEN, y: (f + 0.5) * CELDA_GEN };
      const h = Math.round(Math.min(1, Math.max(0, s.altura(p))) * 65535);
      altura[2 * i] = h & 0xff;
      altura[2 * i + 1] = h >> 8;
      terreno[i] = TERRENO[s.terreno(p)];
      fertilidad[i] = u8(s.fertilidad(p));
      humedad[i] = u8(s.humedad(p));
      bosque[i] = u8(s.densidadBosque(p));
      anillos[i] = ANILLO[anilloEn(m.campamentos, p).anillo];
      if (s.transitable(p)) transitable[i >> 3]! |= 1 << (i & 7);
    }
  }
  const r = (nombre: string, enc: EncodingRaster, d: Uint8Array): Capa => ({ nombre, raster: raster(columnas, filas, enc, d) });
  return [
    r('altura', EncodingRaster.ENCODING_U16_LE, altura),
    r('terreno', EncodingRaster.ENCODING_U8, terreno),
    r('fertilidad', EncodingRaster.ENCODING_U8, fertilidad),
    r('humedad', EncodingRaster.ENCODING_U8, humedad),
    r('densidad_bosque', EncodingRaster.ENCODING_U8, bosque),
    r('anillos', EncodingRaster.ENCODING_U8, anillos),
    r('transitable', EncodingRaster.ENCODING_BITS, transitable),
  ];
}

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

/**
 * Asocia a cada veta su alijo: emparejamiento voraz por distancia (el par más cercano primero, uno a uno).
 * ponytail: heurística; si el generador acaba emitiendo `vetaId` en los alijos, usarlo en su lugar.
 */
function alijoPorVeta(m: MundoGenerado): Map<string, string> {
  const pares = m.vetas.flatMap(v => m.alijos.map(a => ({ v: v.id, a: a.id, d: dist(v.centro, a.posicion) })));
  pares.sort((x, y) => x.d - y.d || colador.compare(x.v, y.v) || colador.compare(x.a, y.a));
  const porVeta = new Map<string, string>(), usados = new Set<string>();
  for (const { v, a } of pares) {
    if (porVeta.has(v) || usados.has(a)) continue;
    porVeta.set(v, a);
    usados.add(a);
  }
  return porVeta;
}

/** Masa de bosque más cercana (por centro) a un punto; "" si no hay ninguna. */
function bosqueMasCercano(m: MundoGenerado, p: Point): string {
  let mejor = '', d = Infinity;
  for (const b of porId(m.masas)) { const x = dist(b.centro, p); if (x < d) { d = x; mejor = b.id; } }
  return mejor;
}

function construirMensaje(m: MundoGenerado): ArtefactoDeMundo {
  const capas = rasterizar(m);
  const rast = (n: string) => capas.find(c => c.nombre === n)!.raster;
  const alijos = alijoPorVeta(m);
  const bandidosDeVeta = new Map<string, string>();
  for (const b of porId(m.bandidos)) if (b.vetaId && !bandidosDeVeta.has(b.vetaId)) bandidosDeVeta.set(b.vetaId, b.id);

  return create(ArtefactoDeMundoSchema, {
    manifiesto: {
      hash: '',
      versionFormato: VERSION_FORMATO,
      generador: m.generador,
      tamano: { x: mil(m.ancho), y: mil(m.alto) },
      factorUnity: FACTOR,
      capas: capas.map(c => ({ nombre: c.nombre, archivo: '', sha256: sha256(c.raster.datos), bytes: BigInt(c.raster.datos.length) })),
    },
    altura: rast('altura'),
    terreno: rast('terreno'),
    fertilidad: rast('fertilidad'),
    humedad: rast('humedad'),
    densidadBosque: rast('densidad_bosque'),
    anillos: rast('anillos'),
    transitable: rast('transitable'),
    rios: porId(m.rios).map(r => ({
      id: r.id, puntos: r.puntos.map(punto), terminaEnLago: r.terminaEnLago, navegable: r.navegable, vados: r.vados.map(punto),
    })),
    bosques: porId(m.masas).map(b => ({
      id: b.id, centro: punto(b.centro), radio: mil(b.radio), densidadMilesimas: milesimas01(b.densidad), area: milArea(b.area),
    })),
    nodos: porId(m.recursos).map(r => ({
      id: r.id,
      tipo: RECURSO[r.tipo].tipo,
      rareza: RECURSO[r.tipo].rareza,
      posicion: punto(r.posicion),
      cantidadInicial: BigInt(r.cantidadInicial) * 1000n, // cantidades de juego en milésimas
      anillo: r.anillo,
    })),
    vetas: porId(m.vetas).map(v => ({
      id: v.id,
      centro: punto(v.centro),
      nodosIds: v.nodosIds,
      campamentoBandidosId: bandidosDeVeta.get(v.id),
      alijoId: alijos.get(v.id),
    })),
    // Por definir con el motor: `origen` (0), `poblacion` (ritmo en 0), `mercado`, `fondos`, contadores y marcas de tiempo (0) ni
    // `eligieron_como_inicial` (0) ni `residentes_ids` (vacío) los genera el mundo: los fija el servidor al arrancar la partida.
    campamentos: porId(m.campamentos).map(c => ({
      id: c.id,
      posicion: punto(c.posicion),
      edificios: EDIFICIOS_CAMPAMENTO,
      poblacion: { valorEnT0: 0n, t0Ms: 0n, porMinuto: 0n },
      mercado: { porId: {} },
    })),
    // `poder` queda por definir (0): lo calcula el motor a partir del nivel.
    bandidosIniciales: porId(m.bandidos).map(b => ({
      id: b.id,
      posicion: punto(b.posicion),
      bosqueId: bosqueMasCercano(m, b.posicion),
      campamentoMercenariosId: anilloEn(m.campamentos, b.posicion).campamento || undefined,
      vetaId: b.vetaId,
      nivel: b.nivel,
    })),
    alijos: porId(m.alijos).map(a => ({ id: a.id, posicion: punto(a.posicion), cantidad: BigInt(a.cantidad) * 1000n, recurso: a.recurso })),
    navegacion: {
      espaciadoMin: Number(mil(m.grafo.espaciado)),
      nodos: [...m.grafo.nodos].sort((a, b) => a.id - b.id).map(n => ({ id: n.id, posicion: punto(n) })),
      aristas: m.grafo.aristas.map(a => ({ a: a.a, b: a.b, costeMilesimas: a.costeMilesimas, vado: a.vado })),
    },
    parcelas: porId(m.parcelas).map(p => ({
      id: p.id, bosqueId: p.masaId, centro: punto(p.centro), densidadMilesimas: milesimas01(p.densidad), anillo: p.anillo,
    })),
  });
}

/** Construye el artefacto de mundo serializado y su hash. Mismo `MundoGenerado` → mismos bytes. */
export function construirArtefacto(m: MundoGenerado): { bytes: Uint8Array; hash: string } {
  const a = construirMensaje(m);
  const hash = sha256(toBinary(ArtefactoDeMundoSchema, a));
  a.manifiesto!.hash = hash;
  return { bytes: toBinary(ArtefactoDeMundoSchema, a), hash };
}

/** Escribe `mundo-<hash>.bin` en `dir` y devuelve la ruta y el hash. */
export function escribirArtefacto(m: MundoGenerado, dir: string): { ruta: string; hash: string } {
  const { bytes, hash } = construirArtefacto(m);
  mkdirSync(dir, { recursive: true });
  const ruta = join(dir, `mundo-${hash}.bin`);
  writeFileSync(ruta, bytes);
  return { ruta, hash };
}
