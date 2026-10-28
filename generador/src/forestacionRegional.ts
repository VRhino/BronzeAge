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
  masas: CampoRuido; detalle: CampoRuido; claros: CampoRuido; bosquetes: CampoRuido;
  segmentos: Segmento[]; indiceRiberas: Record<string, number[]>;
}
export interface SectorForestal {
  id: string; x: number; y: number; ancho: number; alto: number;
  superficieArbolada: number; densidadMedia: number; superficieEquivalente: number;
}
const suave = (x: number) => { const t = Math.max(0, Math.min(1, x)); return t * t * (3 - 2 * t); };
const CELDA_RIBERA = 500, ALCANCE_RIBERA = 260;

/** Forma de la densidad (2026-10-28). Por encima de `codo` la subida hacia el bosque cerrado se estira
 * (`estiramiento`): la mitad del bosque denso pasa a ligero sin que cambie dónde hay bosque. Los `claros`
 * son un ruido propio (no altera las masas) que, donde supera `umbral`, baja la densidad hasta `fondo`
 * de lo que había: claros fundables dentro de las masas grandes, uno cada ~30 u. */
export const PARAMETROS_FORESTALES = {
  codo: 0.43, estiramiento: 2.4,
  claros: { frecuencia: 1 / 220, umbral: 0.66, ancho: 0.06, fondo: 0.35 },
  /** Bosquetes sueltos (2026-10-28): otro ruido propio cuyos picos ponen manchas pequeñas de bosque ligero (hasta `densidad`) en toda
   * la tierra apta, para que las zonas peladas tengan algo de madera. Dentro de un bosque no cambian nada: vale el mayor de los dos. */
  bosquetes: { frecuencia: 1 / 240, umbral: 0.72, ancho: 0.05, densidad: 0.45 },
};

/** Campo inmutable, serializable e independiente de la resolución del visor.
 * La madera es permanente: no hay stock, agotamiento ni recuperación. */
export function crearCampoForestal(seed: number, rios: readonly RioZona[]): CampoForestal {
  const rng = createRng(derivarSeed(seed, 'forestacion-regional-1'));
  const ruido = (frecuenciaBase: number, octavas: number) => generarCampoRuido(rng,
    { frecuenciaBase, octavas, lacunaridad: 2.13, persistencia: 0.52 });
  const campo: CampoForestal = { masas: ruido(1 / 4300, 3), detalle: ruido(1 / 720, 3), claros: ruidoPropio(seed, 'forestacion-claros-1', PARAMETROS_FORESTALES.claros.frecuencia), bosquetes: ruidoPropio(seed, 'forestacion-bosquetes-1', PARAMETROS_FORESTALES.bosquetes.frecuencia), segmentos: [], indiceRiberas: {} };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, id = campo.segmentos.length;
    campo.segmentos.push({ a, b });
    for (let y = Math.floor((Math.min(a.y, b.y) - ALCANCE_RIBERA) / CELDA_RIBERA); y <= Math.floor((Math.max(a.y, b.y) + ALCANCE_RIBERA) / CELDA_RIBERA); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - ALCANCE_RIBERA) / CELDA_RIBERA); x <= Math.floor((Math.max(a.x, b.x) + ALCANCE_RIBERA) / CELDA_RIBERA); x++)
        (campo.indiceRiberas[`${x},${y}`] ??= []).push(id);
  }
  return campo;
}

// Generador aparte para que los claros y los bosquetes no muevan la tirada de las masas ni del detalle.
function ruidoPropio(seed: number, sal: string, frecuenciaBase: number): CampoRuido {
  return generarCampoRuido(createRng(derivarSeed(seed, sal)), { frecuenciaBase, octavas: 2, lacunaridad: 2.13, persistencia: 0.45 });
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
  const { codo, estiramiento, claros, bosquetes } = PARAMETROS_FORESTALES;
  const t = (potencial - 0.24) / 0.38, claro = suave((evaluarRuido(campo.claros, p) - claros.umbral) / claros.ancho);
  const bosquete = bosquetes.densidad * suave((evaluarRuido(campo.bosquetes, p) - bosquetes.umbral) / bosquetes.ancho) * aptitud;
  const densidad = Math.max(suave(t <= codo ? t : codo + (t - codo) / estiramiento) * aptitud * (1 - (1 - claros.fondo) * claro), bosquete);
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
