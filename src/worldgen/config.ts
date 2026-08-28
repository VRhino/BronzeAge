// Parámetros de GENERACIÓN de mundo. Viven aquí y no en `constants.ts` por una razón de contrato, no de
// orden: el resto de `constants.ts` es balance ajustable en caliente desde la interfaz, y la generación NO
// puede serlo. La partida guardada solo almacena la seed y regenera el mapa al cargar, así que si estos
// números pudieran cambiarse en vivo, un save hecho antes del cambio se recuperaría como un mundo distinto
// —silenciosamente— al reabrirlo. "Determinista por seed" y "editable en caliente" son incompatibles;
// mandan estos valores fijos. Para cambiarlos hay que tocar este archivo y subir `WORLDGEN_VERSION`.
//
// Se congelan en tiempo de ejecución (no solo con `as const`, que es un candado de tipos) para que ninguna
// vía dinámica —como la que usa `app/balanceConfig.ts` para el resto de constantes— pueda escribirlos.

import type { BiomaTipo, TerrenoTipo } from '../domain/types';

function congelar<T>(obj: T): T {
  for (const valor of Object.values(obj as Record<string, unknown>)) {
    if (valor && typeof valor === 'object') congelar(valor);
  }
  return Object.freeze(obj);
}

/**
 * Tamaño por defecto del mapa. Doc 1.1: mapa CUADRADO, espacio continuo, parametrizable.
 * 2000x2000 (2x lineal / 4x área) a partir de Fase 0.1: el mapa de 1000x1000 no daba sitio para que se
 * notaran varias formaciones de relieve distintas (la octava más baja de `ELEVACION` tiene una longitud de
 * onda de ~1571 unidades, más que la diagonal del mapa viejo) y los bosques/ríos quedaban visualmente
 * apretados. `ELEVACION`/`FERTILIDAD` NO cambian de escala a propósito: el mapa más grande es lo que hace
 * que quepan más "colinas" de ruido dentro del mundo, no un reescalado del ruido en sí.
 */
export const MAPA_DEFAULT: { ancho: number; alto: number } = congelar({
  ancho: 2000,
  alto: 2000,
});

// Trigo NO genera nodo: depende del campo de fertilidad (ver FERTILIDAD) + Granja.
// Madera TAMPOCO genera nodo propio (Doc 1.4: "proviene de BOSQUES, representados como ZONAS, no puntos") —
// solo se generaban aquí por error de implementación (Sprint 1): un nodo "madera" sin ningún uso en el motor
// (la lenera siempre lee de los bosques, nunca de los nodos), y visualmente confundible con los bosques.
//
// Cantidades ×4 respecto al mapa 1000x1000 original (área ×4 con `MAPA_DEFAULT` a 2000x2000) para mantener
// la MISMA densidad de recursos por unidad de área — sin este ajuste, doblar el mapa sin tocar las
// cantidades absolutas los habría dejado 4x más dispersos, justo lo contrario de lo buscado.
// `espacioMinimo` NO se escala: es una distancia (unidades de mapa), no una densidad — si se duplicara junto
// con la cantidad, el área que cada punto "reclama" (∝ espacioMinimo²) crecería x4 y la cantidad de puntos
// también x4, exigiendo 4x más área de la que el mapa 4x más grande realmente aporta. Dejarlo fijo preserva
// la misma dificultad de encaje del rejection-sampling que ya estaba calibrada (ver `ELEVACION`).
export const RECURSO_RAREZA = congelar({
  comun: { cantidadBase: 240, espacioMinimo: 20 },
  intermedio: { cantidadBase: 80, espacioMinimo: 40 },
  raro: { cantidadBase: 24, espacioMinimo: 120 },
} as const);

export const RECURSO_TIPOS_POR_RAREZA: Record<keyof typeof RECURSO_RAREZA, string[]> = congelar({
  comun: ['piedra'],
  intermedio: ['cobre'],
  raro: ['estano', 'oro'],
});

