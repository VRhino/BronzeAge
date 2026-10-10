// Difusor de canales de VISTA (`Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` §6): mantiene la foto de cada grupo de suscriptores, y
// en cada transición de la partida calcula UNA vez el diff y lo serializa UNA vez para todos los del grupo. Es INFRAESTRUCTURA de
// transporte, como `hub.ts`: qué foto tiene cada canal, para quién y cuándo comparten grupo lo decide `session/vistas/segmentos.ts`.
//
// Garantías que sostienen el resto del contrato:
//  - El delta es SIEMPRE el diff de dos fotos ya filtradas para ese grupo (03 §7.1): nada de lo que no se podía ver entra en ninguna.
//  - La cadena `desde → hasta` no se rompe: una suscripción nueva recibe la foto cacheada del grupo ya volcada, y desde ahí sigue.
//  - Un socket lento no acumula cola: se le saltan los deltas, se marca desactualizado y se le manda foto al recuperarse (03 §6.3).
//  - El derecho a ver se reevalúa en cada flush (03 §7.2).
import type WebSocket from 'ws';
import { diferenciar, type Foto } from '../../session/vistas/diff';
import { anotarMovimiento } from '../../session/vistas/movimiento';
import {
  analizarCanal,
  claveDeGrupo,
  definicionDe,
  proyectarCanal,
  puedeVer,
  type Audiencia,
  type CanalDeVista,
  type ContextoDeProyeccion,
  type ImpurosDePartida,
  type TipoDeCanal,
} from '../../session/vistas/segmentos';
import type { GameSessionState } from '../../session/estado';

/** Lo que el difusor necesita de una partida abierta (la cumple `RunnerDePartida`). */
export interface FuenteDePartida extends ImpurosDePartida {
  getState(): GameSessionState;
  intervaloRelojDeMundoMs(): number | null;
}

export interface OpcionesDifusor {
  /** La partida abierta con ese id, si lo está (en `api.ts`, `RegistroDePartidas.obtener`). */
  fuenteDe: (gameId: string) => FuenteDePartida | undefined;
  /** Ventana de coalescencia (ms): como mucho un flush por partida en ese tiempo. 0 = síncrono (tests). */
  coalescerMs?: number;
  /** Con más de esto en `bufferedAmount` no se envía nada al socket: se marca desactualizado. */
  umbralLentoBytes?: number;
  /** Por debajo de esto un socket desactualizado vuelve a recibir resync + foto. */
  umbralRecuperadoBytes?: number;
  /** Desactualizado durante más de esto (ms): se cierra con 4408. */
  lentoMaxMs?: number;
  /** Tope de suscripciones por socket. */
  maxSuscripcionesPorSocket?: number;
  /** Una foto por canal y socket cada tanto (ms), pedida por `resincronizar`. */
  minMsEntreFotos?: number;
  /** Reloj en ms (inyectable). */
  ahoraMs?: () => number;
  /** Cómo se programa el flush. Por defecto `setTimeout`. */
  programar?: (fn: () => void, ms: number) => unknown;
}

/** Quién es el socket. `derecho` es lo que decide quien abrió la conexión (permiso de administración, membresía de jugador) y se reevalúa en
 * cada flush; a eso se suma lo que sale del estado (`puedeVer`: p. ej. que el jugador tenga héroe). */
export interface ConexionDeVistas {
  audiencia: Audiencia;
  derecho: () => boolean;
}

export type MotivoDeFoto = 'alta' | 'resync' | 'lento' | 'visibilidad';
export type CodigoDeError = 'canal_desconocido' | 'no_autorizado' | 'demasiado_frecuente' | 'formato';

interface Suscriptor {
  conexion: ConexionDeVistas;
  /** Desde cuándo (ms) no recibe nada por ir lento; ausente = al día. */
  desactualizadoDesde?: number;
}

interface Grupo {
  gameId: string;
  /** La de quien creó el grupo: todos los del grupo ven lo mismo, así que cualquiera de ellos proyecta igual. */
  audiencia: Audiencia;
  clave: string;
  canal: CanalDeVista;
  version: number;
  foto: Foto;
  fotoJson?: string;
  suscriptores: Map<WebSocket, Suscriptor>;
}

