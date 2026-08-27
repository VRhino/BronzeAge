// Punto de entrada del módulo de terreno — ver README.md de esta carpeta para qué es y por qué existe.
export { evaluarElevacion, evaluarTerreno } from './elevacion';
export { evaluarFertilidad } from './fertilidad';
export { evaluarBioma } from './biomas';
export { contornosBosques } from './bosques';
export type { BiomaTipo, MapaGenerado, Point, TerrenoTipo } from './tipos';

import type { BiomaTipo } from './tipos';

/** Paleta de presentación — no viene del servidor (`BiomaTipo` es un `enum` de dominio, no lleva color). Cada
 * cliente decide su propia paleta; esta es solo la del boilerplate. */
export function colorDeBioma(bioma: BiomaTipo): string {
  switch (bioma) {
    case 'agua':
      return '#2b6cb0';
    case 'costa':
      return '#7ec8e3';
    case 'estepa':
      return '#c2b280';
    case 'llanuraFertil':
      return '#6b8e23';
    case 'colina':
      return '#a67c52';
    case 'montana':
      return '#7d7d7d';
    case 'cima':
      return '#f5f5f5';
  }
}