/**
 * Piedra/oro recalibrados (overhaul de auto-construcción, verificación batch): Mantenimiento cobra piedra
 * desde nivel 2 y oro desde nivel 3 de forma PERPETUA (todos los ticks, para siempre), pero los nodos son
 * finitos — ningún ajuste de tasa de extracción "resuelve" esto del todo, solo compra tiempo (el jugador
 * real tiene comercio/trueque para importar lo que le falte). El diagnóstico mostró que con los valores
 * viejos (piedra 200-500) el problema NO era la tasa (una sola Cantera a 5/tick ya supera cómodamente el
 * costo de Mantenimiento de nivel 2, ~3.45-6.9/tick) sino que el nodo se agotaba en 40-100 ticks y ahí
 * quedaba en 0 para siempre — nodos ×3 dan un margen mucho más realista antes de necesitar una segunda
 * fuente o comercio. Oro sí tenía además un problema de TASA (ver `produccionBaseOro` en EDIFICIO_CATALOGO):
 * nodos ×4-5 (ya de por sí "raros", RECURSO_RAREZA.raro) para la misma razón que piedra.
 */
export const RECURSO_CANTIDAD_NODO = congelar({
  piedra: { min: 600, max: 1200 },
  cobre: { min: 100, max: 300 },
  estano: { min: 50, max: 150 },
  oro: { min: 150, max: 400 },
} as const);

// Livestock: fauna libre, no sigue las mismas reglas de rareza (no ligada a minerales). cantidadBase ×4,
// espacioMinimo sin tocar — mismo criterio de densidad que RECURSO_RAREZA.
export const LIVESTOCK = congelar({
  cantidadBase: 100,
  espacioMinimo: 30,
  cantidadPorManada: { min: 10, max: 40 },
});

// cantidad ×4 (misma densidad de bosques por área); radio/densidad de cada bosque individual no cambian —
// el tamaño de UN bosque no depende del tamaño del mapa.
// Subido en Fase 0.4 (cantidad 100->170, radio 30-80->45-120, densidadMin 0.4->0.55): pedido explícito de
// diseño de que el mapa se vea "más cubierto de bosques en las zonas que le toca" — más círculos, más
// grandes y de base más densa, así que se solapan más dentro de `BOSQUE_TERRENO_PERMITIDO` (llano/colina) en
// vez de quedar como manchas dispersas. No cambia DÓNDE se permiten (sigue siendo solo llano/colina, por
// terreno) ni el criterio de densidad (sigue ponderando fertilidad, ver `generarBosques`), solo cuánto/cuán
// grande sale cada uno.
export const BOSQUE = congelar({
  cantidad: 170,
  radioMin: 45,
  radioMax: 120,
  densidadMin: 0.55,
  densidadMax: 1.0,
});

/** Bandas de terreno donde se permite el CENTRO de un bosque (Fase 0.1) — elevación media-baja. Solo
 * filtro de elevación: no se exige cercanía a río como filtro duro, forzarlo agotaría el rejection-sampling
 * casi siempre y degeneraría en "coloca igual" (ver `colocarConEspaciado`). */
export const BOSQUE_TERRENO_PERMITIDO: TerrenoTipo[] = congelar(['llano', 'colina']);

// Fertilidad: ruido fractal de gradiente (ver `worldgen/ruido.ts`). Menos octavas y formación base más
// grande que la elevación — la fertilidad son manchas amplias de suelo bueno/malo, no terreno accidentado.
// `escala` es 1/tamaño de la formación más gruesa: 0.002 ≈ manchas de ~500 unidades sobre un mapa de 2000.
export const FERTILIDAD = congelar({
  octavas: 3,
  escala: 0.002,
  lacunaridad: 2,
  persistencia: 0.5,
});

