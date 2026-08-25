// Orquesta la autenticación: cabecera `Authorization` -> proveedor por esquema (`ProveedorIdentidad`) ->
// identidad externa -> `Usuario` (find-or-create vía `RepositorioIdentidad`) -> `Sesion` nueva. Es la ÚNICA
// pieza que conoce ambos puertos a la vez — las rutas HTTP no hablan directamente ni con un proveedor ni con
// el repositorio (doc 2, principio 3: "actor autenticado", nunca un id que el cliente afirme tener).
//
// Dos cabeceras `Authorization` distintas conviven a propósito:
//   - `Authorization: <esquema-de-proveedor> <credencial>` (ej. `dev <sujetoId>`) SOLO para `autenticar()` —
//     el login. El esquema decide qué `ProveedorIdentidad` la verifica.
//   - `Authorization: sesion <sesionId>` para `resolverSesion()` — el resto de peticiones autenticadas
//     presentan la `Sesion` que devolvió el login, nunca vuelven a mandar la credencial original.
import type { ProveedorIdentidad } from './proveedorIdentidad';
import { CredencialInvalidaError } from './proveedorIdentidad';
import type { RepositorioIdentidad } from './repositorio';
import type { Sesion, Usuario } from './tipos';

const DURACION_SESION_MS = 1000 * 60 * 60 * 12; // 12h — arbitrario para esta etapa, revisar en C6/producción
export const ESQUEMA_SESION = 'sesion';

export interface ContextoAutenticacion {
  proveedores: Map<string, ProveedorIdentidad>;
  repositorio: RepositorioIdentidad;
  ahora?: () => string;
  generarSesionId?: () => string;
}

export class CabeceraAutorizacionInvalidaError extends Error {
  constructor(mensaje = "cabecera 'Authorization' ausente o mal formada (esperado: '<esquema> <valor>')") {
    super(mensaje);
    this.name = 'CabeceraAutorizacionInvalidaError';
  }
}

export class ProveedorDesconocidoError extends Error {
  constructor(esquema: string) {
    super(`ningun proveedor de identidad registrado para el esquema '${esquema}'`);
    this.name = 'ProveedorDesconocidoError';
  }
}

/** Parsea `'<esquema> <valor>'`. No decide nada sobre validez de la credencial en sí — eso lo hace el
 * proveedor (o `resolverSesion`) correspondiente. */
function parsearCabecera(cabecera: string | undefined): { esquema: string; valor: string } {
  if (!cabecera) throw new CabeceraAutorizacionInvalidaError();
  const espacio = cabecera.indexOf(' ');
  if (espacio <= 0) throw new CabeceraAutorizacionInvalidaError();
  const esquema = cabecera.slice(0, espacio).toLowerCase();
  const valor = cabecera.slice(espacio + 1).trim();
  if (valor.length === 0) throw new CabeceraAutorizacionInvalidaError();
  return { esquema, valor };
}

/** Login: autentica una cabecera `Authorization: <esquema-de-proveedor> <credencial>` y hace find-or-create
 * del `Usuario` interno correspondiente a esa identidad externa. Emite una `Sesion` NUEVA en cada llamada —
 * no reutiliza una existente, igual que un login real. */
export async function autenticar(
  cabeceraAuthorization: string | undefined,
  ctx: ContextoAutenticacion
): Promise<{ usuario: Usuario; sesion: Sesion }> {
  const { esquema, valor } = parsearCabecera(cabeceraAuthorization);
  const proveedor = ctx.proveedores.get(esquema);
  if (!proveedor) throw new ProveedorDesconocidoError(esquema);

  const identidad = await proveedor.autenticar(valor); // puede lanzar CredencialInvalidaError; se propaga

  const ahora = (ctx.ahora ?? (() => new Date().toISOString()))();
  let usuario = ctx.repositorio.buscarUsuarioPorIdentidadExterna(identidad.proveedor, identidad.sujetoId);
  if (!usuario) {
    usuario = ctx.repositorio.crearUsuario({ creadoEn: ahora });
    ctx.repositorio.vincularIdentidad({
      usuarioId: usuario.id,
      proveedor: identidad.proveedor,
      sujetoId: identidad.sujetoId,
      email: identidad.email,
      vinculadaEn: ahora,
    });
  }
  if (usuario.deshabilitado) throw new CredencialInvalidaError('la cuenta esta deshabilitada');

  const generarSesionId = ctx.generarSesionId ?? (() => `sesion-${Math.random().toString(36).slice(2)}`);
  const emitidaEn = new Date(ahora);
  const sesion: Sesion = {
    id: generarSesionId(),
    usuarioId: usuario.id,
    emitidaEn: emitidaEn.toISOString(),
    expiraEn: new Date(emitidaEn.getTime() + DURACION_SESION_MS).toISOString(),
  };
  ctx.repositorio.crearSesion(sesion);

  return { usuario, sesion };
}

/** Resuelve una cabecera `Authorization: sesion <sesionId>` a la `Sesion`/`Usuario` que representa. Devuelve
 * `undefined` (nunca lanza) si la cabecera falta, el esquema no es `'sesion'`, o la sesión no existe o
 * expiró — el llamador HTTP decide el código de estado, esta función solo resuelve identidad. */
export function resolverSesion(
  cabeceraAuthorization: string | undefined,
  ctx: Pick<ContextoAutenticacion, 'repositorio' | 'ahora'>
): { usuario: Usuario; sesion: Sesion } | undefined {
  let esquema: string, valor: string;
  try {
    ({ esquema, valor } = parsearCabecera(cabeceraAuthorization));
  } catch {
    return undefined;
  }
  if (esquema !== ESQUEMA_SESION) return undefined;

  const sesion = ctx.repositorio.buscarSesion(valor);
  if (!sesion) return undefined;
  const ahora = (ctx.ahora ?? (() => new Date().toISOString()))();
  if (sesion.expiraEn <= ahora) return undefined; // ISO 8601: comparación lexicográfica == temporal

  const usuario = ctx.repositorio.obtenerUsuario(sesion.usuarioId);
  if (!usuario || usuario.deshabilitado) return undefined;
  return { usuario, sesion };
}
