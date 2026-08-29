// Vocabulario temporal del dominio (Fase D, Docs/Arquitectura/10_Modelo_Temporal.md §5).
//
// `Instante` y `Duracion` son `number` (ms) con marca de tipo: el compilador ya no deja sumar dos instantes,
// ni confundir un instante de mundo con un `tick` ordinal, ni pasar una duración donde se espera un momento.
// Es la red que hace la migración de la Fase D verificable — al renombrar un campo `*EnTick: number` a
// `*En: Instante`, `tsc` señala TODOS los sitios que aún lo tratan como tick.
//
// Módulo hoja: cero imports. La conversión `tick -> Instante` NO vive aquí (necesita `SIMULACION` de
// `constants`, que `domain` no puede ver) — está en `session/estado.ts` (`instanteDeTick`). Los plazos del
// juego se declaran ya en minutos en `constants.ts` y el motor los pasa por `minutos()` de aquí (D6, doc 10
// §6). La conversión `Instante <-> ISO 8601` tampoco vive aquí (necesita `Date`, prohibido en el núcleo puro)
// — está en `session/estado.ts` (`isoDeInstante`) y en la frontera HTTP (`server/`).

/** Momento de MUNDO en ms desde la época Unix. Nunca es un `tick` ni una `Duracion`. */
export type Instante = number & { readonly __marca: 'Instante' };

/** Lapso de tiempo de mundo en ms. */
export type Duracion = number & { readonly __marca: 'Duracion' };

/** Marca un `number` de ms como `Instante`. Usar solo en la frontera (deserialización, `instanteDeTick`). */
export function instante(ms: number): Instante {
  return ms as Instante;
}

/** Marca un `number` de ms como `Duracion`. */
export function duracion(ms: number): Duracion {
  return ms as Duracion;
}

export function minutos(n: number): Duracion {
  return (n * 60_000) as Duracion;
}

export function dias(n: number): Duracion {
  return (n * 86_400_000) as Duracion;
}

/** Instante resultante de dejar pasar `d` desde `i`. */
export function sumar(i: Instante, d: Duracion): Instante {
  return (i + d) as Instante;
}

/** Tiempo transcurrido entre dos instantes (`hasta - desde`). Negativo si `hasta` es anterior. */
export function transcurrido(desde: Instante, hasta: Instante): Duracion {
  return (hasta - desde) as Duracion;
}
