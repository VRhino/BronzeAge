import { evaluarElevacion, evaluarTerreno } from './elevacion';
import { ESCALA_TERRENO } from './config';
import type { MapaGenerado } from './types';

/** Malla de intercambio para renderizadores 3D. La altura exacta y los bloqueos
 * de movimiento siguen siendo las consultas continuas del servidor. */
export interface MallaRelieve {
  version: number;
  ancho: number;
  alto: number;
  columnas: number;
  filas: number;
  metrosVerticalesPorElevacion: number;
  interpolacion: 'bilineal_visual';
  errorMaximoInterpolacionMetros: number;
  alturasMetros: number[];
  agua: boolean[];
}

const cache = new WeakMap<MapaGenerado, MallaRelieve>();

export function mallaRelieve(mapa: MapaGenerado): MallaRelieve {
  const existente = cache.get(mapa);
  if (existente) return existente;
  const columnas = 129, filas = 129;
  const alturasMetros: number[] = [], agua: boolean[] = [];
  for (let fila = 0; fila < filas; fila++) for (let col = 0; col < columnas; col++) {
    const p = { x: col * mapa.config.ancho / (columnas - 1), y: fila * mapa.config.alto / (filas - 1) };
    alturasMetros.push(evaluarElevacion(mapa.elevacion, p) * ESCALA_TERRENO.metrosVerticalesPorElevacion);
    agua.push(evaluarTerreno(mapa.elevacion, p) === 'agua');
  }
  let errorMaximoInterpolacionMetros = 0;
  for (let fila = 0; fila < filas - 1; fila++) for (let col = 0; col < columnas - 1; col++) {
    const i = fila * columnas + col;
    const aproximada = (alturasMetros[i]! + alturasMetros[i + 1]!
      + alturasMetros[i + columnas]! + alturasMetros[i + columnas + 1]!) / 4;
    const p = { x: (col + 0.5) * mapa.config.ancho / (columnas - 1),
      y: (fila + 0.5) * mapa.config.alto / (filas - 1) };
    errorMaximoInterpolacionMetros = Math.max(errorMaximoInterpolacionMetros,
      Math.abs(evaluarElevacion(mapa.elevacion, p) * ESCALA_TERRENO.metrosVerticalesPorElevacion - aproximada));
  }
  const resultado: MallaRelieve = { version: mapa.version, ancho: mapa.config.ancho, alto: mapa.config.alto,
    columnas, filas, metrosVerticalesPorElevacion: ESCALA_TERRENO.metrosVerticalesPorElevacion,
    interpolacion: 'bilineal_visual', errorMaximoInterpolacionMetros, alturasMetros, agua };
  cache.set(mapa, resultado);
  return resultado;
}
