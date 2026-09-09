// Qué adaptadores de `ProveedorIdentidad` están dados de alta. Es configuración de infraestructura, no
// negocio: el mecanismo de registro vive con el puerto (`acceso/proveedorIdentidad.ts`), aquí solo la lista.
//
// Es el único punto a tocar para integrar un proveedor real o retirar uno.
import type { ProveedorIdentidad } from '../../acceso/proveedorIdentidad';
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import { proveedorDesarrollo } from './proveedorDesarrollo';
import { crearProveedorClave } from './proveedorClave';

/** Solo el proveedor de desarrollo. Es el default de `crearServidor` cuando no se le inyecta `identidad` —
 * es decir, los tests. NUNCA lo usa el proceso real (ver `proveedoresDeProceso`). */
export function proveedoresPorDefecto(): ProveedorIdentidad[] {
  return [proveedorDesarrollo];
}

/** Lo que arranca el proceso real (`server/index.ts`): cuentas locales con contraseña (`clave`) para los
 * jugadores, más el proveedor `dev` para el cliente de administración, que corre en local. `dev` sigue
 * aceptando cualquier sujeto sin verificar — aceptable mientras la superficie de admin no sea pública. */
export function proveedoresDeProceso(repositorio: RepositorioIdentidad): ProveedorIdentidad[] {
  return [crearProveedorClave(repositorio), proveedorDesarrollo];
}
