// Telemetría de carga de partidas vivas (contrato 03 §14).
//
// Hermana de `auditoria.ts` y con su mismo molde: JSONL append-only por partida (`<gameId>.carga.jsonl`), una
// versión de formato POR LÍNEA, escritura sin esperar sobre una cola encadenada, poda por edad. Pero responde a
// otra pregunta: la auditoría dice QUIÉN pidió QUÉ; esta dice CUÁNTO COSTÓ el servidor mientras tanto.
//
// **Muestreo, no por mensaje.** Cada `intervaloMs` se ensamblan contadores que ya llevan el difusor
// (`tomarVentana`), el runner (`tomarVentana`, `metricas`) y el proceso (`memoryUsage`, `monitorEventLoopDelay`).
// Nada de esto toca el camino caliente: el coste es el de una pasada cada 10 s (`costeMuestraMs` lo mide en la
// propia línea para poder comprobar el < 1 % del tick).
//
// **Ventana de proceso en el difusor.** Los instrumentos del difusor son por tipo de canal y de PROCESO, no de
// partida: con varias partidas abiertas cada muestra repite la misma ventana de vistas. Para un playtest (una
// partida viva) es exacto; con varias, léase como «el proceso».
//
// **Apagada por defecto.** Sin `TELEMETRIA_INTERVALO_MS` ni un `PUT .../playtest` con id, no hay temporizador, no se
// acumula ninguna ventana (`medicion.activa` es false y los puntos calientes ni lo tocan) y no se escribe ningún
// `.carga.jsonl`. Se enciende por proceso (env, `iniciar`) o por partida (`activarPartida`, que es lo que hace el PUT).
//
// **Idle no se anota.** Una partida sin reloj de mundo y sin conexiones no genera líneas: serían miles de filas
// idénticas por día sin ninguna información.
import { monitorEventLoopDelay, type IntervalHistogram } from 'node:perf_hooks';
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import type { DifusorDeVistas, MetricasDeVistas, VentanaDeVistas } from './difusion/difusorDeVistas';
import type { HubDeDifusion } from './difusion/hub';
import { medicion } from './medicionDeCarga';
import type { RegistroDePartidas } from './registroDePartidas';

/** Versión del formato de CADA LÍNEA (mismo criterio que `FORMATO_AUDITORIA_VERSION`). */
export const FORMATO_CARGA_VERSION = 1;

export const claveDeCarga = (gameId: string): string => `${gameId}.carga.jsonl`;

/** Resumen de una serie: cuántas muestras, p50/p95/máx. */
export interface Resumen {
  n: number;
  p50: number;
  p95: number;
  max: number;
}

const redondear = (x: number): number => Math.round(x * 100) / 100;
export function resumir(xs: readonly number[]): Resumen {
  if (xs.length === 0) return { n: 0, p50: 0, p95: 0, max: 0 };
  const o = Float64Array.from(xs).sort(); // una sola ordenación para los tres números
  const en = (p: number): number => redondear(o[Math.floor((o.length - 1) * p)]!);
  return { n: o.length, p50: en(0.5), p95: en(0.95), max: redondear(o[o.length - 1]!) };
}

export interface MuestraDeCanal {
  flushes: number;
  bytesEnviados: number;
  bytesFlush: Resumen;
  bytesFoto: Resumen;
  /** Mayor foto vista desde que arrancó el proceso (de las métricas de vida): referencia para el sondeo aunque en la ventana no se enviara ninguna. */
  bytesFotoVida: number;
  msProyeccion: Resumen;
  msDiff: Resumen;
  msSerializacion: Resumen;
  fotos: { alta: number; resync: number; lento: number; visibilidad: number };
  cambios: { upsert: number; patch: number; remove: number; set: number };
  socketsLentos: number;
  cierres4408: number;
  suscriptores: number;
  claves: number;
}

