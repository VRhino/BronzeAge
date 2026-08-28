// Traduce la cabecera HTTP `Authorization: <esquema> <valor>` a la `Credencial` que entiende el dominio de
// acceso (`acceso/servicioAutenticacion.ts`). Es INFRAESTRUCTURA: el formato de la cabecera es del
// transporte, no del negocio — por eso el servicio de autenticación ya no lo conoce.
//
// Dos esquemas conviven a propósito:
//   - `Authorization: <esquema-de-proveedor> <credencial>` (ej. `dev ana`) para el login.
//   - `Authorization: sesion <sesionId>` para el resto de peticiones autenticadas — la credencial original
//     no se vuelve a mandar nunca.
import type { Credencial } from '../../acceso/servicioAutenticacion';

export class CabeceraAutorizacionInvalidaError extends Error {
  constructor(mensaje = "cabecera 'Authorization' ausente o mal formada (esperado: '<esquema> <valor>')") {
    super(mensaje);
    this.name = 'CabeceraAutorizacionInvalidaError';
  }
}

/** Parsea la cabecera, o lanza si falta o está mal formada. No juzga si la credencial es VÁLIDA — eso es del
 * proveedor correspondiente. */
export function credencialDesdeCabecera(cabecera: string | undefined): Credencial {
  if (!cabecera) throw new CabeceraAutorizacionInvalidaError();
  const espacio = cabecera.indexOf(' ');
  if (espacio <= 0) throw new CabeceraAutorizacionInvalidaError();
  const esquema = cabecera.slice(0, espacio).toLowerCase();
  const valor = cabecera.slice(espacio + 1).trim();
  if (valor.length === 0) throw new CabeceraAutorizacionInvalidaError();
  return { esquema, valor };
}

/** Variante no lanzante, para las rutas que solo necesitan saber si hay una sesión utilizable y responden
 * 401 igual ante "sin cabecera" y "cabecera inservible". */
export function credencialOpcionalDesdeCabecera(cabecera: string | undefined): Credencial | undefined {
  try {
    return credencialDesdeCabecera(cabecera);
  } catch {
    return undefined;
  }
}
