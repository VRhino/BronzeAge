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
import { mkdir, readdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { GameSession, type PartidaExportada } from '../session/gameSession';
import { idDeMapa, instanteDeTick, isoDeInstante } from '../session/estado';
import { instante, type Instante } from '../domain/tiempo';
import { WORLDGEN_VERSION } from '../worldgen';

/**
 * Versión del ENVOLTORIO del archivo de snapshot. NO es `PartidaExportada.state.version` (versión de LA
 * PARTIDA, sube en cada comando aceptado) ni `worldgenVersion` (versión del generador de mundo) — esta sube
 * solo si cambia la FORMA del archivo en sí, para poder rechazar o migrar snapshots de una build anterior
 * sin adivinar por la forma del JSON.
 *
 * v2 (Fase D / doc 10): los campos temporales pasan de `*EnTick: number` (ordinal de tick) a `*En: Instante`
 * (ms de mundo). v3 (Fase D / D3): el contador de construcción `Edificio.ticksRestantes` pasa a
 * `Edificio.completaEn: Instante` (fecha absoluta, no contador — doc 6 §4). v4 (Fase D / D6): el único campo
 * de entidad con unidad de tick en su nombre, `RelacionPolitica.tributo.cantidadPorTick`, pasa a
 * `cantidadPorMinuto` (mismo valor — 1 tick = 1 minuto). v5 (cierre de Fase D): el `tick` provisional de los
 * eventos y del log desaparece — `EventoDominio` se queda solo con `momento`, `EventoLogAdmin.tick` pasa a
 * `momento`. `migrarSnapshot` encadena las conversiones y todas son sin pérdida — la relación tick↔instante
 * es 1:1 (`instanteDeTick`). v6 (movimiento de ejércitos): `jugadores` y `ejercitos`. v7 (niebla de guerra):
 * `memoriaPorFaccion`.
 */
export const FORMATO_SNAPSHOT_VERSION = 7;

export interface SnapshotPartida {
  formatoVersion: number;
  /** Reloj de PARED del guardado (ISO 8601) — cuándo se escribió este archivo en tiempo real, no tiempo de
   * mundo (ese es `instanteDeTick(partida.state.tick)`, derivado). Lo pasa `RunnerDePartida` con `this.ahora()`:
   * `server/` es la capa dueña del reloj de pared (doc 10 §3), y este dato es metadato del archivo, no estado
   * de partida. */
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

/** Se lanza al cargar un snapshot de un formato de envoltorio que esta build no sabe leer NI migrar (hoy:
 * cualquiera fuera del rango v1..v5). */
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
 * se intenta guardar (ver la nota de concurrencia al principio del archivo) — salvo que `forzar` sea `true`:
 * el reemplazo DELIBERADO de una partida descartada (`RegistroDePartidas.descartarYCrear`, Fase C12) siempre
 * empieza en version 0, así que sin este escape la propia red de seguridad contra el escenario "dos procesos
 * escribiendo el mismo gameId" bloquearía el `forzar: true` legítimo que el usuario pidió — el guardado en la
 * creación (`RunnerDePartida.crearYPersistir`) es justo lo que hace visible ese conflicto ahora, antes solo
 * pasaba cuando llegaba el primer comando/tick de la partida nueva.
 */
export async function guardarPartida(directorio: string, sesion: GameSession, guardadoEn: string, opciones: { forzar?: boolean } = {}): Promise<void> {
  const partida = sesion.exportar();
  const ruta = rutaDe(directorio, sesion.gameId);

  const existente = await leerSnapshotSiExiste(ruta);
  if (!opciones.forzar && existente && existente.partida.state.version > partida.state.version) {
    throw new ConflictoDeVersionError(sesion.gameId, existente.partida.state.version, partida.state.version);
  }

  await mkdir(directorio, { recursive: true });
  const snapshot: SnapshotPartida = { formatoVersion: FORMATO_SNAPSHOT_VERSION, guardadoEn, partida };
  const rutaTemporal = `${ruta}.tmp`;
  await writeFile(rutaTemporal, JSON.stringify(snapshot), 'utf-8');
  await rename(rutaTemporal, ruta);
}

/** Lo que devuelve `cargarPartida`: la sesión reconstruida más el reloj de PARED del último guardado
 * (`SnapshotPartida.guardadoEn`). El `guardadoEn` es lo que necesita el reloj de mundo de D5 para saber
 * cuántos ticks se adeudan tras un reinicio — "hace 3 h reales que se guardó el último tick" ⇒ 180 ticks. */
export interface PartidaCargada {
  sesion: GameSession;
  guardadoEn: string;
}

/**
 * Reconstruye la `GameSession` guardada en `directorio/<gameId>.json`, con el `guardadoEn` del snapshot.
 * `null` si no existe ningún snapshot para ese `gameId` — no es un error, es el caso "partida nueva".
 */
export async function cargarPartida(directorio: string, gameId: string): Promise<PartidaCargada | null> {
  const snapshot = await leerSnapshotSiExiste(rutaDe(directorio, gameId));
  if (!snapshot) return null;

  const partida = migrarSnapshot(gameId, snapshot);
  if (partida.worldgenVersion !== WORLDGEN_VERSION) {
    throw new WorldgenVersionNoCoincideError(gameId, partida.worldgenVersion);
  }
  return { sesion: GameSession.importar(partida), guardadoEn: snapshot.guardadoEn };
}

/**
 * Lleva un snapshot al formato actual encadenando migraciones (v1 -> ... -> v5). La versión actual se
 * devuelve tal cual; cualquier versión fuera del rango soportado se rechaza. La migración NO se persiste sola
 * — el próximo comando o tick reescribe el archivo ya en la versión actual.
 *
 * Trabaja sobre el JSON crudo (`Record<string, any>`): las formas de cada versión difieren en nombres de
 * campo, así que tipar el intermedio no aporta — el resultado sí es un `PartidaExportada` válido.
 */
function migrarSnapshot(gameId: string, snapshot: SnapshotPartida): PartidaExportada {
  if (snapshot.formatoVersion === FORMATO_SNAPSHOT_VERSION) return snapshot.partida;
  if (snapshot.formatoVersion < 1 || snapshot.formatoVersion > FORMATO_SNAPSHOT_VERSION) {
    throw new FormatoSnapshotNoSoportadoError(gameId, snapshot.formatoVersion);
  }

  const p = snapshot.partida as unknown as Record<string, any>;
  const s = p.state as Record<string, any>;
  if (snapshot.formatoVersion < 2) migrarV1aV2(s);
  if (snapshot.formatoVersion < 3) migrarV2aV3(s);
  if (snapshot.formatoVersion < 4) migrarV3aV4(s);
  if (snapshot.formatoVersion < 5) migrarV4aV5(s);
  if (snapshot.formatoVersion < 6) migrarV5aV6(s);
  if (snapshot.formatoVersion < 7) migrarV6aV7(s);
  return p as unknown as PartidaExportada;
}

/** v1 -> v2 (Fase D / doc 10): campos temporales `*EnTick: number` (ordinal de tick) -> `*En: Instante` (ms
 * de mundo), relación 1:1 vía `instanteDeTick`. `salidasFaccionPorJugador` guardaba ISO en v1. */
function migrarV1aV2(s: Record<string, any>): void {
  const iso = (isoOTick: unknown) => (typeof isoOTick === 'string' ? instante(Date.parse(isoOTick)) : instanteDeTick(Number(isoOTick)));

  for (const a of s.asentamientos ?? []) {
    a.fundadoEn = instanteDeTick(a.fundadoEnTick ?? 0);
    delete a.fundadoEnTick;
    if (a.ultimaCaravanaCreadaEnTick !== undefined) {
      a.ultimaCaravanaCreadaEn = instanteDeTick(a.ultimaCaravanaCreadaEnTick);
      delete a.ultimaCaravanaCreadaEnTick;
    }
    for (const e of a.escuadrones ?? []) {
      if (e.heridoHastaTick !== undefined) {
        e.heridoHasta = instanteDeTick(e.heridoHastaTick);
        delete e.heridoHastaTick;
      }
    }
    for (const pol of a.politicasActivas ?? []) {
      pol.activadaEn = instanteDeTick(pol.activadaEnTick ?? 0);
      pol.expiraEn = instanteDeTick(pol.expiraEnTick ?? 0);
      delete pol.activadaEnTick;
      delete pol.expiraEnTick;
    }
  }
  for (const ac of s.acuerdos ?? []) {
    ac.creadoEn = instanteDeTick(ac.creadoEnTick ?? 0);
    ac.expiraEn = instanteDeTick(ac.expiraEnTick ?? 0);
    delete ac.creadoEnTick;
    delete ac.expiraEnTick;
  }
  for (const o of s.ordenes ?? []) {
    o.creadoEn = instanteDeTick(o.creadoEnTick ?? 0);
    delete o.creadoEnTick;
  }
  for (const r of s.relaciones ?? []) {
    r.creadoEn = instanteDeTick(r.creadoEnTick ?? 0);
    delete r.creadoEnTick;
  }
  s.bandidosProximoSpawnEn = instanteDeTick(s.bandidosProximoSpawnTick ?? 0);
  delete s.bandidosProximoSpawnTick;
  s.salidasFaccionPorJugador = Object.fromEntries(
    Object.entries(s.salidasFaccionPorJugador ?? {}).map(([jugador, valor]) => [jugador, iso(valor)])
  );
  const mapa = s.estadoMapa as Record<string, any>;
  if (mapa?.regeneraEnTick !== undefined) {
    mapa.regeneraEn = Object.fromEntries(Object.entries(mapa.regeneraEnTick).map(([id, tick]) => [id, instanteDeTick(Number(tick))]));
    delete mapa.regeneraEnTick;
  }
}

/** v2 -> v3 (Fase D / D3): el contador `Edificio.ticksRestantes` -> `Edificio.completaEn: Instante`. Solo un
 * edificio `en_construccion` lleva `completaEn` (instante en que la obra terminará = tick actual + los ticks
 * que le quedaban); `en_cola` y `activo` simplemente pierden el contador. */
function migrarV2aV3(s: Record<string, any>): void {
  const tickActual = Number(s.tick ?? 0);
  for (const a of s.asentamientos ?? []) {
    for (const e of a.edificios ?? []) {
      if (e.ticksRestantes === undefined) continue;
      if (e.estado === 'en_construccion') {
        e.completaEn = instanteDeTick(tickActual + Number(e.ticksRestantes));
      }
      delete e.ticksRestantes;
    }
  }
}

/** v3 -> v4 (Fase D / D6): `RelacionPolitica.tributo.cantidadPorTick` -> `cantidadPorMinuto`. Mismo valor
 * (1 tick = 1 minuto) — solo cambia el nombre para que la unidad la diga el campo. */
function migrarV3aV4(s: Record<string, any>): void {
  for (const r of s.relaciones ?? []) {
    if (r.tributo?.cantidadPorTick === undefined) continue;
    r.tributo.cantidadPorMinuto = r.tributo.cantidadPorTick;
    delete r.tributo.cantidadPorTick;
  }
}

/** v4 -> v5 (cierre de Fase D): el `tick` provisional se retira del contrato de eventos. Los eventos
 * guardados pierden `tick` (ya llevan `momento`); las entradas del log por jugador cambian `tick` por el
 * `momento` de mundo equivalente (`isoDeInstante(instanteDeTick(tick))`). */
function migrarV4aV5(s: Record<string, any>): void {
  for (const e of s.eventosDominio ?? []) delete e.tick;
  for (const entradas of Object.values(s.historialJugadores ?? {})) {
    for (const entrada of entradas as Array<Record<string, any>>) {
      if (entrada.tick === undefined) continue;
      entrada.momento = isoDeInstante(instanteDeTick(Number(entrada.tick)));
      delete entrada.tick;
    }
  }
}

/** v5 -> v6 (movimiento de ejércitos, Doc 5.11/5.12): dos colecciones nuevas en el estado. Se rellenan
 * vacías y ya está — una partida de antes de la mecánica no tiene ejércitos en campaña, y un jugador sin
 * registro en `jugadores` usa `LIDERAZGO.base` por diseño (ver `liderazgoDe`, engine/liderazgo.ts), así que
 * no hay nada que reconstruir ni ningún valor que adivinar. */
function migrarV5aV6(s: Record<string, any>): void {
  s.jugadores ??= [];
  s.ejercitos ??= [];
}

/** v6 -> v7 (niebla de guerra, Paso 2): `memoriaPorFaccion`. Se rellena vacío: nadie ha explorado nada
 * todavía, que es exactamente lo cierto — la partida no venía guardando qué había visto cada Facción, así que
 * inventar un pasado sería peor que empezar a recordar desde el primer tick que corra con la mecánica.
 *
 * Consecuencia asumida, y visible para el jugador: al cargar una partida vieja el mapa se tapa entero salvo
 * lo que se esté viendo en ese momento, y se va destapando de nuevo según se juega. */
function migrarV6aV7(s: Record<string, any>): void {
  s.memoriaPorFaccion ??= {};
}

export interface ResumenPartidaEnDisco {
  gameId: string;
  /** Instante de MUNDO de la partida (doc 10) — `instanteDeTick(state.tick)`, derivado. `state.tick` está en
   * el snapshot en todas las versiones de formato, así que no hace falta migrar para leerlo. */
  instante: Instante;
  version: number;
  mapaId: string;
  /** Reloj de PARED del último guardado (ISO 8601), metadato del archivo — no es tiempo de mundo. */
  guardadoEn: string;
}

/**
 * Descubrimiento de partidas (Fase C12, doc 4: "un cliente externo no puede descubrir a qué conectarse; el
 * gameId llega fuera de banda"). Lee el DIRECTORIO, no `RegistroDePartidas`: ese solo conoce lo abierto EN
 * ESTE PROCESO, y una partida que existe en disco pero nadie ha tocado desde el último reinicio debe seguir
 * siendo descubrible. Lectura ligera — `JSON.parse` de cada snapshot, sin reconstruir ninguna `GameSession` —
 * mismo motivo que `RunnerDePartida` es deliberadamente "una partida por proceso": no hay razón para pagar
 * ese coste solo para listar.
 */
export async function listarPartidas(directorio: string): Promise<ResumenPartidaEnDisco[]> {
  let nombres: string[];
  try {
    nombres = await readdir(directorio);
  } catch (err) {
    // Directorio inexistente = ninguna partida se ha guardado todavía en este despliegue, no un error.
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
    throw err;
  }

  const gameIds = nombres.filter((n) => n.endsWith('.json')).map((n) => n.slice(0, -'.json'.length));
  const resumenes = await Promise.all(
    gameIds.map(async (gameId): Promise<ResumenPartidaEnDisco | null> => {
      let snapshot: SnapshotPartida | null;
      try {
        snapshot = await leerSnapshotSiExiste(rutaDe(directorio, gameId));
      } catch (err) {
        // Un archivo ILEGIBLE (JSON truncado por un corte a mitad de escritura, o basura) no puede tumbar el
        // listado ENTERO. Es la misma clase de fallo que el `identidad.json` de más abajo, que ya dejó el
        // endpoint de descubrimiento inservible una vez: un solo archivo defectuoso en el directorio y
        // `GET /admin/partidas` devolvía 500 para todas las demás partidas. Aquí se descarta esa y las demás
        // se listan — pero se GRITA, porque a diferencia de la carrera de abajo esto sí es un problema real
        // que alguien tiene que mirar, y una partida que desaparece del listado en silencio es peor que un
        // error ruidoso.
        console.error(`[partidas] snapshot ilegible '${gameId}.json', se excluye del listado:`, err);
        return null;
      }
      // `null` aquí sería una carrera con un borrado externo entre `readdir` y esta lectura — se descarta en
      // silencio, no es un fallo de quien pidió la lista.
      if (!snapshot) return null;
      // No todo `.json` del directorio es una partida: `crearRepositorioIdentidadEnDisco` guarda su
      // `identidad.json` AQUÍ MISMO (`server/index.ts`), así que el filtro por extensión lo colaba y
      // `snapshot.partida.state` reventaba con un 500 — bastaba con que alguien hubiera iniciado sesión
      // alguna vez para que el endpoint de descubrimiento (Fase C12) dejara de funcionar del todo. Se
      // descarta como la carrera de arriba: lo que no tiene forma de partida, no es una partida.
      if (!snapshot.partida?.state) return null;
      return {
        gameId,
        instante: instanteDeTick(snapshot.partida.state.tick),
        version: snapshot.partida.state.version,
        mapaId: idDeMapa(snapshot.partida.state.mapa),
        guardadoEn: snapshot.guardadoEn,
      };
    })
  );
  return resumenes.filter((r): r is ResumenPartidaEnDisco => r !== null);
}
