// Qué pasa con una Facción NPC que pierde su último asentamiento (decisiones del usuario, 2026-10-02), resuelto EN EL
// MOMENTO DEL HECHO y no en un barrido del turno NPC.
//
// Hasta ahora lo hacía `acogerHeroesNpc` al principio de cada turno NPC: miraba todas las Facciones y todos los héroes
// buscando derrotas sin resolver y bots sin casa. Era la misma forma que el barrido de residencia que retiró `9f9a098`, y
// al convivir con ella se pisaban: una disolución borraba héroes que seguían como residentes de un campamento de
// mercenarios, y una anexión dejaba bots residiendo en dos sitios (`Consideraciones/Auditoria_Tick_Eventos.md`, B5).
//
// Ahora lo llaman los sitios donde se pierde la última plaza, con el estado de antes y el de después: el tick (ruina y
// asedio de un ejército), el comando `iniciarAsedio`, el asedio por columna (`interaccion`) y el resultado de una batalla
// de Unity. El batch, que encadena motor y NPC sin `GameSession`, lo llama entre los dos.
//
// Las reglas:
//  - **Derrotada por otra Facción NPC → se anexiona.** Sus ciudadanos pasan a la ganadora, sus columnas cambian de
//    bandera y sus relaciones se disuelven. La ganadora reparte a sus héroes bot entre sus asentamientos, **contando el
//    recién conquistado**: cada uno va al que menos residentes tiene (a igualdad, el más cercano a la plaza perdida).
//  - **Derrotada por un jugador o caída por colapso → se disuelve**, con sus héroes bot y sus columnas.
//  - Una Facción que no es NPC no se toca: queda marcada (`derrotadaPor`) como hasta ahora.
//
// En los dos casos los bots salen de los campamentos de mercenarios donde los hubiera dejado el desalojo o la ruina.
import type { Asentamiento, CampamentoMercenarios, Ejercito, Faccion, Heroe, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { anexionar } from '../engine/fusion';
import { salirDeCampamentos } from '../engine/mercenarios';
import { distancia } from '../world/geometria';

/** Lo que una derrota necesita del mundo. Lo cumplen `EstadoSimulacion` (motor y batch) y `GameSessionState`. */
export interface EstadoConDerrotas {
  facciones: Faccion[];
  asentamientos: Asentamiento[];
  heroes: Heroe[];
  ejercitos: Ejercito[];
  relaciones: RelacionPolitica[];
  campamentosMercenarios: CampamentoMercenarios[];
}

/** Una Facción que acaba de perder su último asentamiento. */
export interface Derrota {
  faccionId: string;
  /** `null` si nadie la derrotó: su última plaza colapsó. */
  ganadoraId: string | null;
  /** La plaza que perdió, si sigue en pie (conquistada): el punto desde el que se reparten sus héroes. */
  plazaId?: string;
}

/** Payload de `faccion.anexionada` y `faccion.disuelta`. */
export interface PayloadDerrotaResuelta {
  faccionId: string;
  ganadoraId: string | null;
  heroesBot: string[];
}

/**
 * Las derrotas que ocurrieron entre `antes` y `despues`: Facciones que ya existían, no estaban derrotadas y ahora sí
 * (`registrarDerrota` las marca al perder la última plaza). Es la forma de que cada sitio que puede causar una derrota
 * diga cuáles causó sin tener que llevar la cuenta por su cuenta.
 */
export function derrotasEntre(
  antes: Pick<EstadoConDerrotas, 'facciones' | 'asentamientos'>,
  despues: Pick<EstadoConDerrotas, 'facciones' | 'asentamientos'>
): Derrota[] {
  const previas = new Map(antes.facciones.map((f) => [f.id, f]));
  const siguenEnPie = new Set(despues.asentamientos.map((a) => a.id));
  return despues.facciones
    .filter((f) => f.derrotadaPor !== undefined && previas.has(f.id) && previas.get(f.id)!.derrotadaPor === undefined)
    .map((f) => ({
      faccionId: f.id,
      ganadoraId: f.derrotadaPor ?? null,
      plazaId: antes.asentamientos.find((a) => a.faccionId === f.id && siguenEnPie.has(a.id))?.id,
    }));
}

const residentesDe = (a: Asentamiento): number => new Set([...a.heroesFundadoresIds, ...a.casasCompradas]).size;

/**
 * Resuelve las derrotas de Facciones NPC: anexión si la ganadora es NPC, disolución si no. Devuelve el estado con los
 * cambios y los eventos que los narran. Sin derrotas de Facciones NPC devuelve el MISMO estado.
 */
export function resolverDerrotasNpc<E extends EstadoConDerrotas>(
  estado: E,
  derrotas: readonly Derrota[],
  esNpc: (faccionId: string) => boolean
): { estado: E; eventos: EventoCrudo[] } {
  let actual: E = estado;
  const eventos: EventoCrudo[] = [];

  for (const derrota of derrotas) {
    const perdedora = actual.facciones.find((f) => f.id === derrota.faccionId);
    if (!perdedora || !esNpc(perdedora.id)) continue;
    // Si entre tanto volvió a tener una plaza (no debería en el mismo paso), no está derrotada.
    if (actual.asentamientos.some((a) => a.faccionId === perdedora.id)) continue;
    const bots = actual.heroes.filter((h) => h.controlador === 'bot' && perdedora.ciudadanosIds.includes(h.id)).map((h) => h.id);
    const ganadora = derrota.ganadoraId ? actual.facciones.find((f) => f.id === derrota.ganadoraId) : undefined;
    const sinRelaciones = actual.relaciones.filter((r) => r.faccionAId !== perdedora.id && r.faccionBId !== perdedora.id);
    const campamentos = bots.length > 0 ? salirDeCampamentos(actual.campamentosMercenarios, ...bots) : actual.campamentosMercenarios;

    if (ganadora && esNpc(ganadora.id)) {
      const anexion = anexionar(actual.facciones, actual.asentamientos, ganadora.id, perdedora.id);
      const origen = actual.asentamientos.find((a) => a.id === derrota.plazaId)?.posicion;
      let asentamientos = anexion.asentamientos;
      for (const bot of bots) {
        const casa = asentamientos
          .filter((a) => a.faccionId === ganadora.id)
          .sort(
            (a, b) =>
              residentesDe(a) - residentesDe(b) ||
              (origen ? distancia(a.posicion, origen) - distancia(b.posicion, origen) : 0) ||
              (a.id < b.id ? -1 : 1)
          )[0];
        if (!casa) break;
        asentamientos = asentamientos.map((a) => (a.id === casa.id ? { ...a, casasCompradas: [...a.casasCompradas, bot] } : a));
      }
      actual = {
        ...actual,
        facciones: anexion.facciones,
        asentamientos,
        ejercitos: actual.ejercitos.map((e) => (e.faccionId === perdedora.id ? { ...e, faccionId: ganadora.id } : e)),
        relaciones: sinRelaciones,
        campamentosMercenarios: campamentos,
      };
      // `anexion.eventos` (`diplomacia.anexion`) narra una anexión DIPLOMÁTICA, pactada; esta es por derrota y lleva su propio evento.
      eventos.push({
        codigo: 'faccion.anexionada',
        mensaje: `${perdedora.nombre} pierde su último asentamiento ante ${ganadora.nombre} y se une a ella: sus ${bots.length} héroes se reparten entre sus plazas.`,
        payload: { faccionId: perdedora.id, ganadoraId: ganadora.id, heroesBot: bots } satisfies PayloadDerrotaResuelta,
      });
      continue;
    }

    const quitar = new Set(bots);
    actual = {
      ...actual,
      facciones: actual.facciones.filter((f) => f.id !== perdedora.id),
      heroes: actual.heroes.filter((h) => !quitar.has(h.id)),
      ejercitos: actual.ejercitos.filter((e) => e.faccionId !== perdedora.id),
      relaciones: sinRelaciones,
      campamentosMercenarios: campamentos,
    };
    eventos.push({
      codigo: 'faccion.disuelta',
      mensaje: derrota.ganadoraId
        ? `${perdedora.nombre} pierde su último asentamiento y se disuelve: sus ${bots.length} héroes desaparecen.`
        : `${perdedora.nombre} pierde su último asentamiento por colapso y se disuelve: sus ${bots.length} héroes desaparecen.`,
      payload: { faccionId: perdedora.id, ganadoraId: derrota.ganadoraId, heroesBot: bots } satisfies PayloadDerrotaResuelta,
    });
  }

  return { estado: actual, eventos };
}

/** El "¿es NPC?" de una partida: las Facciones cedidas, o todas si no se dice (el batch gobierna el mundo entero). */
export const esNpcSegun =
  (faccionesNpcIds?: readonly string[]) =>
  (faccionId: string): boolean =>
    faccionesNpcIds ? faccionesNpcIds.includes(faccionId) : true;

/**
 * Para un comando o una operación del sistema: resuelve las derrotas que causó la transición `antes → despues`. Es la
 * única línea que necesita cada sitio que puede causar una derrota.
 */
export function conDerrotasResueltas<E extends EstadoConDerrotas & { faccionesNpcIds: string[] }>(
  antes: Pick<EstadoConDerrotas, 'facciones' | 'asentamientos'>,
  despues: E
): { estado: E; eventos: EventoCrudo[] } {
  const derrotas = derrotasEntre(antes, despues);
  if (derrotas.length === 0) return { estado: despues, eventos: [] };
  return resolverDerrotasNpc(despues, derrotas, esNpcSegun(despues.faccionesNpcIds));
}
