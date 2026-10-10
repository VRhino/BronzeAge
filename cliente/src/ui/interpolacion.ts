// Movimiento fluido de lo móvil entre dos deltas del servidor (Docs/Coordinacion/03_Tiempo_real_por_segmentos.md §4.1): el mundo avanza
// a saltos de un tick, el mapa se pinta a 24 fps y, entre un delta y el siguiente, ejércitos y caravanas siguen su `ruta` con el
// `avancePorTick` del último tick. No pide nada al servidor. PURO.
import type { Point } from '@motor/domain/types';

interface Movil {
  posicionActual: Point;
  ruta?: Point[];
  progreso?: number;
  avancePorTick?: number;
}

/** Un punto a `progreso` (0‥1) de la longitud de una polilínea. */
export function puntoEnRuta(ruta: readonly Point[], progreso: number): Point {
  const p = Math.min(1, Math.max(0, progreso));
  let largo = 0;
  for (let i = 1; i < ruta.length; i++) largo += Math.hypot(ruta[i]!.x - ruta[i - 1]!.x, ruta[i]!.y - ruta[i - 1]!.y);
  if (largo === 0) return ruta[0]!;
  let objetivo = p * largo;
  for (let i = 1; i < ruta.length; i++) {
    const a = ruta[i - 1]!;
    const b = ruta[i]!;
    const tramo = Math.hypot(b.x - a.x, b.y - a.y);
    if (objetivo <= tramo) {
      const t = tramo === 0 ? 0 : objetivo / tramo;
      return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
    }
    objetivo -= tramo;
  }
  return ruta[ruta.length - 1]!;
}

/** Cuánto se aleja un móvil de donde dice su ruta para fiarse de ella (unidades de mapa): si no cuadra, se pinta donde está y no se mueve. */
const TOLERANCIA_RUTA = 2;

/** `fraccion`: 0‥1, cuánto del tick siguiente ha pasado ya (capado: si el delta se retrasa, se detienen en vez de adelantarse sin fin).
 * Devuelve el MISMO array si nada se mueve, para que quien llama sepa que no hay animación. */
export function interpolar<T extends Movil>(moviles: T[], fraccion: number): T[] {
  const f = Math.min(1, Math.max(0, fraccion));
  let cambio = false;
  const salida = moviles.map((m) => {
    if (!m.ruta || m.ruta.length < 2 || m.progreso === undefined || !m.avancePorTick) return m;
    const sobreRuta = puntoEnRuta(m.ruta, m.progreso);
    if (Math.hypot(sobreRuta.x - m.posicionActual.x, sobreRuta.y - m.posicionActual.y) > TOLERANCIA_RUTA) return m;
    cambio = true;
    const visto = Math.min(1, Math.max(0, m.progreso + m.avancePorTick * f));
    return { ...m, progreso: visto, posicionActual: puntoEnRuta(m.ruta, visto) };
  });
  return cambio ? salida : moviles;
}
