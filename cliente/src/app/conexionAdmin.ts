// Conexión WebSocket de administración (`/v1/admin/partidas/:gameId/tiempo-real`, Docs/Coordinacion/03_Tiempo_real_por_segmentos.md §3):
// suscribe canales, guarda el estado de cada uno en un `AlmacenDeCanal` y avisa cuando cambia. Solo habla por red: no interpreta el
// contenido de los canales (eso es `estadoEnVivo.ts`).
//
// Reglas del cliente (03 §3.3): la foto sustituye; un delta que no encadena pide `resincronizar` y se descarta hasta que llegue la foto;
// al reconectar las suscripciones no sobreviven, así que se repiten todas y llegan fotos nuevas.
import type { EventoDominioConVersion } from '@motor/session/estado';
import { AlmacenDeCanal, type DeltaDeCanal } from './almacenDeCanal';

export const CANAL_PARTIDA = 'admin/partida';
/** Canal de HECHOS (no de estado): cada evento de dominio de la partida, sin foto ni deltas. */
export const CANAL_EVENTOS = 'admin/eventos';
export const canalAsentamiento = (id: string): string => `admin/asentamiento/${id}`;
export const canalCampamento = (id: string): string => `admin/campamento/${id}`;
export const canalHeroe = (id: string): string => `admin/heroe/${id}`;

/** Entidades únicas de cada canal de detalle (03 §4: `asentamiento`, `campamento` y `heroe` no son arrays). */
function unicasDe(canal: string): string[] {
  if (canal.startsWith('admin/asentamiento/')) return ['asentamiento'];
  if (canal.startsWith('admin/campamento/')) return ['campamento'];
  if (canal.startsWith('admin/heroe/')) return ['heroe'];
  return [];
}

export interface OpcionesDeConexion {
  /** La URL del socket (con la sesión). Se pide en cada intento: la sesión pudo cambiar. */
  url: () => Promise<string>;
  /** Cambió el contenido de un canal (llegó una foto o un delta). */
  alCambiar: (canal: string) => void;
  /** Cambió el estado de la conexión (abierta, caída, canal esperando foto). */
  alEstado: () => void;
  /** El servidor no implementa ese canal: quien lo pedía cae al sondeo HTTP. */
  alCanalDesconocido: (canal: string) => void;
  /** Un intento de abrir falló: por si la sesión caducó. */
  alFallarApertura: () => void;
  /** Llegó un evento de dominio por `admin/eventos`. */
  alEvento: (evento: EventoDominioConVersion) => void;
  /** El servidor confirmó la suscripción a `admin/eventos` (la primera vez, o tras reconectar o reanudar): hay que cerrar el hueco con el cursor HTTP. */
  alSuscritoEventos: () => void;
}

interface CanalSuscrito {
  almacen: AlmacenDeCanal;
  /** Sin foto todavía, o cadena rota: se descartan los deltas hasta que llegue la foto. */
  esperandoFoto: boolean;
  ultimaPeticionDeFoto: number;
}

const MIN_ENTRE_FOTOS_MS = 2000;

export class ConexionAdmin {
  /** ms entre ticks del reloj de mundo; `null` si la partida no tiene reloj; `undefined` hasta saberlo. */
  intervaloTickMs: number | null | undefined;

