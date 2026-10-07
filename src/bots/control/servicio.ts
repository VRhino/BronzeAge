// El servicio de bots (el «daemon»): envuelve a `ProcesoDeBots` con un ciclo de vida que se maneja desde fuera. Arranca inerte y
// espera la orden de iniciar (por el canal de control, `servidorDeControl.ts`, o por entorno); desde ahí se pausa, se
// reanuda, se para, se forzan llegadas y se maneja bot a bot. No sabe de WebSockets: habla con `suscribir` y `manejar`.
//
// Recuerda lo que estaba haciendo (`rutaEstado`): si el proceso se reinicia estando en marcha, vuelve a ponerse en marcha solo.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { ProcesoDeBots } from '../remoto/procesoDeBots';
import type { AccionDeBot, ComandoDeControl, ConfigBots, ConfigPublica, EstadoServicio, FaseServicio, MensajeDeServicio, Salud } from './contrato';

export interface OpcionesServicio {
  /** Carpeta de los registros de cuentas (uno por partida: `bots-<partida>.json`). */
  directorioRegistros: string;
  /** Dónde recuerda si estaba en marcha. */
  rutaEstado: string;
  /** `CODIGO_REGISTRO_BOTS` del entorno: con él, el panel no tiene que mandarlo. */
  codigoRegistroBots?: string;
  /** Valores que propone el panel al configurar. */
  porDefecto?: Partial<ConfigPublica>;
}

const VENTANA_MS = 60_000;
const HISTORIAL = 150;

interface Guardado {
  deseado: 'corriendo' | 'parado';
  config: ConfigPublica;
}

export class ServicioDeBots {
  private fase: FaseServicio = 'inactivo';
  private proceso?: ProcesoDeBots;
  private config?: ConfigPublica;
  private error?: string;
  private iniciadoEn?: number;
  private vuelta?: Salud['vuelta'];
  private readonly oyentes = new Set<(m: MensajeDeServicio) => void>();
  private readonly peticiones: { en: number; ms: number; ok: boolean }[] = [];
  private readonly acciones: AccionDeBot[] = [];
  private readonly registros: Extract<MensajeDeServicio, { tipo: 'registro' }>[] = [];

  constructor(private readonly opciones: OpcionesServicio) {}

  // --- Quien escucha ---

  suscribir(oyente: (m: MensajeDeServicio) => void): () => void {
    this.oyentes.add(oyente);
    return () => void this.oyentes.delete(oyente);
  }

  /** Lo que se ha hecho últimamente, para quien se conecta ahora. */
  historial(): { acciones: AccionDeBot[]; registros: Extract<MensajeDeServicio, { tipo: 'registro' }>[] } {
    return { acciones: [...this.acciones], registros: [...this.registros] };
  }

  estado(): EstadoServicio {
    const ahora = Date.now();
    return {
      fase: this.fase,
      ahora,
      iniciadoEn: this.iniciadoEn,
      config: this.config,
      tick: this.proceso?.tick(),
      error: this.error,
      llegadas: this.proceso?.llegadas() ?? { hechas: 0, extras: 0, plan: [] },
      salud: this.salud(ahora),
      bots: this.proceso?.runner.info() ?? [],
      pizarras: this.proceso?.runner.pizarrasInfo() ?? [],
      codigoEnEntorno: this.opciones.codigoRegistroBots !== undefined,
      porDefecto: this.opciones.porDefecto ?? {},
    };
  }

  // --- Órdenes ---

  /** Ejecuta una orden del panel. Devuelve sus datos (si los hay) o lanza el motivo del rechazo. */
  async manejar(orden: Exclude<ComandoDeControl, { accion: 'autenticar' }>): Promise<unknown> {
    try {
      return await this.ejecutar(orden);
    } finally {
      this.difundirEstado();
    }
  }

