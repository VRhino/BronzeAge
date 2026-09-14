// Estado de una partida y sus transformaciones PURAS (Docs/Arquitectura/7_Diseno_GameSession.md).
//
// Vive aparte de `gameSession.ts` por dos razones: evita el ciclo de importación que habría entre la clase y
// sus manejadores de comando (los manejadores necesitan el tipo del estado, la clase necesita los
// manejadores), y deja claro que el estado es DATOS — no hay ningún método que lo mute en el sitio, solo
// funciones que devuelven un estado nuevo. Esa disciplina es lo que permite al runner de Fase B3 aplicar un
// comando, intentar persistirlo y DESCARTAR el estado nuevo si la escritura falla.
import type {
  AcuerdoTrueque,
  Asentamiento,
  CaminoComercial,
  CampamentoBandido,
  Caravana,
  Faccion,
  OrdenMercado,
  RelacionPolitica,
  Titulo,
  ZonaFaccion,
  ZonaInfluencia, Ejercito, Heroe } from '../domain/types';
import type { TrazadoAsentamiento } from '../engine/trazado';
import type { MemoriaFaccion } from '../engine/memoria';
import type { EventoDominio } from '../domain/eventos';

/**
 * Un `EventoDominio` tal y como vive en `GameSessionState.eventosDominio` — con `version` (Fase C13, doc 4:
 * "eventosDominio crece sin techo y viaja entero cada vez"). `version` NO es del dominio de juego (el motor,
 * `engine/*`, no sabe qué es una versión de partida), así que no vive en `domain/eventos.ts` — es
 * exclusivamente el punto de la partida en el que `exito()` (`session/comandos/tipos.ts`, el ÚNICO sitio
 * donde sube `GameSessionState.version`) estampó el evento. Permite paginar incrementalmente
 * (`?desde=<version>`) en vez de mandar el histórico completo en cada lectura — ver `eventosDesde` más abajo.
 */
export interface EventoDominioConVersion extends EventoDominio {
  version: number;
}
import type { MapaGenerado } from '../worldgen';
import type { EstadoMapa } from '../world/mapa';
import type { EstadoSimulacion } from '../engine/simulation';
import { SIMULACION } from '../constants';
import { instante, type Instante } from '../domain/tiempo';

/** `SIMULACION.epocaInicial` en ms, para `instanteDeTick` (más abajo). Al principio del módulo para que
 * ninguna función declarada arriba dependa de un `const` en zona muerta temporal. */
const EPOCA_MS = new Date(SIMULACION.epocaInicial).getTime();

/**
 * Entrada de log en texto. Ya no hay un log global en el estado (la consola de administración se derivaba de
 * `eventosDominio`, y esa consola vive ahora en el repo de cliente); este tipo se queda como el de
 * `historialHeroes` y el de `ProyeccionJugador.historial`.
 *
 * Administración: nunca viaja a un jugador tal cual (Docs/Arquitectura/7_Diseno_GameSession.md §7.1 — el log
 * global narra lo que pasa en TODO el mundo, así que incluirlo en una proyección de jugador sería una fuga de
 * información de facciones rivales).
 */
export interface EventoLogAdmin {
  /** Instante de MUNDO (ISO 8601) del hecho — mismo criterio que `EventoDominio.momento`, de donde se deriva.
   * Fase D cerrada: ya no lleva `tick` (doc 6 §4 regla (b)). */
  momento: string;
  mensaje: string;
}

/**
 * Estado completo y serializable de una partida. Sin nada de presentación: la fachada `Mapa` se deriva de
 * `mapa`/`estadoMapa` y nunca se guarda, igual que en `GameStore`.
 */
