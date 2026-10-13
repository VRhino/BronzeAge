import type { Point, RioZona } from './tipos';
import type { CampoElevacion } from './types';
import { evaluarElevacion, gradienteElevacion } from './elevacion';
import { humedadRegionalEn } from './regional';
import { generarCampoRuido, evaluarRuido, type CampoRuido } from './ruido';
import { createRng, derivarSeed } from './rng';
import { puntoEnLago } from './aguaRegional';

export type TipoForestal = 'abierto' | 'denso' | 'ribera' | 'montano';
interface Segmento { a: Point; b: Point }
export interface CampoForestal {
  masas: CampoRuido; detalle: CampoRuido;
  segmentos: Segmento[]; indiceRiberas: Record<string, number[]>;
}
export interface SectorForestal {
  id: string; x: number; y: number; ancho: number; alto: number;
  superficieArbolada: number; densidadMedia: number; superficieEquivalente: number;
}
const suave = (x: number) => { const t = Math.max(0, Math.min(1, x)); return t * t * (3 - 2 * t); };
const CELDA_RIBERA = 500, ALCANCE_RIBERA = 260;

/** Campo inmutable, serializable e independiente de la resolución del visor.
 * La madera es permanente: no hay stock, agotamiento ni recuperación. */
export function crearCampoForestal(seed: number, rios: readonly RioZona[]): CampoForestal {
  const rng = createRng(derivarSeed(seed, 'forestacion-regional-1'));
  const ruido = (frecuenciaBase: number, octavas: number) => generarCampoRuido(rng,
    { frecuenciaBase, octavas, lacunaridad: 2.13, persistencia: 0.52 });
  const campo: CampoForestal = { masas: ruido(1 / 4300, 3), detalle: ruido(1 / 720, 3), segmentos: [], indiceRiberas: {} };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, id = campo.segmentos.length;
    campo.segmentos.push({ a, b });
    for (let y = Math.floor((Math.min(a.y, b.y) - ALCANCE_RIBERA) / CELDA_RIBERA); y <= Math.floor((Math.max(a.y, b.y) + ALCANCE_RIBERA) / CELDA_RIBERA); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - ALCANCE_RIBERA) / CELDA_RIBERA); x <= Math.floor((Math.max(a.x, b.x) + ALCANCE_RIBERA) / CELDA_RIBERA); x++)
        (campo.indiceRiberas[`${x},${y}`] ??= []).push(id);
  }
  return campo;
}

export function evaluarForestal(campo: CampoForestal, elevacion: CampoElevacion, p: Point,
  muestra?: { altura: number; pendiente: number; humedad: number }): { densidad: number; tipo: TipoForestal; costeMovimiento: number } {
  const altura = muestra?.altura ?? evaluarElevacion(elevacion, p);
  if (altura <= 0.375 || elevacion.lagos?.some(l => puntoEnLago(l, p))) return { densidad: 0, tipo: 'abierto', costeMovimiento: 1 };
  const g = muestra ? undefined : gradienteElevacion(elevacion, p, 2);
  const pendiente = muestra?.pendiente ?? Math.hypot(g!.dx, g!.dy) * 1000;
  const humedad = muestra?.humedad ?? humedadRegionalEn(elevacion.regional!, p);
  let distancia = ALCANCE_RIBERA;
  for (const id of campo.indiceRiberas[`${Math.floor(p.x / CELDA_RIBERA)},${Math.floor(p.y / CELDA_RIBERA)}`] ?? []) {
    const { a, b } = campo.segmentos[id]!, dx = b.x - a.x, dy = b.y - a.y;
    const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
    distancia = Math.min(distancia, Math.hypot(p.x - a.x - t * dx, p.y - a.y - t * dy));
  }
  const ribera = 1 - suave(distancia / ALCANCE_RIBERA);
  const masa = evaluarRuido(campo.masas, p), detalle = evaluarRuido(campo.detalle, p);
  // El ruido amplio organiza las masas; el pequeño abre claros y deshace bordes.
  const potencial = humedad * 0.62 + (masa - 0.5) * 1.65 + (detalle - 0.5) * 0.65 + ribera * 0.38;
  const aptitud = suave((altura - 0.375) / 0.055) * (1 - suave((altura - 0.68) / 0.20))
    * (1 - suave((pendiente - 0.18) / 0.48));
  const densidad = suave((potencial - 0.24) / 0.38) * aptitud;
  const tipo: TipoForestal = ribera > 0.3 ? 'ribera' : altura > 0.55 ? 'montano' : humedad > 0.62 ? 'denso' : 'abierto';
  return { densidad, tipo, costeMovimiento: 1 + 0.8 * densidad * densidad };
}

/** Sectores sin solapamiento, solo para resumir superficie y potencial permanente.
 * No se dibujan sus bordes ni determinan la forma de las masas. Muestreo fijo de 100 m. */
export function resumirSectoresForestales(campo: CampoForestal, elevacion: CampoElevacion, ancho: number, alto: number): SectorForestal[] {
  const sectores: SectorForestal[] = [];
  for (let y = 0; y < alto; y += 400) for (let x = 0; x < ancho; x += 400) {
    const sector: SectorForestal = { id: `forestal-${x}-${y}`, x, y, ancho: Math.min(400, ancho - x), alto: Math.min(400, alto - y), superficieArbolada: 0, superficieEquivalente: 0, densidadMedia: 0 };
    for (let dy = 0; dy < sector.alto; dy += 100) for (let dx = 0; dx < sector.ancho; dx += 100) {
      const w = Math.min(100, sector.ancho - dx), h = Math.min(100, sector.alto - dy);
      const { densidad } = evaluarForestal(campo, elevacion, { x: x + dx + w / 2, y: y + dy + h / 2 });
      if (densidad >= 0.15) { sector.superficieArbolada += w * h; sector.superficieEquivalente += w * h * densidad; }
    }
    if (sector.superficieArbolada) { sector.densidadMedia = sector.superficieEquivalente / sector.superficieArbolada; sectores.push(sector); }
  }
  return sectores;
}