interface InstrumentosDeTipo {
  flushes: number;
  fotos: Record<MotivoDeFoto, number>;
  bytesEnviados: number;
  bytesFlush: number[];
  bytesFoto: number[];
  msProyeccion: number[];
  msDiff: number[];
  msSerializacion: number[];
  cambios: { upsert: number; patch: number; remove: number; set: number };
  coalescidos: number[];
  socketsLentos: number;
  cierres4408: number;
  revocados: Record<string, number>;
}

const MUESTRAS = 256;
const PERCENTIL = (xs: readonly number[], p: number): number => {
  if (xs.length === 0) return 0;
  const orden = [...xs].sort((a, b) => a - b);
  return orden[Math.min(orden.length - 1, Math.floor((orden.length - 1) * p))]!;
};
const anotar = (xs: number[], x: number): void => {
  xs.push(x);
  if (xs.length > MUESTRAS) xs.shift();
};
const nuevoInstrumento = (): InstrumentosDeTipo => ({
  flushes: 0,
  fotos: { alta: 0, resync: 0, lento: 0, visibilidad: 0 },
  bytesEnviados: 0,
  bytesFlush: [],
  bytesFoto: [],
  msProyeccion: [],
  msDiff: [],
  msSerializacion: [],
  cambios: { upsert: 0, patch: 0, remove: 0, set: 0 },
  coalescidos: [],
  socketsLentos: 0,
  cierres4408: 0,
  revocados: {},
});

export interface MetricasDeVistas {
  suscriptores: number;
  grupos: number;
  porCanal: Record<
    string,
    {
      suscriptores: number;
      claves: number;
      flushes: number;
      fotosEnviadas: Record<MotivoDeFoto, number>;
      bytesEnviados: number;
      bytesPorFlush: { p50: number; p95: number; max: number };
      bytesFoto: { p50: number; max: number };
      ms: { proyeccion: { p50: number; p95: number }; diff: { p50: number; p95: number }; serializacion: { p50: number; p95: number } };
      cambiosPorOp: { upsert: number; patch: number; remove: number; set: number };
      versionesPorDelta: { p50: number; max: number };
      socketsLentos: number;
      cierres4408: number;
      revocados: Record<string, number>;
    }
  >;
}

export class DifusorDeVistas {
  private readonly opciones: Required<Omit<OpcionesDifusor, 'programar' | 'ahoraMs'>> & Pick<OpcionesDifusor, 'programar' | 'ahoraMs'>;
  private readonly grupos = new Map<string, Map<string, Grupo>>();
  private readonly suscripcionesPorSocket = new Map<WebSocket, Set<string>>();
  private readonly ultimaFoto = new Map<WebSocket, Map<string, number>>();
  private readonly pendientes = new Map<string, unknown>();
  private readonly versionesDesdeFlush = new Map<string, number>();
  private readonly instrumentos = new Map<TipoDeCanal, InstrumentosDeTipo>();

  constructor(opciones: OpcionesDifusor) {
    this.opciones = {
      fuenteDe: opciones.fuenteDe,
      coalescerMs: opciones.coalescerMs ?? 100,
      umbralLentoBytes: opciones.umbralLentoBytes ?? 256 * 1024,
      umbralRecuperadoBytes: opciones.umbralRecuperadoBytes ?? 32 * 1024,
      lentoMaxMs: opciones.lentoMaxMs ?? 15_000,
      maxSuscripcionesPorSocket: opciones.maxSuscripcionesPorSocket ?? 64,
      minMsEntreFotos: opciones.minMsEntreFotos ?? 2_000,
      ahoraMs: opciones.ahoraMs,
      programar: opciones.programar,
    };
  }

  private ahora(): number {
    return (this.opciones.ahoraMs ?? Date.now)();
  }

  private instrumento(tipo: TipoDeCanal): InstrumentosDeTipo {
    let i = this.instrumentos.get(tipo);
    if (!i) this.instrumentos.set(tipo, (i = nuevoInstrumento()));
    return i;
  }

