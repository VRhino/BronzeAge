// Export del mapa generado a un heightmap + splatmap de biomas + metadata utilizables por Unity Terrain
// (Fase 1 futura, ver `Consideraciones/Roadmap_Escalado.md`). Vive en `world/` (no en `worldgen/`) porque es
// una vista DERIVADA del mundo para una herramienta externa, no parte del pipeline determinista por seed:
// cambiar la resolución o la altura máxima aquí nunca sube `WORLDGEN_VERSION` ni afecta a ninguna partida
// guardada. Sin dependencias de DOM a propósito (testeable en Node/vitest) — rasterizar los pesos del
// splatmap a PNG es responsabilidad de la interfaz (`main.ts`, que sí tiene `<canvas>`), este módulo solo
// entrega los pesos crudos.
//
// Convención de escala/ejes (ya anotada como intención futura en el comentario de `RIOS`,
// `worldgen/config.ts`): 1 unidad de mapa = 1 metro. mapX -> Unity X, mapY -> Unity Z, elevación (0-1) ×
// `alturaMaximaMetros` -> Unity Y. La fila 0 de cada rejilla (heightmap y splatmap) corresponde a mapY=0 (el
// "norte" tal como lo dibuja `ui/canvas.ts`) — si el terreno entra espejado en Unity, usar "Flip Vertically"
// del diálogo Import Raw en vez de tocar este archivo.

import type { Asentamiento, BiomaTipo, Point, RegionId } from '../domain/types';
import { BIOMA, evaluarElevacion, evaluarFertilidad, evaluarTerreno, RIOS, type MapaGenerado } from '../worldgen';

export interface OpcionesExportUnity {
  /** Debe cumplir 2^n + 1 (513, 1025, 2049, 4097...) — lo que exige el importador RAW de Unity Terrain. */
  resolucion?: number;
  /** A qué altura real (metros) corresponde el valor de elevación 1.0 del campo continuo (0-1). */
  alturaMaximaMetros?: number;
  /** Resolución del splatmap de biomas — independiente de la del heightmap (Unity no exige 2^n+1 aquí). */
  resolucionSplatmap?: number;
}

export const UNITY_EXPORT_DEFAULT = {
  resolucion: 4097,
  alturaMaximaMetros: 1000,
  resolucionSplatmap: 2048,
} as const;

/** 1 unidad de mapa = 1 metro en el export — ver cabecera del archivo. */
export const UNITY_UNIDADES_POR_METRO = 1;

/** Orden fijo de capas del splatmap — determina el sufijo de archivo de cada máscara y es estable entre
 * exports (importa para que un script de Unity que ya mapeó capa->TerrainLayer no tenga que rehacerlo). */
export const BIOMA_ORDEN: readonly BiomaTipo[] = ['agua', 'costa', 'estepa', 'llanuraFertil', 'colina', 'montana', 'cima'];

interface EntidadExportada {
  id: string;
  x: number;
  z: number;
  alturaY: number;
}

export interface MetadataExportUnity {
  formatoVersion: 1;
  generadoEn: string;
  seed: number;
  region: RegionId | undefined;
  worldgenVersion: number;
  mundoMetros: { ancho: number; alto: number };
  unidadesPorMetro: number;
  ejes: string;
  heightmap: {
    archivo: string;
    resolucion: number;
    bitsPorMuestra: 16;
    byteOrder: 'little-endian';
    alturaMaximaMetros: number;
    filaCero: string;
  };
  splatmap: {
    resolucion: number;
    capas: { bioma: BiomaTipo; archivo: string }[];
    instrucciones: string;
  };
  nodos: (EntidadExportada & { tipo: string; rareza: string; cantidadInicial: number })[];
  bosques: (EntidadExportada & { radioMetros: number; densidad: number })[];
  rios: {
    id: string;
    navegable: boolean;
    terminaEnLago: boolean;
    anchoMetros: number;
    profundidadMetros: number;
    puntos: EntidadExportada[];
  }[];
  asentamientos: (EntidadExportada & { nombre: string; faccionId: string; nivel: number })[];
}

export interface ExportUnityResultado {
  /** Heightmap 16-bit little-endian, `resolucion × resolucion` muestras, listo para "Import Raw" en Unity. */
  heightmapRaw: Uint8Array<ArrayBuffer>;
  /** Peso 0-255 por bioma y celda (clasificación dura, un solo bioma a 255 por celda — ver `BIOMA_ORDEN`).
   * `main.ts` rasteriza cada capa a un PNG en escala de grises; este módulo no toca el DOM. */
  splatmap: { resolucion: number; capas: Record<BiomaTipo, Uint8Array<ArrayBuffer>> };
  metadata: MetadataExportUnity;
  /** Nombre de archivo sin extensión, compartido entre `.raw`, `.json` y las capas del splatmap de un mismo export. */
  nombreBase: string;
}

