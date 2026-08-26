// Copia de los umbrales de `src/worldgen/config.ts` del backend que necesita la evaluación por punto — no
// toda la config de generación (bosques, nodos, ríos: eso lo decide el servidor al generar, este cliente
// nunca genera nada). Cambia solo cuando `WORLDGEN_VERSION` sube en el servidor y afecta a estos umbrales —
// ver el README para la disciplina de mantenimiento.

export const ELEVACION = {
  umbralAgua: 0.375,
  umbralCosta: 0.42,
  umbralColina: 0.545,
  umbralMontana: 0.6,
  umbralCima: 0.685,
} as const;

export const ELEVACION_SUAVIZADO = {
  octavasSuaves: 2,
  pesoMaximo: 0.9,
} as const;

export const BIOMA = {
  umbralFertilLlanura: 0.52,
  radioHumedadRio: 40,
} as const;
