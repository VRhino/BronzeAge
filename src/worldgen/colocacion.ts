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
 * "parte" del bosque, confundiendo qué representa cada uno).
 */
export function colocarConEspaciado(
  rng: RandomFn,
  limites: Limites,
  cantidad: number,
  espacioMinimo: number,
  yaColocados: Point[],
  bosques: ZonaBosque[]
): Point[] {
  const nuevos: Point[] = [];
  const todos = [...yaColocados];
  for (let i = 0; i < cantidad; i++) {
    let colocado: Point | null = null;
    for (let intento = 0; intento < COLOCACION.intentosPorPunto; intento++) {
      const candidato: Point = {
        x: randRange(rng, 0, limites.ancho),
        y: randRange(rng, 0, limites.alto),
      };
      if (!dentroDeAlgunBosque(candidato, bosques) && todos.every((p) => distancia(p, candidato) >= espacioMinimo)) {
        colocado = candidato;
        break;
      }
    }
    // Si no se encontró hueco tras los intentos, se coloca igualmente (mapa saturado) para no bloquear la
    // generación. Es la razón de que algún nodo pueda acabar dentro de un bosque pese al filtro de arriba.
    const final = colocado ?? { x: randRange(rng, 0, limites.ancho), y: randRange(rng, 0, limites.alto) };
    nuevos.push(final);
    todos.push(final);
  }
  return nuevos;
}
