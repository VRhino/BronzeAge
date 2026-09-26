import type { Edificio, RecursoAlmacenado } from '../domain/types';
import { EDIFICIO_CATALOGO } from '../constants';

export function tieneRecursos(almacen: Record<string, RecursoAlmacenado>, costo: Partial<Record<string, number>>): boolean {
  return Object.entries(costo).every(([recurso, cantidad]) => (almacen[recurso]?.cantidad ?? 0) >= (cantidad ?? 0));
}

export function descontarRecursos(
  almacen: Record<string, RecursoAlmacenado>,
  costo: Partial<Record<string, number>>
): Record<string, RecursoAlmacenado> {
  const nuevo = { ...almacen };
  for (const [recurso, cantidad] of Object.entries(costo)) {
    const actual = nuevo[recurso];
    if (!actual) continue;
    const resto = actual.cantidad - (cantidad ?? 0);
    // Residuo de coma flotante, no un gasto de más: las recetas consumen `porUnidad × (disponible / porUnidad)`,
    // que puede pasarse de `disponible` en un ulp y dejar el almacén en -4.4e-16 (lo cazó el test de
    // invariantes al mover el sitio de fundación de los tests, 2026-09-26). Solo se absorbe lo que cabe en
    // `EPSILON_ALMACEN`: un descuento de verdad excesivo sigue saliendo negativo, y los invariantes lo ven.
    nuevo[recurso] = { ...actual, cantidad: resto < 0 && resto > -EPSILON_ALMACEN ? 0 : resto };
  }
  return nuevo;
}

/** Por debajo de esto, un negativo en el almacén es ruido de coma flotante. Las cantidades del juego son de
 * unidades a miles; el error relativo de un double ronda 1e-16, así que 1e-9 deja nueve órdenes de margen. */
const EPSILON_ALMACEN = 1e-9;

export function agregarRecurso(
  almacen: Record<string, RecursoAlmacenado>,
  recurso: string,
  cantidad: number
): Record<string, RecursoAlmacenado> {
  const actual = almacen[recurso] ?? { cantidad: 0, capacidad: 0 };
  return { ...almacen, [recurso]: { ...actual, cantidad: Math.min(actual.capacidad, actual.cantidad + cantidad) } };
}

/**
 * Igual que `agregarRecurso`, pero además reporta cuánto de `cantidad` no cupo en el almacén (capacidad ya
 * llena) — instrumentación de excedente (Doc 4.2): antes ese sobrante se perdía en silencio por el
 * `Math.min` de `agregarRecurso`. `sobrante` es 0 si todo cupo.
 */
export function agregarRecursoConSobrante(
  almacen: Record<string, RecursoAlmacenado>,
  recurso: string,
  cantidad: number
): { almacen: Record<string, RecursoAlmacenado>; sobrante: number } {
  const actual = almacen[recurso] ?? { cantidad: 0, capacidad: 0 };
  const nuevaCantidad = Math.min(actual.capacidad, actual.cantidad + cantidad);
  const sobrante = actual.cantidad + cantidad - nuevaCantidad;
  return { almacen: { ...almacen, [recurso]: { ...actual, cantidad: nuevaCantidad } }, sobrante };
}

export function cantidadDisponible(almacen: Record<string, RecursoAlmacenado>, recurso: string): number {
  return almacen[recurso]?.cantidad ?? 0;
}

/**
 * Cambia la CAPACIDAD de un recurso en `extra` (negativo la reduce). La mueven los dos edificios de almacenaje
 * al entrar y salir de servicio (`aplicarCapacidadDeEdificio`) y el Granero en cada mejora de nivel, con el
 * delta contra el nivel anterior.
 *
 * Un recurso que todavía no exista en el almacén se crea con cantidad 0: la capacidad puede llegar antes que
 * el primer grano. Si la capacidad baja por debajo de lo guardado, lo que no cabe se pierde.
 */
export function ampliarCapacidad(
  almacen: Record<string, RecursoAlmacenado>,
  recurso: string,
  extra: number
): Record<string, RecursoAlmacenado> {
  if (extra === 0) return almacen;
  const actual = almacen[recurso] ?? { cantidad: 0, capacidad: 0 };
  const capacidad = Math.max(0, actual.capacidad + extra);
  return { ...almacen, [recurso]: { cantidad: Math.min(actual.cantidad, capacidad), capacidad } };
}

/** Capacidad de trigo que aporta un Granero en `nivelInterno` — TOTAL, no incremental (ver
 * `EDIFICIO_CATALOGO.granero`). Un nivel inexistente da 0, que es lo que hace que el delta de una mejora
 * salga bien sin casos especiales. */
export function capacidadTrigoDeGranero(nivelInterno: number | undefined): number {
  return EDIFICIO_CATALOGO.granero.niveles[nivelInterno ?? 1]?.capacidadTrigo ?? 0;
}

/**
 * Suma (`signo` 1) o quita (`signo` -1) la capacidad que aporta un edificio de almacenaje: el Almacén a todos
 * los recursos, el Granero al trigo según su nivel interno. Se suma al entrar en servicio (obra o reconstrucción
 * terminada) y se quita al dejar de estarlo (dañado en un saqueo), así la capacidad sigue siempre a los
 * edificios que funcionan. Antes solo se sumaba, y cada conquista seguida de reconstrucción la inflaba para
 * siempre (batch de la Era I, 2026-09-26: 302 800 por recurso con 8 Almacenes, en vez de 2 800).
 */
export function aplicarCapacidadDeEdificio(
  almacen: Record<string, RecursoAlmacenado>,
  edificio: Pick<Edificio, 'tipo' | 'nivelInterno'>,
  signo: 1 | -1
): Record<string, RecursoAlmacenado> {
  if (edificio.tipo === 'almacen') {
    let resultado = almacen;
    for (const recurso of Object.keys(almacen)) {
      resultado = ampliarCapacidad(resultado, recurso, signo * EDIFICIO_CATALOGO.almacen.capacidadPorRecursoAdicional);
    }
    return resultado;
  }
  if (edificio.tipo === 'granero') return ampliarCapacidad(almacen, 'trigo', signo * capacidadTrigoDeGranero(edificio.nivelInterno));
  return almacen;
}