export interface MuestraDeCarga {
  formatoVersion: number;
  tipo: 'muestra';
  /** Reloj de PARED (ISO 8601). */
  momento: string;
  gameId: string;
  playtestId?: string;
  /** ms de pared que abarca la ventana (desde la muestra anterior). */
  ventanaMs: number;
  tick: number;
  version: number;
  conexiones: number;
  colaPendiente: number;
  /** Suscripciones y claves de visibilidad por tipo de canal están dentro de `canales`; aquí el total. */
  suscriptores: number;
  claves: number;
  tickMs: Resumen;
  guardadoMs: Resumen;
  /** Tamaño del último snapshot guardado en la ventana (0 si no hubo). */
  guardadoBytes: number;
  canales: Record<string, MuestraDeCanal>;
  memoriaMb: { rss: number; heapUsado: number };
  /** Retraso del bucle de eventos en la ventana (ms). */
  bucleMs: { p50: number; p99: number; max: number };
  /** Lo que costó ensamblar ESTA muestra (ms): la prueba del < 1 % del tick. */
  costeMuestraMs: number;
}

/** Marca de prueba: se anota en el mismo JSONL para que el `playtestId` vigente sobreviva a un reinicio. */
export interface LineaDePlaytest {
  formatoVersion: number;
  tipo: 'playtest';
  momento: string;
  gameId: string;
  playtestId: string | null;
}

export type LineaDeCarga = MuestraDeCarga | LineaDePlaytest;

export interface FiltroDeCarga {
  desde?: string;
  hasta?: string;
  playtestId?: string;
}

export interface FuentesDeCarga {
  partidas: RegistroDePartidas;
  hub: HubDeDifusion;
  difusor: DifusorDeVistas;
}

const MB = 1024 * 1024;
/** El histograma de Node reporta el retraso INCLUIDO el periodo de muestreo (en reposo, p50 ≈ resolución; en Windows, más por la granularidad del temporizador, ~15 ms). Se resta para que «0» sea un bucle libre. */
const RESOLUCION_BUCLE_MS = 10;
const retrasoMs = (ns: number): number => Math.max(0, ns / 1e6 - RESOLUCION_BUCLE_MS);

/** Intervalo al encender en caliente (sin env). */
export const INTERVALO_POR_DEFECTO_MS = 10_000;

export class RegistroDeCarga {
  private cola: Promise<unknown> = Promise.resolve();
  private temporizador?: ReturnType<typeof setInterval>;
  private readonly bucle: IntervalHistogram = monitorEventLoopDelay({ resolution: RESOLUCION_BUCLE_MS });
  private readonly playtests = new Map<string, string | null>();
  /** Partidas con la telemetría encendida en caliente, y si lo está para todo el proceso (env). */
  private readonly activas = new Set<string>();
  private global = false;
  private intervaloMs = INTERVALO_POR_DEFECTO_MS;
  private ultimaMuestraMs = new Map<string, number>();
  private _fallos = 0;

  constructor(
    private readonly almacen: AlmacenDeObjetos,
    private readonly fuentes: FuentesDeCarga,
    private readonly ahora: () => string = () => new Date().toISOString()
  ) {}

  get fallos(): number {
    return this._fallos;
  }

  /** ¿Se está muestreando algo? */
  get activa(): boolean {
    return this.global || this.activas.size > 0;
  }

  /** Enciende el muestreo para TODAS las partidas vivas (env `TELEMETRIA_INTERVALO_MS`). */
  iniciar(intervaloMs: number): void {
    this.global = true;
    this.intervaloMs = intervaloMs;
    this.reevaluar();
  }

  /** Enciende el muestreo para una partida (lo hace `PUT .../playtest` con id). */
  activarPartida(gameId: string): void {
    this.activas.add(gameId);
    this.reevaluar();
  }

  desactivarPartida(gameId: string): void {
    this.activas.delete(gameId);
    this.reevaluar();
  }

  detener(): void {
    this.global = false;
    this.activas.clear();
    this.reevaluar();
  }

  /** Arranca o para el temporizador y el interruptor de los puntos calientes según haya algo activo. */
  private reevaluar(): void {
    if (this.activa && this.temporizador === undefined) {
      medicion.activa = true;
      // Ventanas limpias: lo acumulado en un encendido anterior no cuenta para este.
      this.fuentes.difusor.tomarVentana();
      for (const r of this.fuentes.partidas.abiertas()) r.tomarVentana();
      this.ultimaMuestraMs.clear();
      this.bucle.enable();
      this.temporizador = setInterval(() => this.muestrear(), this.intervaloMs);
      this.temporizador.unref?.();
    } else if (!this.activa && this.temporizador !== undefined) {
      clearInterval(this.temporizador);
      this.temporizador = undefined;
      this.bucle.disable();
      medicion.activa = false;
    }
  }

  async drenar(): Promise<void> {
    await this.cola;
  }

