// Movimiento de caravanas sobre una polilínea con coste de terreno (Fase 0.3) — generaliza la interpolación
// lineal recta que `trade.ts`/`expansion.ts` implementaban cada uno por su lado (duplicada, sin tocar el
// terreno para nada). `progreso` (0→1) sigue significando "fracción del trayecto completada" — el contrato
// externo no cambia, así que nada que ya lea `caravana.progreso` (gates de llegada, UI) necesita tocarse.
// Lo que cambia es CÓMO se mide esa fracción: sobre la longitud real de `Caravana.ruta` (calculada una vez
// al lanzar la caravana, ver `world/rutas.ts`) en vez de la recta origen→destino, y con un avance por tick
// que depende del coste de terreno en la posición actual (`Mapa.costeEnPunto`) — cruzar colina/montaña de
// verdad cuesta más ticks, sin recalibrar la "velocidad base" ya existente por tipo de caravana
// (`CARAVANA_CATALOGO`).
//
// Caravanas sin `ruta` (partidas guardadas antes de Fase 0.3) NO pasan por aquí — quien las mueve sigue
// usando la fórmula de línea recta de siempre, sin coste de terreno, para no cambiarles el comportamiento.

import type { Point } from '../domain/types';
import { claveArista } from '../world/grafoNavegacion';
import type { Mapa } from '../world/mapa';
import { COSTE_MOVIMIENTO } from '../worldgen/config';

/** Longitud total de una polilínea, sumando cada segmento — mismo cálculo que `longitudRio` en
 * `worldgen/rios.ts`, de uso general aquí (rutas de caravana, no solo ríos). */
export function longitudPolilinea(puntos: readonly Point[]): number {
  let total = 0;
  for (let i = 0; i < puntos.length - 1; i++) {
    total += Math.hypot(puntos[i + 1]!.x - puntos[i]!.x, puntos[i + 1]!.y - puntos[i]!.y);
  }
  return total;
}

/** Punto a `distanciaRecorrida` unidades a lo largo de la polilínea, clampeado a los extremos, y el índice del
 * tramo (`puntos[tramo]` → `puntos[tramo + 1]`) en que cae. */
function ubicarEnPolilinea(puntos: readonly Point[], distanciaRecorrida: number): { punto: Point; tramo: number } {
  if (puntos.length === 0) return { punto: { x: 0, y: 0 }, tramo: 0 };
  if (puntos.length === 1) return { punto: puntos[0]!, tramo: 0 };

  let restante = Math.max(0, distanciaRecorrida);
  for (let i = 0; i < puntos.length - 1; i++) {
    const a = puntos[i]!;
    const b = puntos[i + 1]!;
    const largo = Math.hypot(b.x - a.x, b.y - a.y);
    if (restante <= largo || i === puntos.length - 2) {
      const t = largo === 0 ? 0 : Math.min(1, restante / largo);
      return { punto: { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }, tramo: i };
    }
    restante -= largo;
  }
  return { punto: puntos[puntos.length - 1]!, tramo: puntos.length - 2 };
}

/** Punto a `distanciaRecorrida` unidades a lo largo de la polilínea, clampeado a los extremos. */
export function puntoEnPolilinea(puntos: readonly Point[], distanciaRecorrida: number): Point {
  return ubicarEnPolilinea(puntos, distanciaRecorrida).punto;
}

export interface AvanceEnRuta {
  progreso: number;
  posicion: Point;
}

/**
 * Avanza `progresoActual` (0..1) un tick a lo largo de `ruta`, a `velocidadBase` unidades/tick moduladas
 * por `Mapa.costeEnPunto` en la posición actual (relieve y bosque) — a coste 1 (llano) equivale exactamente al
 * avance de la línea recta.
 *
 * `caminos` (Doc 1.6, decisión 10): aristas de la red (`aristasDeRed`, `engine/redCaminos.ts`). Si el tramo en
 * que está es una de ellas, el coste se multiplica por `COSTE_MOVIMIENTO.factorCamino` (<1 = más rápido).
 */
export function avanzarPosicionEnRuta(
  mapa: Mapa,
  ruta: readonly Point[],
  progresoActual: number,
  velocidadBase: number,
  caminos: ReadonlySet<string> = new Set()
): AvanceEnRuta {
  const longitud = Math.max(1, longitudPolilinea(ruta));
  const distanciaRecorrida = progresoActual * longitud;
  const { punto, tramo } = ubicarEnPolilinea(ruta, distanciaRecorrida);
  const enCamino = ruta.length > 1 && caminos.has(claveArista(ruta[tramo]!, ruta[tramo + 1]!));
  const avance = velocidadBase / (mapa.costeEnPunto(punto) * (enCamino ? COSTE_MOVIMIENTO.factorCamino : 1));
  const nuevaDistancia = Math.min(longitud, distanciaRecorrida + avance);
  return { progreso: Math.min(1, nuevaDistancia / longitud), posicion: puntoEnPolilinea(ruta, nuevaDistancia) };
}
