// Pieza de generación de mundo. Única entrada pública: `generarMapa(config) -> MapaGenerado`.
//
// Contrato:
//  - DETERMINISTA por seed. Misma seed => mismo mapa, bit a bit, siempre. La partida guardada solo
//    almacena la seed y regenera el mapa al cargar, así que esto no es una comodidad de testing: es lo
//    que hace que un save siga significando lo mismo mañana. Por eso los parámetros de generación viven
//    congelados en `config.ts` y no en el balance editable en caliente.
//  - DATOS PUROS a la salida: sin funciones, sin closures, sin estado de partida. El campo de fertilidad
//    se devuelve como parámetros (`CampoFertilidad`) y se consulta con `evaluarFertilidad`.
//  - Cada etapa usa una subsemilla etiquetada. Los cambios en una etapa no desplazan
//    automáticamente el PRNG de las demás. Todo cambio del mapa publicado exige revisar
//    `WORLDGEN_VERSION` y los snapshots de caracterización.

import type { NodoRecurso, Point, WorldConfig } from './tipos';
import { generarBosques } from './bosques';
import { generarCampoElevacion } from './elevacion';
import { generarCampoFertilidad } from './fertilidad';
import { generarLivestock, generarNodosDeRareza } from './nodos';
import { generarRios } from './rios';
import { createRng, derivarSeed } from './rng';
import { generarEstructura } from './estructura';
import { repararConectividad } from './conectividad';
import { WORLDGEN_VERSION, type MapaGenerado } from './types';

export { evaluarBioma } from './biomas';
export { costeEnPunto, esTransitable } from './costeMovimiento';
export { evaluarElevacion, evaluarTerreno, gradienteElevacion } from './elevacion';
export { evaluarFertilidad, evaluarFertilidadEfectiva } from './fertilidad';
export { distanciaARioMasCercano } from './rios';
export { MAPA_DEFAULT, COSTE_MOVIMIENTO, ESCALA_TERRENO, RIOS, BIOMA } from './config';
export { REGIONES } from './regiones';
export { createRng, derivarSeed, restaurarRng, randInt, randRange, type RandomFn } from './rng';
export { generarEstructura, aplicarEstructura, type EstructuraGeografica } from './estructura';
export { WORLDGEN_VERSION, type CampoElevacion, type CampoFertilidad, type MapaGenerado } from './types';
export { type CampoRuido } from './ruido';
export { mallaRelieve, type MallaRelieve } from './mallaRelieve';

export function generarMapa(config: WorldConfig): MapaGenerado {
  const rngDe = (etapa: string) => createRng(derivarSeed(config.seed, etapa));
  const limites = { ancho: config.ancho, alto: config.alto };
  const colocadosGlobal: Point[] = [];
  const estructura = generarEstructura(rngDe('estructura-v1'), limites, config.region);

  // Elevación primero: bosques, ríos y colocación de nodos la necesitan (Fase 0.1). `config.region`
  // (Fase 0.2, opcional) sesga la silueta hacia una zona real conocida — ver `generarCampoElevacion`.
  const elevacion = generarCampoElevacion(rngDe('elevacion-v1'), limites, config.region, estructura);
  repararConectividad(elevacion, limites);

  // Ríos: solo dependen de elevación. Van antes de bosques/nodos porque ambos consultan el bioma, que usa
  // ríos como proxy de humedad.
  const { rios, lagos } = generarRios(rngDe('rios-v2-drenaje'), limites, elevacion);
  elevacion.lagos = lagos;

  // Fertilidad (Fase 0.1 / v6): antes de bosques y nodos, porque AMBOS la consultan — nodos para la
  // colocación condicionada al bioma, bosques (desde v6) para ponderar cuán denso sale cada uno (ver
  // `generarBosques`).
  const fertilidad = generarCampoFertilidad(rngDe('fertilidad-v1'));

  // Los bosques van primero para que los nodos minerales/livestock puedan evitarlos (ver `colocarConEspaciado`).
  // Condicionados al terreno desde Fase 0.1, y desde v6 también a la fertilidad del suelo (ver `generarBosques`).
  const bosques = generarBosques(rngDe('bosques-v1'), limites, elevacion, fertilidad, rios, lagos);

  const nodos: NodoRecurso[] = [
    ...generarNodosDeRareza(rngDe('piedra-v1'), limites, 'comun', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarNodosDeRareza(rngDe('cobre-v1'), limites, 'intermedio', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarNodosDeRareza(rngDe('metales-raros-v1'), limites, 'raro', colocadosGlobal, bosques, elevacion, fertilidad, rios),
    ...generarLivestock(rngDe('fauna-v1'), limites, colocadosGlobal, bosques, elevacion, fertilidad, rios),
  ];

  return { version: WORLDGEN_VERSION, config, bosques, nodos, fertilidad, elevacion, rios, lagos };
}
