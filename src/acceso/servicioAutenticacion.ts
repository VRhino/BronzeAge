// Servicio de aplicación del dominio de acceso: orquesta los dos puertos —`ProveedorIdentidad` (verificar
// una credencial) y `RepositorioIdentidad` (recordar quién es quién)— para resolver el login y las sesiones.
// Es la ÚNICA pieza que conoce ambos a la vez.
//
// No sabe NADA de HTTP: recibe una `Credencial` ya parseada (`{esquema, valor}`), no una cabecera
// `Authorization`. Ese parseo es detalle del transporte y vive en `server/identidad/cabeceraAutorizacion.ts`
// — la separación importa porque la política de aquí (find-or-create de `Usuario`, duración de la sesión,
// rechazar cuentas deshabilitadas) es la misma tanto si mañana la credencial llega por HTTP, por WebSocket
// (Fase C5) o por una cola.
import type { ProveedorIdentidad } from './proveedorIdentidad';
import { CredencialInvalidaError } from './proveedorIdentidad';
import type { RepositorioIdentidad } from './repositorio';
import type { Sesion, Usuario } from './tipos';

const DURACION_SESION_MS = 1000 * 60 * 60 * 12; // 12h — arbitrario para esta etapa, revisar en C6/producción

/** Credencial presentada por un cliente, ya separada en esquema y valor por la capa de transporte. El
 * `esquema` decide qué `ProveedorIdentidad` la verifica (ej. `'dev'`), o si es una sesión ya emitida. */
export interface Credencial {
  esquema: string;
  valor: string;
}

/** Esquema reservado: no lo atiende ningún `ProveedorIdentidad`, identifica una `Sesion` ya emitida por
 * `autenticar()`. */
export const ESQUEMA_SESION = 'sesion';

export interface ContextoAutenticacion {
  proveedores: Map<string, ProveedorIdentidad>;
  repositorio: RepositorioIdentidad;
  ahora?: () => string;
  generarSesionId?: () => string;
}

export class ProveedorDesconocidoError extends Error {
  constructor(esquema: string) {
    super(`ningun proveedor de identidad registrado para el esquema '${esquema}'`);
    this.name = 'ProveedorDesconocidoError';
  }
}

/**
 * Login: verifica la credencial con el proveedor que atiende su esquema y hace find-or-create del `Usuario`
 * interno correspondiente a esa identidad externa. Emite una `Sesion` NUEVA en cada llamada — no reutiliza
 * una existente, igual que un login real.
 */
export async function autenticar(
  credencial: Credencial,
  ctx: ContextoAutenticacion
): Promise<{ usuario: Usuario; sesion: Sesion }> {
  const proveedor = ctx.proveedores.get(credencial.esquema);
  if (!proveedor) throw new ProveedorDesconocidoError(credencial.esquema);

  const identidad = await proveedor.autenticar(credencial.valor); // puede lanzar CredencialInvalidaError

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

/**
 * Resuelve una credencial de sesión (`{esquema: 'sesion', valor: <sesionId>}`) al `Usuario`/`Sesion` que
 * representa. Devuelve `undefined` (nunca lanza) si el esquema no es el de sesión, o si la sesión no existe,
 * expiró o su usuario está deshabilitado — quien llama decide qué responder con eso.
 */
export function resolverSesion(
  credencial: Credencial | undefined,
  ctx: Pick<ContextoAutenticacion, 'repositorio' | 'ahora'>
): { usuario: Usuario; sesion: Sesion } | undefined {
  if (!credencial || credencial.esquema !== ESQUEMA_SESION) return undefined;

  const sesion = ctx.repositorio.buscarSesion(credencial.valor);
  if (!sesion) return undefined;
  const ahora = (ctx.ahora ?? (() => new Date().toISOString()))();
  if (sesion.expiraEn <= ahora) return undefined; // ISO 8601: comparación lexicográfica == temporal

  const usuario = ctx.repositorio.obtenerUsuario(sesion.usuarioId);
  if (!usuario || usuario.deshabilitado) return undefined;
  return { usuario, sesion };
}
