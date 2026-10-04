// El runner de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3): maneja a todos los bots de una partida desde
// fuera, por el puerto. Por bot: su memoria propia (pequeña y desechable), su RNG y su ritmo; por Facción, la pizarra.
//
// Ritmo (§8.2, placeholder a calibrar con batch): cada bot piensa cada `cadaTicks` ticks con un desfase sacado de su
// id, y además se despierta con lo que le toca —su columna llega o alcanza a su presa, le proponen un trueque—.
// Tras cada tick piensan **en orden de id**, cada uno con su RNG: con el adaptador en proceso, misma semilla → mismo
// resultado.
//
// Sesiones (D55): cada bot juega unas horas al día, en uno o dos bloques sacados de su semilla. Al empezar su sesión se
// conecta y al acabar se desconecta, como un humano que abre y cierra el cliente (Doc 1.10.6); fuera de ella no piensa.
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
  | { tipo: 'mudarse'; plazaId: string }
  /** Sin plaza: recorrer el anillo de bandidos de su campamento (y abrir los alijos que vea por el camino). `salio`: ya se alejó
   * de la puerta, así que volver a ella es volver a casa. */
  | { tipo: 'anillo'; campamentoId: string; salio?: boolean; /** Sale sin tropa a buscar (no come) y vuelve al ver presa. */ explorar?: { desde: Instante; paso: number; giro: number } }
  /** Sin plaza: llevar la caravana de fundación a `sitio` y fundar; `descartados`, los sitios donde `fundar` ya dijo que no. */
  | { tipo: 'fundar'; caravanaId: string; sitio?: { x: number; y: number }; descartados: { x: number; y: number }[] }
  /** Sin plaza: va en la columna de un compañero (la caza en grupo o la de fundación). `salio`, como en `anillo`. */
  | { tipo: 'unirse'; ejercitoId: string; para: 'cazar' | 'fundar'; salio?: boolean };

/**
 * Cómo llega y con quién juega (D57): un grupo de amigos (llegan juntos; el `lider` crea la Facción y los demás le piden
 * entrar), un solitario (crea la suya) o uno que llega tarde (pide entrar en una que ya exista). No cambia las reglas, solo las
 * prioridades.
 */
export type Perfil = { tipo: 'amigos'; lider: string } | { tipo: 'solitario' } | { tipo: 'tardio' };

export interface MemoriaBot {
  plan?: Plan;
  /** Su columna y su residencia en la última vista: con ellas sabe qué eventos le tocan. */
  columnaId?: string;
  residenciaId?: string;
  /** Acciones rechazadas que no se reintentan hasta ese instante: el rechazo es información (§3). */
  esperas: Map<string, Instante>;
  /** La solicitud de ingreso pendiente y desde cuándo: quien no recibe respuesta, al rato funda la suya. */
  solicitud?: { faccionId: string; desde: Instante };
  /** Desde cuándo espera, con la caravana comprada, a que vuelvan los compañeros para salir a fundar juntos. */
  esperaFundar?: Instante;
  /** El campamento en el que estaba dentro y su pizarra, en la última vista: con ellos el runner sabe si un compañero lo llama. */
  dentroDe?: string;
  pizarra?: string;
}

/** Lo que recibe un cerebro al pensar. */
export interface ContextoBot {
  yo: string;
  perfil: Perfil;
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
  perfil: Perfil;
  memoria: MemoriaBot;
  rng: RandomFn;
  desfase: number;
  /** Minutos del día [inicio, fin) en que juega; `fin` puede pasar de 1440 (la sesión cruza la medianoche). */
  sesiones: [number, number][];
  /** Lo último que se le dijo al mundo; ausente hasta su primer tick. */
  conectado?: boolean;
}

const MINUTOS_DIA = 24 * 60;
/** Horas de juego al día de un bot (D55, placeholder a calibrar con batch). */
const HORAS_AL_DIA = { min: 2, max: 6 };

/** Uno o dos bloques al día, de 2 a 6 horas en total, a horas sacadas de su RNG (D55). */
function horarioDe(rng: RandomFn): [number, number][] {
  const total = Math.round((HORAS_AL_DIA.min + rng() * (HORAS_AL_DIA.max - HORAS_AL_DIA.min)) * 60);
  const bloques = rng() < 0.5 ? 1 : 2;
  const duracion = Math.round(total / bloques);
  const primero = Math.floor(rng() * MINUTOS_DIA);
  // El segundo bloque, al menos 4 horas después de que acabe el primero: dos sesiones del día, no una partida en dos.
  const segundo = (primero + duracion + 240 + Math.floor(rng() * (MINUTOS_DIA - 2 * duracion - 240))) % MINUTOS_DIA;
  return (bloques === 1 ? [primero] : [primero, segundo]).map((inicio) => [inicio, inicio + duracion]);
}