  /** La partida se borró: se descartan sus grupos y se cierran sus sockets (4404, como hace el hub con los de jugador). */
  olvidarPartida(gameId: string): void {
    const grupos = this.grupos.get(gameId);
    const cerrados = new Set<WebSocket>();
    for (const g of grupos?.values() ?? []) {
      for (const socket of g.suscriptores.keys()) {
        this.suscripcionesPorSocket.delete(socket);
        if (!cerrados.has(socket)) {
          cerrados.add(socket);
          socket.close(4404, 'partida borrada');
        }
      }
    }
    this.grupos.delete(gameId);
    this.pendientes.delete(gameId);
  }

  // --- Suscripción ---

  /**
   * Alta en un canal. Devuelve `undefined` si todo fue bien (ya se envió `suscrito` y la `foto`), o el error a mandar al cliente. Un canal
   * que el servidor no implementa o que no es de esta audiencia es `canal_desconocido` (no se revela que exista); uno que existe pero no se
   * puede ver, `no_autorizado`.
   */
  suscribir(gameId: string, socket: WebSocket, conexion: ConexionDeVistas, nombre: string): { error: string; codigo: CodigoDeError } | undefined {
    const canal = analizarCanal(nombre, conexion.audiencia);
    if (!canal) return { error: 'canal desconocido', codigo: 'canal_desconocido' };
    const fuente = this.opciones.fuenteDe(gameId);
    if (!fuente) return { error: 'no autorizado', codigo: 'no_autorizado' };

    const propias = this.suscripcionesPorSocket.get(socket) ?? new Set<string>();
    if (!propias.has(nombre) && propias.size >= this.opciones.maxSuscripcionesPorSocket) {
      return { error: 'demasiadas suscripciones', codigo: 'formato' };
    }
    const estado = fuente.getState();
    const ctx = this.contexto(fuente);
    if (!conexion.derecho() || !puedeVer(estado, canal, conexion.audiencia)) return { error: 'no autorizado', codigo: 'no_autorizado' };

    const clave = claveDeGrupo(canal, conexion.audiencia, estado);
    let grupo = this.grupos.get(gameId)?.get(clave);
    if (grupo) this.volcarGrupo(grupo, fuente, ctx); // que la foto que se le da esté al día y la cadena siga sin hueco
    // El volcado pudo vaciar el grupo (todos perdieron el derecho) y retirarlo: se rehace como uno nuevo.
    if (grupo && this.grupos.get(gameId)?.get(clave) !== grupo) grupo = undefined;
    if (!grupo) {
      const t0 = performance.now();
      const foto = proyectarCanal(estado, canal, conexion.audiencia, ctx);
      if (!foto) return { error: 'no autorizado', codigo: 'no_autorizado' };
      this.instrumento(canal.tipo).msProyeccion.push(performance.now() - t0);
      grupo = { gameId, audiencia: conexion.audiencia, clave, canal, version: estado.version, foto, suscriptores: new Map() };
      let delJuego = this.grupos.get(gameId);
      if (!delJuego) this.grupos.set(gameId, (delJuego = new Map()));
      delJuego.set(clave, grupo);
    }

    const antes = grupo.suscriptores.has(socket);
    grupo.suscriptores.set(socket, { conexion });
    propias.add(nombre);
    this.suscripcionesPorSocket.set(socket, propias);
    // La alta fija el momento de la primera foto; un `suscribir` repetido no la salta (usa `resincronizar`).
    if (!antes || !this.ultimaFoto.get(socket)?.has(nombre)) this.marcarFoto(socket, nombre);
    this.enviar(socket, JSON.stringify({ tipo: 'suscrito', canal: nombre, intervaloTickMs: fuente.intervaloRelojDeMundoMs() }));
    this.enviarFoto(grupo, socket, 'alta');
    return undefined;
  }

