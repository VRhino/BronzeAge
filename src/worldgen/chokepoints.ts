// Chokepoints estratégicos (Fase 0.3, Doc 1.5 `1_Sistema_de_Mundo_y_Territorio.md`): puertos de montaña,
// detectados como PUNTOS DE SILLA del campo de elevación — geometría (puntos), no aristas de un grafo, ver
// `Fase_0_1_Definicion.md`. Vados de río quedan explícitamente fuera de esta pasada (ver
// `Preguntas_Abiertas.md`), solo pasos de montaña.
//
// Un punto de silla es, en la intuición geográfica, exactamente un puerto de montaña: mínimo local a lo
// largo de la cresta (el camino de menor esfuerzo entre dos picos) y máximo local en la dirección
// perpendicular (el terreno sube a ambos lados). El determinante del Hessiano discreto es negativo
// exactamente en ese caso — y, a diferencia de mirar `exx`/`eyy` por separado, es invariante a la
// orientación de la cordillera respecto a los ejes X/Y del mapa.

import type { Chokepoint, Point } from '../domain/types';
import { CHOKEPOINTS } from './config';
import { colocarConEspaciado, type Limites } from './colocacion';
import { evaluarElevacion, evaluarTerreno } from './elevacion';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';

/** Hessiano discreto de `evaluarElevacion` en `p` por diferencias finitas centradas (mismo criterio que
 * `gradienteElevacion`). Devuelve las dos curvaturas principales (`exx`, `eyy`) y la cruzada (`exy`) — el
 * determinante `exx·eyy − exy²` es negativo exactamente en un punto de silla, sea cual sea la orientación
 * de la cordillera. */
function hessianoElevacion(campo: CampoElevacion, p: Point, paso: number): { exx: number; eyy: number; exy: number } {
  const e0 = evaluarElevacion(campo, p);
  const eXp = evaluarElevacion(campo, { x: p.x + paso, y: p.y });
  const eXm = evaluarElevacion(campo, { x: p.x - paso, y: p.y });
  const eYp = evaluarElevacion(campo, { x: p.x, y: p.y + paso });
  const eYm = evaluarElevacion(campo, { x: p.x, y: p.y - paso });
  const ePP = evaluarElevacion(campo, { x: p.x + paso, y: p.y + paso });
  const eMM = evaluarElevacion(campo, { x: p.x - paso, y: p.y - paso });
  const ePM = evaluarElevacion(campo, { x: p.x + paso, y: p.y - paso });
  const eMP = evaluarElevacion(campo, { x: p.x - paso, y: p.y + paso });

  const paso2 = paso * paso;
  return {
    exx: (eXp - 2 * e0 + eXm) / paso2,
    eyy: (eYp - 2 * e0 + eYm) / paso2,
    exy: (ePP - ePM - eMP + eMM) / (4 * paso2),
  };
}

/** `true` si `p` es un punto de silla pronunciado (puerto de montaña real, no ruido de redondeo — ver
 * `CHOKEPOINTS.curvaturaMinima`). */
function esPuertoDeMontana(campo: CampoElevacion, p: Point): boolean {
  const { exx, eyy, exy } = hessianoElevacion(campo, p, CHOKEPOINTS.pasoHessiano);
  const determinante = exx * eyy - exy * exy;
  if (determinante >= 0) return false;
  return Math.abs(exx) >= CHOKEPOINTS.curvaturaMinima || Math.abs(eyy) >= CHOKEPOINTS.curvaturaMinima;
}

/**
 * Chokepoints del mundo (Fase 0.3): candidatos por rejection-sampling (`colocarConEspaciado`, mismo patrón
 * que ríos/bosques/nodos) sobre terreno 'colina'/'montana' (no 'cima': inhabitable, ver
 * `ELEVACION.umbralCima`), filtrados por el test de punto de silla. Único consumo de RNG: la colocación de
 * candidatos — el test de silla en sí es puramente determinista sobre `elevacion`.
 *
 * Restringido a solo 'montana' brevemente en v13 (Fase 0.4.1, terraceo): 'colina' quedaba como meseta
 * CONSTANTE, sin curvatura real. El terraceo se retiró en Fase 0.4.2 (ver
 * `Consideraciones/Fase_0_4_Definicion_Relieve_Jugable.md`) a favor de un suavizado que SIEMPRE conserva
 * ondulación real (nunca aplana del todo) — 'colina' vuelve a tener curvatura de sobra para el test de silla
 * (medido: 14/14 chokepoints en colina/montana en las 3 seeds de referencia, frente al 65-100% del terraceo),
 * así que se restaura el filtro original.
 */
export function generarChokepoints(rng: RandomFn, limites: Limites, elevacion: CampoElevacion): Chokepoint[] {
  const puntos = colocarConEspaciado(rng, limites, CHOKEPOINTS.cantidad, CHOKEPOINTS.espacioMinimo, [], [], (p) => {
    const terreno = evaluarTerreno(elevacion, p);
    if (terreno !== 'colina' && terreno !== 'montana') return false;
    return esPuertoDeMontana(elevacion, p);
  });

  return puntos.map((posicion, i) => ({ id: `chokepoint-${i}`, posicion, radio: CHOKEPOINTS.radio }));
}
