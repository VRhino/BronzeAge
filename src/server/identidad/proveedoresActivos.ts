// Qué adaptadores de `ProveedorIdentidad` están dados de alta en ESTE proceso. Es configuración de
// infraestructura, no negocio: el mecanismo de registro vive con el puerto (`acceso/proveedorIdentidad.ts`),
// aquí solo se decide la lista.
//
// Es el único punto a tocar para integrar un proveedor real (añadirlo) y, más adelante, para retirar el de
// desarrollo antes de producción (quitarlo). Nada más cambia.
import type { ProveedorIdentidad } from '../../acceso/proveedorIdentidad';
import { proveedorDesarrollo } from './proveedorDesarrollo';

export function proveedoresPorDefecto(): ProveedorIdentidad[] {
  return [proveedorDesarrollo];
}
