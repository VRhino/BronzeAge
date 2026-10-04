// El adaptador REMOTO del puerto de los bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): los bots juegan desde otro
// proceso por la misma superficie que BronzeAgeClient —HTTP para la proyección y los comandos, el WebSocket de tiempo real
// para estar conectado y enterarse de lo que les toca—, cada uno con su cuenta de bot (`CODIGO_REGISTRO_BOTS`, §8.3).
//
// No sabe nada del juego: traduce el puerto a peticiones. La memoria de los bots vive en el runner; aquí solo las cuentas
// (lo que hay que guardar para volver a entrar tras un reinicio) y el mapa del mundo, que se pide una vez.
import { randomBytes } from 'node:crypto';
import type { EventoDominio } from '../../domain/eventos';
import type { ResultadoComando } from '../../session/comandos/tipos';
import { crearMapa, type EstadoMapa, type Mapa } from '../../world/mapa';
import type { MapaGenerado } from '../../worldgen/types';
import { SIMULACION } from '../../constants';
import type { CampamentoElegible, PuertoBot, Respuesta, Vista } from '../puerto';

/** Una cuenta de bot: con esto vuelve a entrar. `heroeId` falta en la cuenta del reloj, que no juega. */
export interface CuentaBot {
  nick: string;
  clave: string;
  heroeId?: string;
}

export interface OpcionesPuertoRemoto {
  /** La raíz del servidor, sin `/v1` (p. ej. `https://bronzeage.onrender.com`). */
  servidor: string;
  gameId: string;
  codigoRegistroBots: string;
}

const EPOCA_MS = new Date(SIMULACION.epocaInicial).getTime();

export class PuertoRemoto implements PuertoBot {
  private readonly porHeroe = new Map<string, CuentaBot>();
  private readonly sesiones = new Map<string, string>();
  private readonly sockets = new Map<string, WebSocket>();
  private readonly canales = new Map<string, Set<string>>();
  private generado?: { mapaId: string; mapa: MapaGenerado };
  private estadoMapa?: EstadoMapa;
  private mapaHecho?: { estado: EstadoMapa; mapa: Mapa };
  private pendientes: EventoDominio[] = [];

  constructor(private readonly opciones: OpcionesPuertoRemoto) {}

  /** Las cuentas de una vez anterior: sus bots vuelven a jugar con ellas. */
  recordar(cuenta: CuentaBot): void {
    if (cuenta.heroeId) this.porHeroe.set(cuenta.heroeId, cuenta);
  }

  /** La cuenta con la que juega un héroe. */
  cuentaDe(heroeId: string): CuentaBot {
    const cuenta = this.porHeroe.get(heroeId);
    if (!cuenta) throw new Error(`sin cuenta para ${heroeId}`);
    return cuenta;
  }

  /** Los eventos que han llegado por tiempo real desde la última vez: los que despiertan a los bots (§8.2). */
  vaciarEventos(): EventoDominio[] {
    const eventos = this.pendientes;
    this.pendientes = [];
    return eventos;
  }

  /** El tick del mundo, leído con `cuenta` (cualquiera vale: el instante lo trae también quien no tiene héroe). */
  async tickActual(cuenta: CuentaBot): Promise<number> {
    const r = await this.pedir<{ instante: number }>(cuenta, 'GET', `/jugador/partidas/${this.opciones.gameId}`);
    return Math.round((r.cuerpo.instante - EPOCA_MS) / SIMULACION.duracionTickMs);
  }

