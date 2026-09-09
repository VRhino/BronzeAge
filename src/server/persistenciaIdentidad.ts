// Guardado del dominio de acceso (usuarios, sesiones, membresías, credenciales locales) en el
// `AlmacenDeObjetos`, bajo una única clave.
//
// Mismo criterio que `persistenciaPartida.ts`: el adaptador garantiza la escritura ATÓMICA (en disco,
// `.tmp` + `rename`) para que un corte a mitad nunca deje el archivo corrupto. Capa `server/` porque toca
// infraestructura y el dominio de acceso (`acceso/`) no puede depender de ella.
//
// A diferencia de una partida, aquí NO hay versión de concurrencia: lo escribe un solo proceso, las
// mutaciones son pequeñas y frecuentes (un login, un unirse), y el adaptador (`repositorioPersistente.ts`)
// las serializa en una cola. El objeto entero se reescribe en cada cambio — es diminuto comparado con un
// snapshot de partida.
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import type { DatosIdentidad } from './identidad/repositorioEnMemoria';

/** Clave del dominio de acceso en el almacén. */
export const CLAVE_IDENTIDAD = 'identidad.json';

/** Versión del envoltorio del archivo — sube solo si cambia su FORMA, para poder rechazar o migrar un
 * archivo de una build anterior sin adivinar por su contenido. */
export const FORMATO_IDENTIDAD_VERSION = 1;

interface ArchivoIdentidad {
  formatoVersion: number;
  datos: DatosIdentidad;
}

const VACIO: DatosIdentidad = { usuarios: [], identidades: [], sesiones: [], membresias: [], credencialesLocales: [] };

/** Se lanza al leer un archivo de un formato que esta build no sabe interpretar. Sin migración automática
 * todavía: no hay despliegues reales con datos que migrar. */
export class FormatoIdentidadNoSoportadoError extends Error {
  constructor(public readonly formatoEnDisco: number) {
    super(`archivo de identidad en formato ${formatoEnDisco}, esta build espera ${FORMATO_IDENTIDAD_VERSION}.`);
    this.name = 'FormatoIdentidadNoSoportadoError';
  }
}

/** Lee el dominio de acceso. Devuelve el snapshot vacío si aún no existe — es el caso "primer arranque", no
 * un error. Tolera un archivo sin `credencialesLocales` (anterior a la introducción del proveedor `clave`):
 * lo rellena con `[]` vía `VACIO`, sin migrar. */
export async function leerIdentidad(almacen: AlmacenDeObjetos): Promise<DatosIdentidad> {
  const contenido = await almacen.leer(CLAVE_IDENTIDAD);
  if (contenido === null) return { ...VACIO };

  const archivo = JSON.parse(contenido) as ArchivoIdentidad;
  if (archivo.formatoVersion !== FORMATO_IDENTIDAD_VERSION) {
    throw new FormatoIdentidadNoSoportadoError(archivo.formatoVersion);
  }
  return { ...VACIO, ...archivo.datos };
}

/** Escribe el snapshot completo (el adaptador lo hace de forma atómica). */
export async function escribirIdentidad(almacen: AlmacenDeObjetos, datos: DatosIdentidad): Promise<void> {
  const archivo: ArchivoIdentidad = { formatoVersion: FORMATO_IDENTIDAD_VERSION, datos };
  await almacen.escribir(CLAVE_IDENTIDAD, JSON.stringify(archivo));
}