function enSesion(bot: Bot, tick: number): boolean {
  const minuto = tick % MINUTOS_DIA;
  return bot.sesiones.some(([inicio, fin]) => (minuto >= inicio && minuto < fin) || minuto + MINUTOS_DIA < fin);
}

export class RunnerDeBots {
  private readonly bots = new Map<string, Bot>();
  private readonly pizarras = new Map<string, Pizarra>();
  private readonly cadaTicks: number;
  private readonly semilla: number;
  private readonly siempre: boolean;
  /** El instante de la última vista: el del tick en curso. */
  private instanteActual?: Instante;

  constructor(
    private readonly puerto: PuertoBot,
    private readonly cerebro: Cerebro,
    opciones: { semilla: number; cadaTicks?: number; /** `siempre`: sin sesiones, conectados todo el día (tests). */ horario?: 'por-semilla' | 'siempre' }
  ) {
    this.cadaTicks = opciones.cadaTicks ?? 5;
    this.semilla = opciones.semilla;
    this.siempre = opciones.horario === 'siempre';
  }

  alta(heroeId: string, perfil: Perfil = { tipo: 'solitario' }): void {
    if (this.bots.has(heroeId)) return;
    const h = hash(heroeId);
    const rng = createRng((this.semilla ^ h) >>> 0);
    const sesiones: [number, number][] = this.siempre ? [[0, MINUTOS_DIA]] : horarioDe(rng);
    this.bots.set(heroeId, { heroeId, perfil, memoria: { esperas: new Map() }, rng, desfase: h % this.cadaTicks, sesiones });
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
      const debe = enSesion(bot, tick);
      if (debe !== bot.conectado) {
        this.puerto.actuar(id, debe ? 'conectarse' : 'desconectarse', { heroeId: id });
        bot.conectado = debe;
      }
      if (!debe) continue;
      const { columnaId, residenciaId } = bot.memoria;
      const leToca =
        (tick + bot.desfase) % this.cadaTicks === 0 ||
        (columnaId !== undefined && columnasTocadas.has(columnaId)) ||
        (residenciaId !== undefined && plazasTocadas.has(residenciaId));
      if (!leToca) continue;
      this.pensar(bot);
      pensaron++;
    }
    // Segunda pasada: los que siguen dentro de un campamento del que acaba de salir un compañero a esperarlos se le unen en
    // este mismo tick, antes de que la columna se mueva. En orden de id, como la primera.
    for (const id of [...this.bots.keys()].sort()) {
      const bot = this.bots.get(id)!;
      if (!bot.conectado || !this.leLlaman(bot)) continue;
      this.pensar(bot);
      pensaron++;
    }
    return pensaron;
  }

  private leLlaman(bot: Bot): boolean {
    const { dentroDe, pizarra } = bot.memoria;
    const salida = dentroDe && pizarra ? this.pizarras.get(pizarra)?.salidas.get(dentroDe) : undefined;
    return !!salida && salida.liderId !== bot.heroeId && salida.hasta > (this.instanteActual ?? -Infinity);
  }

  private pensar(bot: Bot): void {
    const vista = this.puerto.observar(bot.heroeId);
    if (!vista.heroe) return; // sin héroe no hay quien juegue
    this.instanteActual = vista.instante;
    const memoria = bot.memoria;
    memoria.dentroDe = vista.heroe.ubicacion.tipo === 'mercenarios' ? vista.heroe.ubicacion.campamentoId : undefined;
    memoria.columnaId = vista.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === bot.heroeId))?.id;
    memoria.residenciaId = vista.heroe.residenciaId ?? undefined;

    const clavePizarra = vista.faccionId ?? `sin-faccion:${bot.heroeId}`;
    memoria.pizarra = clavePizarra;
    let pizarra = this.pizarras.get(clavePizarra);
    if (!pizarra) this.pizarras.set(clavePizarra, (pizarra = pizarraVacia()));
    if (memoria.residenciaId) pizarra.residencias.set(bot.heroeId, memoria.residenciaId);
    else pizarra.residencias.delete(bot.heroeId);

    const actuar = <T extends TipoComando>(tipo: T, params: ParamsDe<T>) => this.puerto.actuar(bot.heroeId, tipo, params);
    this.cerebro({
      yo: bot.heroeId,
      perfil: bot.perfil,
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

