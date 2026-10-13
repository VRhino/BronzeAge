import type { BiomaTipo, Point, RioZona } from './tipos';
import { BIOMA } from './config';
import { distanciaARioMasCercano } from './rios';
import { evaluarTerreno } from './elevacion';
import { evaluarFertilidadEfectiva } from './fertilidad';
import type { CampoElevacion, CampoFertilidad } from './types';

/**
 * Bioma en un punto cualquiera (Fase 0.1): terreno + fertilidad + humedad (proxy = cercanía a río). Pura,
 * sin RNG, no es un paso del pipeline — se deriva bajo demanda de campos ya generados, igual que `terrenoEn`
 * (ver `Fase_0_1_Definicion.md`). Nunca se guarda un campo de bioma.
 */
export function evaluarBioma(
  elevacion: CampoElevacion,
  fertilidad: CampoFertilidad,
  rios: readonly RioZona[],
  p: Point
): BiomaTipo {
  const terreno = evaluarTerreno(elevacion, p);
  if (terreno !== 'llano') return terreno;

  const fertil = evaluarFertilidadEfectiva(fertilidad, rios, elevacion.lagos ?? [], p) >= BIOMA.umbralFertilLlanura;
  const humedo = distanciaARioMasCercano(rios, p) < BIOMA.radioHumedadRio
    || (elevacion.lagos ?? []).some((lago) => Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y)
      < lago.radio + BIOMA.radioHumedadRio);
  return fertil || humedo ? 'llanuraFertil' : 'estepa';
}
