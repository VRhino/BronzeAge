// El runner de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3): maneja a todos los bots de una partida desde
// fuera, por el puerto. Por bot: su memoria propia (pequeña y desechable), su RNG y su ritmo; por Facción, la pizarra.
//
// Ritmo (§8.2, placeholder a calibrar con batch): cada bot piensa cada `cadaTicks` ticks con un desfase sacado de su
// id, y además se despierta con lo que le toca —su columna llega o alcanza a su presa, le proponen un trueque—.
// Tras cada tick piensan **en orden de id**, cada uno con su RNG: con el adaptador en proceso, misma semilla → mismo
// resultado.
import type { EventoDominio } from '../domain/eventos';
import { instante, type Instante } from '../domain/tiempo';
import { createRng, type RandomFn } from '../worldgen';
import type { ParamsDe, TipoComando, DatosDe } from '../session/comandos/registro';
import type { PuertoBot, Respuesta, Vista } from './puerto';
import { pizarraVacia, type Pizarra } from './pizarra';
import type { Mapa } from '../world/mapa';

/** Lo que el bot está haciendo fuera de casa. Si se pierde (reinicio), lo rehace mirando el mundo (§3.3). */
export type Plan =
  | { tipo: 'cazar'; campamentoId: string }
  | { tipo: 'campana'; plazaId: string }
  | { tipo: 'explorar'; plazaId: string }
  | { tipo: 'mudarse'; plazaId: string };

export interface MemoriaBot {
  plan?: Plan;
  /** Su columna y su residencia en la última vista: con ellas sabe qué eventos le tocan. */
  columnaId?: string;
  residenciaId?: string;
  /** Acciones rechazadas que no se reintentan hasta ese instante: el rechazo es información (§3). */
  esperas: Map<string, Instante>;
}

/** Lo que recibe un cerebro al pensar. */
export interface ContextoBot {
  yo: string;
  vista: Vista;
  memoria: MemoriaBot;
  pizarra: Pizarra;
  rng: RandomFn;
  mapa: Mapa;
  actuar<T extends TipoComando>(tipo: T, params: ParamsDe<T>): Respuesta<DatosDe<T>>;
  /**
   * Como `actuar`, pero si el motor lo rechaza no se reintenta la misma `clave` hasta pasados `esperaMs` de mundo. Es lo
   * que evita que un bot pida cada 5 minutos lo que ya sabe que no puede tener.
   */
  intentar<T extends TipoComando>(clave: string, tipo: T, params: ParamsDe<T>, esperaMs?: number): Respuesta<DatosDe<T>> | undefined;
}

export type Cerebro = (ctx: ContextoBot) => void;

/** Lo que espera un bot tras un rechazo, por defecto: media hora de mundo. */
const ESPERA_TRAS_RECHAZO_MS = 30 * 60_000;

/** Eventos que despiertan a un bot si le tocan (§8.2). */
const DESPIERTAN_POR_COLUMNA = new Set(['ejercito.llega', 'columna.presa_alcanzada', 'ejercito.regresa']);
const DESPIERTAN_POR_PLAZA = new Set(['comercio.trueque_propuesto']);

function hash(texto: string): number {
  let h = 2166136261;
  for (let i = 0; i < texto.length; i++) h = Math.imul(h ^ texto.charCodeAt(i), 16777619);
  return h >>> 0;
}

interface Bot {
  heroeId: string;
  memoria: MemoriaBot;
  rng: RandomFn;
  desfase: number;
}

export class RunnerDeBots {
  private readonly bots = new Map<string, Bot>();
  private readonly pizarras = new Map<string, Pizarra>();
  private readonly cadaTicks: number;
  private readonly semilla: number;

  constructor(
    private readonly puerto: PuertoBot,
    private readonly cerebro: Cerebro,
    opciones: { semilla: number; cadaTicks?: number }
  ) {
    this.cadaTicks = opciones.cadaTicks ?? 5;
    this.semilla = opciones.semilla;
  }

  alta(heroeId: string): void {
    if (this.bots.has(heroeId)) return;
    const h = hash(heroeId);
    this.bots.set(heroeId, { heroeId, memoria: { esperas: new Map() }, rng: createRng((this.semilla ^ h) >>> 0), desfase: h % this.cadaTicks });
  }

  /** Después de cada tick: piensan los que tocan, en orden de id. Devuelve cuántos pensaron. */
  trasTick(tick: number, eventos: readonly EventoDominio[]): number {
    const columnasTocadas = new Set<string>();
    const plazasTocadas = new Set<string>();
    for (const e of eventos) {
      const ejercitoId = (e.payload as { ejercitoId?: unknown } | undefined)?.ejercitoId;
      if (DESPIERTAN_POR_COLUMNA.has(e.codigo) && typeof ejercitoId === 'string') columnasTocadas.add(ejercitoId);
      if (DESPIERTAN_POR_PLAZA.has(e.codigo) && e.asentamientoId) plazasTocadas.add(e.asentamientoId);
    }

    let pensaron = 0;
    for (const id of [...this.bots.keys()].sort()) {
      const bot = this.bots.get(id)!;
      const { columnaId, residenciaId } = bot.memoria;
      const leToca =
        (tick + bot.desfase) % this.cadaTicks === 0 ||
        (columnaId !== undefined && columnasTocadas.has(columnaId)) ||
        (residenciaId !== undefined && plazasTocadas.has(residenciaId));
      if (!leToca) continue;
      this.pensar(bot);
      pensaron++;
    }
    return pensaron;
  }

  private pensar(bot: Bot): void {
    const vista = this.puerto.observar(bot.heroeId);
    if (!vista.heroe) return; // sin héroe no hay quien juegue
    const memoria = bot.memoria;
    memoria.columnaId = vista.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === bot.heroeId))?.id;
    memoria.residenciaId = vista.heroe.residenciaId ?? undefined;

    const clavePizarra = vista.faccionId ?? `sin-faccion:${bot.heroeId}`;
    let pizarra = this.pizarras.get(clavePizarra);
    if (!pizarra) this.pizarras.set(clavePizarra, (pizarra = pizarraVacia()));
    if (memoria.residenciaId) pizarra.residencias.set(bot.heroeId, memoria.residenciaId);
    else pizarra.residencias.delete(bot.heroeId);

    const actuar = <T extends TipoComando>(tipo: T, params: ParamsDe<T>) => this.puerto.actuar(bot.heroeId, tipo, params);
    this.cerebro({
      yo: bot.heroeId,
      vista,
      memoria,
      pizarra,
      rng: bot.rng,
      mapa: this.puerto.mapa(),
      actuar,
      intentar: (clave, tipo, params, esperaMs = ESPERA_TRAS_RECHAZO_MS) => {
        const hasta = memoria.esperas.get(clave);
        if (hasta !== undefined && hasta > vista.instante) return undefined;
        const r = actuar(tipo, params);
        if (r.ok) memoria.esperas.delete(clave);
        else memoria.esperas.set(clave, instante(vista.instante + esperaMs));
        return r;
      },
    });
  }
}

