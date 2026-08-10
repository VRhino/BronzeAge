import type { Point, RioZona } from '../domain/types';
import { RIOS } from './config';
import { colocarConEspaciado, distancia, distanciaASegmento, type Limites } from './colocacion';
import { evaluarTerreno, gradienteElevacion } from './elevacion';
import type { RioTroncalDef } from './regiones';
import type { RandomFn } from './rng';
import type { CampoElevacion } from './types';

/** Longitud total de la polilínea de un río, sumando cada segmento. */
function longitudRio(puntos: readonly Point[]): number {
  let total = 0;
  for (let i = 0; i < puntos.length - 1; i++) total += distancia(puntos[i]!, puntos[i + 1]!);
  return total;
}

/** Puntos de paso por tramo (entre cada par de waypoints autorados) al densificar un río troncal. */
const TRONCAL_SUBDIVISIONES = 8;

/**
 * Construye la polilínea de un río troncal (Nilo, Tigris, Éufrates — ver `RioTroncalDef`) a partir de sus
 * puntos de paso YA RESUELTOS a unidades de mapa. A diferencia de los ríos normales, NO desciende por
 * gradiente: sigue el trazo autorado, pero con un meandro lateral pequeño y determinista (perpendicular al
 * tramo, cero en cada waypoint autorado y máximo a mitad de camino — `sin(π·t)`) para que no se lea como una
 * polilínea de 3-4 segmentos perfectamente recta. `navegable` y `!terminaEnLago` son fijos: un río de esta
 * escala es navegable y desemboca por definición, no por sorteo (ver `RIOS.proporcionNavegable`).
 */
function generarRioTroncal(rng: RandomFn, def: RioTroncalDef, limites: Limites): RioZona {
  const meandro = Math.min(limites.ancho, limites.alto) * 0.015;
  const puntos: Point[] = [];

  for (let tramo = 0; tramo < def.puntos.length - 1; tramo++) {
    const a = def.puntos[tramo]!;
    const b = def.puntos[tramo + 1]!;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const largo = Math.hypot(dx, dy) || 1;
    const nx = -dy / largo;
    const ny = dx / largo;

    // `inicio`: el primer tramo incluye su extremo t=0, los siguientes lo omiten (ya lo puso el tramo
    // anterior como su t=1) — evita el punto duplicado en cada waypoint compartido.
    for (let paso = tramo === 0 ? 0 : 1; paso <= TRONCAL_SUBDIVISIONES; paso++) {
      const t = paso / TRONCAL_SUBDIVISIONES;
      const desplazamiento = (rng() * 2 - 1) * meandro * Math.sin(Math.PI * t);
      puntos.push({ x: a.x + dx * t + nx * desplazamiento, y: a.y + dy * t + ny * desplazamiento });
    }
  }

  return { id: def.id, puntos, terminaEnLago: false, navegable: true };
}

/**
 * Ríos (Fase 0.1): polilíneas, no celdas — nacen en un punto de montaña y descienden por gradiente de
 * máxima pendiente del campo de elevación hasta agua/borde del mapa, o quedan atrapados en un mínimo local
 * (lago). El único consumo de RNG es la colocación de los nacimientos; el descenso es puramente determinista
 * sobre `elevacion` (ver `Fase_0_1_Definicion.md`: geometría vectorial, nunca rejilla).
 */
