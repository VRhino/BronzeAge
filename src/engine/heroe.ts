// El Héroe como personaje (Doc 5.16, doc 01 §12): con qué nace y cómo reparte sus puntos, con las reglas que
// ya tiene Conquest. Sus escuadras viven aparte (`engine/tropa.ts`).
import type { Asentamiento, AtributosHeroe, Heroe, Loadout } from '../domain/types';
import { HEROE } from '../constants';
import { costeLiderazgo, liderazgoComprometido, puedeLlevar } from './liderazgo';
import { cupoGuarnicion } from './asentamientoQuery';

export class HeroeInvalidoError extends Error {}

type Progresion = Pick<
  Heroe,
  | 'nivel'
  | 'experienciaHaciaSiguienteNivel'
  | 'puntosDeAtributoSinGastar'
  | 'puntosDePerkSinGastar'
  | 'atributosBase'
  | 'perksDesbloqueados'
  | 'loadouts'
  | 'inventario'
  | 'equipamiento'
  | 'monedasHeroe'
>;

/** Con qué nace un héroe: como en Conquest (`HeroDataService.CreateNewHero`) —nivel 1, sin puntos, 500 de
 * bronce y un loadout "Default" vacío y activo—, pero sin equipo inicial hasta que exista el catálogo de
 * objetos (decisión del usuario 2026-09-14). */
export function progresionInicial(heroeId: string): Progresion {
  return {
    nivel: 1,
    experienciaHaciaSiguienteNivel: 0,
    puntosDeAtributoSinGastar: 0,
    puntosDePerkSinGastar: 0,
    atributosBase: { fuerza: 0, destreza: 0, armadura: 0, vitalidad: 0 },
    perksDesbloqueados: [],
    loadouts: [{ id: `${heroeId}-loadout-default`, displayName: 'Default', squadIds: [], perksSeleccionados: [], activo: true }],
    inventario: [],
    equipamiento: { arma: null, casco: null, torso: null, guantes: null, pantalones: null, botas: null },
    monedasHeroe: { bronce: HEROE.bronceInicial, plata: 0, oro: 0 },
  };
}

/** Reparte puntos de atributo: cada punto suma 1, sin pasar del tope (`HeroAttributeValidator` de Conquest). */
export function repartirAtributos(heroe: Heroe, reparto: Partial<AtributosHeroe>): Heroe {
  const entradas = Object.entries(reparto) as [keyof AtributosHeroe, number][];
  if (entradas.some(([, puntos]) => !Number.isInteger(puntos) || puntos < 0)) {
    throw new HeroeInvalidoError('Los puntos se reparten en cantidades enteras, sin negativos.');
  }
  const total = entradas.reduce((suma, [, puntos]) => suma + puntos, 0);
  if (total === 0) throw new HeroeInvalidoError('No hay ningún punto que repartir.');
  if (total > heroe.puntosDeAtributoSinGastar) {
    throw new HeroeInvalidoError(`Solo tienes ${heroe.puntosDeAtributoSinGastar} puntos de atributo sin gastar.`);
  }
  const atributosBase = { ...heroe.atributosBase };
  for (const [atributo, puntos] of entradas) {
    atributosBase[atributo] += puntos;
    if (atributosBase[atributo] > HEROE.topeAtributo) throw new HeroeInvalidoError(`${atributo} no puede pasar de ${HEROE.topeAtributo}.`);
  }
  return { ...heroe, atributosBase, puntosDeAtributoSinGastar: heroe.puntosDeAtributoSinGastar - total };
}

/** Liderazgo que suman las escuadras de un loadout (Doc 5.11). Se deriva al servir, nunca se guarda (doc 01 §14). */
export function liderazgoDeLoadout(heroe: Heroe, loadout: Loadout): number {
  const suyas = new Map(heroe.escuadrones.map((e) => [e.id, e]));
  return liderazgoComprometido(loadout.squadIds.flatMap((id) => suyas.get(id) ?? []));
}

export interface DatosLoadout {
  /** Ausente = uno nuevo. */
  id?: string;
  displayName: string;
  squadIds: string[];
  perksSeleccionados: number[];
  /** Ausente = el que tenía, o inactivo si es nuevo. */
  activo?: boolean;
}

/**
 * Guarda o reescribe un loadout (Doc 5.16.5): escuadras suyas que quepan en su Liderazgo y perks que ya tenga.
 * Es una comodidad, así que no mira dónde están las escuadras: eso se comprueba al salir con ellas. Marcarlo
 * activo desactiva los demás, como en Conquest.
 */
