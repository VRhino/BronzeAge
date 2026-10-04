// Dónde están las escuadras y cómo las ve el motor.
//
// Las escuadras viven en su héroe (`Heroe.escuadrones`, doc 01 §13) y los ejércitos y caravanas solo guardan sus
// ids. Eso deja trasladar un campamento sin mover nada y la unicidad por tropa en una sola lista, pero el motor
// militar razona por contenedor —la tropa de ESTA columna, la escolta de ESTA caravana—, así que trabaja sobre
// VISTAS que las llevan puestas (`EjercitoConTropa`, `CaravanaConEscolta`): se montan al entrar en una operación
// y se deshacen al salir, devolviendo al héroe las escuadras que cambiaron (`conEscuadrones`).
import type { Asentamiento, Caravana, Ejercito, Escuadron, Heroe } from '../domain/types';
import { esResidente } from './pertenencia';
import { BATALLA, MILITAR, TROPAS_RECLUTABLES } from '../constants';

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

/** Poder de combate (Doc 5.1: héroe-comandante liderando tropa; el resultado es CÁLCULO, no combate visual, Doc 5.10).
 * `poderBase` sale siempre del catálogo `TROPAS_RECLUTABLES` vía `tropaId` (Doc 5.7/5.8) — toda tropa lo tiene,
 * y la experiencia la mejora sin cambiarla nunca de identidad (Doc 5.8, a petición del usuario). */
export function poderEscuadron(e: Escuadron): number {
  const poderBase = TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)!.poderBase;
  return poderBase * e.cantidad * (1 + e.nivel * MILITAR.bonusPoderPorNivelEscuadra);
}

/** Nivel de una escuadra con esta experiencia acumulada (Doc 5.16.3): sube cada vez que llena lo que pide su nivel,
 * hasta `MILITAR.nivelMaximoEscuadra`. */
export function nivelDeEscuadra(experiencia: number): number {
  let nivel = 1;
  let resto = experiencia;
  for (const pide of MILITAR.experienciaParaSubirEscuadra) {
    if (nivel >= MILITAR.nivelMaximoEscuadra || resto < pide) break;
    resto -= pide;
    nivel++;
  }
  return nivel;
}

/** La escuadra con experiencia ganada, y su nivel al día. */
export function conExperiencia(e: Escuadron, ganada: number): Escuadron {
  // La tropa prestada por un campamento no gana experiencia (D45).
  if (e.prestada) return e;
  const experiencia = e.experiencia + ganada;
  return { ...e, experiencia, nivel: nivelDeEscuadra(experiencia) };
}

/**
 * Los que entran en una batalla que se resuelve con números, con el tope de héroes por bando (Doc 5.15.1): los de
 * más nivel y, a igual nivel, los de escuadras más fuertes. Los demás no combaten.
 */
export function alTopeDeBatalla<H extends { id: string; nivel: number }>(heroes: readonly H[], escuadrasDe: (heroe: H) => readonly Escuadron[], tope: number): H[] {
  const poderDe = (h: H) => escuadrasDe(h).reduce((suma, e) => suma + poderEscuadron(e), 0);
  return [...heroes].sort((a, b) => b.nivel - a.nivel || poderDe(b) - poderDe(a) || (a.id < b.id ? -1 : 1)).slice(0, tope);
}

/** Los héroes que defienden una plaza en persona: residentes que están DENTRO, conectados (D64) y sanos (Doc 5.12.4, 5.16.4). */
export function heroesQueDefienden(asentamiento: Asentamiento, heroes: readonly Heroe[], heridos: ReadonlySet<string>): Heroe[] {
  return heroes.filter(
    (h) =>
      esResidente(asentamiento, h.id) &&
      !heridos.has(h.id) &&
      !h.fuera &&
      h.ubicacion.tipo === 'asentamiento' &&
      h.ubicacion.asentamientoId === asentamiento.id
  );
}

/** Los que de verdad defienden cuando el asedio se resuelve con números: los que defienden en persona, con el tope
 * de héroes por bando (`alTopeDeBatalla`). Son el bando que pierde si la plaza cae. */
export function heroesQueEntranADefender(asentamiento: Asentamiento, heroes: readonly Heroe[], heridos: ReadonlySet<string>): Heroe[] {
  return alTopeDeBatalla(heroesQueDefienden(asentamiento, heroes, heridos), loadoutEnCampamento, BATALLA.capacidad.asedio);
}

function loadoutEnCampamento(h: Heroe): Escuadron[] {
  const ids = new Set(h.loadouts.find((l) => l.activo)?.squadIds ?? []);
  return h.escuadrones.filter((e) => ids.has(e.id) && e.contenedor.tipo === 'campamento');
}

/**
 * Quién defiende una plaza en un asedio que se resuelve con números (Doc 5.12.4): su guarnición, y los residentes que
 * entran a defender (`heroesQueEntranADefender`) con las escuadras de su loadout activo que tengan en el campamento
 * (decisión del usuario 2026-09-14). El loadout de un herido no defiende; la guarnición sí, porque no tiene héroe
 * (Doc 5.16.4). El resto del campamento no defiende. El Liderazgo del loadout ya se comprobó al guardarlo.
 */
export function defensaDe(asentamiento: Asentamiento, heroes: readonly Heroe[], heridos: ReadonlySet<string>): Escuadron[] {
  const enPersona = new Set(heroesQueEntranADefender(asentamiento, heroes, heridos).flatMap((h) => h.loadouts.find((l) => l.activo)?.squadIds ?? []));
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