  private anexar(gameId: string, linea: LineaDeCarga): void {
    this.cola = this.cola.then(async () => {
      try {
        await this.almacen.anexar(claveDeCarga(gameId), `${JSON.stringify(linea)}\n`);
      } catch (err) {
        this._fallos++;
        console.error(`[telemetria] no se pudo anotar la carga de ${gameId}:`, err);
      }
    });
  }

  /** `playtestId` vigente de una partida (en memoria; la primera vez, el último anotado en el JSONL). */
  async playtestDe(gameId: string): Promise<string | null> {
    if (!this.playtests.has(gameId)) {
      const { lineas } = await leerCarga(this.almacen, gameId);
      const ultima = [...lineas].reverse().find((l): l is LineaDePlaytest => l.tipo === 'playtest');
      if (!this.playtests.has(gameId)) this.playtests.set(gameId, ultima?.playtestId ?? null);
    }
    return this.playtests.get(gameId) ?? null;
  }

  /** Fija (o, con `null`, cierra) la prueba vigente. Se anota para sobrevivir a un reinicio. */
  fijarPlaytest(gameId: string, id: string | null): void {
    this.playtests.set(gameId, id);
    this.anexar(gameId, { formatoVersion: FORMATO_CARGA_VERSION, tipo: 'playtest', momento: this.ahora(), gameId, playtestId: id });
  }

  /** Una pasada: una línea por partida abierta que esté viva. Síncrona y barata; la escritura va a la cola. */
  muestrear(): void {
    const t0 = performance.now();
    const ventanaVistas = this.fuentes.difusor.tomarVentana();
    const vida = this.fuentes.difusor.metricas().porCanal;
    const bucle = { p50: retrasoMs(this.bucle.percentile(50)), p99: retrasoMs(this.bucle.percentile(99)), max: retrasoMs(this.bucle.max) };
    this.bucle.reset();
    const memoria = process.memoryUsage();
    const ahoraPared = Date.now();
    for (const runner of this.fuentes.partidas.abiertas()) {
      const gameId = runner.gameId;
      const m = runner.metricas();
      const v = runner.tomarVentana();
      const conexiones = this.fuentes.hub.conexionesAbiertas(gameId);
      const anterior = this.ultimaMuestraMs.get(gameId);
      this.ultimaMuestraMs.set(gameId, ahoraPared);
      if (!this.global && !this.activas.has(gameId)) continue;
      if (!m.relojDeMundoActivo && conexiones === 0) continue;
      const subs = this.fuentes.difusor.suscripcionesDe(gameId);
      const muestra = this.armar(gameId, m.tick, m.version, conexiones, m.colaPendiente, ahoraPared - (anterior ?? ahoraPared), v, ventanaVistas, subs, memoria, bucle, vida);
      muestra.costeMuestraMs = redondear(performance.now() - t0);
      const playtestId = this.playtests.get(gameId);
      if (playtestId) muestra.playtestId = playtestId;
      this.anexar(gameId, muestra);
    }
  }

  private armar(
    gameId: string, tick: number, version: number, conexiones: number, colaPendiente: number, ventanaMs: number,
    v: { ticksMs: number[]; guardadoMs: number[]; guardadoBytes: number[] },
    vistas: VentanaDeVistas, subs: Record<string, { suscriptores: number; claves: number }>,
    memoria: NodeJS.MemoryUsage, bucle: { p50: number; p99: number; max: number }, vida: MetricasDeVistas['porCanal']
  ): MuestraDeCarga {
    const canales: Record<string, MuestraDeCanal> = {};
    let suscriptores = 0;
    let claves = 0;
    for (const tipo of new Set([...Object.keys(vistas.porCanal), ...Object.keys(subs)])) {
      const w = vistas.porCanal[tipo];
      const s = subs[tipo] ?? { suscriptores: 0, claves: 0 };
      suscriptores += s.suscriptores;
      claves += s.claves;
      canales[tipo] = {
        flushes: w?.flushes ?? 0,
        bytesEnviados: w?.bytesEnviados ?? 0,
        bytesFlush: resumir(w?.bytesFlush ?? []),
        bytesFoto: resumir(w?.bytesFoto ?? []),
        bytesFotoVida: vida[tipo]?.bytesFoto.max ?? 0,
        msProyeccion: resumir(w?.msProyeccion ?? []),
        msDiff: resumir(w?.msDiff ?? []),
        msSerializacion: resumir(w?.msSerializacion ?? []),
        fotos: w?.fotos ?? { alta: 0, resync: 0, lento: 0, visibilidad: 0 },
        cambios: w?.cambios ?? { upsert: 0, patch: 0, remove: 0, set: 0 },
        socketsLentos: w?.socketsLentos ?? 0,
        cierres4408: w?.cierres4408 ?? 0,
        ...s,
      };
    }
    return {
      formatoVersion: FORMATO_CARGA_VERSION,
      tipo: 'muestra',
      momento: this.ahora(),
      gameId,
      ventanaMs,
      tick,
      version,
      conexiones,
      colaPendiente,
      suscriptores,
      claves,
      tickMs: resumir(v.ticksMs),
      guardadoMs: resumir(v.guardadoMs),
      guardadoBytes: v.guardadoBytes.length === 0 ? 0 : v.guardadoBytes[v.guardadoBytes.length - 1]!,
      canales,
      memoriaMb: { rss: Math.round(memoria.rss / MB), heapUsado: Math.round(memoria.heapUsed / MB) },
      bucleMs: { p50: redondear(bucle.p50), p99: redondear(bucle.p99), max: redondear(bucle.max) },
      costeMuestraMs: 0,
    };
  }

