// Copia de `Mapa.contornosBosques()` (`src/world/mapa.ts` del backend) — NO de `worldgen/bosques.ts`, que
// genera las posiciones (consume RNG, server-only, igual que `generarCampoRuido`). Los bosques mismos
// (`ZonaBosque[]`: centro, radio, densidad) ya llegan como dato público en `GET .../mapa/:mapaId` desde C11a
// — lo único que faltaba aquí era fusionarlos en una silueta, que es geometría pura sobre datos ya públicos.
import { formaCirculo, unirFormas } from './poligonos';
import type { Point, ZonaBosque } from './tipos';

/** Debe coincidir con `PASO_FUSION_BOSQUES` en `src/world/mapa.ts` — controla la resolución de la rejilla de
 * fusión como fracción del ancho del mapa. Ver la nota de mantenimiento en el README de esta carpeta. */
const PASO_FUSION_BOSQUES = 1 / 400;

/**
 * Los bosques como UNA silueta: contorno de la unión de todos sus discos, en lazos cerrados. Los claros que
 * queden encerrados por una corona de bosques salen como lazos de orientación opuesta — dibujarlos todos en
 * un mismo `Path2D` con la regla de relleno `nonzero` (la de por defecto en canvas) los recorta solos.
 */
export function contornosBosques(bosques: readonly ZonaBosque[], anchoMapa: number): Point[][] {
  const paso = anchoMapa * PASO_FUSION_BOSQUES;
  return unirFormas(
    bosques.map((b) => formaCirculo(b.centro, b.radio)),
    { paso, tolerancia: paso * 0.4, areaMinima: paso * paso * 4 }
  );
}
