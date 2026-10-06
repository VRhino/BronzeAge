// Repartir un botín entre las columnas del bando que gana (Doc 5.12.3): a partes iguales, y lo que a una no le cabe
// pasa a las que aún tengan sitio. Lo que no cabe en ninguna se pierde.
import type { Ejercito } from '../domain/types';
import { cargarBotin } from './ejercitos';

const EPSILON = 1e-9;

const llevado = (c: Ejercito): number => Object.values(c.suministro).reduce((suma, cantidad) => suma + cantidad, 0);

/**
 * `capacidad` es lo que le cabe a cada columna en total (`capacidadCargaDe`). Cada ronda reparte lo pendiente a partes
 * iguales entre las que aún tienen sitio; una que se llena sale de la siguiente. Termina en, como mucho, tantas rondas
 * como columnas: en cada una o se reparte todo o se llena alguna.
 */
export function repartirBotin<E extends Ejercito>(columnas: readonly E[], botin: Readonly<Record<string, number>>, capacidad: (columna: E) => number): E[] {
  let actuales = [...columnas];
  const pendiente: Record<string, number> = { ...botin };
  for (let ronda = 0; ronda < columnas.length; ronda++) {
    const conSitio = actuales.map((c, i) => (capacidad(c) - llevado(c) > EPSILON ? i : -1)).filter((i) => i >= 0);
    if (conSitio.length === 0 || Object.values(pendiente).every((cantidad) => cantidad <= EPSILON)) break;
    const parte = Object.fromEntries(Object.entries(pendiente).map(([recurso, cantidad]) => [recurso, cantidad / conSitio.length]));
    for (const i of conSitio) {
      const antes = actuales[i]!;
      const despues = cargarBotin(antes, parte, capacidad(antes));
      for (const recurso of Object.keys(pendiente)) pendiente[recurso]! -= (despues.suministro[recurso] ?? 0) - (antes.suministro[recurso] ?? 0);
      actuales[i] = despues;
    }
  }
  return actuales;
}
