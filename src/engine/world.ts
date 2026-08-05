import type { NodoRecurso, Point, Rareza, World, WorldConfig, ZonaBosque } from '../domain/types';
import {
  BOSQUE,
  FERTILIDAD,
  LIVESTOCK,
  RECURSO_CANTIDAD_NODO,
  RECURSO_RAREZA,
  RECURSO_TIPOS_POR_RAREZA,
} from '../constants';
import { createRng, randInt, randRange, type RandomFn } from './rng';

function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** Rejection sampling: coloca `cantidad` puntos respetando espaciado mínimo entre sí. */
function placeWithSpacing(
  rng: RandomFn,
  config: WorldConfig,
  cantidad: number,
  espacioMinimo: number,
  yaColocados: Point[],
  intentosPorPunto = 30
): Point[] {
  const nuevos: Point[] = [];
  const todos = [...yaColocados];
  for (let i = 0; i < cantidad; i++) {
    let colocado: Point | null = null;
    for (let intento = 0; intento < intentosPorPunto; intento++) {
      const candidato: Point = {
        x: randRange(rng, 0, config.ancho),
        y: randRange(rng, 0, config.alto),
      };
      if (todos.every((p) => distance(p, candidato) >= espacioMinimo)) {
        colocado = candidato;
        break;
      }
    }
    // Si no se encontró hueco tras los intentos, se coloca igualmente (mapa saturado) para no bloquear la generación.
    const final = colocado ?? { x: randRange(rng, 0, config.ancho), y: randRange(rng, 0, config.alto) };
    nuevos.push(final);
    todos.push(final);
  }
  return nuevos;
}

function generarRecursosDeRareza(
  rng: RandomFn,
  config: WorldConfig,
  rareza: Rareza,
  colocadosGlobal: Point[]
): NodoRecurso[] {
  const { cantidadBase, espacioMinimo } = RECURSO_RAREZA[rareza];
  const tipos = RECURSO_TIPOS_POR_RAREZA[rareza];
  const nodos: NodoRecurso[] = [];
  for (const tipo of tipos) {
    const posiciones = placeWithSpacing(rng, config, cantidadBase, espacioMinimo, colocadosGlobal);
    const rango = RECURSO_CANTIDAD_NODO[tipo as keyof typeof RECURSO_CANTIDAD_NODO];
    for (const posicion of posiciones) {
      colocadosGlobal.push(posicion);
      nodos.push({
        id: `${tipo}-${nodos.length}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
        tipo: tipo as NodoRecurso['tipo'],
        rareza,
        posicion,
        cantidad: Math.round(randRange(rng, rango.min, rango.max)),
      });
    }
  }
  return nodos;
}

function generarLivestock(rng: RandomFn, config: WorldConfig, colocadosGlobal: Point[]): NodoRecurso[] {
  const posiciones = placeWithSpacing(rng, config, LIVESTOCK.cantidadBase, LIVESTOCK.espacioMinimo, colocadosGlobal);
  return posiciones.map((posicion, i) => {
    colocadosGlobal.push(posicion);
    return {
      id: `livestock-${i}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
      tipo: 'livestock' as const,
      rareza: 'comun' as const,
      posicion,
      cantidad: randInt(rng, LIVESTOCK.cantidadPorManada.min, LIVESTOCK.cantidadPorManada.max),
    };
  });
}

function generarBosques(rng: RandomFn, config: WorldConfig): ZonaBosque[] {
  const bosques: ZonaBosque[] = [];
  const centros: Point[] = [];
  for (let i = 0; i < BOSQUE.cantidad; i++) {
    const centro: Point = { x: randRange(rng, 0, config.ancho), y: randRange(rng, 0, config.alto) };
    centros.push(centro);
    bosques.push({
      id: `bosque-${i}`,
      centro,
      radio: randRange(rng, BOSQUE.radioMin, BOSQUE.radioMax),
      densidad: randRange(rng, BOSQUE.densidadMin, BOSQUE.densidadMax),
    });
  }
  return bosques;
}

/** Campo de fertilidad continuo (0-1) por suma de senos con fase aleatoria — sin dependencias externas. */
function crearCampoFertilidad(rng: RandomFn): (p: Point) => number {
  const octavas = Array.from({ length: FERTILIDAD.octavas }, (_, i) => ({
    freq: FERTILIDAD.escala * (i + 1),
    faseX: randRange(rng, 0, Math.PI * 2),
    faseY: randRange(rng, 0, Math.PI * 2),
    peso: 1 / (i + 1),
  }));
  const pesoTotal = octavas.reduce((acc, o) => acc + o.peso, 0);

  return (p: Point): number => {
    let valor = 0;
    for (const o of octavas) {
      const s = Math.sin(p.x * o.freq + o.faseX) * Math.cos(p.y * o.freq + o.faseY);
      valor += ((s + 1) / 2) * o.peso;
    }
    return valor / pesoTotal;
  };
}

export function generateWorld(config: WorldConfig): World {
  const rng = createRng(config.seed);
  const colocadosGlobal: Point[] = [];

  const recursos: NodoRecurso[] = [
    ...generarRecursosDeRareza(rng, config, 'comun', colocadosGlobal),
    ...generarRecursosDeRareza(rng, config, 'intermedio', colocadosGlobal),
    ...generarRecursosDeRareza(rng, config, 'raro', colocadosGlobal),
    ...generarLivestock(rng, config, colocadosGlobal),
  ];

  const bosques = generarBosques(rng, config);
  const fertilidadEn = crearCampoFertilidad(rng);

  return { config, recursos, bosques, fertilidadEn };
}
