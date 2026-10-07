// El proceso de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3-§4): el runner de bots sobre el puerto remoto,
// contra un servidor de verdad. Lo único que guarda es su registro —las cuentas de sus bots, su perfil y cuántas llegadas
// lleva—, para volver a entrar con los mismos héroes tras un reinicio. La memoria de cada bot es desechable (§3).
//
// Cada `cadaMs` lee el tick del mundo; por cada tick nuevo da de alta las llegadas que tocan (contadas desde que el proceso
// arrancó en esta partida, D56) y hace pensar a los bots, que se conectan y desconectan según su horario (D55).
//
// Lo administra `control/servicio.ts` (pausar, forzar una llegada, retirar un bot…); aquí solo hay lo que el bucle necesita.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { RunnerDeBots, type Perfil } from '../runner';
import { cerebroDeBot } from '../cerebro';
import { AMIGOS_POR_GRUPO, darDeAlta, planDeLlegadas, type Llegada } from '../llegadas';
import { PuertoRemoto, type CuentaBot } from './puertoRemoto';
import type { AccionDeBot, LlegadaPlan } from '../control/contrato';

export interface ConfigProcesoDeBots {
  servidor: string;
  gameId: string;
  codigoRegistroBots: string;
  /** Dónde guarda su registro (JSON). */
  registro: string;
  total: number;
  diasLlegada: number;
  semilla: number;
  /** Cada cuánto mira si avanzó el mundo (ms). */
  cadaMs: number;
  /** Para los tests: sin horario, conectados todo el día. */
  horario?: 'por-semilla' | 'siempre';
}

/** Lo que el proceso cuenta de sí mismo a quien lo vigila (el panel de administración); todo opcional. */
export interface ObservadorDelProceso {
  alActuar?: (accion: AccionDeBot) => void;
  alPeticion?: (p: { ms: number; ok: boolean }) => void;
  alRegistro?: (nivel: 'info' | 'error', texto: string) => void;
  /** Tras cada vuelta que vio avanzar el mundo. */
  alVuelta?: (v: { ms: number; pensaron: number; tick: number }) => void;
}

interface BotRegistrado extends CuentaBot {
  heroeId: string;
  perfil: Perfil;
  nombre?: string;
  retirado?: true;
}

interface Registro {
  gameId: string;
  /** El tick en que este proceso empezó a dar de alta bots en la partida: las llegadas cuentan desde ahí. */
  inicioTick: number;
  llegadasHechas: number;
  /** Las que se forzaron a mano desde el panel, fuera del plan. */
  llegadasExtra?: number;
  reloj: CuentaBot;
  bots: BotRegistrado[];
}

/** Cuántos ticks recupera como mucho si se quedó atrás: más no sirve, piensan con el mundo de ahora. */
const RECUPERA_TICKS = 5;

export class ProcesoDeBots {
  private registro?: Registro;
  private ultimoTick = -1;
  private parado = false;
  private pausado = false;
  private cadaMs: number;
  private despertar?: () => void;
  /** Lo que se está haciendo con el registro y los bots: una vuelta, una llegada forzada o una retirada, de una en una. */
  private cola: Promise<unknown> = Promise.resolve();
  readonly puerto: PuertoRemoto;
  readonly runner: RunnerDeBots;

  constructor(
    private readonly config: ConfigProcesoDeBots,
    private readonly observador: ObservadorDelProceso = {}
  ) {
    this.cadaMs = config.cadaMs;
    this.puerto = new PuertoRemoto({
      servidor: config.servidor,
      gameId: config.gameId,
      codigoRegistroBots: config.codigoRegistroBots,
      alPeticion: observador.alPeticion,
    });
    this.runner = new RunnerDeBots(this.puerto, cerebroDeBot, {
      semilla: config.semilla,
      horario: config.horario,
      alFallar: (heroeId, err) => this.registrar('error', `${heroeId}: ${err instanceof Error ? err.message : err}`),
      alActuar: observador.alActuar,
    });
  }

