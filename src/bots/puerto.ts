// El puerto de un bot (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): lo único que un cerebro conoce del mundo.
// **Observar** es la proyección del jugador, con su niebla; **actuar** es un comando de jugador, con su autorización.
// Un bot no puede hacer nada que un jugador no pueda hacer, ni saber nada que un jugador no sepa (§2).
//
// Dos adaptadores con la misma forma (§4), y el cerebro no se entera de cuál tiene: el EN PROCESO (aquí, sobre
// `GameSession`, para el batch y los tests) y el REMOTO (`remoto/puertoRemoto.ts`: HTTP y tiempo real, desde otro
// proceso). Por eso todo es asíncrono: en el remoto cada cosa es una petición.
import type { GameSession } from '../session/gameSession';
import type { ResultadoComando } from '../session/comandos/tipos';
import { REGISTRO_COMANDOS, type DatosDe, type ParamsDe, type TipoComando } from '../session/comandos/registro';
import { verificarAutorizacion } from '../session/comandos/autorizacion';
import { campamentosParaElegir, proyectarParaJugador } from '../session/proyecciones/jugador';
import { crearHeroe, type ParamsCrearHeroe } from '../session/comandos/crearHeroe';
import type { GeometriaAsentamientos } from '../session/estado';
import type { Asentamiento } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../engine/zones';
import { trazadoParaAsentamiento } from '../engine/trazado';

/** Lo que ve un héroe: su proyección de jugador. */
export type Vista = ReturnType<typeof proyectarParaJugador>;

/** Lo que ve quien todavía no tiene héroe: los campamentos donde puede nacer (D3, D79). */
export type CampamentoElegible = ReturnType<typeof campamentosParaElegir>[number];

/** La respuesta a un comando. `noAutorizado` es el 403 de la API: el comando ni se intentó. */
export type Respuesta<R> = ResultadoComando<R> & { noAutorizado?: string };

export interface PuertoBot {
  observar(heroeId: string): Promise<Vista>;
  actuar<T extends TipoComando>(heroeId: string, tipo: T, params: ParamsDe<T>): Promise<Respuesta<DatosDe<T>>>;
  /**
   * Abrir y cerrar el cliente (Doc 1.10.6): estar conectado es tenerlo abierto. En proceso son los comandos `conectarse` y
   * `desconectarse`; en remoto, abrir y cerrar su conexión de tiempo real, que es lo que hace un humano.
   */
  conectar(heroeId: string): Promise<void>;
  desconectar(heroeId: string): Promise<void>;
  /** La geografía del mundo: pública y la misma para todos (un cliente la pide una vez por `mapaId`). Va con el estado
   * de sus nodos de la última vista: lo que hay que mirar lo mira el cerebro con ella. */
  mapa(): Mapa;
  /**
   * Llega un bot (§8.3), como un humano: su cuenta (de bot, así que el héroe nace `controlador: 'bot'`), la pantalla de
   * elección (D3, D79) —`elegir` decide el campamento— y su héroe. Devuelve su id, o `undefined` si algo se rechaza.
   */
  llegar(nombre: string, datos: DatosDeHeroe, elegir: (campamentos: CampamentoElegible[]) => string | undefined): Promise<string | undefined>;
}

/** Lo que pone quien crea un héroe, salvo el campamento (lo elige al ver la pantalla) y el controlador (lo pone el servidor). */
export type DatosDeHeroe = Omit<ParamsCrearHeroe, 'controlador' | 'campamentoId'>;

/**
 * El adaptador en proceso: llama a `GameSession` igual que la ruta HTTP (`rutas/comandos.ts`) —autorización con el
 * rol `jugador` y el héroe como actor, luego el comando—, y proyecta con la geometría del momento.
 */
export function puertoEnProceso(sesion: GameSession): PuertoBot {
  // Misma caché por identidad que `RunnerDePartida.geometriaAsentamientos`: el array no cambia mientras nadie lo toque.
  let cache: { sobre: readonly Asentamiento[]; valor: GeometriaAsentamientos } | null = null;
  const geometria = (): GeometriaAsentamientos => {
    const asentamientos = sesion.getState().asentamientos;
    if (cache?.sobre !== asentamientos) {
      const zonas = computeTodasLasZonas(asentamientos);
      cache = {
        sobre: asentamientos,
        valor: {
          zonas,
          zonasFusionadas: computeZonasFusionadasPorFaccion(zonas, asentamientos),
          trazadoPorAsentamiento: Object.fromEntries(asentamientos.map((a) => [a.id, trazadoParaAsentamiento(a)])),
        },
      };
    }
    return cache.valor;
  };

  const actuar: PuertoBot['actuar'] = async (heroeId, tipo, params) => {
    const chequeo = verificarAutorizacion(tipo, params, sesion.getState(), { rol: 'jugador', heroeId });
    if (!chequeo.autorizado) return { ok: false, eventos: [], version: sesion.getState().version, noAutorizado: chequeo.motivo };
    return sesion.ejecutar(REGISTRO_COMANDOS[tipo] as never, params as never, { actor: heroeId });
  };
  return {
    observar: async (heroeId) => proyectarParaJugador(sesion.getState(), heroeId, geometria()),
    actuar,
    conectar: async (heroeId) => void (await actuar(heroeId, 'conectarse', { heroeId })),
    desconectar: async (heroeId) => void (await actuar(heroeId, 'desconectarse', { heroeId })),
    mapa: () => sesion.getMapa(),
    llegar: async (nombre, datos, elegir) => {
      const campamentoId = elegir(campamentosParaElegir(sesion.getState()));
      if (!campamentoId) return undefined;
      return sesion.ejecutar(crearHeroe, { ...datos, campamentoId, controlador: 'bot' }, { actor: `cuenta-${nombre}` }).datos?.heroeId;
    },
  };
}
