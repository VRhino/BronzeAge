// Dónde están las escuadras y cómo las ve el motor.
//
// Las escuadras viven en su héroe (`Heroe.escuadrones`, doc 01 §13) y los ejércitos y caravanas solo guardan sus
// ids. Eso deja trasladar un campamento sin mover nada y la unicidad por tropa en una sola lista, pero el motor
// militar razona por contenedor —la tropa de ESTA columna, la escolta de ESTA caravana—, así que trabaja sobre
// VISTAS que las llevan puestas (`EjercitoConTropa`, `CaravanaConEscolta`): se montan al entrar en una operación
// y se deshacen al salir, devolviendo al héroe las escuadras que cambiaron (`conEscuadrones`).
import type { Asentamiento, Caravana, Ejercito, Escuadron, Heroe } from '../domain/types';
import { esResidente } from './pertenencia';

export type EjercitoConTropa = Ejercito & { escuadrones: Escuadron[] };
export type CaravanaConEscolta = Caravana & { escolta?: Escuadron[] };
export type IndiceTropa = ReadonlyMap<string, Escuadron>;

export function indiceTropa(heroes: readonly Heroe[]): Map<string, Escuadron> {
  return new Map(heroes.flatMap((h) => h.escuadrones.map((e) => [e.id, e] as const)));
}

export function conTropa(ejercito: Ejercito, indice: IndiceTropa): EjercitoConTropa {
  return { ...ejercito, escuadrones: ejercito.escuadronIds.flatMap((id) => indice.get(id) ?? []) };
}

/** Deshace la vista: el ejército vuelve a guardar solo ids, y sus escuadras quedan marcadas dentro de él. */
export function sinTropa({ escuadrones, ...ejercito }: EjercitoConTropa): { ejercito: Ejercito; tropa: Escuadron[] } {
  return {
    ejercito: { ...ejercito, escuadronIds: escuadrones.map((e) => e.id) },
    tropa: escuadrones.map((e) => ({ ...e, contenedor: { tipo: 'ejercito', ejercitoId: ejercito.id }, enGuarnicion: false })),
  };
}

export function conEscolta(caravana: Caravana, indice: IndiceTropa): CaravanaConEscolta {
  return caravana.escoltaIds ? { ...caravana, escolta: caravana.escoltaIds.flatMap((id) => indice.get(id) ?? []) } : caravana;
}

export function sinEscolta({ escolta, ...caravana }: CaravanaConEscolta): { caravana: Caravana; tropa: Escuadron[] } {
  const tropa = (escolta ?? []).map((e) => ({ ...e, contenedor: { tipo: 'escolta' as const, caravanaId: caravana.id }, enGuarnicion: false }));
  return { caravana: { ...caravana, escoltaIds: tropa.length > 0 ? tropa.map((e) => e.id) : undefined }, tropa };
}

/** Vuelven al campamento: a la plaza donde reside su héroe, o a ninguna si es huérfano (Doc 5.15.2). */
export function alCampamento(tropa: readonly Escuadron[]): Escuadron[] {
  return tropa.map((e) => ({ ...e, contenedor: { tipo: 'campamento' } }));
}

/** Devuelve al campamento las escuadras con esos ids: una escolta que vuelve con su caravana (Doc 3.13.4). */
export function alCampamentoPorIds(heroes: readonly Heroe[], ids: readonly string[]): Heroe[] {
  if (ids.length === 0) return heroes as Heroe[];
  const indice = indiceTropa(heroes);
  return conEscuadrones(heroes, alCampamento(ids.flatMap((id) => indice.get(id) ?? [])));
}

/** El campamento de una plaza: lo que sus residentes no llevan consigo (Doc 5.15.2). Todo él come del almacén,
 * pero solo defiende lo que dice `defensaDe`. */