  /** Recupera el registro (o lo crea, con su cuenta del reloj) y vuelve a poner a jugar a sus bots. */
  async arrancar(): Promise<void> {
    const guardado = leerRegistro(this.config.registro, this.config.gameId);
    if (guardado) await this.comprobarQueEsDeEsteMundo(guardado);
    const reloj = guardado?.reloj ?? (await this.puerto.crearCuenta('reloj'));
    this.registro = guardado ?? { gameId: this.config.gameId, inicioTick: await this.puerto.tickActual(reloj), llegadasHechas: 0, reloj, bots: [] };
    this.registro.bots.forEach((b, i) => {
      this.puerto.recordar(b);
      this.runner.alta(b.heroeId, b.perfil, b.nombre ?? `Bot ${i + 1}`, b.retirado);
    });
    this.guardar();
  }

  /** Una vuelta: si el mundo avanzó, llegadas y turno de los bots. Devuelve el tick en que está. */
  vuelta(): Promise<number> {
    return this.exclusivo(async () => {
      const desde = Date.now();
      const registro = this.registro!;
      const tick = await this.puerto.tickActual(registro.reloj);
      if (tick <= this.ultimoTick) return tick;
      const plan = this.planDeLlegadas();
      let pensaron = 0;
      for (let t = Math.max(this.ultimoTick + 1, tick - RECUPERA_TICKS + 1); t <= tick; t++) {
        while (registro.llegadasHechas < plan.length && registro.inicioTick + plan[registro.llegadasHechas]!.tick <= t) {
          await this.altaDe(plan[registro.llegadasHechas]!);
          registro.llegadasHechas++;
          this.guardar();
        }
        pensaron += await this.runner.trasTick(t, t === tick ? this.puerto.vaciarEventos() : []);
      }
      this.ultimoTick = tick;
      this.observador.alVuelta?.({ ms: Date.now() - desde, pensaron, tick });
      return tick;
    });
  }

  /** Vueltas cada `cadaMs` hasta `parar`. Un fallo de una vuelta (la red, un reinicio del servidor) se grita y se sigue. */
  async correr(): Promise<void> {
    while (!this.parado) {
      if (!this.pausado) {
        try {
          await this.vuelta();
        } catch (err) {
          this.registrar('error', `vuelta: ${err instanceof Error ? err.message : err}`);
        }
      }
      await new Promise<void>((resolver) => {
        const timer = setTimeout(resolver, this.cadaMs);
        this.despertar = () => (clearTimeout(timer), resolver());
      });
    }
  }

  /** Para el bucle, espera a que acabe lo que estaba haciendo y cierra sus conexiones: sus bots se desconectan como quien cierra el cliente. */
  async parar(): Promise<void> {
    this.parado = true;
    this.despertar?.();
    await this.cola;
    await this.puerto.cerrar();
  }

  // --- Administración ---

  /** Pausado, los bots siguen conectados pero no piensan ni llegan más: el mundo sigue sin ellos. */
  pausar(pausado: boolean): void {
    this.pausado = pausado;
  }

  /** Cada cuánto mira si avanzó el mundo; vale desde la siguiente vuelta. */
  ajustarCada(ms: number): void {
    this.cadaMs = ms;
    this.despertar?.();
  }

  /** Da de alta ya una llegada fuera del plan (un solitario, un tardío o un grupo de amigos). */
  forzarLlegada(perfil: Llegada['perfil']): Promise<number> {
    return this.exclusivo(async () => {
      const registro = this.registro!;
      const antes = registro.bots.length;
      const llegada: Llegada = { tick: this.ultimoTick, grupo: -1, perfil, cuantos: perfil === 'amigos' ? AMIGOS_POR_GRUPO : 1 };
      registro.llegadasExtra = (registro.llegadasExtra ?? 0) + 1;
      try {
        await this.altaDe(llegada);
      } finally {
        this.guardar();
      }
      return registro.bots.length - antes;
    });
  }

  /** Deja de manejar a un bot (se desconecta) y lo anota en el registro para que no vuelva tras un reinicio. */
  retirar(heroeId: string): Promise<void> {
    return this.exclusivo(async () => {
      const bot = this.registro!.bots.find((b) => b.heroeId === heroeId);
      if (!bot) throw new Error(`no hay un bot ${heroeId}`);
      await this.runner.retirar(heroeId);
      bot.retirado = true;
      this.guardar();
    });
  }