  desuscribir(gameId: string, socket: WebSocket, nombre: string): void {
    const conexion = this.suscripcionesPorSocket.get(socket);
    conexion?.delete(nombre);
    for (const grupo of this.grupos.get(gameId)?.values() ?? []) {
      if (grupo.canal.nombre === nombre && grupo.suscriptores.delete(socket)) this.limpiarSiVacio(grupo);
    }
  }

  /** `resincronizar`: la foto de nuevo, a lo sumo una cada `minMsEntreFotos` por canal y socket. */
  resincronizar(gameId: string, socket: WebSocket, nombre: string): { error: string; codigo: CodigoDeError } | undefined {
    const grupo = [...(this.grupos.get(gameId)?.values() ?? [])].find((g) => g.canal.nombre === nombre && g.suscriptores.has(socket));
    if (!grupo) return { error: 'no autorizado', codigo: 'no_autorizado' };
    const ultima = this.ultimaFoto.get(socket)?.get(nombre);
    if (ultima !== undefined && this.ahora() - ultima < this.opciones.minMsEntreFotos) return { error: 'demasiado frecuente', codigo: 'demasiado_frecuente' };
    const fuente = this.opciones.fuenteDe(gameId);
    if (fuente) this.volcarGrupo(grupo, fuente, this.contexto(fuente));
    this.enviarFoto(grupo, socket, 'resync');
    return undefined;
  }

  desconectar(gameId: string, socket: WebSocket): void {
    this.suscripcionesPorSocket.delete(socket);
    this.ultimaFoto.delete(socket);
    for (const grupo of [...(this.grupos.get(gameId)?.values() ?? [])]) {
      if (grupo.suscriptores.delete(socket)) this.limpiarSiVacio(grupo);
    }
  }

  private limpiarSiVacio(grupo: Grupo): void {
    if (grupo.suscriptores.size > 0) return;
    this.grupos.get(grupo.gameId)?.delete(grupo.clave);
    if (this.grupos.get(grupo.gameId)?.size === 0) this.grupos.delete(grupo.gameId);
  }

  private marcarFoto(socket: WebSocket, nombre: string): void {
    let m = this.ultimaFoto.get(socket);
    if (!m) this.ultimaFoto.set(socket, (m = new Map()));
    m.set(nombre, this.ahora());
  }

  /** Lo que comparten las proyecciones de un mismo flush: las del mismo héroe se calculan una vez. */
  private contexto(fuente: FuenteDePartida): ContextoDeProyeccion {
    return { impuros: fuente, memo: new Map() };
  }

  // --- Disparo ---

  /** La partida cambió (comando, tick, administración): programa un flush coalescido. Sin suscriptores no hace nada. */
  notificar(gameId: string): void {
    if (!this.grupos.has(gameId)) return;
    this.versionesDesdeFlush.set(gameId, (this.versionesDesdeFlush.get(gameId) ?? 0) + 1);
    if (this.opciones.coalescerMs <= 0) {
      this.volcar(gameId);
      return;
    }
    if (this.pendientes.has(gameId)) return;
    const programar = this.opciones.programar ?? ((fn, ms) => setTimeout(fn, ms));
    this.pendientes.set(
      gameId,
      programar(() => {
        this.pendientes.delete(gameId);
        this.volcar(gameId);
      }, this.opciones.coalescerMs)
    );
  }

  /** Flush inmediato de todos los grupos de la partida (lo que dispara el temporizador; los tests lo llaman directo). */
  volcar(gameId: string): void {
    const fuente = this.opciones.fuenteDe(gameId);
    const grupos = this.grupos.get(gameId);
    if (!fuente || !grupos) return;
    const ctx = this.contexto(fuente);
    for (const grupo of [...grupos.values()]) this.volcarGrupo(grupo, fuente, ctx);
    this.versionesDesdeFlush.delete(gameId);
  }

