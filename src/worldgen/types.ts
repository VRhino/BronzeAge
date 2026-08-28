// Tipos de la pieza de generación de mundo. `src/worldgen/` no conoce el motor: solo depende de
// `domain/types` (entidades compartidas) y de su propia configuración — se puede ejecutar, testear y
// versionar por separado.

import type { NodoRecurso, RioZona, WorldConfig, ZonaBosque } from '../domain/types';
import type { Limites } from './colocacion';
import type { CampoRuido } from './ruido';
import type { RegionGeografica } from './regiones';

// Los dos campos continuos del mundo son ruido fractal (ver `ruido.ts`): mismo tipo de dato, distintos
// parámetros (ver `FERTILIDAD`/`ELEVACION` en `config.ts`). Se mantienen como alias con nombre propio
// porque el resto del código habla de "el campo de fertilidad" y "el campo de elevación", no de "ruido" —
// y porque nada fuera de `fertilidad.ts`/`elevacion.ts` debe depender de su forma interna.

/** Campo continuo de fertilidad del suelo. Se consulta con `evaluarFertilidad`. */
export type CampoFertilidad = CampoRuido;

/**
 * Campo continuo de relieve. Se consulta con `evaluarElevacion`/`evaluarTerreno`. Deja de ser un simple
 * alias de `CampoRuido` en Fase 0.2: `region` (opcional) es la guía geográfica de la región elegida al
 * generar el mundo (ver `regiones.ts`) — `undefined` reproduce el mundo libre de siempre, byte a byte
 * (`evaluarElevacion` solo consulta `ruido` en ese caso). Sigue siendo DATOS PUROS, sin funciones.
 */
export interface CampoElevacion {
  ruido: CampoRuido;
  region?: RegionGeografica;
  /** Borde natural del mundo libre (Fase 0.4, ver `elevacion.ts`/`ELEVACION_BORDE` en `config.ts`):
   * `undefined` cuando el mundo tiene `region` (las regiones autoradas ya definen su propio borde). */
  borde?: { ruido: CampoRuido; limites: Limites };
}

/**
 * Resultado completo de la generación: DATOS PUROS, sin funciones ni estado de partida. Todo lo que el
 * motor necesita saber del mapa sale de aquí.
 *
 * El agotamiento de yacimientos (que hoy muta `NodoRecurso.cantidad` in-place) todavía vive dentro de
 * `nodos` — se separará como estado de partida en un paso posterior del plan.
 */
export interface MapaGenerado {
  /**
   * Versión del ALGORITMO de generación, no del formato de datos. La partida guardada solo almacena la
   * seed y regenera el mapa al cargar (decisión de diseño), así que un cambio en el pipeline produciría
   * un mundo distinto para la misma seed. Subir este número al tocar la generación permite DETECTAR ese
   * caso al importar en vez de cargar en silencio un mundo que no es el que se guardó.
   */
  version: number;
  config: WorldConfig;
  bosques: ZonaBosque[];
  nodos: NodoRecurso[];
  fertilidad: CampoFertilidad;
  /** Campo continuo de relieve (Fase 0.1). Consultar con `evaluarElevacion`/`evaluarTerreno` — nunca se
   * itera como rejilla, ver `Fase_0_1_Definicion.md`. */
  elevacion: CampoElevacion;
  /** Ríos como polilíneas (Fase 0.1). El bioma se deriva de esto + elevación + fertilidad bajo demanda
   * (`evaluarBioma`) — no existe un campo de bioma guardado. */
  rios: RioZona[];
}

