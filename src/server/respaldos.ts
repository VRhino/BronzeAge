// Respaldos de partida y restauración (Fase E2, doc 2: "Auditoría, snapshots, backups y restauración").
//
// QUÉ problema resuelve, que la persistencia normal NO resuelve. `guardarPartida` escribe SIEMPRE sobre
// `<gameId>.json`: cada comando sustituye al anterior, así que en disco hay exactamente UNA versión de cada
// partida y no existe forma de volver atrás. Eso es correcto para operar —el snapshot es el estado vigente,
// no un historial— pero deja tres agujeros que un servidor persistente no puede permitirse: un bug que
// corrompa el estado se guarda encima del bueno; un `descartarYCrear` mal dado no se deshace; y un disco que
// falle se lleva la partida entera. Un respaldo es una COPIA FECHADA que nada vuelve a tocar.
//
// **La restauración es la mitad que importa.** Un respaldo que nunca se ha restaurado no se sabe si es un
// respaldo: puede estar truncado, en un formato que la build actual ya no lee, o describir una partida que
// no arranca. Por eso `restaurar` no es un `copyFile` — comprueba que el archivo se puede CARGAR de verdad
// (`cargarPartida`, con su migración de formato incluida) antes de tocar el snapshot vigente. Si el respaldo
// está roto, se sabe antes de haber destruido nada.
//
// Convive con `auditoria.ts` sin mezclarse: el registro de auditoría explica QUIÉN llevó la partida hasta
// aquí, un respaldo permite VOLVER. Se respaldan juntos porque una restauración sin su auditoría deja un
// hueco inexplicable en el registro.
import { copyFile, mkdir, readdir, rename, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { cargarPartida } from './persistenciaPartida';
import { rutaDeAuditoria } from './auditoria';

/** Subdirectorio de respaldos, hermano de los snapshots. Aparte y no mezclado con ellos para que
 * `listarPartidas` —que lee el directorio de datos— no confunda un respaldo con una partida viva. */
export const DIRECTORIO_RESPALDOS = 'respaldos';

/** Un respaldo en disco. `momento` sale del NOMBRE del archivo, no de `mtime`: copiar o mover el directorio
 * de datos cambia las fechas del sistema de archivos y no debe cambiar qué respaldo es más reciente. */
export interface Respaldo {
  gameId: string;
  /** Reloj de pared del respaldo (ISO 8601). */
  momento: string;
  /** Nombre del archivo dentro de `respaldos/` — lo que `restaurar` acepta como identificador. */
  archivo: string;
  bytes: number;
}

/** Los `:` de un ISO 8601 no valen en un nombre de archivo en Windows. Se sustituyen por `-`, y
 * `momentoDeArchivo` deshace la conversión — el nombre sigue siendo legible y ordenable alfabéticamente,
 * que es lo que permite ordenar respaldos sin abrirlos. */
function momentoParaArchivo(momento: string): string {
  return momento.replace(/:/g, '-');
}

function momentoDeArchivo(sello: string): string {
  // `2026-09-05T10-44-44.906Z` -> `2026-09-05T10:44:44.906Z`: solo los dos primeros `-` DESPUÉS de la `T`,
  // nunca los de la fecha.
  const t = sello.indexOf('T');
  if (t === -1) return sello;
  return sello.slice(0, t) + sello.slice(t).replace(/-/g, ':');
}

const SUFIJO_PARTIDA = '.json';
const SUFIJO_AUDITORIA = '.auditoria.jsonl';

function nombreDeRespaldo(gameId: string, momento: string, sufijo: string): string {
  return `${gameId}--${momentoParaArchivo(momento)}${sufijo}`;
}

/**
 * Copia el snapshot vigente de una partida (y su auditoría, si la tiene) al directorio de respaldos.
 *
 * Devuelve `null` si la partida no tiene snapshot todavía — respaldar algo que no existe no es un error, es
 * el caso "partida recién declarada, aún sin escribir".
 *
 * **Copia, no mueve ni reescribe.** El snapshot vigente no se toca en ningún momento: si el proceso muere a
 * mitad del respaldo, lo que queda a medias es la COPIA, y la partida sigue intacta. Un respaldo a medias se
 * detecta al restaurar (`cargarPartida` falla) en vez de contaminar la partida viva.
 */
export async function respaldarPartida(directorio: string, gameId: string, momento: string): Promise<Respaldo | null> {
  const origen = join(directorio, `${gameId}${SUFIJO_PARTIDA}`);
  let bytes: number;
  try {
    bytes = (await stat(origen)).size;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw err;
  }

  const destinoDir = join(directorio, DIRECTORIO_RESPALDOS);
  await mkdir(destinoDir, { recursive: true });
  const archivo = nombreDeRespaldo(gameId, momento, SUFIJO_PARTIDA);
  await copyFile(origen, join(destinoDir, archivo));

  // La auditoría acompaña al snapshot: restaurar la partida sin ella dejaría el registro contando una
  // historia que ya no corresponde al estado. Que no exista es normal (partida sin comandos todavía).
  try {
    await copyFile(rutaDeAuditoria(directorio, gameId), join(destinoDir, nombreDeRespaldo(gameId, momento, SUFIJO_AUDITORIA)));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'ENOENT') throw err;
  }

  return { gameId, momento, archivo, bytes };
}