  private volcarGrupo(grupo: Grupo, fuente: FuenteDePartida, ctx: ContextoDeProyeccion): void {
    const estado = fuente.getState();
    const instr = this.instrumento(grupo.canal.tipo);

    // Reevaluación del derecho (03 §7.2): quien lo pierde recibe `revocado` y sale del grupo, antes de proyectar nada para él.
    for (const [socket, s] of [...grupo.suscriptores]) {
      if (s.conexion.derecho() && puedeVer(estado, grupo.canal, s.conexion.audiencia)) continue;
      this.enviar(socket, JSON.stringify({ tipo: 'revocado', canal: grupo.canal.nombre, motivo: 'sin_derecho' }));
      grupo.suscriptores.delete(socket);
      this.suscripcionesPorSocket.get(socket)?.delete(grupo.canal.nombre);
      instr.revocados['sin_derecho'] = (instr.revocados['sin_derecho'] ?? 0) + 1;
    }
    if (grupo.suscriptores.size === 0) {
      this.limpiarSiVacio(grupo);
      return;
    }
    if (estado.version === grupo.version) return;

    const t0 = performance.now();
    const proyectada = proyectarCanal(estado, grupo.canal, grupo.audiencia, ctx);
    // La partida se reemplazó (empieza en otra versión, más baja): la cadena no continúa, foto nueva a todos (03 §3.2).
    if (proyectada && estado.version < grupo.version) {
      grupo.foto = proyectada;
      grupo.version = estado.version;
      grupo.fotoJson = undefined;
      for (const socket of grupo.suscriptores.keys()) {
        this.enviar(socket, JSON.stringify({ tipo: 'resync', canal: grupo.canal.nombre, motivo: 'partida_reemplazada' }));
        this.enviarFoto(grupo, socket, 'resync');
      }
      return;
    }
    if (!proyectada) {
      // Aquello que miraban ya no existe (plaza destruida, héroe borrado): se les quita el canal.
      const mensaje = JSON.stringify({ tipo: 'revocado', canal: grupo.canal.nombre, motivo: 'ya_no_existe' });
      for (const [socket] of grupo.suscriptores) {
        this.enviar(socket, mensaje);
        this.suscripcionesPorSocket.get(socket)?.delete(grupo.canal.nombre);
      }
      instr.revocados['ya_no_existe'] = (instr.revocados['ya_no_existe'] ?? 0) + grupo.suscriptores.size;
      grupo.suscriptores.clear();
      this.limpiarSiVacio(grupo);
      return;
    }
    const t1 = performance.now();
    const def = definicionDe(grupo.canal);
    const nueva = def.moviles ? anotarMovimiento(proyectada, grupo.foto, def.moviles) : proyectada;
    const dif = diferenciar(grupo.foto, nueva, def.esquema);
    const t2 = performance.now();

    const instante = nueva.instante;
    const avanzoElTick = instante !== grupo.foto.instante;
    const desde = grupo.version;
    grupo.foto = nueva;
    grupo.version = estado.version;
    grupo.fotoJson = undefined;
    anotar(instr.msProyeccion, t1 - t0);
    anotar(instr.msDiff, t2 - t1);
    anotar(instr.coalescidos, this.versionesDesdeFlush.get(grupo.gameId) ?? 1);
    instr.flushes++;
    for (const c of dif.cambios) instr.cambios[c.op]++;

    // Nada que contar y el reloj no avanzó: no hay mensaje (la cadena sigue, `hasta` saltará cuando haya algo).
    if (dif.cambios.length === 0 && Object.keys(dif.escalares).length === 0 && !avanzoElTick) return;

    const t3 = performance.now();
    const mensaje = JSON.stringify({ tipo: 'delta', gameId: grupo.gameId, canal: grupo.canal.nombre, desde, hasta: estado.version, instante, cambios: dif.cambios, escalares: dif.escalares });
    anotar(instr.msSerializacion, performance.now() - t3);
    anotar(instr.bytesFlush, Buffer.byteLength(mensaje));

    const ahora = this.ahora();
    for (const [socket, s] of grupo.suscriptores) {
      if (socket.readyState !== socket.OPEN) continue;
      const buffer = socket.bufferedAmount ?? 0;
      if (s.desactualizadoDesde !== undefined) {
        if (buffer < this.opciones.umbralRecuperadoBytes) {
          s.desactualizadoDesde = undefined;
          this.enviar(socket, JSON.stringify({ tipo: 'resync', canal: grupo.canal.nombre, motivo: 'cliente_lento' }));
          this.enviarFoto(grupo, socket, 'lento');
        } else if (ahora - s.desactualizadoDesde > this.opciones.lentoMaxMs) {
          instr.cierres4408++;
          socket.close(4408, 'cliente lento');
        }
        continue;
      }
      if (buffer > this.opciones.umbralLentoBytes) {
        s.desactualizadoDesde = ahora;
        instr.socketsLentos++;
        continue;
      }
      this.enviar(socket, mensaje, grupo.canal.tipo);
    }
  }

