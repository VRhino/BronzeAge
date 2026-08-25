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
import { generarChokepoints } from './chokepoints';
import { generarCampoElevacion } from './elevacion';
import { generarCampoFertilidad } from './fertilidad';
import { generarLivestock, generarNodosDeRareza } from './nodos';
import { generarRios } from './rios';
import { createRng } from './rng';
import { WORLDGEN_VERSION, type MapaGenerado } from './types';

export { evaluarBioma } from './biomas';
export { generarChokepoints } from './chokepoints';
export { costeEnPunto } from './costeMovimiento';
export { evaluarElevacion, evaluarTerreno, gradienteElevacion } from './elevacion';
export { evaluarFertilidad } from './fertilidad';
export { distanciaARioMasCercano } from './rios';
export { MAPA_DEFAULT, COSTE_MOVIMIENTO, CHOKEPOINTS, RIOS, BIOMA } from './config';
export { REGIONES } from './regiones';
export { createRng, restaurarRng, randInt, randRange, type RandomFn } from './rng';
export { WORLDGEN_VERSION, type CampoElevacion, type CampoFertilidad, type MapaGenerado } from './types';
export { type CampoRuido } from './ruido';

export function generarMapa(config: WorldConfig): MapaGenerado {
  const rng = createRng(config.seed);
  const limites = { ancho: config.ancho, alto: config.alto };
  const colocadosGlobal: Point[] = [];

  // Elevación primero: bosques, ríos y colocación de nodos la necesitan (Fase 0.1). `config.region`
  // (Fase 0.2, opcional) sesga la silueta hacia una zona real conocida — ver `generarCampoElevacion`.
  const elevacion = generarCampoElevacion(rng, limites, config.region);

  // Ríos: solo dependen de elevación. Van antes de bosques/nodos porque ambos consultan el bioma, que usa
  // ríos como proxy de humedad.
  const rios = generarRios(rng, limites, elevacion);

  // Fertilidad (Fase 0.1 / v6): antes de bosques y nodos, porque AMBOS la consultan — nodos para la
  // colocación condicionada al bioma, bosques (desde v6) para ponderar cuán denso sale cada uno (ver
  // `generarBosques`).
  const fertilidad = generarCampoFertilidad(rng);

  // Los bosques van primero para que los nodos minerales/livestock puedan evitarlos (ver `colocarConEspaciado`).
  // Condicionados al terreno desde Fase 0.1, y desde v6 también a la fertilidad del suelo (ver `generarBosques`).
  const bosques = generarBosques(rng, limites, elevacion, fertilidad);

  const nodos: NodoRecurso[] = [
    ...generarNodosDeRareza(rng, limites, 'comun', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarNodosDeRareza(rng, limites, 'intermedio', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarNodosDeRareza(rng, limites, 'raro', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarLivestock(rng, limites, colocadosGlobal, bosques, elevacion, fertilidad, rios),
  ];

  // Chokepoints (Fase 0.3, v7): al final del pipeline — solo depende de elevación, así que colocarlo aquí
  // no desplaza el consumo de PRNG de ningún paso anterior ya calibrado (ver `WORLDGEN_VERSION`).
  const chokepoints = generarChokepoints(rng, limites, elevacion);

  return { version: WORLDGEN_VERSION, config, bosques, nodos, fertilidad, elevacion, rios, chokepoints };
}
