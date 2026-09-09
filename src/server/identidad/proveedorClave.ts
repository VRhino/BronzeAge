// Adaptador de PRODUCCIÓN del puerto `ProveedorIdentidad` (`acceso/proveedorIdentidad.ts`): cuentas locales
// con nick + contraseña, para el playtest. A diferencia de `proveedorDesarrollo` (que confía en la cabecera
// sin verificar nada), este comprueba la contraseña contra un hash guardado.
//
// El hash es scrypt de `node:crypto` (stdlib, sin dependencias) con salt aleatoria por cuenta y comparación
// en tiempo constante. El SECRETO se guarda en `RepositorioIdentidad` (`CredencialLocal`), que ya es el puerto
// de persistencia del dominio de acceso y ya serializa sus escrituras a `partidas/identidad.json`.
//
// Formato de la credencial de login: `Authorization: clave <nick>:<contraseña>`. El alta es aparte
// (`registrarCredencial`, la usa `POST /v1/registro` en `rutas/sesiones.ts`): una contraseña necesita
// distinguir "nick nuevo" de "verificar", cosa que un find-or-create al estilo `dev` no puede.
import { randomBytes, scryptSync, timingSafeEqual } from 'node:crypto';
import type { ProveedorIdentidad } from '../../acceso/proveedorIdentidad';
import { CredencialInvalidaError } from '../../acceso/proveedorIdentidad';
import type { RepositorioIdentidad } from '../../acceso/repositorio';

export const ESQUEMA_CLAVE = 'clave';

const CLAVE_MIN = 6;
const CLAVE_MAX = 200; // tope: scrypt sobre una entrada enorme es un vector de DoS barato.
const NICK_MAX = 40;

/** Se lanza al intentar registrar un nick que ya tiene cuenta. La ruta lo traduce a 409. */
export class NickYaRegistradoError extends Error {
  constructor(nick: string) {
    super(`el nick '${nick}' ya está registrado`);
    this.name = 'NickYaRegistradoError';
  }
}

/** Se lanza cuando el nick o la contraseña no cumplen el mínimo. La ruta lo traduce a 400. */
export class DatosDeRegistroInvalidosError extends Error {
  constructor(mensaje: string) {
    super(mensaje);
    this.name = 'DatosDeRegistroInvalidosError';
  }
}

/** Clave de búsqueda estable: sin espacios alrededor y en minúsculas, para que `Ana` y `ana ` sean la misma
 * cuenta y nadie se quede fuera por un caprichо de mayúsculas. */
export function normalizarNick(nick: string): string {
  return nick.trim().toLowerCase();
}

function hashDeClave(clave: string, salt: string): string {
  return scryptSync(clave, salt, 64).toString('hex');
}

/** Alta de una cuenta local. Devuelve el nick normalizado (el que será `sujetoId` de la identidad). */
export function registrarCredencial(
  repositorio: RepositorioIdentidad,
  nick: string,
  clave: string,
  ahora: string
): string {
  const nickNormalizado = normalizarNick(nick);
  if (nickNormalizado.length === 0 || nickNormalizado.length > NICK_MAX) {
    throw new DatosDeRegistroInvalidosError(`el nick debe tener entre 1 y ${NICK_MAX} caracteres`);
  }
  if (clave.length < CLAVE_MIN || clave.length > CLAVE_MAX) {
    throw new DatosDeRegistroInvalidosError(`la contraseña debe tener entre ${CLAVE_MIN} y ${CLAVE_MAX} caracteres`);
  }
  if (repositorio.buscarCredencialLocal(nickNormalizado)) throw new NickYaRegistradoError(nickNormalizado);

  const salt = randomBytes(16).toString('hex');
  repositorio.guardarCredencialLocal({ nick: nickNormalizado, salt, hash: hashDeClave(clave, salt), creadaEn: ahora });
  return nickNormalizado;
}

export function crearProveedorClave(repositorio: RepositorioIdentidad): ProveedorIdentidad {
  return {
    esquema: ESQUEMA_CLAVE,

    async autenticar(valor: string) {
      const separador = valor.indexOf(':'); // primer ':': la contraseña puede contener ':'
      if (separador <= 0) throw new CredencialInvalidaError("se esperaba '<nick>:<contraseña>'");
      const nick = normalizarNick(valor.slice(0, separador));
      const clave = valor.slice(separador + 1);

      const credencial = repositorio.buscarCredencialLocal(nick);
      // Mismo error para "nick no existe" y "contraseña incorrecta": no revelamos qué nicks hay registrados.
      if (!credencial) throw new CredencialInvalidaError();
      const esperado = Buffer.from(credencial.hash, 'hex');
      const recibido = Buffer.from(hashDeClave(clave, credencial.salt), 'hex');
      if (esperado.length !== recibido.length || !timingSafeEqual(esperado, recibido)) {
        throw new CredencialInvalidaError();
      }
      return { proveedor: ESQUEMA_CLAVE, sujetoId: nick };
    },
  };
}
