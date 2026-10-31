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

/** Multiplicador de coste de movimiento por terreno (1 = sin penalización). */
export const COSTE_MOVIMIENTO = congelar({
  llano: 1,
  costa: 1.1,
  colina: 1.8,
  montana: 3.5,
  cima: 12,
  agua: 15,
});
