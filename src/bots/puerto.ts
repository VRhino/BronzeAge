// El puerto de un bot (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): lo único que un cerebro conoce del mundo.
// **Observar** es la proyección del jugador, con su niebla; **actuar** es un comando de jugador, con su autorización.
// Un bot no puede hacer nada que un jugador no pueda hacer, ni saber nada que un jugador no sepa (§2).
//
// Hoy hay un adaptador, el EN PROCESO (sobre `GameSession`, para el batch y los tests). El remoto (HTTP + tiempo
// real, en un proceso aparte) llegará con el despliegue y tendrá la misma forma: el cerebro no se entera.
import type { GameSession } from '../session/gameSession';
import type { ResultadoComando } from '../session/comandos/tipos';
import { REGISTRO_COMANDOS, type DatosDe, type ParamsDe, type TipoComando } from '../session/comandos/registro';
import { verificarAutorizacion } from '../session/comandos/autorizacion';
import { proyectarParaJugador } from '../session/proyecciones/jugador';
import type { GeometriaAsentamientos } from '../session/estado';
import type { Asentamiento } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { computeTodasLasZonas, computeZonasFusionadasPorFaccion } from '../engine/zones';
import { trazadoParaAsentamiento } from '../engine/trazado';

/** Lo que ve un héroe: su proyección de jugador. */
export type Vista = ReturnType<typeof proyectarParaJugador>;

/** La respuesta a un comando. `noAutorizado` es el 403 de la API: el comando ni se intentó. */
export type Respuesta<R> = ResultadoComando<R> & { noAutorizado?: string };

export interface PuertoBot {
  observar(heroeId: string): Vista;
  actuar<T extends TipoComando>(heroeId: string, tipo: T, params: ParamsDe<T>): Respuesta<DatosDe<T>>;
  /** La geografía del mundo: pública y la misma para todos (un cliente la pide una vez por `mapaId`). */
  mapa(): Mapa;
}

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

  return {
    observar: (heroeId) => proyectarParaJugador(sesion.getState(), heroeId, geometria()),
    mapa: () => sesion.getMapa(),
    actuar: (heroeId, tipo, params) => {
      const chequeo = verificarAutorizacion(tipo, params, sesion.getState(), { rol: 'jugador', heroeId });
      if (!chequeo.autorizado) return { ok: false, eventos: [], version: sesion.getState().version, noAutorizado: chequeo.motivo };
      return sesion.ejecutar(REGISTRO_COMANDOS[tipo] as never, params as never, { actor: heroeId });
    },
  };
}
