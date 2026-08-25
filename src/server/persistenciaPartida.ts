// Guardado de partida en disco (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3).
//
// QUÉ se persiste ya estaba resuelto antes de este archivo: `GameSession.exportar()` devuelve
// `PartidaExportada` con estado, tick, eventos de dominio, mapa/yacimientos, IDs y —desde el commit
// anterior— el estado del RNG. Lo único que faltaba era CÓMO llevarlo a disco, y por eso vive aparte de
// `session/`: `GameSession` es deliberadamente síncrona y sin E/S (doc 7 §2), así que cualquier cosa que
// toque el sistema de archivos tiene que vivir en una capa que sí pueda ser async — de ahí `server/`, la
// primera pieza de una capa que en Fase B/C también tendrá el `RunnerDePartida` y la API HTTP.
//
// Escritura ATÓMICA (`.tmp` + `rename`, decidido en el doc 4): escribir directo sobre el nombre final
// dejaría un archivo a medias si el proceso muere a mitad escritura. Con `rename` —atómico en el mismo
// volumen, tanto en NTFS como en los filesystems POSIX habituales— el archivo final SIEMPRE es una versión
// completa y válida, o no cambia en absoluto.
//
// Versión de concurrencia: la cola serial por `gameId` del futuro `RunnerDePartida` ya elimina la
// concurrencia de escritura en operación normal (doc 4: "la ventaja principal de SQLite" que no hace falta
// aquí). La comprobación de versión de este módulo no es el mecanismo principal de control de concurrencia
// — es una red de seguridad que detecta el síntoma de un bug real (dos procesos escribiendo el mismo
// `gameId`) y se niega a perder datos en silencio en vez de prevenirlo por diseño.
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { GameSession, type PartidaExportada } from '../session/gameSession';
import { WORLDGEN_VERSION } from '../worldgen';

/**
 * Versión del ENVOLTORIO del archivo de snapshot. NO es `PartidaExportada.state.version` (versión de LA
 * PARTIDA, sube en cada comando aceptado) ni `worldgenVersion` (versión del generador de mundo) — esta sube
 * solo si cambia la FORMA del archivo en sí, para poder rechazar o migrar snapshots de una build anterior
 * sin adivinar por la forma del JSON.
 */
export const FORMATO_SNAPSHOT_VERSION = 1;

export interface SnapshotPartida {
  formatoVersion: number;
  /** Momento de simulación del guardado (ISO 8601) — lo decide el llamador, igual que en todo lo demás de
   * `session/`: este módulo tampoco lee el reloj por su cuenta. */
  guardadoEn: string;
  partida: PartidaExportada;
}

/** Se lanza cuando el snapshot en disco ya tiene una versión de partida MÁS AVANZADA que la que se intenta
 * guardar — ver la nota sobre versión de concurrencia arriba. Guardar dos veces la MISMA versión no lanza
 * esto: es un reintento válido tras un fallo a mitad de escritura, no un conflicto. */
export class ConflictoDeVersionError extends Error {
  constructor(
    public readonly gameId: string,
    public readonly enDisco: number,
    public readonly intentada: number
  ) {
    super(
      `partida '${gameId}': se intentó guardar la versión ${intentada} pero en disco ya hay la ${enDisco} — ` +
        'alguien más escribió este snapshot primero. No se sobreescribe.'
    );
  }
}

/** Se lanza al cargar un snapshot de un formato de envoltorio que esta build no sabe leer. Sin migración
 * automática todavía: no hay snapshots reales en producción que migrar. */
export class FormatoSnapshotNoSoportadoError extends Error {
  constructor(
    public readonly gameId: string,
    public readonly formatoEnDisco: number
  ) {
    super(`partida '${gameId}': snapshot en formato ${formatoEnDisco}, esta build espera ${FORMATO_SNAPSHOT_VERSION}.`);
  }
}

/** Se lanza al cargar un snapshot generado con otra versión del algoritmo de generación de mundo — mismo
 * motivo de rechazo que ya usa `GameStore.importarSimulacion` para el formato de archivo v2: con otro
 * algoritmo, la misma seed produce un mapa distinto y la partida cargada no sería la que se guardó. */
export class WorldgenVersionNoCoincideError extends Error {
  constructor(
    public readonly gameId: string,
    public readonly versionEnDisco: number
  ) {
    super(
      `partida '${gameId}': el mundo se generó con la versión ${versionEnDisco} del generador y esta build usa la ` +
        `${WORLDGEN_VERSION} — el mapa no se puede reconstruir igual a partir de la seed.`
    );
  }
}

function rutaDe(directorio: string, gameId: string): string {
  return join(directorio, `${gameId}.json`);
}

async function leerSnapshotSiExiste(ruta: string): Promise<SnapshotPartida | null> {
  try {
    const contenido = await readFile(ruta, 'utf-8');
    return JSON.parse(contenido) as SnapshotPartida;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw err;
  }
}

/**
 * Guarda el estado ACTUAL de `sesion` en `directorio/<gameId>.json`. Crea el directorio si no existe.
 *
 * Lanza `ConflictoDeVersionError` si el snapshot en disco ya tiene una versión de partida mayor que la que
 * se intenta guardar (ver la nota de concurrencia al principio del archivo).
 */
export async function guardarPartida(directorio: string, sesion: GameSession, momento: string): Promise<void> {
  const partida = sesion.exportar();
  const ruta = rutaDe(directorio, sesion.gameId);

  const existente = await leerSnapshotSiExiste(ruta);
  if (existente && existente.partida.state.version > partida.state.version) {
    throw new ConflictoDeVersionError(sesion.gameId, existente.partida.state.version, partida.state.version);
  }

  await mkdir(directorio, { recursive: true });
  const snapshot: SnapshotPartida = { formatoVersion: FORMATO_SNAPSHOT_VERSION, guardadoEn: momento, partida };
  const rutaTemporal = `${ruta}.tmp`;
  await writeFile(rutaTemporal, JSON.stringify(snapshot), 'utf-8');
  await rename(rutaTemporal, ruta);
}

/**
 * Reconstruye la `GameSession` guardada en `directorio/<gameId>.json`. `null` si no existe ningún snapshot
 * para ese `gameId` — no es un error, es el caso "partida nueva".
 */
export async function cargarPartida(directorio: string, gameId: string): Promise<GameSession | null> {
  const snapshot = await leerSnapshotSiExiste(rutaDe(directorio, gameId));
  if (!snapshot) return null;
  if (snapshot.formatoVersion !== FORMATO_SNAPSHOT_VERSION) {
    throw new FormatoSnapshotNoSoportadoError(gameId, snapshot.formatoVersion);
  }
  if (snapshot.partida.worldgenVersion !== WORLDGEN_VERSION) {
    throw new WorldgenVersionNoCoincideError(gameId, snapshot.partida.worldgenVersion);
  }
  return GameSession.importar(snapshot.partida);
}