  /** El plan de llegadas con lo que ya se hizo. */
  llegadas(): { hechas: number; extras: number; inicioTick?: number; plan: LlegadaPlan[] } {
    const hechas = this.registro?.llegadasHechas ?? 0;
    return {
      hechas,
      extras: this.registro?.llegadasExtra ?? 0,
      inicioTick: this.registro?.inicioTick,
      plan: this.planDeLlegadas().map((l, i) => ({ grupo: l.grupo, perfil: l.perfil, cuantos: l.cuantos, tick: l.tick, hecha: i < hechas })),
    };
  }

  /** El tick del mundo en la última vuelta que lo vio avanzar. */
  tick(): number | undefined {
    return this.ultimoTick >= 0 ? this.ultimoTick : undefined;
  }

  // --- Internos ---

  /**
   * Un registro es de un mundo concreto: si la partida se regeneró con el mismo ID, sus cuentas siguen existiendo pero sus héroes
   * no, y los bots serían fantasmas sin que nada lo dijera. Se detecta porque el mundo está antes de donde el registro empezó, o
   * porque algún héroe suyo ya no existe (se miran unos pocos).
   */
  private async comprobarQueEsDeEsteMundo(r: Registro): Promise<void> {
    const noEs = (motivo: string) =>
      new Error(`el registro ${this.config.registro} no es de este mundo (${motivo}): si la partida '${r.gameId}' se regeneró, hay que borrarlo antes de iniciar`);
    const tick = await this.puerto.tickActual(r.reloj);
    if (tick < r.inicioTick) throw noEs(`el mundo está en el tick ${tick} y el registro empezó en el ${r.inicioTick}`);
    for (const bot of r.bots.filter((b) => !b.retirado).slice(0, 5)) {
      this.puerto.recordar(bot);
      if (!(await this.puerto.observar(bot.heroeId)).heroe) throw noEs(`el héroe ${bot.heroeId} ya no existe`);
    }
  }

  private planDeLlegadas(): Llegada[] {
    return planDeLlegadas(this.config.semilla, this.config.total, this.config.diasLlegada);
  }

  /**
   * Da de alta una llegada y apunta cada bot en el registro en cuanto existe. Si falla a medias (el segundo de unos amigos),
   * los que ya están en el mundo quedan registrados y la llegada se da por hecha: repetirla crearía héroes de más. Si falla
   * antes de que nazca ninguno, sube el error y se reintenta.
   */
  private async altaDe(llegada: Llegada): Promise<void> {
    const registro = this.registro!;
    // Siguiente número libre: contar los bots del registro repetiría un nombre si una llegada anterior saltó alguno (su héroe no llegó a nacer).
    let n = Math.max(registro.bots.length, ...registro.bots.map((b) => Number(/(\d+)$/.exec(b.nombre ?? '')?.[1] ?? 0)));
    let nacidos = 0;
    try {
      await darDeAlta(this.puerto, this.runner, llegada, () => `Bot ${++n}`, ({ heroeId, perfil, nombre }) => {
        registro.bots.push({ ...this.puerto.cuentaDe(heroeId), heroeId, perfil, nombre });
        nacidos++;
        this.guardar();
      });
    } catch (err) {
      if (nacidos === 0) throw err;
      this.registrar('error', `llegada incompleta (${nacidos}/${llegada.cuantos}): ${err instanceof Error ? err.message : err}`);
    }
  }

  private exclusivo<T>(paso: () => Promise<T>): Promise<T> {
    const hecho = this.cola.then(paso, paso);
    this.cola = hecho.catch(() => undefined);
    return hecho;
  }

  private registrar(nivel: 'info' | 'error', texto: string): void {
    if (nivel === 'error') console.error(`[bots] ${texto}`);
    this.observador.alRegistro?.(nivel, texto);
  }

  private guardar(): void {
    mkdirSync(dirname(this.config.registro), { recursive: true });
    writeFileSync(this.config.registro, JSON.stringify(this.registro, null, 2));
  }
}

/** El registro de esta partida, si lo hay. Uno ilegible no se pisa: sin él, sus cuentas no tendrían quien las use. */
function leerRegistro(ruta: string, gameId: string): Registro | undefined {
  let texto: string;
  try {
    texto = readFileSync(ruta, 'utf-8');
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
    throw err;
  }
  const r = JSON.parse(texto) as Registro;
  if (r.gameId !== gameId) throw new Error(`${ruta} es el registro de la partida '${r.gameId}', no de '${gameId}'`);
  return r;
}