export interface GameSessionState {
  gameId: string;
  mapa: MapaGenerado;
  estadoMapa: EstadoMapa;
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  /** Jugadores con estado propio de partida (Doc 5.11) — hoy solo su Liderazgo. NO es el registro de
   * identidad (eso vive en `session/identidad`): es estado de juego. Un jugador que no aparezca aquí usa
   * `LIDERAZGO.base`, así que la lista solo necesita crecer cuando alguien se desvíe del valor por defecto. */
  heroes: Heroe[];
  /** Ejércitos en campaña (Doc 5.12) — los mueve `avanzarEjercitos` al final de la cadena del tick. */
  ejercitos: Ejercito[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  caminos: CaminoComercial[];
  campamentosBandidos: CampamentoBandido[];
  bandidosProximoSpawnEn: Instante;
  /** Facciones que gobierna el NPC en vez de un jugador humano. Vive en la partida y no en el runner
   * (doc 7 §7.2): cambia el resultado del tick, así que un reinicio con otra configuración divergiría de lo
   * que el snapshot dice haber pasado. En partida real no cambia en caliente una vez elegida. */
  faccionesNpcIds: string[];
  /** Contador de avances del motor — su unidad interna PROVISIONAL (doc 10). Se guarda porque el instante de
   * mundo se DERIVA de él (`instanteDeTick`), no al revés. Para cualquier contrato hacia afuera (DTOs,
   * eventos) la referencia temporal es el `instante`/`momento`, nunca este número — ver `EstadoAdmin.instante`,
   * `ProyeccionJugador.instante`, `EventoDominio.momento`. */
  tick: number;
  /** Sube en cada mutación aceptada. Un comando rechazado NUNCA la incrementa — base del control de
   * concurrencia optimista de Fase B3. */
  version: number;
  historialHeroes: Record<string, EventoLogAdmin[]>;
  /**
   * `heroeId` -> momento (ISO 8601) en que abandonó su última Facción (`dejarFaccion`, a petición del
   * usuario 2026-08-27). Única razón de ser: `crearFaccion` lo consulta para el cooldown de
   * `CIUDADANIA.cooldownCreacionFaccionDias` — anti-abuso contra "crear, abandonar, crear" en bucle. No es
   * historial (no guarda TODAS las salidas, solo la última) ni afecta a `unirseAFaccion`, que no tiene cooldown.
   */
  salidasFaccionPorHeroe: Record<string, Instante>;
  /**
   * Todo lo que ha ocurrido en la partida, en forma estructurada: la ÚNICA representación de los hechos, en
   * texto estructurado y no plano (antes se persistía además un `log: EventoLogAdmin[]` en paralelo, el mismo
   * hecho dos veces, y la copia de texto plano no se podía filtrar por audiencia).
   *
   * Vive aquí EN MEMORIA, pero NO en el snapshot: desde el formato v13 se persiste aparte, en un JSONL
   * append-only (`server/eventosDePartida.ts`), y `cargarPartida` lo rehidrata. El snapshot solo crece con el
   * estado de juego, no con el historial.
   *
   * Administración: NO viaja a un jugador tal cual (doc 7 §7.1 — narra lo que pasa en TODO el mundo, así que
   * mandarlo entero sería una fuga de información de facciones rivales). Las proyecciones por audiencia de
   * Fase C filtran sobre `codigo`/`payload`, que es justamente para lo que existe.
   */
  eventosDominio: EventoDominioConVersion[];
  /** Lo que cada Facción RECUERDA del mundo (niebla de guerra — `engine/memoria.ts`), por `faccionId`: qué
   * terreno ha llegado a ver y la última ficha de cada plaza ajena que vio. Lo que ve AHORA no está aquí: se
   * deriva al proyectar. Una Facción ausente no ha visto nada, así que un snapshot viejo no necesita
   * migración — solo empieza a recordar a partir del primer tick que corra con la mecánica. */
  memoriaPorFaccion: Record<string, MemoriaFaccion>;
}

/** Proyecta el estado de partida al subconjunto que consume el motor. El motor no conoce `gameId`, `version`,
 * logs ni `faccionesNpcIds` — y no debe. */
export function estadoSimulacionDe(estado: GameSessionState): EstadoSimulacion {
  return {
    asentamientos: estado.asentamientos,
    facciones: estado.facciones,
    caravanas: estado.caravanas,
    ejercitos: estado.ejercitos,
    acuerdos: estado.acuerdos,
    ordenes: estado.ordenes,
    relaciones: estado.relaciones,
    titulos: estado.titulos,
    caminos: estado.caminos,
    campamentosBandidos: estado.campamentosBandidos,
    bandidosProximoSpawnEn: estado.bandidosProximoSpawnEn,
    memoriaPorFaccion: estado.memoriaPorFaccion,
    heroes: estado.heroes,
  };
}

/** Vuelca en el estado el resultado de un tick del motor. No toca `version` ni los logs: de eso se encarga
 * `exito()` (`comandos/tipos.ts`), que es el único sitio donde se incrementa la versión. */
export function conResultadoDeSimulacion(estado: GameSessionState, simulacion: EstadoSimulacion): GameSessionState {
  return {
    ...estado,
    asentamientos: simulacion.asentamientos,
    facciones: simulacion.facciones,
    caravanas: simulacion.caravanas,
    ejercitos: simulacion.ejercitos,
    acuerdos: simulacion.acuerdos,
    ordenes: simulacion.ordenes,
    relaciones: simulacion.relaciones,
    titulos: simulacion.titulos,
    caminos: simulacion.caminos,
    campamentosBandidos: simulacion.campamentosBandidos,
    bandidosProximoSpawnEn: simulacion.bandidosProximoSpawnEn,
    memoriaPorFaccion: simulacion.memoriaPorFaccion,
    heroes: simulacion.heroes,
  };
}

/**
 * Eventos con `version` estrictamente mayor que `desde` (Fase C13) — la mitad "sin filtrar por audiencia" del
 * cursor incremental; `proyectarParaJugador`/`eventosDominioParaJugador` en `session/proyecciones/jugador.ts`
 * hacen la versión filtrada. `estado.eventosDominio` vive más nuevo primero (`exito()` los antepone); se
 * devuelven en orden CRONOLÓGICO (más viejo primero) porque es el orden natural para que un cliente los vaya
 * aplicando/anexando a su log.
 */
export function eventosDesde(estado: GameSessionState, desde: number): EventoDominioConVersion[] {
  const nuevos: EventoDominioConVersion[] = [];
  for (const evento of estado.eventosDominio) {
    if (evento.version <= desde) break; // más viejo que el cursor: todo lo que sigue también lo es
    nuevos.push(evento);
  }
  return nuevos.reverse();
}

/**
 * Hecho administrativo que NO es un comando de partida (hoy solo los cambios de balance, ver
 * `GameSession.registrarEventoAdministrativo`). Mantiene `mensaje` como texto libre a propósito: es un rastro
 * temporal, y lo sustituye la auditoría real de Fase C (actor, fecha, versión previa y nueva). `tick` solo
 * sirve para derivar el `momento` de mundo.
 */
export function eventoAdministrativo(tick: number, mensaje: string): EventoDominio {
  return { codigo: 'administrativo', mensaje, momento: isoDeInstante(instanteDeTick(tick)) };
}

/**
 * Identidad del mapa de una partida (Fase C11, doc 9: "el mapa deja de ser estado, es un asset"). Función
 * PURA de `MapaGenerado.config` (`seed` + `region` — `ancho`/`alto` no varían nunca hoy, siempre
 * `MAPA_DEFAULT`, así que no hace falta incluirlos) y del algoritmo de generación (`version`,
 * `WORLDGEN_VERSION`). Determinista: el mismo mapa siempre produce el mismo id, y solo cambia si el mapa
 * cambia de verdad (`regenerarMundo`/`forzar`, o una subida de `WORLDGEN_VERSION`).
 *
 * No es un hash: es legible a propósito, para poder leer un id en un log o una URL y saber de qué mapa se
 * trata sin decodificar nada. Nunca se guarda — se deriva cada vez que hace falta, igual que `instanteDeTick`.
 */
export function idDeMapa(mapa: GameSessionState['mapa']): string {
  return `v${mapa.version}-s${mapa.config.seed}${mapa.config.region ? `-${mapa.config.region}` : ''}`;
}

/**
 * Instante de MUNDO (`Instante`, ms desde época) correspondiente a un `tick` (Fase D, doc 10 §2):
 * `epocaInicial + tick × duracionTickMs`. Igual que `idDeMapa`: función pura, **derivada del estado, nunca
 * almacenada** — así un snapshot antiguo se reconstruye sin datos nuevos y dos partidas con la misma seed en
 * el mismo tick están en el mismo instante.
 *
 * Es el ÚNICO puente entre el `tick` (unidad interna del motor) y el tiempo que fechan las reglas. El reloj
 * de pared (`Date.now()`) no participa: vive en `server/` y solo para cosas que no son estado de partida.
 */
export function instanteDeTick(tick: number): Instante {
  return instante(EPOCA_MS + tick * SIMULACION.duracionTickMs);
}

/** `Instante` (ms) → ISO 8601, para lo que sale por el cable (`EventoDominio.momento`, DTOs). Vive aquí y no
 * en el núcleo puro porque construye un `Date`; nunca lee el reloj. */
export function isoDeInstante(i: Instante): string {
  return new Date(i).toISOString();
}

/** Geometría por frame de TODOS los asentamientos (Fase C10) — lo que calcula
 * `RunnerDePartida.geometriaAsentamientos()`. Vive aquí y no en `server/` porque `proyectarParaJugador`
 * (`session/proyecciones/jugador.ts`) necesita el TIPO para filtrarla a lo propio del jugador, y `session/` no
 * puede importar de `server/` (test de arquitectura). */
export interface GeometriaAsentamientos {
  zonas: ZonaInfluencia[];
  zonasFusionadas: ZonaFaccion[];
  trazadoPorAsentamiento: Record<string, TrazadoAsentamiento>;
}

/**
 * Vista de administración del estado completo: TODO sin filtrar por audiencia (a diferencia de
 * `proyectarParaJugador`, esto sigue siendo "todas las Facciones, log global" — Fase C3), pero con `mapa`
 * sustituido por `mapaId` (Fase C11). El mapa no cambia nunca durante la partida (125 KB medidos, idénticos
 * byte a byte del tick 0 al 200 — doc 6 §6.4): mandarlo entero en cada lectura de estado es la misma fuga de
 * ancho de banda que ya se corrigió en la proyección de jugador, solo que aquí no se había notado porque el
 * cliente de administración es el único que la sufre hoy.
 *
 * El cliente pide el mapa real, una vez, por `GET .../mapa/:mapaId` — servible con cache eterna porque el id
 * ya captura su identidad completa.
 *
 * `preciosReferencia` (auditoría de doc 9, 2026-08-26) NO lo rellena esta función: es una regla de entrada
 * PRIVILEGIADA (necesita el almacén de TODOS los asentamientos), calculada con caché de un minuto real en
 * `RunnerDePartida.preciosReferencia()` — impuro, vive en `server/`, no aquí. Se declara en este tipo porque
 * es lo que de verdad viaja por el cable (mismo patrón que `RespuestaComando.proyeccion` en C6: el tipo
 * describe el CONTRATO, no todo tiene que salir de una sola función pura). Quien construye la respuesta HTTP
 * es responsable de fusionarlo — ver `server/rutas/admin.ts` y `jugador.ts`.
 *
 * `zonas`/`zonasFusionadas`/`trazadoPorAsentamiento` (Fase C10, doc 9): la geometría por frame que antes
 * calculaba `render()` en cada `mousemove` — entrada privilegiada (mira TODOS los asentamientos), así que
 * tampoco la calcula esta función pura: la añade `RunnerDePartida.geometriaAsentamientos()`, mismo criterio
 * que `preciosReferencia` aunque sin TTL (ver el comentario de `cacheGeometria` en ese archivo).
 */
export type EstadoAdmin = Omit<GameSessionState, 'mapa' | 'eventosDominio'> &
  GeometriaAsentamientos & {
    mapaId: string;
    /** Instante de MUNDO de la partida (doc 10 / D4) — `instanteDeTick(tick)`, derivado como `mapaId`, no
     * almacenado. Es la referencia temporal del contrato: `tick` sigue viajando pero es la unidad interna. */
    instante: Instante;
    preciosReferencia: Record<string, number>;
    /** ms de reloj de PARED entre ticks para esta partida, o `null` si su reloj de mundo está parado. Impuro
     * como `preciosReferencia`: vive en el `RunnerDePartida` (`server/`), no en el estado — lo fusiona la
     * ruta HTTP. La consola de administración lo muestra en la pestaña "Mundo". */
    relojDeMundoIntervaloMs: number | null;
  };

/** `eventosDominio` sale de aquí desde el 2026-09-05 (follow-up de C13): crecía sin techo y era el 87-88 %
 * de esta respuesta, para reenviar en cada lectura un historial que el cliente ya tenía. Se pide por el
 * cursor `GET .../eventos?desde=<version>`. Ver la nota de cabecera de `session/proyecciones/jugador.ts`. */
const CAMPOS_IMPUROS = ['preciosReferencia', 'zonas', 'zonasFusionadas', 'trazadoPorAsentamiento', 'relojDeMundoIntervaloMs'] as const;

/** Devuelve todo MENOS los campos impuros de arriba: los añade el llamador HTTP — spread sobre este resultado
 * más `{ preciosReferencia: runner.preciosReferencia(), relojDeMundoIntervaloMs:
 * runner.intervaloRelojDeMundoMs(), ...runner.geometriaAsentamientos() }` completa un `EstadoAdmin`. */
export function vistaAdminDeEstado(estado: GameSessionState): Omit<EstadoAdmin, (typeof CAMPOS_IMPUROS)[number]> {
  const { mapa, eventosDominio, ...resto } = estado;
  return { ...resto, mapaId: idDeMapa(mapa), instante: instanteDeTick(estado.tick) };
}

/** Añade una entrada al historial de un jugador concreto (administración, igual que el log). */
export function conHistorialDeJugador(estado: GameSessionState, heroeId: string, mensaje: string): GameSessionState {
  if (!heroeId) return estado;
  const previo = estado.historialHeroes[heroeId] ?? [];
  return {
    ...estado,
    historialHeroes: {
      ...estado.historialHeroes,
      [heroeId]: [{ momento: isoDeInstante(instanteDeTick(estado.tick)), mensaje }, ...previo],
    },
  };
}
