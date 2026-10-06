// Emblemas del sigilo de Facción: iconos de game-icons.net (CC BY 3.0, autores en `CREDITOS.md`), elegidos por su
// parecido con los símbolos documentados de cada civilización (backend `Identidad_Visual_Definicion.md` §7).
// `emblemas.json` guarda el trazado de cada uno en un lienzo de 512×512; para cambiar un icono, ver `CREDITOS.md`.
import emblemas from './emblemas.json';

export interface Emblema {
  /** Autor en game-icons.net: hay que citarlo (CC BY 3.0). */
  autor: string;
  icono: string;
  d: string;
}

export const EMBLEMAS: Record<string, Emblema> = emblemas;

/**
 * El emblema como grupo SVG de 100×100 (origen arriba a la izquierda). Se pinta dos veces: un halo (que contrasta con
 * el color del emblema) y encima el relleno, para que se lea sobre cualquier campo. Vacío si el id no tiene icono.
 */
export function svgEmblema(emblemaId: string, relleno = '#f4efe4', halo = '#1c1b1a'): string {
  const e = EMBLEMAS[emblemaId];
  if (!e) return '';
  return `<g transform="scale(${100 / 512})">`
    + `<path d="${e.d}" fill="${halo}" stroke="${halo}" stroke-width="36" stroke-linejoin="round"/>`
    + `<path d="${e.d}" fill="${relleno}"/></g>`;
}