// Elevación (Fase 0.1): ruido fractal de gradiente (ver `worldgen/ruido.ts`). `escala` es 1/tamaño de la
// formación más gruesa — 0.00167 ≈ cordilleras/valles de ~600 unidades sobre un mapa de 2000 — y cada
// octava añade detalle a la mitad de escala (600, 300, 150, 75, 37 unidades) con la mitad de peso. Cinco
// octavas es el punto donde el relieve tiene textura fina sin que el detalle llegue a mover las bandas de
// terreno: la octava más fina pesa 1/16, o sea ±0.03 de elevación.
//
// Umbrales de `evaluarTerreno` CALIBRADOS contra la distribución real del campo, no repartidos por el rango
// nominal 0-1. El ruido fractal se concentra alrededor de 0.5 (la normalización usa la cota teórica, ver
// `evaluarRuido`), así que umbrales "razonables a ojo" caerían casi todos en percentiles extremos: una banda
// de montaña en el percentil 99 sería una franja casi puntual donde no caben los 120 de
// `RECURSO_RAREZA.raro.espacioMinimo` que piden los nodos de oro/estaño, y todos acabarían en el fallback
// de "mapa saturado, coloca igual" (ver `colocarConEspaciado`) — la colocación condicionada dejaría de
// tener efecto real.
export const ELEVACION = congelar({
  octavas: 5,
  escala: 0.00167,
  lacunaridad: 2,
  persistencia: 0.5,
  // Percentiles observados (medidos sobre rejilla 400², seeds 1/42/7 — muy estables entre seeds):
  // agua ~7%, costa ~11%, llano ~50%, colina ~18%, montaña ~11%, cima ~3%.
  umbralAgua: 0.375,
  umbralCosta: 0.42,
  umbralColina: 0.545,
  umbralMontana: 0.6,
  // Franja MÁS alta del campo: cima inhabitable — no se puede fundar ni extraer ahí (ver
  // settlement.ts/construction.ts). Deliberadamente estrecha: la banda 'montana' de abajo sigue siendo
  // amplia y minable, esto solo recorta la punta.
  umbralCima: 0.685,
});

// Suavizado de espacios jugables (Fase 0.4.2 — reemplaza por completo el terraceo de Fase 0.4.1
// (`ELEVACION_TERRAZAS`, retirado): mesetas de altura CONSTANTE se veían bien en el heightmap pero no eran
// lo que pedía la referencia real (captura del mapa de campaña de Total War: Troy, ver
// `Consideraciones/Fase_0_4_Definicion_Relieve_Jugable.md`) — ahí el terreno jugable es ONDULADO y CONTINUO,
// sin escalones visibles; y el terraceo además atascaba ríos en las mesetas perfectamente planas (gradiente
// ~0, ver `RIOS.gradienteMinimo`).
//
// En vez de cuantizar el VALOR de elevación, `evaluarElevacion` mezcla el ruido completo con una versión
// del MISMO ruido evaluada con menos octavas (`octavasSuaves`, ver `evaluarRuidoParcial` en `ruido.ts`) —
// quita el detalle fino (las octavas de longitud de onda corta, ~37-150 unidades, que se leían como bultos
// accidentados) pero conserva la ondulación ancha (~300-600 unidades) de las octavas gruesas, así que el
// gradiente nunca es exactamente cero en ningún punto: los ríos siguen teniendo pendiente real que seguir.
// Investigado (ver el documento de arriba): es la técnica de "redistribución/reducción de detalle" que usa
// la industria para terreno jugable de aspecto natural (Red Blob Games, *Making Maps with Noise
// Functions*), más simple que simular erosión hidráulica de verdad — que exigiría una rejilla horneada,
// rompiendo el contrato de campo continuo evaluable en cualquier punto (`Fase_0_1_Definicion.md`).
//
// `octavasSuaves: 2` dejan las dos octavas más gruesas de las 5 de `ELEVACION` — la forma ancha del relieve,
// sin las 3 octavas finas. `pesoMaximo` NO es 1.0 a propósito: incluso en el centro de la banda jugable
// queda un 10% del ruido completo mezclado, para que de cerca no se note un cambio de textura demasiado
// limpio/artificial entre el mundo suavizado y el terreno de montaña sin tocar.
export const ELEVACION_SUAVIZADO = congelar({
  octavasSuaves: 2,
  pesoMaximo: 0.9,
});