function validarResolucionHeightmap(resolucion: number): void {
  const potencia = Math.log2(resolucion - 1);
  if (!Number.isInteger(potencia) || potencia < 5) {
    throw new Error(
      `Resolución de heightmap inválida: ${resolucion}. Unity Terrain exige 2^n + 1 (513, 1025, 2049, 4097...).`
    );
  }
}

/** Elevación 0-1 muestreada en toda la rejilla, fila a fila (fila 0 = mapY=0). */
function generarAlturasBase(generado: MapaGenerado, resolucion: number): Float32Array {
  const { ancho, alto } = generado.config;
  const alturas = new Float32Array(resolucion * resolucion);
  for (let fila = 0; fila < resolucion; fila++) {
    const mapY = (fila / (resolucion - 1)) * alto;
    const base = fila * resolucion;
    for (let col = 0; col < resolucion; col++) {
      const mapX = (col / (resolucion - 1)) * ancho;
      alturas[base + col] = evaluarElevacion(generado.elevacion, { x: mapX, y: mapY });
    }
  }
  return alturas;
}

/**
 * Recorre cada río como una polilínea y llama a `aplicar` para toda celda de una rejilla `resolucion ×
 * resolucion` que caiga a menos de `radioMetros` de algún punto del trazo (incluye la fracción de radio
 * consumida, 0=centro..1=borde, para que el llamador pueda aplicar su propio falloff).
 *
 * ÚNICA vía de "cercanía a río" sobre una rejilla completa: `distanciaARioMasCercano` (worldgen/rios.ts) es
 * O(ríos×puntos) por CONSULTA — perfecto para una consulta suelta del motor, inviable llamarlo una vez por
 * celda cuando hay millones (heightmap 4097² o splatmap 2048²). En vez de eso se estampa un disco por cada
 * paso de cada río, exactamente igual que "pintar" un trazo grueso — el paso de muestreo es `radioMetros`
 * (no el tamaño de celda): dos discos de radio R centrados a distancia <= R siempre se solapan (cubren el
 * segmento entre ambos sin huecos), así que no hace falta samplear más denso que eso.
 */
function estamparCercaDeRios(
  generado: MapaGenerado,
  radioMetros: number,
  resolucion: number,
  aplicar: (idx: number, fraccionRadio: number) => void
): void {
  const { ancho, alto } = generado.config;
  const celda = Math.min(ancho, alto) / (resolucion - 1);
  const radioCeldas = Math.max(1, radioMetros / celda);
  const pasoMuestreo = Math.max(celda, radioMetros);

  const estampar = (mapX: number, mapY: number) => {
    const col = (mapX / ancho) * (resolucion - 1);
    const fila = (mapY / alto) * (resolucion - 1);
    const colMin = Math.max(0, Math.floor(col - radioCeldas));
    const colMax = Math.min(resolucion - 1, Math.ceil(col + radioCeldas));
    const filaMin = Math.max(0, Math.floor(fila - radioCeldas));
    const filaMax = Math.min(resolucion - 1, Math.ceil(fila + radioCeldas));
    for (let f = filaMin; f <= filaMax; f++) {
      for (let c = colMin; c <= colMax; c++) {
        const dist = Math.hypot(c - col, f - fila);
        if (dist > radioCeldas) continue;
        aplicar(f * resolucion + c, dist / radioCeldas);
      }
    }
  };

  for (const rio of generado.rios) {
    if (rio.puntos.length === 0) continue;
    if (rio.puntos.length === 1) {
      estampar(rio.puntos[0]!.x, rio.puntos[0]!.y);
      continue;
    }
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const a = rio.puntos[i]!;
      const b = rio.puntos[i + 1]!;
      const pasos = Math.max(1, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / pasoMuestreo));
      for (let s = 0; s <= pasos; s++) {
        const t = s / pasos;
        estampar(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
      }
    }
  }
}

