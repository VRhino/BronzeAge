// Pieza de generación de mundo. Única entrada pública: `generarMapa(config) -> MapaGenerado`.
//
// Contrato:
//  - DETERMINISTA por seed. Misma seed => mismo mapa, bit a bit, siempre. La partida guardada solo
//    almacena la seed y regenera el mapa al cargar, así que esto no es una comodidad de testing: es lo
//    que hace que un save siga significando lo mismo mañana. Por eso los parámetros de generación viven
//    congelados en `config.ts` y no en el balance editable en caliente.
//  - DATOS PUROS a la salida: sin funciones, sin closures, sin estado de partida. El campo de fertilidad
//    se devuelve como parámetros (`CampoFertilidad`) y se consulta con `evaluarFertilidad`.
//  - El ORDEN de consumo del PRNG es parte del contrato. Añadir, quitar o reordenar una sola llamada
//    desplaza todo lo generado después: cambia el mundo de todas las seeds. Si hace falta hacerlo, se
//    sube `WORLDGEN_VERSION` y se regeneran los snapshots de caracterización a conciencia.

import type { NodoRecurso, Point, WorldConfig } from '../domain/types';
import { generarBosques } from './bosques';
import { generarCampoFertilidad } from './fertilidad';
import { generarLivestock, generarNodosDeRareza } from './nodos';
import { createRng } from './rng';
import { WORLDGEN_VERSION, type MapaGenerado } from './types';

export { evaluarFertilidad } from './fertilidad';
export { MAPA_DEFAULT } from './config';
export { createRng, randInt, randRange, type RandomFn } from './rng';
export { WORLDGEN_VERSION, type CampoFertilidad, type MapaGenerado, type OctavaFertilidad } from './types';

export function generarMapa(config: WorldConfig): MapaGenerado {
  const rng = createRng(config.seed);
  const limites = { ancho: config.ancho, alto: config.alto };
  const colocadosGlobal: Point[] = [];

  // Los bosques van primero para que los nodos minerales/livestock puedan evitarlos (ver `colocarConEspaciado`).
  const bosques = generarBosques(rng, limites);

  const nodos: NodoRecurso[] = [
    ...generarNodosDeRareza(rng, limites, 'comun', colocadosGlobal, bosques),
    ...generarNodosDeRareza(rng, limites, 'intermedio', colocadosGlobal, bosques),
    ...generarNodosDeRareza(rng, limites, 'raro', colocadosGlobal, bosques),
    ...generarLivestock(rng, limites, colocadosGlobal, bosques),
  ];

  const fertilidad = generarCampoFertilidad(rng);

  return { version: WORLDGEN_VERSION, config, bosques, nodos, fertilidad };
}