/** Respaldos de una partida, del más reciente al más antiguo. Vacío si no hay ninguno. */
export async function listarRespaldos(directorio: string, gameId: string): Promise<Respaldo[]> {
  const destinoDir = join(directorio, DIRECTORIO_RESPALDOS);
  let archivos: string[];
  try {
    archivos = await readdir(destinoDir);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
    throw err;
  }

  const prefijo = `${gameId}--`;
  const respaldos: Respaldo[] = [];
  for (const archivo of archivos) {
    // El sufijo de auditoría se descarta aquí a propósito: un respaldo es UNA entrada de la lista, y su
    // auditoría es un adjunto suyo, no un respaldo aparte que se pudiera restaurar por su cuenta.
    if (!archivo.startsWith(prefijo) || !archivo.endsWith(SUFIJO_PARTIDA) || archivo.endsWith(SUFIJO_AUDITORIA)) continue;
    const sello = archivo.slice(prefijo.length, archivo.length - SUFIJO_PARTIDA.length);
    respaldos.push({ gameId, momento: momentoDeArchivo(sello), archivo, bytes: (await stat(join(destinoDir, archivo))).size });
  }
  // Descendente por NOMBRE, que para un ISO 8601 es lo mismo que por fecha — sin depender del `mtime`.
  return respaldos.sort((a, b) => (a.archivo < b.archivo ? 1 : -1));
}

/** Se lanza cuando el respaldo que se pide restaurar no existe, o existe pero no se puede cargar. En ambos
 * casos el snapshot vigente queda SIN TOCAR — es la garantía que hace segura la operación. */
export class RespaldoInservibleError extends Error {
  constructor(
    public readonly archivo: string,
    motivo: string
  ) {
    super(`el respaldo '${archivo}' no sirve para restaurar: ${motivo}`);
    this.name = 'RespaldoInservibleError';
  }
}

/**
 * Restaura una partida desde un respaldo. Devuelve la versión de partida que queda vigente.
 *
 * **Orden deliberado**: primero se COMPRUEBA que el respaldo carga (`cargarPartida` sobre una copia en un
 * directorio aparte — con su migración de formato, así que un respaldo de una build anterior sigue valiendo),
 * y solo entonces se sustituye el snapshot vigente con un `rename` atómico. Nunca hay un instante en que la
 * partida esté medio restaurada: o sigue la vieja, o está la nueva entera.
 *
 * **NO reabre la partida.** Si el proceso la tiene abierta, su `RunnerDePartida` sigue con el estado viejo en
 * memoria y lo escribiría encima al siguiente comando. Quien llame a esto es responsable de que la partida
 * esté cerrada — la ruta que lo exponga debe exigirlo, igual que `descartarYCrear` exige su propio permiso.
 * Se deja explícito aquí en vez de resolverlo por dentro porque este módulo no conoce el registro, y hacer
 * que lo conozca ataría la restauración a que exista un proceso servidor corriendo.
 */
export async function restaurarPartida(directorio: string, gameId: string, archivo: string): Promise<number> {
  const origen = join(directorio, DIRECTORIO_RESPALDOS, archivo);
  try {
    await stat(origen);
  } catch {
    throw new RespaldoInservibleError(archivo, 'no existe');
  }

  // Verificación en un directorio aparte: `cargarPartida` busca por `<gameId>.json`, así que comprobar el
  // respaldo exige ponerlo bajo ese nombre en algún sitio — y ese sitio no puede ser el directorio real,
  // porque sería exactamente la escritura que queremos evitar hasta haber verificado.
  const pruebas = join(directorio, DIRECTORIO_RESPALDOS, `.verificacion-${gameId}`);
  await mkdir(pruebas, { recursive: true });
  const candidato = join(pruebas, `${gameId}${SUFIJO_PARTIDA}`);
  try {
    await copyFile(origen, candidato);
    const cargada = await cargarPartida(pruebas, gameId);
    if (!cargada) throw new RespaldoInservibleError(archivo, 'no contiene una partida legible');
    const version = cargada.sesion.getState().version;

    // Verificado: ahora sí, sustitución atómica del snapshot vigente.
    await rename(candidato, join(directorio, `${gameId}${SUFIJO_PARTIDA}`));
    return version;
  } catch (err) {
    if (err instanceof RespaldoInservibleError) throw err;
    throw new RespaldoInservibleError(archivo, err instanceof Error ? err.message : String(err));
  } finally {
    await rm(pruebas, { recursive: true, force: true });
  }
}

/**
 * Conserva los `conservar` respaldos más recientes de una partida y borra el resto. Devuelve cuántos se
 * borraron.
 *
 * Por CUENTA y no por edad, al revés que la poda de la auditoría, y por una razón concreta: una partida
 * inactiva durante el periodo de retención se quedaría sin ningún respaldo justo cuando más difícil es
 * regenerarlo. Con un mínimo por cuenta, siempre queda de dónde volver.
 */
export async function podarRespaldos(directorio: string, gameId: string, conservar: number): Promise<number> {
  if (conservar < 1) throw new RangeError('podarRespaldos: hay que conservar al menos 1 respaldo.');
  const respaldos = await listarRespaldos(directorio, gameId);
  const sobrantes = respaldos.slice(conservar);
  const destinoDir = join(directorio, DIRECTORIO_RESPALDOS);
  for (const respaldo of sobrantes) {
    await rm(join(destinoDir, respaldo.archivo), { force: true });
    // Y su auditoría adjunta, que si no quedaría huérfana ocupando sitio para siempre.
    await rm(join(destinoDir, nombreDeRespaldo(gameId, respaldo.momento, SUFIJO_AUDITORIA)), { force: true });
  }
  return sobrantes.length;
}