/**
 * Se sube cuando cambia el pipeline de generación de forma que altere el mundo para una seed dada.
 * v2 (Fase 0.1): se inserta generación de elevación y ríos, se reordena fertilidad, y bosques/nodos pasan a
 * condicionar su colocación al terreno — cambia el orden de consumo del PRNG de arriba a abajo del
 * pipeline, así que TODAS las seeds/mundos guardados producen un mundo distinto.
 * v3: `MAPA_DEFAULT` pasa de 1000x1000 a 2000x2000 y las cantidades de bosques/nodos/livestock/ríos suben
 * ×4 (misma densidad por área) — aunque estos son valores de `config.ts` y no reordenan el PRNG, cambian
 * cuántos puntos se sortean en cada paso, así que también desplazan todo lo generado después para la misma
 * seed.
 * v4: nueva banda de elevación `cima` (por encima de `montana`, inhabitable — ver `ELEVACION.umbralCima`)
 * excluida de todo predicado de colocación (`RECURSO_BIOMA_PERMITIDO`/`BOSQUE_TERRENO_PERMITIDO` nunca la
 * listan) — el rejection-sampling rechaza algunos candidatos que antes aceptaba, así que necesita más
 * intentos en algunos puntos y el PRNG se desplaza otra vez.
 * v5: elevación y fertilidad pasan de suma de senos alineados a los ejes a RUIDO FRACTAL DE GRADIENTE (ver
 * `ruido.ts`) — el mundo entero cambia de forma, no solo de números: el relieve deja de ser un enrejado
 * regular. Umbrales de terreno/bioma recalibrados contra la nueva distribución. El import ya rechaza duro
 * por `worldgenVersion` en vez de migrar (contrato existente, ver `gameStore.importarSimulacion`).
 * v6: la fertilidad se genera ANTES que los bosques (se movió por delante de `generarBosques` en el
 * pipeline) para que `generarBosques` pueda consultarla — la densidad de cada bosque ahora pondera la
 * fertilidad del suelo en su centro en vez de salir de un sorteo uniforme puro (ver `bosques.ts`). Mismo
 * cambio de categoría que v2/v4: no toca cuántas veces se llama a `randRange`, pero mueve CUÁNDO se llama
 * `generarCampoFertilidad` (que sí consume RNG) respecto al bucle de bosques, así que desplaza el consumo
 * de PRNG de ahí en adelante para toda seed.
 * v7 (Fase 0.3): nuevo paso `generarChokepoints` (puertos de montaña, geometría determinista — ver
 * `chokepoints.ts`), añadido al final del pipeline (después de nodos/livestock) para no desplazar el
 * consumo de PRNG de ningún paso ya calibrado — pero SÍ consume RNG propio (la colocación de candidatos),
 * así que toda seed produce nodos/bosques/ríos idénticos a v6 y un mundo distinto solo a partir de ahí.
 * v8 (Fase 0.4): tres cambios de relieve/bosques a la vez. (1) `evaluarElevacion` comprime la variación
 * dentro de la banda jugable (llano/colina) — ver `ELEVACION.factorAplanadoJugable`. (2) mundo libre (sin
 * `region`) genera un segundo campo de ruido de borde (`generarCampoElevacion` llama a `generarCampoRuido`
 * una vez más antes de devolver) que fuerza los bordes del mapa a costa profunda o montaña empinada — ver
 * `ELEVACION_BORDE`; consume RNG propio, así que desplaza TODO lo generado después para mundos sin región
 * (con región, sigue sin consumirlo — mismo criterio que `riosTroncales` en v-anterior). (3) `BOSQUE.cantidad`
 * sube de 100 a 170 (más radio/densidad también) — más puntos sorteados en el bucle de bosques. Los tres
 * cambian el mundo resultante de toda seed existente; ninguno reordena qué se genera, solo cuánto/cómo.
 * v9 (Fase 0.4.1): reemplaza el aplanado parcial de v8 por TERRACEO — `evaluarElevacion` cuantiza la banda
 * jugable (llano/colina) en `ELEVACION_TERRAZAS.niveles` mesetas de altura CONSTANTE, con una rampa corta
 * solo al entrar a cada una (ver `elevacionTerraceada` en `elevacion.ts`), pedido explícito de diseño tras
 * ver el mapa 3D en Unity (mapas de campaña tipo Total War: llano de verdad, no "menos accidentado"). No
 * consume RNG ni reordena el pipeline — pura función de `e`, mismo criterio de categoría que v3/v4/v8(1).
 * v10: `ELEVACION_TERRAZAS.niveles` de 4 a 1, EXPERIMENTAL (a petición del usuario, para ver el caso límite
 * de una sola meseta jugable) — mismo mecanismo de v9, sin cambios de código, solo el parámetro.
 * v11: `ELEVACION_TERRAZAS` a 2 niveles con `anchoTransicion` ancho (0.45) — rampa notablemente más suave
 * entre ambas mesetas, a petición del usuario tras ver v10. Solo parámetros, mismo mecanismo.
 * v12: `anchoTransicion` único (0.45, compartido por la rampa de entrada Y la de salida a montaña) se
 * separa en `anchoTransicionEntrada`(0.7)/`anchoTransicionSalida`(0.1) — con un solo valor compartido, la
 * meseta alta (única con las dos rampas) se quedaba casi sin núcleo plano y la transición se seguía leyendo
 * empinada. Solo parámetros/cómo se leen en `elevacionTerraceada`, mismo mecanismo.
 * v13: `generarChokepoints` deja de aceptar terreno 'colina' como candidato (solo 'montana') — consecuencia
 * directa del terraceo (v9+): 'colina' es ahora una meseta constante sin curvatura real, así que casi nunca
 * pasaba el test de punto de silla y solo desperdiciaba intentos de `colocarConEspaciado` antes de caer al
 * fallback de "mapa saturado" (bajó el % de chokepoints en terreno montañoso por debajo del 80% exigido por
 * el test de invariantes). Cambia CUÁNDO cada candidato encuentra hueco dentro de sus intentos, así que
 * desplaza el consumo de PRNG de ahí en adelante — mismo criterio que v4 (nueva banda 'cima' excluida).
 * v14 (Fase 0.4.2): retira POR COMPLETO el terraceo de v9-v13 (`ELEVACION_TERRAZAS`/`elevacionTerraceada`,
 * eliminados, no dejados como opción desactivable — decisión explícita del usuario) y lo reemplaza por
 * SUAVIZADO: `evaluarElevacion` mezcla el ruido completo con una versión de menos octavas del mismo campo
 * (`ELEVACION_SUAVIZADO`, `evaluarRuidoParcial` en `ruido.ts`) — ver
 * `Consideraciones/Fase_0_4_Definicion_Relieve_Jugable.md` para el porqué (la referencia real —mapa de
 * campaña de Total War: Troy— pedía relieve ondulado y continuo, no mesetas escalonadas). El suavizado
 * nunca aplana del todo (a diferencia del terraceo), así que `generarChokepoints` recupera 'colina' como
 * terreno válido (restringida a solo 'montana' en v13) — medido: 14/14 chokepoints en colina/montana en las
 * 3 seeds de referencia. Cambia CUÁNDO cada candidato de chokepoint encuentra hueco dentro de sus intentos
 * (terrain check distinto), así que desplaza el consumo de PRNG de ahí en adelante — mismo criterio que v13.
 * v15 (2026-08-26): retira POR COMPLETO chokepoints — decisión explícita del usuario ("no me está dando nada
 * en este momento"), alcance total: `generarChokepoints`/`worldgen/chokepoints.ts` (geometría), control por
 * zona de influencia y peaje en oro (`engine/chokepoints.ts`, `engine/trade.ts`), la constante
 * `CHOKEPOINTS_PEAJE`, el tipo de dominio `Chokepoint`, y el renderizado en el cliente. `generarChokepoints`
 * era el ÚLTIMO paso del pipeline (v7: "para no desplazar el consumo de PRNG de ningún paso anterior"), así
 * que quitarlo tiene la misma propiedad a la inversa — bosques/nodos/ríos/elevación salen BIT A BIT idénticos
 * a v14 para la misma seed, el pipeline simplemente termina un paso antes y no consume el RNG que los
 * candidatos de chokepoint gastaban. `MapaGenerado.chokepoints` desaparece del tipo.
 */
export const WORLDGEN_VERSION = 15;
