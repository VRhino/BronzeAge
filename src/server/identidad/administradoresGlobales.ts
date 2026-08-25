// Quién es `administrador_global` en ESTA instancia. Es configuración de despliegue —infraestructura—, no
// una regla de negocio: la política de qué puede hacer ese rol vive en `acceso/rolesDePartida.ts`.
//
// Se configura por identidad EXTERNA (`proveedor:sujetoId`), que es lo que un operador conoce y puede
// escribir en una variable de entorno, no por el `usuarioId` interno, que se asigna solo la primera vez que
// alguien entra. La traducción la hace el repositorio, que ya guarda ese vínculo.
//
// Por defecto NO hay ningún administrador: `crearServidor()` sin configuración explícita no deja crear
// partidas a nadie. Es deliberado — conceder administración por omisión es la clase de default que sobrevive
// hasta producción sin que nadie lo note. Quien arranca el proceso (`server/index.ts`) decide y lo declara.
import type { RepositorioIdentidad } from '../../acceso/repositorio';

/** Identidad externa con permiso de administración global, tal y como se configura. */
export interface AdministradorConfigurado {
  proveedor: string;
  sujetoId: string;
}

export interface DirectorioDeAdministradores {
  esAdministradorGlobal(usuarioId: string): boolean;
}

export function crearDirectorioDeAdministradores(
  configurados: readonly AdministradorConfigurado[],
  repositorio: RepositorioIdentidad
): DirectorioDeAdministradores {
  const claves = new Set(configurados.map((a) => `${a.proveedor}:${a.sujetoId}`));
  return {
    esAdministradorGlobal(usuarioId) {
      if (claves.size === 0) return false;
      const identidad = repositorio.buscarIdentidadDeUsuario(usuarioId);
      return identidad !== undefined && claves.has(`${identidad.proveedor}:${identidad.sujetoId}`);
    },
  };
}

/**
 * Parsea una lista `proveedor:sujetoId` separada por comas (formato de variable de entorno). Entradas vacías
 * o sin `:` se descartan en silencio salvo que no quede ninguna válida: una config mal escrita que dejara la
 * instancia sin administradores es un fallo de arranque, no algo que deba descubrirse con un 403 más tarde.
 */
export function parsearAdministradores(valor: string | undefined): AdministradorConfigurado[] {
  if (!valor || valor.trim() === '') return [];
  const entradas = valor
    .split(',')
    .map((e) => e.trim())
    .filter((e) => e !== '');
  const parseadas: AdministradorConfigurado[] = [];
  for (const entrada of entradas) {
    const separador = entrada.indexOf(':');
    if (separador <= 0 || separador === entrada.length - 1) continue;
    parseadas.push({ proveedor: entrada.slice(0, separador), sujetoId: entrada.slice(separador + 1) });
  }
  if (entradas.length > 0 && parseadas.length === 0) {
    throw new Error(`ADMINISTRADORES no contiene ninguna entrada valida ('proveedor:sujetoId'): '${valor}'`);
  }
  return parseadas;
}
