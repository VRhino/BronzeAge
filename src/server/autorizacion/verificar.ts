// Punto de entrada de la autorización de comandos (Fase C2). `verificarAutorizacion` es lo único que
// `server/api.ts` necesita llamar: resuelve la fila de `MATRIZ_AUTORIZACION` para el `tipo` de comando y la
// aplica al actor resuelto (`ActorDeComando`, viene de la `Membresia` — nunca de un id que el cliente afirme
// tener, doc 2 principio 3).
import type { TipoComando } from '../../session/comandos/registro';
import type { ParamsDe } from '../../session/comandos/registro';
import type { GameSessionState } from '../../session/estado';
import type { ActorDeComando } from './matriz';
import { MATRIZ_AUTORIZACION } from './matriz';

export type MotivoDenegacion = 'rol_insuficiente' | 'condicion_dominio';

export type ResultadoAutorizacion = { autorizado: true } | { autorizado: false; motivo: MotivoDenegacion };

export function verificarAutorizacion<T extends TipoComando>(
  tipo: T,
  params: ParamsDe<T>,
  estado: GameSessionState,
  actor: ActorDeComando
): ResultadoAutorizacion {
  const entrada = MATRIZ_AUTORIZACION[tipo];
  if (!entrada.rolesPermitidos.includes(actor.rol)) return { autorizado: false, motivo: 'rol_insuficiente' };

  // La condición de dominio solo aplica al eje "jugador" (doc 5: "sin restricción si es admin") — un actor
  // que pasó el filtro de rol por `administrador_partida`/`moderador` no la evalúa.
  if (actor.rol === 'jugador' && entrada.condicionJugador && !entrada.condicionJugador(estado, actor, params)) {
    return { autorizado: false, motivo: 'condicion_dominio' };
  }
  return { autorizado: true };
}