  /** Muestras de una partida (solo `tipo: 'muestra'`), filtradas. Drena antes, como la auditoría. */
  async leer(gameId: string, filtro?: FiltroDeCarga): Promise<{ muestras: MuestraDeCarga[]; playtestVigente: string | null; corruptas: number }> {
    await this.drenar();
    const { lineas, corruptas } = await leerCarga(this.almacen, gameId, filtro);
    return {
      muestras: lineas.filter((l): l is MuestraDeCarga => l.tipo === 'muestra'),
      playtestVigente: await this.playtestDe(gameId),
      corruptas,
    };
  }
}

/** Lee el JSONL de una partida; `[]` si no existe; las líneas ilegibles se cuentan, no tumban. Las marcas de playtest no se filtran por `playtestId`. */
export async function leerCarga(
  almacen: AlmacenDeObjetos,
  gameId: string,
  filtro?: FiltroDeCarga
): Promise<{ lineas: LineaDeCarga[]; corruptas: number }> {
  const contenido = await almacen.leer(claveDeCarga(gameId));
  if (contenido === null) return { lineas: [], corruptas: 0 };
  const lineas: LineaDeCarga[] = [];
  let corruptas = 0;
  for (const texto of contenido.split('\n')) {
    if (texto.trim() === '') continue;
    let l: LineaDeCarga;
    try {
      l = JSON.parse(texto) as LineaDeCarga;
    } catch {
      corruptas++;
      continue;
    }
    if (filtro?.desde !== undefined && l.momento < filtro.desde) continue;
    if (filtro?.hasta !== undefined && l.momento > filtro.hasta) continue;
    if (filtro?.playtestId !== undefined && l.tipo === 'muestra' && l.playtestId !== filtro.playtestId) continue;
    lineas.push(l);
  }
  return { lineas, corruptas };
}

/** Líneas máximas que se conservan tras la poda (a 10 s: ~14 h de una partida viva sostenida por cada 5000). */
export const MAX_LINEAS_CARGA = 100_000;

/**
 * Poda por edad Y por tamaño: conserva `momento >= limite`, y de ellas las últimas `maxLineas`. Devuelve cuántas
 * descartó. Mantiene siempre la última marca de playtest, aunque sea vieja, para no perder la prueba vigente.
 */
export async function podarCarga(almacen: AlmacenDeObjetos, gameId: string, limite: string, maxLineas = MAX_LINEAS_CARGA): Promise<number> {
  const { lineas, corruptas } = await leerCarga(almacen, gameId);
  let conservadas = lineas.filter((l) => l.momento >= limite).slice(-maxLineas);
  const ultimaMarca = [...lineas].reverse().find((l) => l.tipo === 'playtest');
  if (ultimaMarca && !conservadas.includes(ultimaMarca)) conservadas = [ultimaMarca, ...conservadas];
  const descartadas = lineas.length - conservadas.length + corruptas;
  if (descartadas === 0) return 0;
  await almacen.escribir(claveDeCarga(gameId), conservadas.map((l) => `${JSON.stringify(l)}\n`).join(''));
  return descartadas;
}
