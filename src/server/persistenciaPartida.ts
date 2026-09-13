// Guardado de partida en disco (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3).
//
// QUÉ se persiste ya estaba resuelto antes de este archivo: `GameSession.exportar()` devuelve
// `PartidaExportada` con estado, tick, eventos de dominio, mapa/yacimientos, IDs y —desde el commit
// anterior— el estado del RNG. Lo único que faltaba era CÓMO llevarlo a disco, y por eso vive aparte de
// `session/`: `GameSession` es deliberadamente síncrona y sin E/S (doc 7 §2), así que cualquier cosa que
// toque el sistema de archivos tiene que vivir en una capa que sí pueda ser async — de ahí `server/`, la
// primera pieza de una capa que en Fase B/C también tendrá el `RunnerDePartida` y la API HTTP.
//
// Escritura ATÓMICA: la garantiza el adaptador de `AlmacenDeObjetos` (en disco, `.tmp` + `rename`, decidido
// en el doc 4). Escribir directo sobre el nombre final dejaría un archivo a medias si el proceso muere a
// mitad. Este módulo solo pide `almacen.escribir(clave, contenido)` y confía en que sea todo-o-nada.
//
// Versión de concurrencia: la cola serial por `gameId` del `RunnerDePartida` ya elimina la concurrencia de
// escritura en operación normal (doc 4: "la ventaja principal de SQLite" que no hace falta aquí). La
// comprobación de versión de este módulo no es el mecanismo principal de control de concurrencia — es una
// red de seguridad que detecta el síntoma de un bug real (dos procesos escribiendo el mismo `gameId`) y se
// niega a perder datos en silencio en vez de prevenirlo por diseño.
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import { GameSession, type PartidaExportada } from '../session/gameSession';
import { idDeMapa, instanteDeTick } from '../session/estado';
import type { Instante } from '../domain/tiempo';
import { generarMapa, WORLDGEN_VERSION, type MapaGenerado } from '../worldgen';
import { LAYOUT_VERSION } from '../constants';
import { leerEventos } from './eventosDePartida';

/**
 * Versión del ENVOLTORIO del archivo de snapshot. NO es `PartidaExportada.state.version` (versión de LA
 * PARTIDA, sube en cada comando aceptado) ni `worldgenVersion` (versión del generador de mundo) — esta sube
 * solo si cambia la FORMA del archivo en sí, para poder rechazar snapshots de una build anterior sin adivinar
 * por la forma del JSON.
 *
 * Historia (v1→v12): modelo temporal en `Instante`/`momento`, construcción por fecha en vez de contador,
 * `jugadores`/`ejercitos`, niebla de guerra, jugador situado, unión en campo, comercio físico, revamp de
 * caravanas. Cada salto tuvo su función de migración encadenada en `cargarPartida`.
 *
 * **Esa cadena se retiró el 2026-09-09**: no quedaban partidas de un formato anterior que arrastrar (todas
 * las de disco ya estaban en v12), y mantener 11 conversiones vivas por si acaso era coste sin consumidor.
 * `cargarPartida` ahora acepta solo el formato vigente y rechaza el resto — mismo criterio que
 * `persistenciaIdentidad.ts` desde el principio. La próxima mecánica que cambie la forma del snapshot sube
 * este número y, SI en ese momento existen partidas que preservar, vuelve a añadir su migración puntual.
 *
 * v13 (2026-09-09): el snapshot deja de contener dos cosas que lo engordaban sin ser fuente de verdad —
 *   - `state.mapa` se guarda solo como `{ version, config }` y se REGENERA al cargar (`generarMapa`, función
 *     pura de la seed; el rechazo por `worldgenVersion` ya garantizaba que se puede). Eran ~130 KB constantes.
 *   - `state.eventosDominio` se va a `<gameId>.eventos.jsonl` (append-only, ver `eventosDePartida.ts`). Eran
 *     ~300 KB y creciendo, reescritos en cada comando.
 */
export const FORMATO_SNAPSHOT_VERSION = 13;

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

/** Se lanza al cargar un snapshot de un formato de envoltorio distinto del vigente. Ya no se migra ningún
 * formato anterior (ver la nota en `FORMATO_SNAPSHOT_VERSION`). */
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

/** Se lanza al cargar un snapshot guardado con otra revisión de la geometría del asentamiento
 * (`LAYOUT_VERSION`, constants.ts): la huella de cada edificio se deriva del catálogo vigente, así que sus
 * `posicion` guardadas se reinterpretarían en silencio. Los snapshots anteriores a BA-005 no traen el campo
 * (`undefined`) y caen aquí también. */
export class LayoutVersionNoCoincideError extends Error {
  constructor(
    public readonly gameId: string,
    public readonly versionEnDisco: number | undefined
  ) {
    super(
      `partida '${gameId}': guardada con la revisión geométrica ${versionEnDisco ?? '(anterior a BA-005)'} y esta ` +
        `build usa la ${LAYOUT_VERSION} — los edificios guardados no encajan en las huellas actuales.`
    );
  }
}

/** Clave del snapshot en el almacén. */
export function claveDeSnapshot(gameId: string): string {
  return `${gameId}.json`;
}