// Borde natural del mundo LIBRE (Fase 0.4, sin región — ver `elevacion.ts`): sin esto, los bordes del mapa
// eran un corte arbitrario del ruido fractal, tan probable que cayera en llano como en agua o montaña. Un
// segundo campo de ruido, de frecuencia mucho más gruesa que `ELEVACION` (para que decida en tramos largos
// del perímetro, no punto a punto — como las bahías/crestas de `regiones.ts`), decide si cada tramo del borde
// es costa profunda o pared de montaña; el peso decae suavemente desde el borde hacia adentro (`anchoFraccion`
// del tamaño del mapa) para que la transición no sea un muro artificial de un pixel. Solo se genera/aplica
// para el mundo libre: las regiones autoradas de `REGIONES` ya definen su propio borde a mano (p. ej. los
// ríos troncales de Nilo/Mesopotamia cruzan el borde norte/sur — un borde genérico de montaña ahí rompería
// ese diseño). `objetivoMontana` muy alto (por encima de `umbralCima`) y `objetivoCosta` muy bajo (por debajo
// de `umbralAgua`) a propósito: el borde debe leerse inequívocamente como "empinado"/"profundo", no como una
// colina o una playa más.
export const ELEVACION_BORDE = congelar({
  anchoFraccion: 0.09,
  octavas: 2,
  escala: 0.0012,
  lacunaridad: 2,
  persistencia: 0.5,
  objetivoCosta: 0.12,
  objetivoMontana: 0.97,
});

// Ríos (Fase 0.1): nacen en montaña y descienden por gradiente de máxima pendiente (ver worldgen/rios.ts).
// `cantidad` ×4 (misma densidad de ríos por área que el mapa 1000x1000 original); `espacioMinimoEntreNacimientos`
// sin tocar, mismo criterio de densidad que RECURSO_RAREZA. `pasosMax`=700 * `pasoDescenso`=8 = 5600, más que
// la diagonal de un mapa 2000x2000 (~2828): cubre el peor caso sin dejar caminatas sin terminar.
// `pasoGradiente`=50 mide la pendiente sobre una distancia MAYOR que las octavas finas del relieve (la más
// fina son formaciones de ~37 unidades, ver `ELEVACION`): así el cauce sigue la forma general del valle en
// vez de quedar atrapado en cada hoyo del detalle fractal. `gradienteMinimo` es el umbral por debajo del
// cual se considera que ya no hay pendiente clara y el río termina en lago (la mediana del gradiente del
// campo es ~1e-3, así que 2e-4 solo detiene un cauce en un extremo local de verdad).
// Navegabilidad (comercio fluvial de fases futuras, ver `RioZona.navegable`): candidato = desemboca de
// verdad (`!terminaEnLago`, no un charco atrapado en una hondonada de montaña) — de esos, el 40% MÁS LARGO
// se marca navegable. Se eligió PROPORCIONAL al número de ríos con desembocadura de ese mundo, no un umbral
// de longitud fijo: medido sobre 7 seeds, el número de ríos con desembocadura real varía mucho de un mundo a
// otro (6 a 18 de los 24 generados), así que un umbral absoluto daba de 2 a 17 navegables según la seed —
// muy lejos de "algunos, no todos" en varias de ellas. La proporción da un resultado consistente (~2-7
// navegables) sea cual sea la seed. Ancho/profundidad son metros de exportación (tallado del heightmap en
// fases futuras, ver Doc de export a Unity/Godot), no afectan nada del motor 2D actual.
export const RIOS = congelar({
  cantidad: 24,
  espacioMinimoEntreNacimientos: 150,
  pasoDescenso: 8,
  pasoGradiente: 50,
  gradienteMinimo: 0.0002,
  pasosMax: 700,
  proporcionNavegable: 0.4,
  anchoMetros: 10,
  profundidadMetros: 3,
  anchoNavegableMetros: 30,
  profundidadNavegableMetros: 6,
});

