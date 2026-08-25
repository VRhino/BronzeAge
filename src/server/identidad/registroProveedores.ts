// Registro de proveedores de identidad por ESQUEMA — mismo patrón que `session/comandos/registro.ts` para
// comandos: un único lugar que enumera los adaptadores activos, para que `servicioAutenticacion.ts` no
// conozca ninguno de ellos por nombre. Añadir un proveedor nuevo es añadirlo a `proveedoresPorDefecto`;
// retirar uno (ej. quitar el de desarrollo antes de producción) es borrar esa línea, sin tocar nada más.
import type { ProveedorIdentidad } from './proveedorIdentidad';
import { proveedorDesarrollo } from './proveedorDesarrollo';

/** Proveedores activos del proceso real. Hoy solo el de desarrollo — el único punto que hay que tocar
 * cuando se integre un proveedor de verdad (añadirlo aquí) y, más adelante, cuando se retire el de
 * desarrollo (quitarlo de aquí). */
export function proveedoresPorDefecto(): ProveedorIdentidad[] {
  return [proveedorDesarrollo];
}

export function crearRegistroProveedores(proveedores: ProveedorIdentidad[]): Map<string, ProveedorIdentidad> {
  const registro = new Map<string, ProveedorIdentidad>();
  for (const proveedor of proveedores) {
    if (registro.has(proveedor.esquema)) {
      throw new Error(`esquema de proveedor de identidad duplicado: '${proveedor.esquema}'`);
    }
    registro.set(proveedor.esquema, proveedor);
  }
  return registro;
}