  /** Da de alta una cuenta de bot y la une a la partida. La del reloj se crea así, sin héroe. */
  async crearCuenta(nombre: string): Promise<CuentaBot> {
    const nick = `${nombre.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-')}-${randomBytes(3).toString('hex')}`;
    const cuenta: CuentaBot = { nick, clave: randomBytes(18).toString('base64url') };
    const alta = await fetch(this.url('/registro'), {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ nick, clave: cuenta.clave, codigo: this.opciones.codigoRegistroBots }),
    });
    if (alta.status !== 201) throw new Error(`registro de ${nick}: ${alta.status} ${await alta.text()}`);
    await this.pedir(cuenta, 'POST', `/jugador/partidas/${this.opciones.gameId}/membresia`);
    return cuenta;
  }

  // --- El puerto ---

  async llegar(nombre: string, datos: Parameters<PuertoBot['llegar']>[1], elegir: (c: CampamentoElegible[]) => string | undefined) {
    const cuenta = await this.crearCuenta(nombre);
    const pantalla = await this.pedir<{ campamentos?: CampamentoElegible[] }>(cuenta, 'GET', `/jugador/partidas/${this.opciones.gameId}`);
    const campamentoId = elegir(pantalla.cuerpo.campamentos ?? []);
    if (!campamentoId) return undefined;
    const r = await this.comando<{ heroeId: string }>(cuenta, 'crearHeroe', { ...datos, campamentoId });
    const heroeId = r.datos?.heroeId;
    if (!heroeId) return undefined;
    cuenta.heroeId = heroeId;
    this.porHeroe.set(heroeId, cuenta);
    return heroeId;
  }

  async observar(heroeId: string): Promise<Vista> {
    const cuenta = this.cuentaDe(heroeId);
    const { cuerpo } = await this.pedir<Vista>(cuenta, 'GET', `/jugador/partidas/${this.opciones.gameId}`);
    this.estadoMapa = cuerpo.estadoMapa;
    if (this.generado?.mapaId !== cuerpo.mapaId) {
      const mapa = await this.pedir<MapaGenerado>(cuenta, 'GET', `/jugador/partidas/${this.opciones.gameId}/mapa/${cuerpo.mapaId}`);
      this.generado = { mapaId: cuerpo.mapaId, mapa: mapa.cuerpo };
    }
    // Lo de su plaza llega por su canal (un trueque que le proponen): se suscribe al de donde vive.
    if (cuerpo.heroe?.residenciaId) this.suscribir(heroeId, `asentamiento/${cuerpo.heroe.residenciaId}`);
    return cuerpo;
  }

  async actuar<T extends Parameters<PuertoBot['actuar']>[1]>(heroeId: string, tipo: T, params: Parameters<PuertoBot['actuar']>[2]) {
    return this.comando(this.cuentaDe(heroeId), tipo, params) as never;
  }

  mapa(): Mapa {
    if (!this.generado || !this.estadoMapa) throw new Error('el mapa llega con la primera vista: observar antes');
    if (this.mapaHecho?.estado !== this.estadoMapa) this.mapaHecho = { estado: this.estadoMapa, mapa: crearMapa(this.generado.mapa, this.estadoMapa) };
    return this.mapaHecho.mapa;
  }

  /** Abrir el cliente: su conexión de tiempo real. Al abrirla, el servidor lo da por conectado (Doc 1.10.6). */
  async conectar(heroeId: string): Promise<void> {
    if (this.sockets.has(heroeId)) return;
    const cuenta = this.cuentaDe(heroeId);
    const sesion = await this.sesionDe(cuenta);
    const ws = new WebSocket(`${this.url(`/jugador/partidas/${this.opciones.gameId}/tiempo-real`).replace(/^http/, 'ws')}?sesion=${sesion}`);
    await new Promise<void>((resolver, fallar) => {
      ws.addEventListener('open', () => resolver(), { once: true });
      ws.addEventListener('error', () => fallar(new Error(`tiempo real de ${cuenta.nick}: no conecta`)), { once: true });
    });
    ws.addEventListener('message', (m) => {
      const mensaje = JSON.parse(String(m.data)) as { tipo: string; evento?: EventoDominio };
      if (mensaje.tipo === 'evento' && mensaje.evento) this.pendientes.push(mensaje.evento);
    });
    ws.addEventListener('close', () => {
      this.sockets.delete(heroeId);
      this.canales.delete(heroeId);
    });
    this.sockets.set(heroeId, ws);
    this.canales.set(heroeId, new Set());
    this.suscribir(heroeId, 'mapa/general');
  }

  /**
   * Cerrar el cliente: cerrar su conexión. Si era la última, el servidor lo desconecta (sale 2:30 después). Si no la tenía
   * abierta —un bot que nace fuera de su horario—, no hay cierre que se lo diga: se lo dice el comando.
   */
  async desconectar(heroeId: string): Promise<void> {
    const ws = this.sockets.get(heroeId);
    if (!ws) {
      await this.actuar(heroeId, 'desconectarse', { heroeId });
      return;
    }
    const cerrado = new Promise<void>((resolver) => ws.addEventListener('close', () => resolver(), { once: true }));
    ws.close();
    await cerrado;
  }

  /** Cierra todas las conexiones (al apagar el proceso). */
  async cerrar(): Promise<void> {
    await Promise.all([...this.sockets.keys()].map((id) => this.desconectar(id)));
  }

  // --- Transporte ---

  private suscribir(heroeId: string, canal: string): void {
    const ws = this.sockets.get(heroeId);
    const canales = this.canales.get(heroeId);
    if (!ws || !canales || canales.has(canal)) return;
    canales.add(canal);
    ws.send(JSON.stringify({ accion: 'suscribir', canal }));
  }


  /** Un comando como lo manda un cliente. Un 403 es la autorización diciendo que no (el `noAutorizado` del puerto). */
  private async comando<R>(cuenta: CuentaBot, tipo: string, params: unknown): Promise<Respuesta<R>> {
    const r = await this.pedir<{ resultado?: ResultadoComando<R>; error?: string; version?: number }>(
      cuenta,
      'POST',
      `/jugador/partidas/${this.opciones.gameId}/comandos`,
      { tipo, params },
      [403]
    );
    if (r.estado === 403) return { ok: false, eventos: [], version: 0, noAutorizado: /\(([^)]+)\)/.exec(r.cuerpo.error ?? '')?.[1] ?? 'no_autorizado' };
    return r.cuerpo.resultado!;
  }

  private url(ruta: string): string {
    return `${this.opciones.servidor.replace(/\/$/, '')}/v1${ruta}`;
  }

  private async sesionDe(cuenta: CuentaBot, renovar = false): Promise<string> {
    const actual = this.sesiones.get(cuenta.nick);
    if (actual && !renovar) return actual;
    const r = await fetch(this.url('/sesiones'), { method: 'POST', headers: { authorization: `clave ${cuenta.nick}:${cuenta.clave}` } });
    if (r.status !== 201) throw new Error(`login de ${cuenta.nick}: ${r.status} ${await r.text()}`);
    const { sesionId } = (await r.json()) as { sesionId: string };
    this.sesiones.set(cuenta.nick, sesionId);
    return sesionId;
  }

  /** Una petición con la sesión de la cuenta; si la sesión caducó (401), entra otra vez y repite una vez. */
  private async pedir<C>(cuenta: CuentaBot, metodo: 'GET' | 'POST', ruta: string, cuerpo?: unknown, admitidos: number[] = []): Promise<{ estado: number; cuerpo: C }> {
    for (const renovar of [false, true]) {
      const sesion = await this.sesionDe(cuenta, renovar);
      const r = await fetch(this.url(ruta), {
        method: metodo,
        headers: { authorization: `sesion ${sesion}`, ...(cuerpo !== undefined ? { 'content-type': 'application/json' } : {}) },
        ...(cuerpo !== undefined ? { body: JSON.stringify(cuerpo) } : {}),
      });
      if (r.status === 401 && !renovar) continue;
      if (!r.ok && !admitidos.includes(r.status)) throw new Error(`${metodo} ${ruta} (${cuenta.nick}): ${r.status} ${await r.text()}`);
      return { estado: r.status, cuerpo: (await r.json()) as C };
    }
    throw new Error('inalcanzable');
  }
}
