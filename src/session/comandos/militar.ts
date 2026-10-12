// Comandos militares: reclutar tropa. Los de combate (`atacar`, `inspeccionar`...) viven en `interaccion.ts`.
import { poblacionDeTropa, reclutarTropa as reclutarTropaEngine } from '../../engine/tropas';
import { TROPAS_RECLUTABLES } from '../../constants';
import { contadoresDeReclutamiento, sumarContadores, tecnologiasDe } from '../../engine/tecnologia';
import { esCiudadano } from '../../engine/faccion';
import type { OrigenTropa } from '../../domain/types';

import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento } from './ayudas';
import { evento } from './eventos';

/** Reclutamiento: lo narra esta capa (el motor devuelve el asentamiento actualizado, sin eventos). */
export interface PayloadReclutamiento {
  asentamientoId: string;
  heroeId: string;
  tropaId: string;
  /** De qué clase salieron: la decide el escalón de la tropa (Doc 5.8). */
  origen: OrigenTropa;
  reclutados: number;
}

export interface ParamsReclutarTropa {
  asentamientoId: string;
  heroeId: string;
  tropaId: string;
  /** Ya no decide nada: la clase de población la pone el escalón de la tropa (Doc 5.8). Se admite para no romper a
   * los clientes que todavía la mandan. */
  origen?: 'pesants' | 'artesanos';
}

export const reclutarTropa = comando<ParamsReclutarTropa, { reclutados: number }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const cantidadDe = (heroes: GameSessionState['heroes']): number =>
    heroes.find((h) => h.id === params.heroeId)?.escuadrones.find((e) => e.tropaId === params.tropaId)?.cantidad ?? 0;

  const antes = cantidadDe(estado.heroes);
  const faccionDelJugador = estado.facciones.find((f) => esCiudadano(f, params.heroeId));
  const r = reclutarTropaEngine(
    asentamiento,
    estado.heroes,
    estado.ejercitos,
    params.heroeId,
    faccionDelJugador?.id ?? '',
    params.tropaId,
    tecnologiasDe(estado.tecnologia, asentamiento.faccionId).adoptadas,
    ctx.ids.siguiente()
  );
  const reclutados = cantidadDe(r.heroes) - antes;

  const siguiente = conHistorialDeJugador(
    {
      ...conAsentamiento(estado, r.asentamiento),
      heroes: r.heroes,
      tecnologia: sumarContadores(estado.tecnologia, contadoresDeReclutamiento(params.tropaId, reclutados, antes === 0)),
    },
    params.heroeId,
    `Recluta ${reclutados} de "${params.tropaId}" en ${asentamiento.id}.`
  );
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === params.tropaId)!;
  const origen = poblacionDeTropa(tropa);
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'tropas.reclutadas',
        mensaje: `${params.heroeId} recluta ${reclutados} de la tropa "${params.tropaId}" (${origen}).`,
        payload: { asentamientoId: params.asentamientoId, heroeId: params.heroeId, tropaId: params.tropaId, origen, reclutados } satisfies PayloadReclutamiento,
        asentamientoId: asentamiento.id,
      }),
    ],
    { reclutados }
  );
});

// `combateCampoAbierto` e `interceptarCaravana` VIVÍAN AQUÍ y se retiraron en el Paso 11 del movimiento de
// ejércitos (2026-09-04). No se han perdido: son ahora resoluciones del MOTOR disparadas por la geometría
// (`resolverEncuentros`, engine/ejercitos.ts), y el jugador llega a ellas mandando un ejército en vez de
// declarando un ataque desde el sofá. Ver Doc 5.12.3 y §2.6 del documento de ejecución.
//
// Retirarlos no era solo limpieza: `interceptarCaravana` resolvía contra una defensa base FIJA, que es
// exactamente lo que la escolta (Doc 5.13.3) sustituyó — mantener los dos habría dejado dos reglas distintas
// para el mismo hecho según por dónde se entrara.

// Un campamento de bandidos se ataca con una columna que llegue a él (Doc 1.9), con `atacar` (`interaccion.ts`); el
// NPC hace lo mismo (`cazarBandidos`, `npcGobernanza.ts`).
