// Copia de `evaluarBioma` de `src/worldgen/biomas.ts` del backend.
import { BIOMA } from './config';
import { evaluarTerreno } from './elevacion';
import { evaluarFertilidad } from './fertilidad';
import { distanciaARioMasCercano } from './rios';
import type { BiomaTipo, CampoElevacion, CampoFertilidad, Point, RioZona } from './tipos';

export function evaluarBioma(elevacion: CampoElevacion, fertilidad: CampoFertilidad, rios: readonly RioZona[], p: Point): BiomaTipo {
  const terreno = evaluarTerreno(elevacion, p);
  if (terreno !== 'llano') return terreno;

  const fertil = evaluarFertilidad(fertilidad, p) >= BIOMA.umbralFertilLlanura;
  const humedo = distanciaARioMasCercano(rios, p) < BIOMA.radioHumedadRio;
  return fertil || humedo ? 'llanuraFertil' : 'estepa';
}