  // --- Envío ---

  private enviar(socket: WebSocket, mensaje: string, tipo?: TipoDeCanal): void {
    if (socket.readyState !== socket.OPEN) return;
    socket.send(mensaje);
    if (tipo) this.instrumento(tipo).bytesEnviados += Buffer.byteLength(mensaje);
  }

  /** La foto del grupo, con su JSON cacheado hasta el próximo flush: dos altas seguidas serializan una vez. */
  private enviarFoto(grupo: Grupo, socket: WebSocket, motivo: MotivoDeFoto): void {
    const instr = this.instrumento(grupo.canal.tipo);
    if (!grupo.fotoJson) {
      const t0 = performance.now();
      const { instante, version: _v, tick: _t, ...datos } = grupo.foto;
      grupo.fotoJson = JSON.stringify({ tipo: 'foto', gameId: grupo.gameId, canal: grupo.canal.nombre, version: grupo.version, instante, datos });
      anotar(instr.msSerializacion, performance.now() - t0);
      anotar(instr.bytesFoto, Buffer.byteLength(grupo.fotoJson));
    }
    instr.fotos[motivo]++;
    this.enviar(socket, grupo.fotoJson, grupo.canal.tipo);
    if (motivo !== 'alta') this.marcarFoto(socket, grupo.canal.nombre);
  }

  // --- Métricas ---

  metricas(): MetricasDeVistas {
    const porCanal: MetricasDeVistas['porCanal'] = {};
    let suscriptores = 0;
    let grupos = 0;
    const cuentas = new Map<TipoDeCanal, { suscriptores: number; claves: number }>();
    for (const delJuego of this.grupos.values()) {
      for (const g of delJuego.values()) {
        grupos++;
        suscriptores += g.suscriptores.size;
        const c = cuentas.get(g.canal.tipo) ?? { suscriptores: 0, claves: 0 };
        c.suscriptores += g.suscriptores.size;
        c.claves++;
        cuentas.set(g.canal.tipo, c);
      }
    }
    for (const [tipo, i] of this.instrumentos) {
      const c = cuentas.get(tipo) ?? { suscriptores: 0, claves: 0 };
      porCanal[tipo] = {
        ...c,
        flushes: i.flushes,
        fotosEnviadas: { ...i.fotos },
        bytesEnviados: i.bytesEnviados,
        bytesPorFlush: { p50: PERCENTIL(i.bytesFlush, 0.5), p95: PERCENTIL(i.bytesFlush, 0.95), max: Math.max(0, ...i.bytesFlush) },
        bytesFoto: { p50: PERCENTIL(i.bytesFoto, 0.5), max: Math.max(0, ...i.bytesFoto) },
        ms: {
          proyeccion: { p50: PERCENTIL(i.msProyeccion, 0.5), p95: PERCENTIL(i.msProyeccion, 0.95) },
          diff: { p50: PERCENTIL(i.msDiff, 0.5), p95: PERCENTIL(i.msDiff, 0.95) },
          serializacion: { p50: PERCENTIL(i.msSerializacion, 0.5), p95: PERCENTIL(i.msSerializacion, 0.95) },
        },
        cambiosPorOp: { ...i.cambios },
        versionesPorDelta: { p50: PERCENTIL(i.coalescidos, 0.5), max: Math.max(0, ...i.coalescidos) },
        socketsLentos: i.socketsLentos,
        cierres4408: i.cierres4408,
        revocados: { ...i.revocados },
      };
    }
    return { suscriptores, grupos, porCanal };
  }
}

