import type { LagoZona, Point, RioZona } from './tipos';
import { distanciaASegmento, type Limites } from './colocacion';
import type { RioTroncalDef } from './regiones';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';
import { generarRiosPorDrenaje } from './drenaje';
import { evaluarElevacion } from './elevacion';

/** Puntos de paso por tramo (entre cada par de waypoints autorados) al densificar un río troncal. */
const TRONCAL_SUBDIVISIONES = 8;

/**
 * Construye la polilínea de un río troncal (Nilo, Tigris, Éufrates — ver `RioTroncalDef`) a partir de sus
 * puntos de paso YA RESUELTOS a unidades de mapa. A diferencia de los ríos de drenaje D8, sigue
 * el trazo autorado. `navegable` y `!terminaEnLago` son fijos: un río de esta
 * escala es navegable por diseño; su adecuación al relieve se verifica por separado.
 */
function generarRioTroncal(def: RioTroncalDef): RioZona {
  const puntos: Point[] = [];

  for (let tramo = 0; tramo < def.puntos.length - 1; tramo++) {
    const a = def.puntos[tramo]!;
    const b = def.puntos[tramo + 1]!;
    const dx = b.x - a.x;
    const dy = b.y - a.y;

    // `inicio`: el primer tramo incluye su extremo t=0, los siguientes lo omiten (ya lo puso el tramo
    // anterior como su t=1) — evita el punto duplicado en cada waypoint compartido.
    for (let paso = tramo === 0 ? 0 : 1; paso <= TRONCAL_SUBDIVISIONES; paso++) {
      const t = paso / TRONCAL_SUBDIVISIONES;
      puntos.push({ x: a.x + dx * t, y: a.y + dy * t });
    }
  }

  return { id: def.id, puntos, fin: 'troncal', terminaEnLago: false, navegable: true };
}

/** Ruta activa de v16: drenaje D8 para ríos procedurales; troncales regionales
 * autorados conservan su trazado y se añaden sin afectar el análisis de cuencas. */
export function generarRios(rng: RandomFn, limites: Limites, elevacion: CampoElevacion): { rios: RioZona[]; lagos: LagoZona[] } {
  const troncales = (elevacion.region?.riosTroncales ?? []).map((def) => generarRioTroncal(def));
  const base = { ...elevacion, caucesTroncales: [] };
  elevacion.caucesTroncales = troncales.map((rio) => {
    let anterior = Infinity;
    const cotas = rio.puntos.map((p) => {
      anterior = Math.min(evaluarElevacion(base, p) - 0.012, anterior - 0.0005);
      return anterior;
    });
    const xs = rio.puntos.map((p) => p.x), ys = rio.puntos.map((p) => p.y);
    return { puntos: rio.puntos, cotas, radio: Math.min(limites.ancho, limites.alto) * 0.02,
      caja: { minX: Math.min(...xs), maxX: Math.max(...xs), minY: Math.min(...ys), maxY: Math.max(...ys) } };
  });
  // Los corredores del Tigris y Éufrates se solapan cerca de la salida. Una
  // cota más baja del vecino puede crear un pequeño ascenso en la propia línea.
  // Relajar aguas abajo en un máximo finito de pasadas conserva el descenso
  // de ambos sin alterar la forma fuera de sus corredores.
  let estabilizado = false;
  for (let pasada = 0; pasada < 8; pasada++) {
    let correcciones = 0;
    for (let r = 0; r < troncales.length; r++) {
      const puntos = troncales[r]!.puntos;
      const cotas = elevacion.caucesTroncales[r]!.cotas;
      let anterior = evaluarElevacion(elevacion, puntos[0]!);
      for (let i = 1; i < puntos.length; i++) {
        const actual = evaluarElevacion(elevacion, puntos[i]!);
        if (actual > anterior - 0.0001) {
          const rebaja = actual - (anterior - 0.0001);
          for (let j = i; j < cotas.length; j++) cotas[j]! -= rebaja;
          correcciones++;
        }
        anterior = evaluarElevacion(elevacion, puntos[i]!);
      }
    }
    if (correcciones === 0) { estabilizado = true; break; }
  }
  if (!estabilizado) throw new Error('No converge la cota de los ríos troncales');
  const resultado = generarRiosPorDrenaje(rng, limites, elevacion);
  return { rios: [...resultado.rios, ...troncales], lagos: resultado.lagos };
}

/**
 * Distancia mínima de un punto a cualquier segmento de cualquier río — proxy de humedad para `evaluarBioma`.
 * O(ríos × puntos), usada por fertilidad y biomas.
 */
export function distanciaARioMasCercano(rios: readonly RioZona[], p: Point): number {
  let minimo = Infinity;
  for (const rio of rios) {
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const d = distanciaASegmento(p, rio.puntos[i]!, rio.puntos[i + 1]!);
      if (d < minimo) minimo = d;
    }
  }
  return minimo;
}
