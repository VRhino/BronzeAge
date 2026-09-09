// Guardado del dominio de acceso en disco (cierre de Fase C: "Membresia y Sesion viven en memoria — se
// pierden al reiniciar el proceso").
//
// Mismo criterio y misma técnica que `persistenciaPartida.ts`: escritura ATÓMICA (`.tmp` + `rename`) para
// que un corte a mitad de escritura nunca deje un archivo corrupto, y capa `server/` porque toca `fs` y el
// dominio de acceso (`acceso/`) no puede depender de infraestructura.
//
// A diferencia de una partida, aquí NO hay versión de concurrencia: el dominio de acceso lo escribe un solo
// proceso (el mismo que sirve la API), las mutaciones son pequeñas y frecuentes (un login, un unirse), y el
// adaptador (`repositorioEnDisco.ts`) las serializa en una cola. El archivo entero se reescribe en cada
// cambio — es diminuto comparado con un snapshot de partida.
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import type { DatosIdentidad } from './identidad/repositorioEnMemoria';

/** Versión del envoltorio del archivo — sube solo si cambia su FORMA, para poder rechazar o migrar un
 * archivo de una build anterior sin adivinar por su contenido. */
export const FORMATO_IDENTIDAD_VERSION = 1;

interface ArchivoIdentidad {
  formatoVersion: number;
  datos: DatosIdentidad;
}

const VACIO: DatosIdentidad = { usuarios: [], identidades: [], sesiones: [], membresias: [] };

/** Se lanza al leer un archivo de un formato que esta build no sabe interpretar. Sin migración automática
 * todavía: no hay despliegues reales con datos que migrar. */
export class FormatoIdentidadNoSoportadoError extends Error {
  constructor(public readonly formatoEnDisco: number) {
    super(`archivo de identidad en formato ${formatoEnDisco}, esta build espera ${FORMATO_IDENTIDAD_VERSION}.`);
    this.name = 'FormatoIdentidadNoSoportadoError';
  }
}

/** Lee el archivo de identidad. Devuelve el snapshot vacío si no existe todavía — es el caso "primer
 * arranque", no un error. */
export async function leerIdentidad(ruta: string): Promise<DatosIdentidad> {
  let contenido: string;
  try {
    contenido = await readFile(ruta, 'utf-8');
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return { ...VACIO };
    throw err;
  }
  const archivo = JSON.parse(contenido) as ArchivoIdentidad;
  if (archivo.formatoVersion !== FORMATO_IDENTIDAD_VERSION) {
    throw new FormatoIdentidadNoSoportadoError(archivo.formatoVersion);
  }
  return { ...VACIO, ...archivo.datos };
}

/** Escribe el snapshot completo de forma atómica. Crea el directorio si no existe. */
export async function escribirIdentidad(ruta: string, datos: DatosIdentidad): Promise<void> {
  await mkdir(dirname(ruta), { recursive: true });
  const archivo: ArchivoIdentidad = { formatoVersion: FORMATO_IDENTIDAD_VERSION, datos };
  const rutaTemporal = `${ruta}.tmp`;
  await writeFile(rutaTemporal, JSON.stringify(archivo), 'utf-8');
  await rename(rutaTemporal, ruta);
}
