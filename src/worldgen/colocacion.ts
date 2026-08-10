import type { Point, ZonaBosque } from '../domain/types';
import { COLOCACION } from './config';
import { randRange, type RandomFn } from './rng';

export interface Limites {
  ancho: number;
  alto: number;
}

export function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

export function dentroDeAlgunBosque(p: Point, bosques: ZonaBosque[]): boolean {
  return bosques.some((b) => distancia(p, b.centro) < b.radio);
}

/**
 * Rejection sampling: coloca `cantidad` puntos respetando espaciado mínimo entre sí y evitando los bosques
 * (simplificación visual deliberada: un nodo mineral dentro del círculo de un bosque se leía como si fuera
 * "parte" del bosque, confundiendo qué representa cada uno). `aceptaTerreno` (Fase 0.1) es un filtro extra
 * opcional — omitirlo reproduce el comportamiento anterior byte a byte. Deliberadamente agnóstico de
 * bioma/terreno (recibe una función, no un enum): cada llamador arma su propio predicado.
 */
export function colocarConEspaciado(
  rng: RandomFn,
  limites: Limites,
  cantidad: number,
  espacioMinimo: number,
  yaColocados: Point[],
  bosques: ZonaBosque[],
  aceptaTerreno?: (p: Point) => boolean
): Point[] {
  const nuevos: Point[] = [];
  const todos = [...yaColocados];
  for (let i = 0; i < cantidad; i++) {
    let colocado: Point | null = null;
    /**
     * Mejor candidato que cumple el TERRENO pero no el espaciado. Cuando las dos condiciones no se pueden
     * satisfacer a la vez, se sacrifica el espaciado y no el terreno: "los metales salen en la montaña" es
     * una regla del mundo, mientras que el espaciado solo reparte. Importa de verdad porque el mapa está
     * saturado justo para las rarezas que más dependen del terreno — cuando le toca el turno a estaño/oro
     * (`espacioMinimo` 120) ya hay cientos de nodos comunes colocados, cada uno bloqueando ese radio, así
     * que el camino habitual para ellos es este respaldo, no el hueco limpio.
     */
    let respaldoEnTerreno: Point | null = null;

    for (let intento = 0; intento < COLOCACION.intentosPorPunto; intento++) {
      const candidato: Point = {
        x: randRange(rng, 0, limites.ancho),
        y: randRange(rng, 0, limites.alto),
      };
      if (dentroDeAlgunBosque(candidato, bosques) || (aceptaTerreno && !aceptaTerreno(candidato))) continue;
      if (todos.every((p) => distancia(p, candidato) >= espacioMinimo)) {
        colocado = candidato;
        break;
      }
      respaldoEnTerreno ??= candidato;
    }

    // Último recurso, si ni siquiera se encontró un punto en terreno válido: se coloca igualmente para no
    // bloquear la generación. Es la razón de que algún nodo pueda acabar dentro de un bosque o fuera de su
    // bioma pese a los filtros de arriba.
    const final = colocado ?? respaldoEnTerreno ?? { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
    nuevos.push(final);
    todos.push(final);
  }
  return nuevos;
}