export function generarRios(rng: RandomFn, limites: Limites, elevacion: CampoElevacion): RioZona[] {
  // Además de exigir terreno de montaña, el candidato a nacimiento se descarta si su gradiente ya está por
  // debajo de `gradienteMinimo` — nacer exactamente en un extremo local (cima sin pendiente clara) producía
  // ríos de un solo punto, más frecuente cuanto más nacimientos se sortean (Fase 0.1, mapa 2000x2000).
  const nacimientos = colocarConEspaciado(
    rng,
    limites,
    RIOS.cantidad,
    RIOS.espacioMinimoEntreNacimientos,
    [],
    [],
    (p) => {
      if (evaluarTerreno(elevacion, p) !== 'montana') return false;
      const { dx, dy } = gradienteElevacion(elevacion, p, RIOS.pasoGradiente);
      return Math.hypot(dx, dy) >= RIOS.gradienteMinimo;
    }
  );

  const rios = nacimientos.map((fuente, i) => {
    const puntos: Point[] = [fuente];
    let actual = fuente;
    let terminaEnLago = true;

    for (let paso = 0; paso < RIOS.pasosMax; paso++) {
      const { dx, dy } = gradienteElevacion(elevacion, actual, RIOS.pasoGradiente);
      const magnitud = Math.hypot(dx, dy);
      if (magnitud < RIOS.gradienteMinimo) {
        terminaEnLago = true;
        break;
      }

      const siguiente: Point = {
        x: actual.x - (dx / magnitud) * RIOS.pasoDescenso,
        y: actual.y - (dy / magnitud) * RIOS.pasoDescenso,
      };

      if (siguiente.x < 0 || siguiente.x > limites.ancho || siguiente.y < 0 || siguiente.y > limites.alto) {
        const clamped: Point = {
          x: Math.min(Math.max(siguiente.x, 0), limites.ancho),
          y: Math.min(Math.max(siguiente.y, 0), limites.alto),
        };
        puntos.push(clamped);
        terminaEnLago = false;
        break;
      }

      puntos.push(siguiente);
      actual = siguiente;

      if (evaluarTerreno(elevacion, actual) === 'agua') {
        terminaEnLago = false;
        break;
      }
    }

    return { id: `rio-${i}`, puntos, terminaEnLago };
  });

  // Navegabilidad (ver comentario de `RIOS.proporcionNavegable` en `config.ts`): candidatos = los que
  // desembocan de verdad, ordenados por longitud descendente; se marca navegable el `proporcionNavegable`
  // superior. Solo sobre los ríos ALEATORIOS de arriba — los troncales (abajo) no compiten por este cupo,
  // son navegables por definición. No consume RNG — es puramente derivado de la geometría ya generada arriba.
  const candidatos = rios.filter((r) => !r.terminaEnLago).sort((a, b) => longitudRio(b.puntos) - longitudRio(a.puntos));
  const numNavegables = Math.round(candidatos.length * RIOS.proporcionNavegable);
  const idsNavegables = new Set(candidatos.slice(0, numNavegables).map((r) => r.id));
  const riosAleatorios = rios.map((rio) => ({ ...rio, navegable: idsNavegables.has(rio.id) }));

  // Troncales (Fase 0.2, Nilo/Mesopotamia — ver `RegionGeografica.riosTroncales`): se generan DESPUÉS y
  // aparte de los aleatorios, así que regiones sin ellos (o el mundo libre) consumen el RNG exactamente
  // igual que antes de que existiera esta pieza — ninguna seed ya calibrada (Grecia, Anatolia, Egeo) se
  // desplaza por este añadido.
  const troncales = (elevacion.region?.riosTroncales ?? []).map((def) => generarRioTroncal(rng, def, limites));

  return [...riosAleatorios, ...troncales];
}

/**
 * Distancia mínima de un punto a cualquier segmento de cualquier río — proxy de humedad para `evaluarBioma`.
 * O(ríos × puntos): barato con `RIOS.cantidad` pequeño y caminatas acotadas por `RIOS.pasosMax`.
 */
export function distanciaARioMasCercano(rios: readonly RioZona[], p: Point): number {
  let minimo = Infinity;
  for (const rio of rios) {
    for (let i = 0; i < rio.puntos.length - 1; i++) {
      const d = distanciaASegmento(p, rio.puntos[i]!, rio.puntos[i + 1]!);
      if (d < minimo) minimo = d;
    }
  }
  return minimo;
}
