import type { RecursoAlmacenado } from '../domain/types';

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
    if (actual) nuevo[recurso] = { ...actual, cantidad: actual.cantidad - (cantidad ?? 0) };
  }
  return nuevo;
}

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
