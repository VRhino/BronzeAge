import type { Point, TerrenoTipo, LagoZona } from './tipos';
import { ELEVACION } from './config';
import { evaluarRelieveRegional, type RelieveRegional } from './regional';
import { puntoEnLago } from './aguaRegional';
import { aplicarCostaRegional, type CampoCostero } from './costasRegional';
import { aplicarDetalle, type DetalleRelieve } from './relieveDetalle';

/** Cauce principal autorado que talla un valle en el relieve. */
export interface CauceTroncal { puntos: { x: number; y: number }[]; cotas: number[]; radio: number;
  caja: { minX: number; maxX: number; minY: number; maxY: number } }

/** Campo continuo de relieve: datos puros, se consulta con `evaluarElevacion`/`evaluarTerreno`. */
export interface CampoElevacion {
  regional: RelieveRegional;
  costas?: CampoCostero;
  lagos?: LagoZona[];
  caucesTroncales?: CauceTroncal[];
  /** Formas a escala de pantalla encima del relieve regional (se añade al final, con ríos, lagos y costas ya hechos). */
  detalle?: DetalleRelieve;
}

/** Curva cúbica de suavizado: 0 en `edge0`, 1 en `edge1`, con derivada nula en ambos extremos. */
function smoothstep(edge0: number, edge1: number, x: number): number {
  const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

/** Elevación en un punto cualquiera, normalizada 0-1. Función pura, definida en todo el plano. */
export function evaluarElevacion(campo: CampoElevacion, p: Point): number {
  let e = evaluarRelieveRegional(campo.regional, p);
  for (const valle of campo.caucesTroncales ?? []) {
    if (p.x < valle.caja.minX - valle.radio || p.x > valle.caja.maxX + valle.radio
      || p.y < valle.caja.minY - valle.radio || p.y > valle.caja.maxY + valle.radio) continue;
    let distancia = Infinity, cota = e;
    for (let i = 1; i < valle.puntos.length; i++) {
      const a = valle.puntos[i - 1]!, b = valle.puntos[i]!, dx = b.x - a.x, dy = b.y - a.y;
      const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)));
      const d = Math.hypot(p.x - a.x - t * dx, p.y - a.y - t * dy);
      if (d < distancia) { distancia = d; cota = valle.cotas[i - 1]! * (1 - t) + valle.cotas[i]! * t; }
    }
    if (e >= 0.375 && distancia < valle.radio) e += (cota - e) * smoothstep(0, 1, 1 - distancia / valle.radio);
  }
  for (const lago of campo.lagos ?? []) {
    if (lago.contorno) { if (puntoEnLago(lago, p)) return lago.elevacionCentro; continue; }
    const d = Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y);
    if (d < lago.radio) e = Math.min(e, e + (lago.elevacionCentro - e) * smoothstep(0, 1, 1 - d / lago.radio));
  }
  if (campo.detalle) e = aplicarDetalle(campo.detalle, p, e);
  return campo.costas ? aplicarCostaRegional(campo.costas, p, e) : e;
}

/**
 * Gradiente de elevación por diferencias finitas centradas — única forma de "pendiente" posible sobre un
 * campo sin derivada analítica. `paso` en unidades de mapa: cuanto mayor, más se promedian los detalles
 * finos y más se ve solo la forma general del relieve.
 */
export function gradienteElevacion(campo: CampoElevacion, p: Point, paso: number): { dx: number; dy: number } {
  const dx = (evaluarElevacion(campo, { x: p.x + paso, y: p.y }) - evaluarElevacion(campo, { x: p.x - paso, y: p.y })) / (2 * paso);
  const dy = (evaluarElevacion(campo, { x: p.x, y: p.y + paso }) - evaluarElevacion(campo, { x: p.x, y: p.y - paso })) / (2 * paso);
  return { dx, dy };
}

/** Clasificación por umbral en bandas de relieve; los lagos son agua. Pura y definida en todo el plano.
 * `altura`, si ya se tiene la de `p`, evita recalcularla. */
export function evaluarTerreno(campo: CampoElevacion, p: Point, altura?: number): TerrenoTipo {
  for (const lago of campo.lagos ?? []) {
    if (puntoEnLago(lago, p)) return 'agua';
  }
  const e = altura ?? evaluarElevacion(campo, p);
  if (e < ELEVACION.umbralAgua) return 'agua';
  if (e < ELEVACION.umbralCosta) return 'costa';
  if (e < ELEVACION.umbralColina) return 'llano';
  if (e < ELEVACION.umbralMontana) return 'colina';
  if (e < ELEVACION.umbralCima) return 'montana';
  return 'cima';
}
