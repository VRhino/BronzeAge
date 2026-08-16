import type { Point, TerrenoTipo } from '../domain/types';
import type { Limites } from './colocacion';
import { ELEVACION, ELEVACION_BORDE, ELEVACION_SUAVIZADO } from './config';
import { evaluarGuia, resolverRegion, REGIONES, type RegionId } from './regiones';
import { evaluarRuido, evaluarRuidoParcial, generarCampoRuido } from './ruido';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';

/** Curva cúbica de suavizado (mismo polinomio que `S()` en `regiones.ts` y `suavizar()` en `ruido.ts` — se
 * repite localmente en vez de compartirse porque cada módulo la usa sobre variables distintas). `x<=edge0`
 * da 0, `x>=edge1` da 1, con derivada nula en ambos extremos (sin quiebre visible en la pendiente). */
function smoothstep(edge0: number, edge1: number, x: number): number {
  const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

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
  const region = regionId ? resolverRegion(REGIONES[regionId], limites) : undefined;

  // Borde natural (Fase 0.4, ver `ELEVACION_BORDE` en `config.ts`): solo el mundo libre lo genera — las
  // regiones autoradas ya definen su propio borde a mano. Se genera DESPUÉS del ruido de elevación para no
  // desplazar su consumo de RNG (mismo criterio que `riosTroncales`), pero SÍ desplaza todo lo que venga
  // después en mundos sin región (ver `WORLDGEN_VERSION`).
  const borde = region
    ? undefined
    : {
        ruido: generarCampoRuido(rng, {
          octavas: ELEVACION_BORDE.octavas,
          frecuenciaBase: ELEVACION_BORDE.escala,
          lacunaridad: ELEVACION_BORDE.lacunaridad,
          persistencia: ELEVACION_BORDE.persistencia,
        }),
        limites,
      };

  return { ruido, region, borde };
}

/** Factor [0,1] de cercanía al borde del mapa: 1 justo en el borde, decae suavemente a 0 a `anchoFraccion`
 * del tamaño del mapa hacia adentro (ver `ELEVACION_BORDE`). */
function factorBorde(limites: Limites, p: Point): number {
  const distBorde = Math.min(p.x, limites.ancho - p.x, p.y, limites.alto - p.y);
  const anchoBorde = (ELEVACION_BORDE.anchoFraccion * (limites.ancho + limites.alto)) / 2;
  return 1 - smoothstep(0, anchoBorde, distBorde);
}

/** Peso [0,1] de vigencia del suavizado jugable: máximo en el centro de la banda llano/colina, decae a 0
 * según `e` se acerca a agua (por abajo) o a montaña/cima (por arriba) — así montaña/cima conservan el
 * relieve accidentado de siempre, y la transición desde costa/hacia montaña no tiene un cambio de textura
 * brusco sin relación con el resto del terreno. */
function bandaJugablePeso(e: number): number {
  const pesoBajo = smoothstep(ELEVACION.umbralAgua, ELEVACION.umbralCosta, e);
  const pesoAlto = 1 - smoothstep(ELEVACION.umbralMontana, ELEVACION.umbralCima, e);
  return pesoBajo * pesoAlto;
}

/**
 * Versión "suave" del relieve en `p` (Fase 0.4.2, ver `ELEVACION_SUAVIZADO` en `config.ts`): el MISMO ruido
 * de `campo`, pero evaluado con menos octavas (`evaluarRuidoParcial`) — conserva la ondulación ancha
 * (cientos de unidades) y descarta el detalle fino (decenas de unidades) que se leía accidentado a escala
 * de metros reales. A diferencia de cuantizar el valor (terraceo, retirado), el resultado sigue siendo
 * ruido de verdad: nunca perfectamente plano, así que el gradiente para ríos nunca es exactamente cero.
 *
 * Con región, se mezcla con la MISMA guía que usa `evaluarElevacion` — la guía ya es una forma suave por
 * construcción (crestas/bahías/cuencas analíticas, ver `regiones.ts`), no necesita su propia versión
 * reducida.
 */
function elevacionSuavizada(campo: CampoElevacion, p: Point): number {
  const ruidoSuave = evaluarRuidoParcial(campo.ruido, p, ELEVACION_SUAVIZADO.octavasSuaves);
  if (!campo.region) return ruidoSuave;
  const guia = evaluarGuia(campo.region, p);
  return guia * campo.region.pesoGuia + ruidoSuave * (1 - campo.region.pesoGuia);
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
  let e = campo.region ? evaluarGuia(campo.region, p) * campo.region.pesoGuia + ruido * (1 - campo.region.pesoGuia) : ruido;

  // Borde (Fase 0.4): empuja hacia costa profunda o montaña empinada cerca del extremo del mapa — ver
  // `factorBorde`/`ELEVACION_BORDE`. Mismo patrón que `cuenca`/`valle` en `regiones.ts` (blend hacia un
  // objetivo por un factor [0,1]), aplicado aquí sobre el borde en vez de un rasgo autorado.
  let fBorde = 0;
  if (campo.borde) {
    fBorde = factorBorde(campo.borde.limites, p);
    if (fBorde > 0) {
      const tipo = evaluarRuido(campo.borde.ruido, p);
      const objetivo = ELEVACION_BORDE.objetivoCosta + tipo * (ELEVACION_BORDE.objetivoMontana - ELEVACION_BORDE.objetivoCosta);
      e = e + (objetivo - e) * fBorde;
    }
  }

  // Suavizado jugable (Fase 0.4.2, ver `ELEVACION_SUAVIZADO`/`elevacionSuavizada`): mezcla hacia una versión
  // de menos octavas del mismo ruido, en vez de cuantizar en niveles (terraceo, retirado — ver
  // `Consideraciones/Fase_0_4_Definicion_Relieve_Jugable.md`). Se desactiva progresivamente dentro del borde
  // (`1 - fBorde`) por el mismo motivo que antes: la pared de montaña/costa del borde debe quedar tan
  // empinada como dicta `objetivo`, sin que el suavizado la atenúe de vuelta.
  const pesoSuave = bandaJugablePeso(e) * (1 - fBorde) * ELEVACION_SUAVIZADO.pesoMaximo;
  if (pesoSuave > 0) {
    const suave = elevacionSuavizada(campo, p);
    e = e + (suave - e) * pesoSuave;
  }

  return Math.min(1, Math.max(0, e));
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
