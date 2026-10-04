// El proceso de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3-§4): el runner de bots sobre el puerto remoto,
// contra un servidor de verdad. Lo único que guarda es su registro —las cuentas de sus bots, su perfil y cuántas llegadas
// lleva—, para volver a entrar con los mismos héroes tras un reinicio. La memoria de cada bot es desechable (§3).
//
// Cada `cadaMs` lee el tick del mundo; por cada tick nuevo da de alta las llegadas que tocan (contadas desde que el proceso
// arrancó en esta partida, D56) y hace pensar a los bots, que se conectan y desconectan según su horario (D55).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { RunnerDeBots, type Perfil } from '../runner';
import { cerebroDeBot } from '../cerebro';
import { darDeAlta, planDeLlegadas } from '../llegadas';
import { PuertoRemoto, type CuentaBot } from './puertoRemoto';

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

interface Registro {
  gameId: string;
  /** El tick en que este proceso empezó a dar de alta bots en la partida: las llegadas cuentan desde ahí. */
  inicioTick: number;
  llegadasHechas: number;
  reloj: CuentaBot;
  bots: (CuentaBot & { heroeId: string; perfil: Perfil })[];
}

/** Cuántos ticks recupera como mucho si se quedó atrás: más no sirve, piensan con el mundo de ahora. */
const RECUPERA_TICKS = 5;

export class ProcesoDeBots {
  private registro?: Registro;
  private ultimoTick = -1;
  private parado = false;
  readonly puerto: PuertoRemoto;
  readonly runner: RunnerDeBots;

  constructor(private readonly config: ConfigProcesoDeBots) {
    this.puerto = new PuertoRemoto({ servidor: config.servidor, gameId: config.gameId, codigoRegistroBots: config.codigoRegistroBots });
    this.runner = new RunnerDeBots(this.puerto, cerebroDeBot, {
      semilla: config.semilla,
      horario: config.horario,
      alFallar: (heroeId, err) => console.error(`[bots] ${heroeId}:`, err instanceof Error ? err.message : err),
    });
  }

  /** Recupera el registro (o lo crea, con su cuenta del reloj) y vuelve a poner a jugar a sus bots. */
  async arrancar(): Promise<void> {
    const guardado = leerRegistro(this.config.registro, this.config.gameId);
    const reloj = guardado?.reloj ?? (await this.puerto.crearCuenta('reloj'));
    this.registro = guardado ?? { gameId: this.config.gameId, inicioTick: await this.puerto.tickActual(reloj), llegadasHechas: 0, reloj, bots: [] };
    for (const b of this.registro.bots) {
      this.puerto.recordar(b);
      this.runner.alta(b.heroeId, b.perfil);
    }
    this.guardar();
  }

  /** Una vuelta: si el mundo avanzó, llegadas y turno de los bots. Devuelve el tick en que está. */
  async vuelta(): Promise<number> {
    const registro = this.registro!;
    const tick = await this.puerto.tickActual(registro.reloj);
    if (tick <= this.ultimoTick) return tick;
    const plan = planDeLlegadas(this.config.semilla, this.config.total, this.config.diasLlegada);
    for (let t = Math.max(this.ultimoTick + 1, tick - RECUPERA_TICKS + 1); t <= tick; t++) {
      while (registro.llegadasHechas < plan.length && registro.inicioTick + plan[registro.llegadasHechas]!.tick <= t) {
        const llegada = plan[registro.llegadasHechas]!;
        let n = registro.bots.length;
        const creados = await darDeAlta(this.puerto, this.runner, llegada, () => `Bot ${++n}`);
        for (const c of creados) registro.bots.push({ ...this.puerto.cuentaDe(c.heroeId), heroeId: c.heroeId, perfil: c.perfil });
        registro.llegadasHechas++;
        this.guardar();
      }
      await this.runner.trasTick(t, t === tick ? this.puerto.vaciarEventos() : []);
    }
    this.ultimoTick = tick;
    return tick;
  }

  /** Vueltas cada `cadaMs` hasta `parar`. Un fallo de una vuelta (la red, un reinicio del servidor) se grita y se sigue. */
  async correr(): Promise<void> {
    while (!this.parado) {
      try {
        await this.vuelta();
      } catch (err) {
        console.error('[bots] vuelta:', err instanceof Error ? err.message : err);
      }
      await new Promise((r) => setTimeout(r, this.config.cadaMs));
    }
  }

  /** Para el bucle y cierra sus conexiones: sus bots se desconectan como quien cierra el cliente. */
  async parar(): Promise<void> {
    this.parado = true;
    await this.puerto.cerrar();
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
