// Parámetros del relieve que comparten todas las capas. Cambiarlos cambia el artefacto (otro hash).
// Se congelan en tiempo de ejecución, no solo con `as const`.

function congelar<T>(obj: T): T {
  for (const valor of Object.values(obj as Record<string, unknown>)) {
    if (valor && typeof valor === 'object') congelar(valor);
  }
  return Object.freeze(obj);
}

/** Umbrales de `evaluarTerreno` sobre la elevación normalizada 0-1. */
export const ELEVACION = congelar({
  umbralAgua: 0.375,
  umbralCosta: 0.42,
  umbralColina: 0.545,
  umbralMontana: 0.6,
  umbralCima: 0.685,
});

/** Contrato estratégico 3D: X/Z en unidades de mapa, Y en metros. */
export const ESCALA_TERRENO = congelar({
  metrosHorizontalesPorUnidad: 1,
  metrosVerticalesPorElevacion: 1000,
  pendienteMaximaTransitable: 0.7,
  pasoPendiente: 12,
});

/** Multiplicador de coste de movimiento (1 = llano sin pendiente). Desde el 2026-10-31 (decisión del autor) frena solo la
 * pendiente, no la altura: `1 + min(topePendiente, porPendiente · pendiente)`, con la pendiente en m/m. Con 35° (0,7, la
 * máxima transitable) vale 3,5, lo que antes costaba la montaña llana. El agua no se pisa; su valor es el de siempre. */
export const COSTE_MOVIMIENTO = congelar({
  porPendiente: 3.6,
  topePendiente: 2.5,
  agua: 15,
});