export function guardarLoadout(heroe: Heroe, datos: DatosLoadout, nuevoId: () => string): Heroe {
  const displayName = datos.displayName.trim();
  if (!displayName) throw new HeroeInvalidoError('El loadout necesita un nombre.');
  const anterior = datos.id === undefined ? undefined : heroe.loadouts.find((l) => l.id === datos.id);
  if (datos.id !== undefined && !anterior) throw new HeroeInvalidoError(`El loadout ${datos.id} no existe.`);
  if (new Set(datos.squadIds).size !== datos.squadIds.length) throw new HeroeInvalidoError('Hay una escuadra repetida.');
  const suyas = new Map(heroe.escuadrones.map((e) => [e.id, e]));
  const ajena = datos.squadIds.find((id) => !suyas.has(id));
  if (ajena) throw new HeroeInvalidoError(`La escuadra ${ajena} no es tuya.`);
  if (!puedeLlevar(heroe, datos.squadIds.map((id) => suyas.get(id)!))) throw new HeroeInvalidoError('Estas escuadras superan tu Liderazgo.');
  const perkAjeno = datos.perksSeleccionados.find((p) => !heroe.perksDesbloqueados.includes(p));
  if (perkAjeno !== undefined) throw new HeroeInvalidoError(`No tienes el perk ${perkAjeno}.`);

  const loadout: Loadout = {
    id: anterior?.id ?? nuevoId(),
    displayName,
    squadIds: [...datos.squadIds],
    perksSeleccionados: [...datos.perksSeleccionados],
    activo: datos.activo ?? anterior?.activo ?? false,
  };
  const otros = (l: Loadout): Loadout => (loadout.activo && l.activo ? { ...l, activo: false } : l);
  const loadouts = anterior
    ? heroe.loadouts.map((l) => (l.id === loadout.id ? loadout : otros(l)))
    : [...heroe.loadouts.map(otros), loadout];
  return { ...heroe, loadouts };
}

/** Cupo de guarnición que ya ocupa un héroe (Doc 5.15.3), en la escala del coste de Liderazgo. */
export function guarnicionOcupada(heroe: Heroe): number {
  return liderazgoComprometido(heroe.escuadrones.filter((e) => e.enGuarnicion));
}

const conGuarnicion = (heroe: Heroe, squadId: string, enGuarnicion: boolean): Heroe => ({
  ...heroe,
  escuadrones: heroe.escuadrones.map((e) => (e.id === squadId ? { ...e, enGuarnicion } : e)),
});

/**
 * Entrega una escuadra de su campamento a la guarnición de la plaza donde reside (Doc 5.15.3), dentro del cupo que
 * esa plaza le da. Si el cupo baja después, lo asignado se queda: solo impide asignar más (decisión del usuario
 * 2026-09-14).
 */
export function asignarGuarnicion(heroe: Heroe, residencia: Asentamiento | undefined, squadId: string): Heroe {
  if (!residencia) throw new HeroeInvalidoError('No resides en ningún asentamiento: no tienes guarnición.');
  const escuadra = heroe.escuadrones.find((e) => e.id === squadId);
  if (!escuadra) throw new HeroeInvalidoError(`La escuadra ${squadId} no es tuya.`);
  if (escuadra.contenedor.tipo !== 'campamento') throw new HeroeInvalidoError('Esa escuadra no está en tu campamento.');
  if (escuadra.enGuarnicion) throw new HeroeInvalidoError('Esa escuadra ya está en la guarnición.');
  const cupo = cupoGuarnicion(residencia);
  const ocupado = guarnicionOcupada(heroe) + costeLiderazgo(escuadra.tropaId);
  if (ocupado > cupo) throw new HeroeInvalidoError(`Supera tu cupo de guarnición en ${residencia.id}: ${ocupado} de ${cupo}.`);
  return conGuarnicion(heroe, squadId, true);
}

export function retirarGuarnicion(heroe: Heroe, squadId: string): Heroe {
  if (!heroe.escuadrones.find((e) => e.id === squadId)?.enGuarnicion) throw new HeroeInvalidoError('Esa escuadra no está en la guarnición.');
  return conGuarnicion(heroe, squadId, false);
}

export function borrarLoadout(heroe: Heroe, loadoutId: string): Heroe {
  if (!heroe.loadouts.some((l) => l.id === loadoutId)) throw new HeroeInvalidoError(`El loadout ${loadoutId} no existe.`);
  return { ...heroe, loadouts: heroe.loadouts.filter((l) => l.id !== loadoutId) };
}
