// Copia de `evaluarFertilidad` de `src/worldgen/fertilidad.ts` del backend. Sin limitaciones: la fertilidad
// no depende de región.
import { evaluarRuido } from './ruido';
import type { CampoFertilidad, Point } from './tipos';

export function evaluarFertilidad(campo: CampoFertilidad, p: Point): number {
  return evaluarRuido(campo, p);
}