export function campamentoDe(asentamiento: Asentamiento, heroes: readonly Heroe[]): Escuadron[] {
  return heroes
    .filter((h) => esResidente(asentamiento, h.id))
    .flatMap((h) => h.escuadrones.filter((e) => e.contenedor.tipo === 'campamento'));
}

/** La guarnición de una plaza: lo que sus residentes han entregado a la IA (Doc 5.15.3). */
export function guarnicionDe(asentamiento: Asentamiento, heroes: readonly Heroe[]): Escuadron[] {
  return campamentoDe(asentamiento, heroes).filter((e) => e.enGuarnicion);
}

/** Los héroes que defienden una plaza en persona: residentes que están DENTRO y sanos (Doc 5.12.4, 5.16.4). Son el
 * bando que pierde si la plaza cae. */
export function heroesQueDefienden(asentamiento: Asentamiento, heroes: readonly Heroe[], heridos: ReadonlySet<string>): Heroe[] {
  return heroes.filter(
    (h) =>
      esResidente(asentamiento, h.id) &&
      !heridos.has(h.id) &&
      h.ubicacion.tipo === 'asentamiento' &&
      h.ubicacion.asentamientoId === asentamiento.id
  );
}

/**
 * Quién defiende una plaza en un asedio que se resuelve con números (Doc 5.12.4): su guarnición, y cada residente
 * que está DENTRO y sano con las escuadras de su loadout activo que tenga en el campamento (decisión del usuario
 * 2026-09-14). El loadout de un herido no defiende; la guarnición sí, porque no tiene héroe (Doc 5.16.4). El resto
 * del campamento no defiende. El Liderazgo del loadout ya se comprobó al guardarlo.
 */
export function defensaDe(asentamiento: Asentamiento, heroes: readonly Heroe[], heridos: ReadonlySet<string>): Escuadron[] {
  const enPersona = new Set(heroesQueDefienden(asentamiento, heroes, heridos).flatMap((h) => h.loadouts.find((l) => l.activo)?.squadIds ?? []));
  return heroes
    .filter((h) => esResidente(asentamiento, h.id))
    .flatMap((h) => h.escuadrones.filter((e) => e.contenedor.tipo === 'campamento' && (e.enGuarnicion || enPersona.has(e.id))));
}

/** Suelta la guarnición de un héroe que deja de residir donde la tenía: solo se guarnece la residencia (5.15.3). */
export function sinGuarnicion(heroes: readonly Heroe[], heroeId: string): Heroe[] {
  return heroes.map((h) =>
    h.id === heroeId && h.escuadrones.some((e) => e.enGuarnicion)
      ? { ...h, escuadrones: h.escuadrones.map((e) => (e.enGuarnicion ? { ...e, enGuarnicion: false } : e)) }
      : h
  );
}

/** Devuelve a su héroe las escuadras que cambiaron (por id) y añade las nuevas. Una escuadra sin héroe es un
 * estado corrupto, no algo que ignorar: sin dueño no hay dónde guardarla. */
export function conEscuadrones(heroes: readonly Heroe[], actualizadas: readonly Escuadron[]): Heroe[] {
  if (actualizadas.length === 0) return heroes as Heroe[];
  const porHeroe = new Map<string, Escuadron[]>();
  for (const e of actualizadas) porHeroe.set(e.heroeId, [...(porHeroe.get(e.heroeId) ?? []), e]);
  const sinDueno = [...porHeroe.keys()].find((id) => !heroes.some((h) => h.id === id));
  if (sinDueno) throw new Error(`Escuadra del héroe ${sinDueno}, que no existe.`);
  return heroes.map((h) => {
    const suyas = porHeroe.get(h.id);
    if (!suyas) return h;
    const porId = new Map(suyas.map((e) => [e.id, e]));
    const escuadrones = h.escuadrones.map((e) => porId.get(e.id) ?? e);
    for (const e of suyas) if (!h.escuadrones.some((x) => x.id === e.id)) escuadrones.push(e);
    return { ...h, escuadrones };
  });
}
