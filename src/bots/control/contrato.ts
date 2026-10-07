// Contrato del canal de control del servicio de bots (WebSocket admin ↔ servicio de bots). Sin imports a propósito: lo usa
// el cliente admin solo como tipos, igual que `contratos/v1`, y no arrastra nada del motor.
//
// Protocolo (JSON por mensaje de texto):
//   cliente → servicio   {id, accion, ...}          la primera acción tiene que ser `autenticar`
//   servicio → cliente   {tipo:'respuesta', id, ok, error?, datos?}
//                      | {tipo:'estado', estado}     al conectar y tras cada vuelta, cambio de fase o comando
//                      | {tipo:'accion', accion}     cada comando que manda un bot, en vivo
//                      | {tipo:'historial', acciones, registros}   al conectar, lo último que pasó
//                      | {tipo:'registro', nivel, texto, en}   lo que el servicio grita (errores, arranques)

export type FaseServicio = 'inactivo' | 'arrancando' | 'corriendo' | 'pausado' | 'parando' | 'error';

export type PerfilBot = { tipo: 'amigos'; lider: string } | { tipo: 'solitario' } | { tipo: 'tardio' };
export type TipoPerfil = PerfilBot['tipo'];

/** Lo que se pone al iniciar. La semilla y los días de llegada no se tocan en caliente: cambiarían el plan de llegadas. */
export interface ConfigBots {
  /** Raíz del servidor del juego, sin `/v1`. */
  servidor: string;
  partida: string;
  /** `CODIGO_REGISTRO_BOTS`. Si el servicio ya lo tiene por entorno, no hace falta mandarlo; nunca vuelve en el estado. */
  codigoRegistroBots?: string;
  total: number;
  diasLlegada: number;
  semilla: number;
  /** Cada cuánto mira si avanzó el mundo (ms). */
  cadaMs: number;
  horario: 'por-semilla' | 'siempre';
}

export type ConfigPublica = Omit<ConfigBots, 'codigoRegistroBots'>;

export type FaseBot = 'sin-plaza' | 'en-casa' | 'en-columna' | 'sin-datos';

export interface AccionDeBot {
  /** Id del héroe que actuó. */
  heroeId: string;
  tipo: string;
  ok: boolean;
  /** `codigoError` del motor, o el motivo de la autorización si el comando ni se intentó. */
  motivo?: string;
  ms: number;
  /** Reloj de pared del servicio (ms epoch). */
  en: number;
}

/** Cómo se maneja a un bot desde el panel: `auto` sigue su horario; `conectado`/`desconectado` lo fuerzan; `congelado` sigue su
 * horario de conexión pero no piensa. */
export type ModoBot = 'auto' | 'conectado' | 'desconectado' | 'congelado';

export interface BotInfo {
  heroeId: string;
  nombre: string;
  perfil: PerfilBot;
  fase: FaseBot;
  conectado?: boolean;
  modo: ModoBot;
  /** Retirado: ya no juega ni se vuelve a conectar. Sigue en el registro. */
  retirado: boolean;
  /** Resumen del plan en curso («cazar campamento-3»). */
  plan?: string;
  faccionId?: string;
  residenciaId?: string;
  columnaId?: string;
  /** Acciones rechazadas en espera: no se reintentan hasta su instante de mundo. */
  esperas: { clave: string; hasta: number }[];
  /** Horario de juego: minutos del día [inicio, fin). */
  sesiones: [number, number][];
  pensamientos: number;
  /** Instante de mundo (ms) de su última vista. */
  vistaEn?: number;
  ultimaAccion?: AccionDeBot;
  errores: number;
  ultimoError?: { mensaje: string; en: number };
}

export interface LlegadaPlan {
  grupo: number;
  perfil: TipoPerfil;
  cuantos: number;
  /** Ticks desde que el proceso arrancó en la partida. */
  tick: number;
  hecha: boolean;
}

/** Lo que comparten los bots de una Facción (o un bot sin Facción, solo). */
export interface PizarraInfo {
  id: string;
  bots: string[];
  encargos: { clave: string; heroeId: string }[];
  bandidos: number;
  salidas: number;
  explorados: number;
  /** El detalle de lo anterior, para el panel (instantes en ms de mundo). */
  detalle: {
    bandidosVistos: { id: string; x: number; y: number; poder: number; vistoEn: number }[];
    salidasAbiertas: { campamentoId: string; ejercitoId: string; liderId: string; hasta: number; para: 'cazar' | 'fundar' }[];
    exploradosEn: { campamentoId: string; en: number }[];
    residencias: { heroeId: string; plazaId: string }[];
    listos: { heroeId: string; campamentoId: string; conRacion: boolean; hasta: number }[];
  };
}

export interface Salud {
  /** La última vuelta: cuánto tardó y cuántos bots pensaron. */
  vuelta?: { ms: number; pensaron: number; tick: number; en: number };
  /** Ventana de los últimos 60 s. */
  peticionesMin: number;
  erroresMin: number;
  latenciaP50: number;
  latenciaP95: number;
  socketsAbiertos: number;
  sesionesRenovadas: number;
}

export interface EstadoServicio {
  fase: FaseServicio;
  /** Reloj de pared del servicio (ms epoch) cuando se mandó este estado. */
  ahora: number;
  /** Desde cuándo está en `corriendo` (ms epoch). */
  iniciadoEn?: number;
  config?: ConfigPublica;
  /** Tick del mundo en la última vuelta. */
  tick?: number;
  error?: string;
  llegadas: { hechas: number; extras: number; plan: LlegadaPlan[]; inicioTick?: number };
  salud: Salud;
  bots: BotInfo[];
  pizarras: PizarraInfo[];
  /** Si el servicio ya tiene `CODIGO_REGISTRO_BOTS` por entorno (el panel no lo pide). */
  codigoEnEntorno: boolean;
  /** Los valores que el servicio propone al configurar (de su entorno). */
  porDefecto: Partial<ConfigPublica>;
}

export type MensajeDeServicio =
  | { tipo: 'respuesta'; id: number; ok: boolean; error?: string; datos?: unknown }
  | { tipo: 'estado'; estado: EstadoServicio }
  | { tipo: 'accion'; accion: AccionDeBot }
  /** Al conectar: lo último que pasó, para no ver un panel vacío. */
  | { tipo: 'historial'; acciones: AccionDeBot[]; registros: { nivel: 'info' | 'error'; texto: string; en: number }[] }
  | { tipo: 'registro'; nivel: 'info' | 'error'; texto: string; en: number };

export type ComandoDeControl =
  | { accion: 'autenticar'; token: string }
  | { accion: 'iniciar'; config: ConfigBots }
  | { accion: 'pausar' }
  | { accion: 'reanudar' }
  | { accion: 'parar' }
  /** Sin `partida`, la de la última configuración. Con ella, la de esa partida (la consola la borra al borrar la partida). */
  | { accion: 'reiniciarRegistro'; partida?: string }
  | { accion: 'ajustar'; cadaMs: number }
  | { accion: 'forzarLlegada'; perfil: TipoPerfil }
  | { accion: 'modoBot'; heroeId: string; modo: ModoBot }
  | { accion: 'retirarBot' | 'pensarYa' | 'volcarMemoria'; heroeId: string };

export type ComandoConId = ComandoDeControl & { id: number };