  private async ejecutar(orden: Exclude<ComandoDeControl, { accion: 'autenticar' }>): Promise<unknown> {
    switch (orden.accion) {
      case 'iniciar':
        return this.iniciar(orden.config);
      case 'pausar':
        this.exigir('corriendo');
        this.proceso!.pausar(true);
        this.fase = 'pausado';
        return this.recordar('corriendo');
      case 'reanudar':
        this.exigir('pausado');
        this.proceso!.pausar(false);
        this.fase = 'corriendo';
        return this.recordar('corriendo');
      case 'parar':
        return this.parar();
      case 'reiniciarRegistro':
        return this.reiniciarRegistro();
      case 'ajustar':
        this.enMarcha().ajustarCada(positivo(orden.cadaMs, 'cadaMs'));
        this.config = { ...this.config!, cadaMs: orden.cadaMs };
        return this.recordar('corriendo');
      case 'forzarLlegada':
        return { creados: await this.enMarcha().forzarLlegada(orden.perfil) };
      case 'modoBot':
        return this.enMarcha().runner.fijarModo(orden.heroeId, orden.modo);
      case 'retirarBot':
        return this.enMarcha().retirar(orden.heroeId);
      case 'pensarYa':
        return this.enMarcha().runner.pensarYa(orden.heroeId);
      case 'volcarMemoria':
        return this.enMarcha().runner.memoriaDe(orden.heroeId);
    }
  }

  /** Arranca el proceso con esta configuración. Resuelve cuando ya está jugando, no cuando acaba. */
  async iniciar(entrada: ConfigBots): Promise<void> {
    if (this.fase !== 'inactivo' && this.fase !== 'error') throw new Error(`ya está ${this.fase}`);
    const config = this.validar(entrada);
    const { codigoRegistroBots, ...publica } = config;
    this.fase = 'arrancando';
    this.error = undefined;
    this.config = publica;
    // Los héroes de una partida regenerada vuelven a ser heroe-0, heroe-1…: sin esto, las acciones de la corrida anterior se
    // atribuirían a los bots nuevos que reutilizan esos ids.
    this.acciones.length = 0;
    this.difundir({ tipo: 'historial', acciones: [], registros: [...this.registros] });
    this.difundirEstado();
    const proceso = new ProcesoDeBots(
      { servidor: config.servidor, gameId: config.partida, codigoRegistroBots: codigoRegistroBots!, registro: this.rutaRegistro(config.partida), ...pick(config) },
      {
        alActuar: (a) => this.guardarAccion(a),
        alPeticion: (p) => this.peticiones.push({ en: Date.now(), ...p }),
        alRegistro: (nivel, texto) => this.registrar(nivel, texto),
        alVuelta: (v) => {
          this.vuelta = { ...v, en: Date.now() };
          this.difundirEstado();
        },
      }
    );
    try {
      await proceso.arrancar();
    } catch (err) {
      this.fase = 'error';
      this.error = err instanceof Error ? err.message : String(err);
      this.registrar('error', `no arranca: ${this.error}`);
      throw err;
    }
    this.proceso = proceso;
    this.fase = 'corriendo';
    this.iniciadoEn = Date.now();
    this.recordar('corriendo');
    this.registrar('info', `jugando en '${config.partida}' contra ${config.servidor}`);
    void proceso.correr();
  }

  /** Cierra las conexiones de los bots y deja el servicio inerte, con su configuración a mano para volver a iniciar. */
  async parar(): Promise<void> {
    if (!this.proceso) throw new Error('no está en marcha');
    const proceso = this.proceso;
    this.fase = 'parando';
    this.difundirEstado();
    await proceso.parar();
    this.proceso = undefined;
    this.iniciadoEn = undefined;
    this.fase = 'inactivo';
    this.recordar('parado');
    this.registrar('info', 'parado: los bots se han desconectado');
  }

  /** Apagado del proceso: cierra las conexiones de los bots sin tocar lo que estaba deseado, para retomarlo al volver. */
  async cerrar(): Promise<void> {
    await this.proceso?.parar();
  }

  /** Borra el registro de cuentas de la última partida: los bots que tenía quedan en el mundo sin quien los maneje. */
  reiniciarRegistro(): void {
    this.exigir('inactivo', 'error');
    if (!this.config) throw new Error('no hay una partida configurada');
    rmSync(this.rutaRegistro(this.config.partida), { force: true });
    this.registrar('info', `registro de '${this.config.partida}' borrado`);
  }

  /** Al arrancar el proceso: si estaba en marcha antes de caerse, vuelve a ponerse. */
  async reanudarSiToca(): Promise<boolean> {
    const guardado = leer(this.opciones.rutaEstado);
    if (guardado?.deseado !== 'corriendo' || !this.opciones.codigoRegistroBots) return false;
    await this.iniciar({ ...guardado.config, codigoRegistroBots: this.opciones.codigoRegistroBots });
    return true;
  }

  // --- Internos ---

