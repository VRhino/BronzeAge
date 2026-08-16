import type { Asentamiento, Point, ZonaInfluencia } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { SITIO, ZONA_INFLUENCIA } from '../constants';
import { pointInPolygon } from '../world/geometria';

// `pointInPolygon` es geometría pura y vive en `world/geometria.ts` (la usa también la fachada `Mapa`);
// se reexporta desde aquí porque el motor la consume históricamente por esta vía.
export { pointInPolygon };

function circlePolygon(center: Point, radius: number, segments: number): Point[] {
  const pts: Point[] = [];
  for (let i = 0; i < segments; i++) {
    const angle = (i / segments) * Math.PI * 2;
    pts.push({ x: center.x + Math.cos(angle) * radius, y: center.y + Math.sin(angle) * radius });
  }
  return pts;
}

/** Sutherland-Hodgman: recorta `polygon` quedándose con el semiplano donde signedDistance(p) >= 0. */
function clipByHalfPlane(polygon: Point[], planePoint: Point, insideNormal: Point): Point[] {
  if (polygon.length === 0) return polygon;
  const signedDist = (p: Point) => (p.x - planePoint.x) * insideNormal.x + (p.y - planePoint.y) * insideNormal.y;

  const output: Point[] = [];
  for (let i = 0; i < polygon.length; i++) {
    const current = polygon[i]!;
    const previous = polygon[(i - 1 + polygon.length) % polygon.length]!;
    const currentInside = signedDist(current) >= 0;
    const previousInside = signedDist(previous) >= 0;

    if (currentInside !== previousInside) {
      const dPrev = signedDist(previous);
      const dCurr = signedDist(current);
      const t = dPrev / (dPrev - dCurr);
      output.push({ x: previous.x + t * (current.x - previous.x), y: previous.y + t * (current.y - previous.y) });
    }
    if (currentInside) output.push(current);
  }
  return output;
}

/**
 * Frontera "viva": el punto de corte entre dos zonas se reparte proporcionalmente al radio potencial
 * actual de cada asentamiento (poder relativo), no queda fijo tras el primer contacto — ver Doc 1.2.
 */
function computeBorderClip(propio: Asentamiento, rival: Asentamiento): { planePoint: Point; insideNormal: Point } {
  const dx = rival.posicion.x - propio.posicion.x;
  const dy = rival.posicion.y - propio.posicion.y;
  const dist = Math.hypot(dx, dy) || 1e-6;
  const dirX = dx / dist;
  const dirY = dy / dist;

  const proporcionPropio = propio.radioPotencial / (propio.radioPotencial + rival.radioPotencial);
  const distanciaFrontera = dist * proporcionPropio;

  const planePoint: Point = {
    x: propio.posicion.x + dirX * distanciaFrontera,
    y: propio.posicion.y + dirY * distanciaFrontera,
  };
  // Normal "hacia adentro" = hacia el propio centro (opuesta a la dirección hacia el rival).
  const insideNormal: Point = { x: -dirX, y: -dirY };
  return { planePoint, insideNormal };
}

/**
 * Zona de influencia resultante = círculo de radio potencial recortado contra las fronteras duras
 * con TODOS los asentamientos de OTRA facción (mismo bando se permite solapar/fusionar, no se recorta).
 */
export function computeZonaInfluencia(
  asentamiento: Asentamiento,
  todos: Asentamiento[]
): ZonaInfluencia {
  let poligono = circlePolygon(asentamiento.posicion, asentamiento.radioPotencial, ZONA_INFLUENCIA.segmentosPoligono);

  for (const otro of todos) {
    if (otro.id === asentamiento.id) continue;
    if (otro.faccionId === asentamiento.faccionId) continue;
    const { planePoint, insideNormal } = computeBorderClip(asentamiento, otro);
    poligono = clipByHalfPlane(poligono, planePoint, insideNormal);
    if (poligono.length === 0) break;
  }

  return { asentamientoId: asentamiento.id, poligono };
}

export function computeTodasLasZonas(asentamientos: Asentamiento[]): ZonaInfluencia[] {
  return asentamientos.map((a) => computeZonaInfluencia(a, asentamientos));
}

/** Un punto está libre para fundar si no cae dentro de la zona de influencia ya recortada de ningún asentamiento existente. */
export function posicionLibreParaFundar(p: Point, asentamientos: Asentamiento[]): boolean {
  const zonas = computeTodasLasZonas(asentamientos);
  return zonas.every((z) => !pointInPolygon(p, z.poligono));
}

/**
 * Fertilidad de REFERENCIA de un asentamiento para sus Granjas (Vista de Asentamiento, a petición del
 * usuario). Las Granjas se construyen DENTRO del espacio plano del asentamiento (sin campo de fertilidad
 * propio), pero rinden como si estuvieran sobre el mejor suelo que la zona de influencia toca en el mapa
 * general: se muestrea el radio potencial (mismo patrón que la antigua colocación por fertilidad),
 * quedándose con la fertilidad MÁS ALTA de los puntos que caen dentro del polígono de zona. Ese único valor
 * lo comparten TODAS las Granjas del asentamiento (`avanzarConstruccion`/`produccionPorTick`).
 *
 * `zonaPoligono` vacío (sin zona calculada) o sin ningún punto dentro cae al valor del propio centro —
 * nunca devuelve 0 por falta de muestras.
 */
export function mejorFertilidadEnZona(asentamiento: Asentamiento, zonaPoligono: Point[], mapa: Mapa): number {
  const candidatos: Point[] = [asentamiento.posicion];
  for (let i = 0; i < SITIO.muestrasFertilidad; i++) {
    const angulo = (i / SITIO.muestrasFertilidad) * Math.PI * 2;
    const radio = ((i % 5) / 5) * asentamiento.radioPotencial;
    const p: Point = {
      x: asentamiento.posicion.x + Math.cos(angulo) * radio,
      y: asentamiento.posicion.y + Math.sin(angulo) * radio,
    };
    if (zonaPoligono.length === 0 || pointInPolygon(p, zonaPoligono)) candidatos.push(p);
  }
  return mapa.mejorPorFertilidad(candidatos)?.fertilidad ?? mapa.fertilidadEn(asentamiento.posicion);
}