async function leerSnapshotSiExiste(almacen: AlmacenDeObjetos, gameId: string): Promise<SnapshotPartida | null> {
  const contenido = await almacen.leer(claveDeSnapshot(gameId));
  return contenido === null ? null : (JSON.parse(contenido) as SnapshotPartida);
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
export async function guardarPartida(almacen: AlmacenDeObjetos, sesion: GameSession, guardadoEn: string, opciones: { forzar?: boolean } = {}): Promise<void> {
  const partida = sesion.exportar();

  const existente = await leerSnapshotSiExiste(almacen, sesion.gameId);
  if (!opciones.forzar && existente && existente.partida.state.version > partida.state.version) {
    throw new ConflictoDeVersionError(sesion.gameId, existente.partida.state.version, partida.state.version);
  }

  const snapshot: SnapshotPartida = {
    formatoVersion: FORMATO_SNAPSHOT_VERSION,
    guardadoEn,
    partida: {
      ...partida,
      state: {
        ...partida.state,
        // El mapa es función pura de la seed (`generarMapa`) — se guarda solo lo mínimo para regenerarlo
        // (`config`) e identificarlo (`version`, para `idDeMapa` en `listarPartidas`), no los ~130 KB de
        // terreno/nodos/ríos. `cargarPartida` lo reconstruye; el rechazo por `worldgenVersion` garantiza
        // que sale idéntico al guardado.
        mapa: { version: partida.state.mapa.version, config: partida.state.mapa.config } as MapaGenerado,
        // El historial vive en `<gameId>.eventos.jsonl` (append-only, ver `eventosDePartida.ts`), no aquí —
        // el snapshot se reescribe entero en cada comando y `eventosDominio` solo crece. Lo anexa
        // `RunnerDePartida` tras cada guardado; `cargarPartida` lo rehidrata.
        eventosDominio: [],
      },
    },
  };
  await almacen.escribir(claveDeSnapshot(sesion.gameId), JSON.stringify(snapshot));
}

/** Lo que devuelve `cargarPartida`. Un objeto de un solo campo y no la `GameSession` a secas porque D5 le
 * añadió el `guardadoEn` del snapshot, que el reloj de mundo necesitaba para calcular el catch-up tras un
 * reinicio; **ese campo se retiró el 2026-09-05**, cuando se decidió que el mundo no avanza mientras el
 * servidor está caído y dejó de haber catch-up que calcular. El envoltorio se queda: es el punto natural
 * donde volver a colgar metadatos del archivo si alguna vez hacen falta, y quitarlo tocaría treinta llamadas
 * a cambio de nada. El `guardadoEn` sigue existiendo donde sí tiene consumidor: `SnapshotPartida` (metadato
 * del archivo) y `ResumenPartidaEnDisco` (lo sirve `GET /admin/partidas`). */
export interface PartidaCargada {
  sesion: GameSession;
}

/**
 * Reconstruye la `GameSession` guardada bajo la clave `<gameId>.json`.
 * `null` si no existe ningún snapshot para ese `gameId` — no es un error, es el caso "partida nueva".
 */
export async function cargarPartida(almacen: AlmacenDeObjetos, gameId: string): Promise<PartidaCargada | null> {
  const snapshot = await leerSnapshotSiExiste(almacen, gameId);
  if (!snapshot) return null;

  // Sin migración: solo el formato vigente se carga, el resto se rechaza (ver `FORMATO_SNAPSHOT_VERSION`).
  if (snapshot.formatoVersion !== FORMATO_SNAPSHOT_VERSION) {
    throw new FormatoSnapshotNoSoportadoError(gameId, snapshot.formatoVersion);
  }
  const partida = snapshot.partida;
  if (partida.worldgenVersion !== WORLDGEN_VERSION) {
    throw new WorldgenVersionNoCoincideError(gameId, partida.worldgenVersion);
  }
  if (partida.layoutVersion !== LAYOUT_VERSION) {
    throw new LayoutVersionNoCoincideError(gameId, partida.layoutVersion);
  }

  // Reconstruye lo que el snapshot ya no guarda (formato v13):
  //  - el mapa, función pura de la seed — `worldgenVersion` ya se comprobó, así que sale idéntico al guardado.
  //  - el historial de eventos, desde el JSONL hermano (filtrado a `<= version` por si un append quedó por
  //    delante de un snapshot revertido).
  partida.state.mapa = generarMapa(partida.state.mapa.config);
  partida.state.eventosDominio = await leerEventos(almacen, gameId, partida.state.version);
  return { sesion: GameSession.importar(partida) };
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
 * gameId llega fuera de banda"). Lee el ALMACÉN, no `RegistroDePartidas`: ese solo conoce lo abierto EN ESTE
 * PROCESO, y una partida que existe en disco pero nadie ha tocado desde el último reinicio debe seguir siendo
 * descubrible. Lectura ligera — `JSON.parse` de cada snapshot, sin reconstruir ninguna `GameSession` — mismo
 * motivo que `RunnerDePartida` es deliberadamente "una partida por proceso": no hay razón para pagar ese
 * coste solo para listar.
 */
export async function listarPartidas(almacen: AlmacenDeObjetos): Promise<ResumenPartidaEnDisco[]> {
  const nombres = await almacen.listar('');
  // `.json` pero no `.eventos.jsonl`/`.auditoria.jsonl` (acaban en `.jsonl`); `identidad.json` sí cuela y se
  // descarta abajo por su forma.
  const gameIds = nombres.filter((n) => n.endsWith('.json')).map((n) => n.slice(0, -'.json'.length));
  const resumenes = await Promise.all(
    gameIds.map(async (gameId): Promise<ResumenPartidaEnDisco | null> => {
      let snapshot: SnapshotPartida | null;
      try {
        snapshot = await leerSnapshotSiExiste(almacen, gameId);
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
      // No todo `.json` del almacén es una partida: el repositorio de identidad guarda su `identidad.json`
      // en el mismo almacén (`server/index.ts`), así que el filtro por extensión lo colaba y
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