  private validar(c: ConfigBots): ConfigBots {
    const servidor = texto(c.servidor, 'servidor').replace(/\/$/, '');
    if (!/^https?:\/\//.test(servidor)) throw new Error('servidor: tiene que empezar por http:// o https://');
    const partida = texto(c.partida, 'partida');
    if (!/^[\w.-]+$/.test(partida)) throw new Error('partida: solo letras, números, guion y punto');
    const codigoRegistroBots = c.codigoRegistroBots?.trim() || this.opciones.codigoRegistroBots;
    if (!codigoRegistroBots) throw new Error('falta el código de registro de bots');
    // El código del entorno solo viaja al servidor para el que se configuró el servicio.
    const fijo = this.opciones.porDefecto?.servidor?.replace(/\/$/, '');
    if (!c.codigoRegistroBots?.trim() && fijo && fijo !== servidor) throw new Error(`servidor: este servicio solo da su código a ${fijo}`);
    if (c.horario !== 'por-semilla' && c.horario !== 'siempre') throw new Error('horario: por-semilla o siempre');
    return {
      servidor,
      partida,
      codigoRegistroBots,
      total: positivo(c.total, 'total'),
      diasLlegada: positivo(c.diasLlegada, 'diasLlegada'),
      semilla: positivo(c.semilla, 'semilla'),
      cadaMs: positivo(c.cadaMs, 'cadaMs'),
      horario: c.horario,
    };
  }

  private exigir(...fases: FaseServicio[]): void {
    if (!fases.includes(this.fase)) throw new Error(`no se puede en estado ${this.fase}`);
  }

  private enMarcha(): ProcesoDeBots {
    if (!this.proceso || (this.fase !== 'corriendo' && this.fase !== 'pausado')) throw new Error(`no está en marcha (${this.fase})`);
    return this.proceso;
  }

  private rutaRegistro(partida: string): string {
    return join(this.opciones.directorioRegistros, `bots-${partida}.json`);
  }

  private recordar(deseado: Guardado['deseado']): void {
    if (!this.config) return;
    mkdirSync(dirname(this.opciones.rutaEstado), { recursive: true });
    writeFileSync(this.opciones.rutaEstado, JSON.stringify({ deseado, config: this.config } satisfies Guardado, null, 2));
  }

  private guardarAccion(accion: AccionDeBot): void {
    this.acciones.push(accion);
    if (this.acciones.length > HISTORIAL) this.acciones.shift();
    this.difundir({ tipo: 'accion', accion });
  }

  private registrar(nivel: 'info' | 'error', textoLinea: string): void {
    const m = { tipo: 'registro', nivel, texto: textoLinea, en: Date.now() } as const;
    this.registros.push(m);
    if (this.registros.length > HISTORIAL) this.registros.shift();
    this.difundir(m);
  }

  private difundirEstado(): void {
    if (this.oyentes.size > 0) this.difundir({ tipo: 'estado', estado: this.estado() });
  }

  private difundir(m: MensajeDeServicio): void {
    for (const oyente of this.oyentes) oyente(m);
  }

  private salud(ahora: number): Salud {
    while (this.peticiones[0] && this.peticiones[0].en < ahora - VENTANA_MS) this.peticiones.shift();
    const ms = this.peticiones.map((p) => p.ms).sort((a, b) => a - b);
    const percentil = (p: number) => ms[Math.min(ms.length - 1, Math.floor(ms.length * p))] ?? 0;
    const conexiones = this.proceso?.puerto.conexiones();
    return {
      vuelta: this.vuelta,
      peticionesMin: this.peticiones.length,
      erroresMin: this.peticiones.filter((p) => !p.ok).length,
      latenciaP50: percentil(0.5),
      latenciaP95: percentil(0.95),
      socketsAbiertos: conexiones?.sockets ?? 0,
      sesionesRenovadas: conexiones?.sesionesRenovadas ?? 0,
    };
  }
}

const pick = (c: ConfigBots) => ({ total: c.total, diasLlegada: c.diasLlegada, semilla: c.semilla, cadaMs: c.cadaMs, horario: c.horario });

function texto(v: unknown, nombre: string): string {
  if (typeof v !== 'string' || v.trim() === '') throw new Error(`${nombre}: falta`);
  return v.trim();
}

function positivo(v: unknown, nombre: string): number {
  if (typeof v !== 'number' || !Number.isFinite(v) || v <= 0) throw new Error(`${nombre}: tiene que ser un número mayor que 0`);
  return v;
}

function leer(ruta: string): Guardado | undefined {
  try {
    return JSON.parse(readFileSync(ruta, 'utf-8')) as Guardado;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
    throw err;
  }
}
