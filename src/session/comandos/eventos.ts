// Eventos de dominio que produce un COMANDO (no el tick).
//
// Fase A5 estructuró los 13 subsistemas que emiten DENTRO de `avanzarSimulacion`, y dejó fuera a propósito
// los eventos de comando (ver el marcador de A5 en Docs/Arquitectura/4_Plan_Evolucion_Tareas.md: "son eventos
// de COMANDO, no de tick... más cercano al punto 6 del doc 2"). Este módulo es el equivalente de
// `comoEventosDominio` (`engine/simulation.ts`) para ese otro lado: añade el contexto que el comando conoce
// (`momento` del `ContextoComando`, `tick` del estado) a lo que el comando narra.
//
// Por qué importa para Fase C: la autorización y las PROYECCIONES POR AUDIENCIA se construyen sobre los
// comandos. Un evento con `codigo` estable y `payload` tipado se puede filtrar por visibilidad; una cadena en
// castellano no. Los eventos de combate son el caso extremo — quién atacó a quién es justo lo que no puede
// viajar a cualquiera.
//
// Convención de códigos: la misma que fijó A5 para el motor — cada comando declara sus códigos como literales
// junto a su lógica y exporta sus propias interfaces `Payload*`, en vez de una unión discriminada global de
// ~40 variantes. Un consumidor que filtra por `codigo` sabe con qué forma castear.
import type { EventoCrudo, EventoDominio } from '../../domain/eventos';
import type { GameSessionState } from '../estado';
import type { ContextoComando } from './tipos';

/** Lo que un comando narra, antes de que se le añada el contexto temporal. Misma forma que el `EventoCrudo`
 * ya migrado del motor, más `asentamientoId` (que en el tick lo pone `simulation.ts` centralmente porque lo
 * sabe por el bucle; aquí lo sabe el propio comando). */
export interface EventoDeComando {
  codigo: string;
  mensaje: string;
  payload?: unknown;
  /** Asentamiento al que se atribuye. Ausente en eventos de alcance global (facción, diplomacia, mercado). */
  asentamientoId?: string;
}

/** Un evento de comando con su contexto temporal ya puesto. */
export function evento(ctx: ContextoComando, estado: GameSessionState, e: EventoDeComando): EventoDominio {
  return { ...e, momento: ctx.momento, tick: estado.tick };
}

/** Varios de una vez, cuando el comando narra más de un hecho. */
export function eventos(ctx: ContextoComando, estado: GameSessionState, lista: EventoDeComando[]): EventoDominio[] {
  return lista.map((e) => evento(ctx, estado, e));
}

/**
 * Adapta lo que devuelve una función del MOTOR que ya emite `EventoCrudo[]` (`engine/combate.ts`,
 * `engine/fusion.ts`, `engine/diplomacia.ts`) — el motor no conoce ni el momento, ni el tick, ni a qué
 * asentamiento atribuirlo, igual que dentro del tick. Un `EventoCrudo` que todavía sea `string` plano se
 * envuelve como `'legado'`, mismo atajo que usa `simulation.ts` para un subsistema sin migrar.
 */
export function desdeCrudos(
  ctx: ContextoComando,
  estado: GameSessionState,
  crudos: EventoCrudo[],
  asentamientoId?: string
): EventoDominio[] {
  return crudos.map((crudo) => {
    const base = typeof crudo === 'string' ? { codigo: 'legado', mensaje: crudo } : crudo;
    return { ...base, momento: ctx.momento, tick: estado.tick, asentamientoId };
  });
}
