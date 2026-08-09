// Parámetros de GENERACIÓN de mundo. Viven aquí y no en `constants.ts` por una razón de contrato, no de
// orden: el resto de `constants.ts` es balance ajustable en caliente desde la interfaz, y la generación NO
// puede serlo. La partida guardada solo almacena la seed y regenera el mapa al cargar, así que si estos
// números pudieran cambiarse en vivo, un save hecho antes del cambio se recuperaría como un mundo distinto
// —silenciosamente— al reabrirlo. "Determinista por seed" y "editable en caliente" son incompatibles;
// mandan estos valores fijos. Para cambiarlos hay que tocar este archivo y subir `WORLDGEN_VERSION`.
//
// Se congelan en tiempo de ejecución (no solo con `as const`, que es un candado de tipos) para que ninguna
// vía dinámica —como la que usa `app/balanceConfig.ts` para el resto de constantes— pueda escribirlos.

function congelar<T>(obj: T): T {
  for (const valor of Object.values(obj as Record<string, unknown>)) {
    if (valor && typeof valor === 'object') congelar(valor);
  }
  return Object.freeze(obj);
}

/** Tamaño por defecto del mapa. Doc 1.1: mapa CUADRADO, espacio continuo, parametrizable. */
export const MAPA_DEFAULT: { ancho: number; alto: number } = congelar({
  ancho: 1000,
  alto: 1000,
});

// Trigo NO genera nodo: depende del campo de fertilidad (ver FERTILIDAD) + Granja.
// Madera TAMPOCO genera nodo propio (Doc 1.4: "proviene de BOSQUES, representados como ZONAS, no puntos") —
// solo se generaban aquí por error de implementación (Sprint 1): un nodo "madera" sin ningún uso en el motor
// (la lenera siempre lee de los bosques, nunca de los nodos), y visualmente confundible con los bosques.
// Cantidad de nodos por 1000x1000 y espaciado mínimo entre nodos de la misma rareza (unidades de mapa).
export const RECURSO_RAREZA = congelar({
  comun: { cantidadBase: 60, espacioMinimo: 20 },
  intermedio: { cantidadBase: 20, espacioMinimo: 40 },
  raro: { cantidadBase: 6, espacioMinimo: 120 },
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

// Livestock: fauna libre, no sigue las mismas reglas de rareza (no ligada a minerales).
export const LIVESTOCK = congelar({
  cantidadBase: 25,
  espacioMinimo: 30,
  cantidadPorManada: { min: 10, max: 40 },
});

export const BOSQUE = congelar({
  cantidad: 25,
  radioMin: 30,
  radioMax: 80,
  densidadMin: 0.4,
  densidadMax: 1.0,
});

// Fertilidad: ruido continuo por suma de funciones seno con distintas frecuencias (sin dependencias externas).
export const FERTILIDAD = congelar({
  octavas: 3,
  escala: 0.006,
});

/** Intentos de rejection sampling por punto antes de rendirse y colocarlo igualmente (ver `colocacion.ts`). */
export const COLOCACION = congelar({
  intentosPorPunto: 30,
});