/**
 * Talla el cauce de cada río en el heightmap (a petición del usuario, ver comentario de `RIOS` en
 * `worldgen/config.ts`: los ríos son solo polilíneas visuales hasta ahora, sin efecto en el relieve). Usa un
 * buffer de "profundidad deseada" con MAX en vez de restar directamente en cada estampado — así dos ríos que
 * se acercan, o dos puntos consecutivos del mismo río, no se restan dos veces sobre la misma celda.
 */
function tallarRios(alturas: Float32Array, generado: MapaGenerado, resolucion: number, alturaMaximaMetros: number): void {
  const profundidadNormalizada = RIOS.profundidadMetros / alturaMaximaMetros;
  const carve = new Float32Array(resolucion * resolucion);

  estamparCercaDeRios(generado, RIOS.anchoMetros / 2, resolucion, (idx, fraccionRadio) => {
    const t = 1 - fraccionRadio;
    const suavizado = t * t * (3 - 2 * t); // smoothstep: talla llena en el centro, cero en el borde
    const profundidad = suavizado * profundidadNormalizada;
    if (profundidad > carve[idx]!) carve[idx] = profundidad;
  });

  for (let i = 0; i < alturas.length; i++) {
    alturas[i] = Math.max(0, alturas[i]! - carve[i]!);
  }
}

/** Alturas 0-1 -> RAW de 16 bits little-endian (formato que espera "Import Raw" de Unity en Windows). */
function alturasARaw(alturas: Float32Array): Uint8Array<ArrayBuffer> {
  const bytes = new Uint8Array(alturas.length * 2);
  const vista = new DataView(bytes.buffer);
  for (let i = 0; i < alturas.length; i++) {
    const valor = Math.round(Math.min(1, Math.max(0, alturas[i]!)) * 65535);
    vista.setUint16(i * 2, valor, true);
  }
  return bytes;
}

/** `true` en cada celda de la rejilla a menos de `BIOMA.radioHumedadRio` de un río — proxy de humedad, ver
 * `evaluarBioma` (misma condición, versión rasterizada vía `estamparCercaDeRios` en vez de por consulta). */
function calcularCercaDeRio(generado: MapaGenerado, resolucion: number): Uint8Array {
  const cerca = new Uint8Array(resolucion * resolucion);
  estamparCercaDeRios(generado, BIOMA.radioHumedadRio, resolucion, (idx) => {
    cerca[idx] = 1;
  });
  return cerca;
}

/**
 * Splatmap de biomas: para cada celda de una rejilla `resolucion × resolucion`, clasificación dura (un único
 * bioma a peso 255, el resto a 0 — `BIOMA_ORDEN` fija el orden/nombre de archivo de cada capa).
 *
 * Reimplementa la rama de `evaluarBioma` (worldgen/biomas.ts) en vez de llamarla: esa función consulta
 * `distanciaARioMasCercano` por punto, aceptable para una consulta suelta del motor pero no para 2048² o más
 * — aquí la humedad sale de `calcularCercaDeRio`, precalculada una vez por estampado (ver esa función). Si
 * `evaluarBioma` cambia de criterio, esta función debe actualizarse junto a ella.
 */
function generarPesosSplatmap(generado: MapaGenerado, resolucion: number): Record<BiomaTipo, Uint8Array<ArrayBuffer>> {
  const { ancho, alto } = generado.config;
  const cercaDeRio = calcularCercaDeRio(generado, resolucion);
  const capas = Object.fromEntries(BIOMA_ORDEN.map((bioma) => [bioma, new Uint8Array(resolucion * resolucion)])) as Record<
    BiomaTipo,
    Uint8Array<ArrayBuffer>
  >;

  for (let fila = 0; fila < resolucion; fila++) {
    const mapY = (fila / (resolucion - 1)) * alto;
    const base = fila * resolucion;
    for (let col = 0; col < resolucion; col++) {
      const mapX = (col / (resolucion - 1)) * ancho;
      const idx = base + col;
      const p: Point = { x: mapX, y: mapY };
      const terreno = evaluarTerreno(generado.elevacion, p);

      let bioma: BiomaTipo;
      if (terreno !== 'llano') {
        bioma = terreno;
      } else {
        const fertil = evaluarFertilidad(generado.fertilidad, p) >= BIOMA.umbralFertilLlanura;
        bioma = fertil || cercaDeRio[idx] === 1 ? 'llanuraFertil' : 'estepa';
      }

      capas[bioma]![idx] = 255;
    }
  }

  return capas;
}

