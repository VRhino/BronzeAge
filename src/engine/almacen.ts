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

export function cantidadDisponible(almacen: Record<string, RecursoAlmacenado>, recurso: string): number {
  return almacen[recurso]?.cantidad ?? 0;
}
