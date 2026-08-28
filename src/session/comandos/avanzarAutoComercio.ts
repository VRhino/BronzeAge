import type { Mapa } from '../../world/mapa';
import { avanzarAutoComercioSimulado } from '../../engine/simulacionAutoComercio';
import { SIMULACION_AUTO_COMERCIO } from '../../constants';
import { conResultadoDeSimulacion, estadoSimulacionDe, type GameSessionState } from '../estado';
import { exito, sinCambios, type ContextoComando, type TransicionComando } from './tipos';

/**
 * Trueque automático SOLO PARA SIMULACIÓN (`SIMULACION_AUTO_COMERCIO`, constants.ts): apagado por defecto y
 * deliberadamente FUERA de `avanzarSimulacion` — el juego real es 100% manual (Doc 3.2). Existe para que las
 * corridas de balance en batch tengan comercio sin un jugador detrás.
 *
 * Operación del sistema, como `avanzarTick`/`avanzarFaccionesNpc`, y en el mismo punto de la secuencia que
 * ocupaba en `GameStore`: después del tick del motor y antes del turno del NPC de gobernanza.
 *
 * Con la palanca apagada no hace nada y **no incrementa la versión**: el motor devuelve el mismo estado.
 */
export function avanzarAutoComercio(
  estado: GameSessionState,
  mapa: Mapa,
  _ctx: ContextoComando,
  _params: void
): TransicionComando<void> {
  if (!SIMULACION_AUTO_COMERCIO.activo) {
    return sinCambios(estado);
  }

  const resultado = avanzarAutoComercioSimulado(estadoSimulacionDe(estado), mapa, estado.tick);
  // Sin eventos propios: este módulo del motor no narra nada, solo mueve recursos y caravanas.
  return exito(conResultadoDeSimulacion(estado, resultado), []);
}
