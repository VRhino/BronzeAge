// Comandos militares: reclutar tropa y las cuatro formas de combate (asedio, campo abierto, intercepción de
// caravana y ataque a campamento de bandidos).
//
// Los cuatro de combate son de los pocos comandos de jugador que consumen aleatoriedad (`ctx.rng` — jitter de
// combate, ver `engine/combate.ts`). Por eso importa que el rng venga del contexto y no de un global: es lo
// que mantiene una partida reproducible aunque un jugador ataque en mitad de ella.
//
// Sus eventos son los MÁS sensibles a visibilidad de toda la capa de comandos —quién atacó a quién— así que
// son los que más ganan con `codigo`/`payload` estructurados: las proyecciones por audiencia de Fase C
// filtran sobre eso. Los payloads de combate los declara `engine/combate.ts`, que es quien resuelve.
import { reclutarTropa as reclutarTropaEngine } from '../../engine/tropas';
import { esCiudadano } from '../../engine/faccion';
import {
  CombateInvalidoError,
  desalojarResidentes,
  iniciarAsedio as iniciarAsedioEngine,
} from '../../engine/combate';
import { heridosEn, herir } from '../../engine/heroe';
import { conEscuadrones, defensaDe, heroesQueDefienden, sinTropa } from '../../engine/tropa';
import type { Escuadron } from '../../domain/types';

/** Lo que se puede sacar del campamento a combatir: la guarnición la maneja la IA de la plaza (Doc 5.15.3), y las
 * escuadras de un héroe herido no combaten (Doc 5.16.4). */
function combatientes(tropa: readonly Escuadron[], ids: readonly string[], heridos: ReadonlySet<string>): Escuadron[] {
  if (tropa.some((e) => ids.includes(e.id) && heridos.has(e.heroeId))) throw new CombateInvalidoError('Las escuadras de un héroe herido no combaten.');
  return tropa.filter((e) => !e.enGuarnicion);
}

/** Los héroes que llevan a la batalla las escuadras elegidas: si pierden, quedan heridos (Doc 5.16.4). */
const duenosDe = (tropa: readonly Escuadron[], ids: readonly string[]): string[] => [...new Set(tropa.filter((e) => ids.includes(e.id)).map((e) => e.heroeId))];
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { campamentoEn, comando, conAsentamiento, exigirAsentamiento } from './ayudas';
import { desdeCrudos, evento } from './eventos';

/** Reclutamiento: lo narra esta capa (el motor devuelve el asentamiento actualizado, sin eventos). */
export interface PayloadReclutamiento {
  asentamientoId: string;
  heroeId: string;
  tropaId: string;
  origen: 'pesants' | 'artesanos';
  reclutados: number;
}

export interface ParamsReclutarTropa {
  asentamientoId: string;
  heroeId: string;
  tropaId: string;
  origen: 'pesants' | 'artesanos';
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
    params.origen,
    ctx.ids.siguiente()
  );
  const reclutados = cantidadDe(r.heroes) - antes;

  const siguiente = conHistorialDeJugador(
    { ...conAsentamiento(estado, r.asentamiento), heroes: r.heroes },
    params.heroeId,
    `Recluta ${reclutados} de "${params.tropaId}" en ${asentamiento.id}.`
  );
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'tropas.reclutadas',
        mensaje: `${params.heroeId} recluta ${reclutados} de la tropa "${params.tropaId}" (${params.origen}).`,
        payload: { ...params, reclutados } satisfies PayloadReclutamiento,
        asentamientoId: asentamiento.id,
      }),
    ],
    { reclutados }
  );
});

export interface ParamsIniciarAsedio {
  atacanteId: string;
  defensorId: string;
  escuadronIds: string[];
}

export const iniciarAsedio = comando<ParamsIniciarAsedio, { conquistado: boolean }>((estado, _mapa, ctx, params) => {
  const atacante = exigirAsentamiento(estado, params.atacanteId);
  const defensor = exigirAsentamiento(estado, params.defensorId);
  const heridos = heridosEn(estado.heroes, ctx.instante);
  const tropa = combatientes(campamentoEn(estado, atacante), params.escuadronIds, heridos);
  const defensores = heroesQueDefienden(defensor, estado.heroes, heridos);
  const defensa = defensaDe(defensor, estado.heroes, heridos);

  const resultado = iniciarAsedioEngine(
    atacante,
    tropa,
    defensor,
    defensa,
    params.escuadronIds,
    estado.facciones,
    estado.relaciones,
    ctx.instante,
    ctx.rng
  );
  // Los héroes del bando que pierde quedan heridos (Doc 5.16.4). Una plaza ocupada rebota sin combate (`tropa` vacía).
  const vencidos = resultado.tropa.length === 0 ? [] : resultado.conquistado ? defensores.map((h) => h.id) : duenosDe(tropa, params.escuadronIds);
  const tras: GameSessionState = {
    ...conAsentamiento(estado, resultado.defensor),
    facciones: resultado.facciones,
    heroes: herir(conEscuadrones(estado.heroes, resultado.tropa), vencidos, ctx.instante),
  };
  // Los residentes derrotados se van con su campamento a 0 a la plaza más cercana de su Facción, y quien estaba dentro
  // queda fuera, junto a ella, con las escuadras con las que defendió (Doc 5.15.5).
  const lucharon = new Set(defensa.filter((e) => !e.enGuarnicion).map((e) => e.id));
  const desalojo = resultado.conquistado
    ? desalojarResidentes(defensor, tras.asentamientos, tras.heroes, tras.ejercitos, lucharon, ctx.instante)
    : undefined;
  const siguiente: GameSessionState = desalojo
    ? {
        ...tras,
        asentamientos: desalojo.asentamientos,
        heroes: desalojo.heroes,
        ejercitos: [...tras.ejercitos, ...desalojo.columnas.map((c) => sinTropa(c).ejercito)],
      }
    : tras;
  return exito(siguiente, desdeCrudos(ctx, resultado.eventos, atacante.id), { conquistado: resultado.conquistado });
});

// `combateCampoAbierto` e `interceptarCaravana` VIVÍAN AQUÍ y se retiraron en el Paso 11 del movimiento de
// ejércitos (2026-09-04). No se han perdido: son ahora resoluciones del MOTOR disparadas por la geometría
// (`resolverEncuentros`, engine/ejercitos.ts), y el jugador llega a ellas mandando un ejército en vez de
// declarando un ataque desde el sofá. Ver Doc 5.12.3 y §2.6 del documento de ejecución.
//
// Retirarlos no era solo limpieza: `interceptarCaravana` resolvía contra una defensa base FIJA, que es
// exactamente lo que la escolta (Doc 5.13.3) sustituyó — mantener los dos habría dejado dos reglas distintas
// para el mismo hecho según por dónde se entrara.

// `atacarCampamentoBandidos` (atacar un campamento desde una plaza, con escuadras del campamento) se retiró el
// 2026-09-15: un campamento se ataca con una columna que llegue a él (Doc 1.9), con `atacar` (`interaccion.ts`). El
// motor conserva el ataque desde una plaza para los NPC (`npcGobernanza.ts`).