// Bioma (Fase 0.1): terreno llano se reparte entre estepa/llanuraFertil por fertilidad alta o cercanía a
// río (humedad) — el resto de bandas de terreno (agua/costa/colina/montana) SON el bioma, sin más criterio.
// `umbralFertilLlanura`=0.52 ≈ percentil 57 del campo de fertilidad medido: algo menos de la mitad del
// llano sale fértil, más lo que gane por cercanía a río.
export const BIOMA = congelar({
  umbralFertilLlanura: 0.52,
  radioHumedadRio: 40,
});

/**
 * Biomas donde SE PERMITE que aparezca cada tipo de recurso (colocación condicionada al terreno, Fase 0.1).
 * Piedra se deja permisivo a propósito: es el recurso común (60/seed) y no debe faltar. Los metales
 * (cobre/estaño/oro) quedan restringidos a colina/montaña — "metales en montaña" pedido explícitamente.
 * Oro se deja SOLO en montaña (el más exclusivo, es el más raro); cobre/estaño se permiten también en
 * colina porque, con `RECURSO_RAREZA.raro.espacioMinimo`=120 y solo 6 nodos por tipo, restringir dos
 * tipos a la vez a la banda de montaña (más pequeña) hacía que casi todos cayeran en el fallback de mapa
 * saturado sin más sitio donde encajar — el filtro dejaba de tener efecto real.
 * El conteo por tipo sigue siendo `RECURSO_RAREZA[rareza].cantidadBase`: esta tabla solo condiciona DÓNDE
 * caen, no cuántos hay (ver `colocarConEspaciado`, que mantiene su fallback de mapa saturado).
 */
export const RECURSO_BIOMA_PERMITIDO: Record<string, BiomaTipo[]> = congelar({
  piedra: ['colina', 'montana', 'llanuraFertil', 'estepa'],
  cobre: ['colina', 'montana'],
  estano: ['colina', 'montana'],
  oro: ['montana'],
  livestock: ['llanuraFertil', 'estepa'],
});

/** Intentos de rejection sampling por punto antes de rendirse y colocarlo igualmente (ver `colocacion.ts`).
 * Subido de 30 a 40 en Fase 0.1: la colocación condicionada al terreno reduce el área válida por candidato
 * (bioma + espaciado a la vez), y unos intentos más baratos de más reducen cuánto se cae al fallback. */
export const COLOCACION = congelar({
  intentosPorPunto: 40,
});

// Coste de movimiento por terreno (Fase 0.3, ver `worldgen/costeMovimiento.ts`): multiplicador sobre la
// velocidad base de una caravana (1 = sin penalización). agua/cima muy altos mas no INFINITY a propósito:
// un coste finito, aunque prohibitivo, deja que A* (`world/rutas.ts`) SIEMPRE encuentre algún camino en vez
// de fallar cuando el mapa obliga a cruzar un borde de agua/cima estrecho — se prefiere una ruta carísima
// (y por tanto evitada casi siempre por el propio algoritmo) a un pathfinding que pueda no converger.
// Cifras PLACEHOLDER sin calibrar por simulación todavía, mismo criterio que el resto de constantes nuevas.
export const COSTE_MOVIMIENTO = congelar({
  llano: 1,
  costa: 1.1,
  colina: 1.8,
  montana: 3.5,
  cima: 12,
  agua: 15,
  /** Multiplicador (<1 = más rápido) mientras la posición está sobre/cerca de un `CaminoComercial` (Doc
   * 1.6, ver `engine/caminos.ts`) — el camino ya construido compensa el coste del terreno que atraviesa. */
  factorCamino: 0.5,
  /** Distancia máxima a un tramo de camino comercial para contar como "sobre el camino" (ver
   * `distanciaASegmento` en `worldgen/colocacion.ts`). */
  radioCamino: 15,
});