  private socket: WebSocket | undefined;
  private readonly deseados = new Set<string>();
  private readonly canales = new Map<string, CanalSuscrito>();
  private cerrado = false;
  /** `admin/eventos` suscrito y confirmado en el socket actual: mientras sea así no hace falta pedir eventos por HTTP. */
  eventosActivos = false;
  /** Pestaña oculta: el socket está cerrado a propósito (y con él las suscripciones del servidor) y no se reconecta hasta `reanudar`. */
  private pausada = false;
  private intentos = 0;
  private reintento: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly opciones: OpcionesDeConexion) {}

  abrir(): void {
    void this.conectar();
  }

  get estaPausada(): boolean {
    return this.pausada;
  }

  /** Suelta el socket sin perder qué se quería ver ni lo último recibido: el servidor deja de contar estas suscripciones. */
  pausar(): void {
    if (this.pausada) return;
    this.pausada = true;
    clearTimeout(this.reintento);
    this.socket?.close();
  }

  /** Vuelve a abrir y a suscribir todo; llegan fotos nuevas (mientras tanto se sigue viendo lo último recibido). */
  reanudar(): void {
    if (!this.pausada) return;
    this.pausada = false;
    this.intentos = 0;
    void this.conectar();
  }

  cerrar(): void {
    this.cerrado = true;
    clearTimeout(this.reintento);
    this.socket?.close();
    this.socket = undefined;
  }

  /** ¿Se puede fiar de `admin/partida` en vez del sondeo? Socket abierto y con foto vigente. */
  get sano(): boolean {
    const c = this.canales.get(CANAL_PARTIDA);
    return this.socket?.readyState === WebSocket.OPEN && !!c && !c.esperandoFoto && c.almacen.conFoto;
  }

  almacen(canal: string): AlmacenDeCanal | undefined {
    const c = this.canales.get(canal);
    return c?.almacen.conFoto && !c.esperandoFoto ? c.almacen : undefined;
  }

  /** Lo último recibido de un canal aunque ahora espere una foto nueva (reconexión, pestaña recién vuelta): para seguir mostrando algo sin parpadeo. */
  ultimo(canal: string): AlmacenDeCanal | undefined {
    const c = this.canales.get(canal);
    return c?.almacen.conFoto ? c.almacen : undefined;
  }

  /** Los canales de detalle abiertos ahora, por si hace falta compararlos con lo que la interfaz quiere vigilar. */
  get suscritos(): ReadonlySet<string> {
    return this.deseados;
  }

  suscribir(canal: string): void {
    if (this.deseados.has(canal)) return;
    this.deseados.add(canal);
    if (canal === CANAL_EVENTOS) {
      this.enviar({ accion: 'suscribir', canal });
      return;
    }
    this.canales.set(canal, {
      almacen: new AlmacenDeCanal(unicasDe(canal), canal === CANAL_PARTIDA ? { titulos: 'tituloId' } : {}),
      esperandoFoto: true,
      ultimaPeticionDeFoto: 0,
    });
    this.enviar({ accion: 'suscribir', canal });
  }

  desuscribir(canal: string): void {
    if (!this.deseados.delete(canal)) return;
    this.canales.delete(canal);
    this.enviar({ accion: 'desuscribir', canal });
  }

  private enviar(mensaje: Record<string, unknown>): void {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(mensaje));
  }

  private async conectar(): Promise<void> {
    if (this.cerrado || this.pausada) return;
    let url: string;
    try {
      url = await this.opciones.url();
    } catch {
      this.programarReintento();
      return;
    }
    if (this.cerrado || this.pausada) return;
    const socket = new WebSocket(url);
    this.socket = socket;
    let abierto = false;
    socket.onopen = () => {
      abierto = true;
      this.intentos = 0;
      // Sin estado que sobreviva al cierre: se repite todo lo que se quería ver y llegan fotos nuevas.
      for (const nombre of this.deseados) {
        const c = this.canales.get(nombre);
        if (c) c.esperandoFoto = true;
        this.enviar({ accion: 'suscribir', canal: nombre });
      }
      this.opciones.alEstado();
    };
    socket.onmessage = (ev) => {
      try {
        this.recibir(JSON.parse(String(ev.data)) as Record<string, unknown>);
      } catch {
        // un mensaje que no se entiende no tumba la conexión: si faltó algo, el siguiente delta no encadena y se pide foto
      }
    };
    socket.onclose = (ev) => {
      if (this.socket !== socket) return;
      this.socket = undefined;
      this.eventosActivos = false;
      for (const c of this.canales.values()) c.esperandoFoto = true;
      this.opciones.alEstado();
      if (!abierto) this.opciones.alFallarApertura();
      // 4404: la partida se borró; reconectar no sirve de nada.
      if (!this.cerrado && !this.pausada && ev.code !== 4404) this.programarReintento();
    };
  }

  private programarReintento(): void {
    clearTimeout(this.reintento);
    const espera = Math.min(15_000, 1000 * 2 ** this.intentos++);
    this.reintento = setTimeout(() => void this.conectar(), espera);
  }

  private pedirFoto(canal: string): void {
    const c = this.canales.get(canal);
    if (!c) return;
    c.esperandoFoto = true;
    const espera = Math.max(0, c.ultimaPeticionDeFoto + MIN_ENTRE_FOTOS_MS - Date.now());
    setTimeout(() => {
      if (!this.canales.has(canal)) return;
      c.ultimaPeticionDeFoto = Date.now();
      this.enviar({ accion: 'resincronizar', canal });
    }, espera);
  }

  private recibir(m: Record<string, unknown>): void {
    const canal = m['canal'] as string | undefined;
    switch (m['tipo']) {
      case 'evento':
        if (canal === CANAL_EVENTOS) this.opciones.alEvento(m['evento'] as EventoDominioConVersion);
        return;
      case 'suscrito':
        if (canal === CANAL_EVENTOS) {
          this.eventosActivos = true;
          this.opciones.alSuscritoEventos();
        }
        if (canal === CANAL_PARTIDA) this.intervaloTickMs = (m['intervaloTickMs'] as number | null | undefined) ?? null;
        return;
      case 'foto': {
        const c = canal ? this.canales.get(canal) : undefined;
        if (!c || !canal) return;
        c.almacen.cargarFoto(m['version'] as number, m['instante'] as number, m['datos'] as Record<string, unknown>);
        c.esperandoFoto = false;
        this.opciones.alCambiar(canal);
        this.opciones.alEstado();
        return;
      }
      case 'delta': {
        const c = canal ? this.canales.get(canal) : undefined;
        if (!c || !canal || c.esperandoFoto) return;
        const resultado = c.almacen.aplicarDelta(m as unknown as DeltaDeCanal);
        if (resultado === 'hueco') {
          this.pedirFoto(canal);
          this.opciones.alEstado();
        } else if (resultado === 'aplicado') this.opciones.alCambiar(canal);
        return;
      }
      case 'resync': {
        // La foto llega sola a continuación.
        const c = canal ? this.canales.get(canal) : undefined;
        if (c) c.esperandoFoto = true;
        this.opciones.alEstado();
        return;
      }
      case 'revocado':
        if (canal === CANAL_EVENTOS) this.eventosActivos = false;
        if (canal) {
          this.deseados.delete(canal);
          this.canales.delete(canal);
          this.opciones.alCambiar(canal);
        }
        return;
      case 'error':
        if (!canal) return;
        if (canal === CANAL_EVENTOS) this.eventosActivos = false;
        if (m['codigo'] === 'canal_desconocido') {
          this.deseados.delete(canal);
          this.canales.delete(canal);
          this.opciones.alCanalDesconocido(canal);
        } else if (m['codigo'] === 'demasiado_frecuente') this.pedirFoto(canal);
        return;
    }
  }
}
