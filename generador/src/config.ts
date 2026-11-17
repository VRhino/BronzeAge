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

/** Capas de fertilidad de la vid, la oliva, la cebada y el sésamo (Tanda Economía, Héroe y Cultivos §12). Cada capa es el producto de factores 0..1, cada
 * uno una rampa suave desde → hasta; `altura` va en elevación normalizada 0-1, `pendiente` en m/m, `humedad` en 0..1 y las distancias
 * en unidades del generador (1 u = 1 m). Ambas valen 0 en el agua; la vid, también en las cumbres (su `alturaBaja.hasta` es `ELEVACION.umbralCima`). */
export const CULTIVOS = congelar({
  vid: {
    /** Laderas: sube de la tierra plana a la pendiente suave y cae hacia la escarpada. */
    pendienteSube: { desde: 0.02, hasta: 0.1 },
    pendienteBaja: { desde: 0.3, hasta: 0.55 },
    /** Seca o a medias: pierde de la humedad media a la alta. */
    humedadBaja: { desde: 0.35, hasta: 0.7 },
    alturaBaja: { desde: 0.6, hasta: 0.685 },
  },
  oliva: {
    /** Tierra baja: pierde de la llanura a la colina. */
    alturaBaja: { desde: 0.45, hasta: 0.57 },
    /** Cerca del mar: pleno hasta `desde` unidades de la costa y nada a `hasta`. */
    distanciaCosta: { desde: 1500, hasta: 6000 },
    humedadBaja: { desde: 0.4, hasta: 0.75 },
    pendienteBaja: { desde: 0.1, hasta: 0.4 },
  },
  /** Cebada y sésamo (2026-11-09): el trigo en la tierra húmeda donde la vid y la oliva no crecen. `humedadSube` es de las dos; la cebada
   * pide además altura (`alturaSube`: tierra húmeda y alta) y el sésamo la contraria (`alturaBaja`: tierra húmeda y baja, en todo el mapa). */
  cebada: {
    humedadSube: { desde: 0.55, hasta: 0.65 },
    alturaSube: { desde: 0.45, hasta: 0.48 },
  },
  sesamo: {
    humedadSube: { desde: 0.55, hasta: 0.65 },
    alturaBaja: { desde: 0.45, hasta: 0.48 },
  },
});
