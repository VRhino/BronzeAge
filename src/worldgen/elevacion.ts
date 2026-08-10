import type { Point, TerrenoTipo } from '../domain/types';
import type { Limites } from './colocacion';
import { ELEVACION } from './config';
import { evaluarGuia, resolverRegion, REGIONES, type RegionId } from './regiones';
import { evaluarRuido, generarCampoRuido } from './ruido';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';

/**
 * Campo de elevación continuo (0-1): ruido fractal de gradiente (ver `ruido.ts`), opcionalmente mezclado con
 * la guía geográfica de una región real (Fase 0.2, ver `regiones.ts`). Devuelve los PARÁMETROS del campo, no
 * una función ya cerrada sobre ellos, para que sea serializable/comparable y muestreable a cualquier
 * resolución — es literalmente un heightmap en la forma que menos fricción da hacia un terreno 3D futuro
 * (ver `Fase_0_1_Definicion.md`).
 *
 * `regionId`: `undefined` (por defecto) reproduce el mundo libre de siempre byte a byte — `evaluarElevacion`
 * ni siquiera consulta `region` en ese caso. Con una región, su guía (fracciones [0,1]) se RESUELVE una sola
 * vez aquí, a las unidades de mapa de `limites` — así el resto del pipeline (`evaluarElevacion`,
 * `evaluarTerreno`, `gradienteElevacion`) no necesita saber que existe.
 */
export function generarCampoElevacion(rng: RandomFn, limites: Limites, regionId?: RegionId): CampoElevacion {
  const ruido = generarCampoRuido(rng, {
    octavas: ELEVACION.octavas,
    frecuenciaBase: ELEVACION.escala,
    lacunaridad: ELEVACION.lacunaridad,
    persistencia: ELEVACION.persistencia,
  });
  return { ruido, region: regionId ? resolverRegion(REGIONES[regionId], limites) : undefined };
}

/**
 * Elevación en un punto cualquiera, normalizada 0-1. Función PURA, definida en todo el plano — igual que
 * `evaluarFertilidad`, el muestreo en anillo (colocación de nacimientos de río, bosques, nodos) puede
 * salirse del borde del mapa.
 *
 * Sin región: el ruido manda solo (comportamiento anterior a Fase 0.2, sin cambios). Con región: se mezcla
 * `guía·pesoGuia + ruido·(1-pesoGuia)` — la guía da la silueta reconocible, el ruido sigue aportando el
 * detalle/variedad entre seeds de una misma región (ver `RegionGeografica.pesoGuia`).
 */
export function evaluarElevacion(campo: CampoElevacion, p: Point): number {
  const ruido = evaluarRuido(campo.ruido, p);
  if (!campo.region) return ruido;
  const guia = evaluarGuia(campo.region, p);
  return guia * campo.region.pesoGuia + ruido * (1 - campo.region.pesoGuia);
}

/**
 * Gradiente de elevación por diferencias finitas centradas — única forma de "pendiente" posible sobre un
 * campo sin derivada analítica. `paso` en unidades de mapa: cuanto mayor, más se promedian los detalles
 * finos y más se ve solo la forma general del relieve. Lo usa `generarRios` para descender por máxima
 * pendiente, y ahí ese promediado es justo lo que se quiere (ver `RIOS.pasoGradiente`).
 */
export function gradienteElevacion(campo: CampoElevacion, p: Point, paso: number): { dx: number; dy: number } {
  const dx = (evaluarElevacion(campo, { x: p.x + paso, y: p.y }) - evaluarElevacion(campo, { x: p.x - paso, y: p.y })) / (2 * paso);
  const dy = (evaluarElevacion(campo, { x: p.x, y: p.y + paso }) - evaluarElevacion(campo, { x: p.x, y: p.y - paso })) / (2 * paso);
  return { dx, dy };
}

/**
 * Clasificación por umbral en bandas de relieve (Fase 0.1). Pura y definida en todo el plano, igual que
 * `evaluarElevacion` — importa para el rejection-sampling de bosques/ríos/nodos, que muestrean fuera del
 * mapa.
 */
export function evaluarTerreno(campo: CampoElevacion, p: Point): TerrenoTipo {
  const e = evaluarElevacion(campo, p);
  if (e < ELEVACION.umbralAgua) return 'agua';
  if (e < ELEVACION.umbralCosta) return 'costa';
  if (e < ELEVACION.umbralColina) return 'llano';
  if (e < ELEVACION.umbralMontana) return 'colina';
  if (e < ELEVACION.umbralCima) return 'montana';
  return 'cima';
}
