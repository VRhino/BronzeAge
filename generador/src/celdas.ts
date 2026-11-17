import type { Point } from './tipos';
import type { CampoElevacion } from './elevacion';
import type { CampoForestal } from './forestacionRegional';
import { densidadForestal } from './forestacionRegional';
import { esTransitable } from './costeMovimiento';
import { muestrearRelieve } from './muestras';

/** Rejilla de celdas con el relieve de sus centros. */
export interface CeldasMuestreadas {
  columnas: number; filas: number; lado: number;
  altura: Float64Array; pendiente: Float64Array; humedad: Float64Array; densidad: Float64Array;
  transitable: Uint8Array;
  /** Índice de la celda cuyo centro es exactamente `p`, o -1 si `p` no es el centro de ninguna. */
  indice(p: Point): number;
}

/**
 * Evalúa una sola vez el relieve en el centro de cada celda de `lado` unidades. Los centros son `((i % columnas) + 0.5) * lado`,
 * los mismos que usan las parcelas, la transitabilidad del grafo y el artefacto: así comparten el cálculo sin cambiar un bit.
 */
export function muestrearCeldas(relieve: { elevacion: CampoElevacion; forestacion: CampoForestal },
  limites: { ancho: number; alto: number }, lado: number): CeldasMuestreadas {
  const columnas = Math.ceil(limites.ancho / lado), filas = Math.ceil(limites.alto / lado), n = columnas * filas;
  const altura = new Float64Array(n), pendiente = new Float64Array(n), humedad = new Float64Array(n), densidad = new Float64Array(n);
  const transitable = new Uint8Array(n);
  for (let i = 0; i < n; i++) {
    const centro = { x: ((i % columnas) + 0.5) * lado, y: (Math.floor(i / columnas) + 0.5) * lado };
    const m = muestrearRelieve(relieve.elevacion, centro);
    altura[i] = m.altura; pendiente[i] = m.pendiente; humedad[i] = m.humedad;
    densidad[i] = densidadForestal(relieve.forestacion, relieve.elevacion, centro, m);
    transitable[i] = esTransitable(relieve.elevacion, centro, m.altura) ? 1 : 0;
  }
  const indice = (p: Point): number => {
    const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado);
    if (cx < 0 || cy < 0 || cx >= columnas || cy >= filas) return -1;
    return p.x === (cx + 0.5) * lado && p.y === (cy + 0.5) * lado ? cy * columnas + cx : -1;
  };
  return { columnas, filas, lado, altura, pendiente, humedad, densidad, transitable, indice };
}