/**
 * Genera el heightmap (RAW 16-bit), el splatmap de biomas (un peso 0-255 por capa, ver `BIOMA_ORDEN`) y la
 * metadata (posiciones de nodos/bosques/ríos/asentamientos en metros, listas para instanciar en
 * Unity sin recalcular nada) de un mundo generado.
 * `asentamientos` es opcional porque `MapaGenerado` no los conoce — son estado de partida, no del mundo.
 */
export function exportarParaUnityTerrain(
  generado: MapaGenerado,
  asentamientos: readonly Asentamiento[] = [],
  opciones: OpcionesExportUnity = {}
): ExportUnityResultado {
  const resolucion = opciones.resolucion ?? UNITY_EXPORT_DEFAULT.resolucion;
  const alturaMaximaMetros = opciones.alturaMaximaMetros ?? UNITY_EXPORT_DEFAULT.alturaMaximaMetros;
  const resolucionSplatmap = opciones.resolucionSplatmap ?? UNITY_EXPORT_DEFAULT.resolucionSplatmap;
  validarResolucionHeightmap(resolucion);

  const alturas = generarAlturasBase(generado, resolucion);
  tallarRios(alturas, generado, resolucion, alturaMaximaMetros);
  const heightmapRaw = alturasARaw(alturas);

  const capasSplatmap = generarPesosSplatmap(generado, resolucionSplatmap);

  const alturaEnMetros = (p: Point) => evaluarElevacion(generado.elevacion, p) * alturaMaximaMetros;
  const nombreBase = `bronzeage-unity-seed${generado.config.seed}`;

  const metadata: MetadataExportUnity = {
    formatoVersion: 1,
    generadoEn: new Date().toISOString(),
    seed: generado.config.seed,
    region: generado.config.region,
    worldgenVersion: generado.version,
    mundoMetros: { ancho: generado.config.ancho, alto: generado.config.alto },
    unidadesPorMetro: UNITY_UNIDADES_POR_METRO,
    ejes: 'mapX -> Unity X · mapY -> Unity Z · elevación(0-1) × alturaMaximaMetros -> Unity Y',
    heightmap: {
      archivo: `${nombreBase}.raw`,
      resolucion,
      bitsPorMuestra: 16,
      byteOrder: 'little-endian',
      alturaMaximaMetros,
      filaCero:
        'mapY=0 (norte, igual que ui/canvas.ts) — si el terreno entra espejado en Unity, usar "Flip Vertically" en el diálogo Import Raw',
    },
    splatmap: {
      resolucion: resolucionSplatmap,
      capas: BIOMA_ORDEN.map((bioma) => ({ bioma, archivo: `${nombreBase}-splat-${bioma}.png` })),
      instrucciones:
        'Clasificación dura (0 o 255, sin mezcla): cada PNG en escala de grises es el peso de ESE bioma en cada ' +
        'celda. En Unity: crear un TerrainLayer por bioma (mismo orden que aquí), y por script leer cada PNG ' +
        '(import con "Non-Readable" desactivado) y llamar a TerrainData.SetAlphamaps normalizando los 7 pesos ' +
        'a que sumen 1 por celda (en clasificación dura ya suman 255 exacto en una sola capa, basta con /255).',
    },
    nodos: generado.nodos.map((n) => ({
      id: n.id,
      tipo: n.tipo,
      rareza: n.rareza,
      cantidadInicial: n.cantidadInicial,
      x: n.posicion.x,
      z: n.posicion.y,
      alturaY: alturaEnMetros(n.posicion),
    })),
    bosques: generado.bosques.map((b) => ({
      id: b.id,
      radioMetros: b.radio,
      densidad: b.densidad,
      x: b.centro.x,
      z: b.centro.y,
      alturaY: alturaEnMetros(b.centro),
    })),
    rios: generado.rios.map((r) => ({
      id: r.id,
      navegable: r.navegable,
      terminaEnLago: r.terminaEnLago,
      anchoMetros: RIOS.anchoMetros,
      profundidadMetros: RIOS.profundidadMetros,
      puntos: r.puntos.map((p, i) => ({ id: `${r.id}-${i}`, x: p.x, z: p.y, alturaY: alturaEnMetros(p) })),
    })),
    asentamientos: asentamientos.map((a) => ({
      id: a.id,
      nombre: a.nombre ?? a.id,
      faccionId: a.faccionId,
      nivel: a.nivel,
      x: a.posicion.x,
      z: a.posicion.y,
      alturaY: alturaEnMetros(a.posicion),
    })),
  };

  return { heightmapRaw, splatmap: { resolucion: resolucionSplatmap, capas: capasSplatmap }, metadata, nombreBase };
}
