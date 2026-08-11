import type { Asentamiento, CargoTipo, Faccion, PoliticaActiva } from '../domain/types';
import { POLITICAS, POLITICA_CATALOGO } from '../constants';

export class PoliticaInvalidaError extends Error {}

type PoliticaDef = (typeof POLITICA_CATALOGO)[number];

function definicion(politicaId: string): PoliticaDef {
  const def = POLITICA_CATALOGO.find((p) => p.id === politicaId);
  if (!def) throw new PoliticaInvalidaError('La política no existe en el catálogo.');
  return def;
}

/** Gobernador escala de 2 a 5 slots según nivel de Facción (Doc 4.4); el resto de cargos tienen slots fijos. */
export function slotsDisponibles(cargo: CargoTipo, nivelFaccion: number): number {
  const cfg = POLITICAS.slotsPorCargo[cargo];
  if (cargo !== 'gobernador') return cfg.base;
  const extra = Math.floor(nivelFaccion / POLITICAS.nivelFaccionPorSlotExtraGobernador);
  return Math.min(cfg.maximo, cfg.base + extra);
}

function activasPorCargo(asentamiento: Asentamiento, cargo: CargoTipo): PoliticaActiva[] {
  return asentamiento.politicasActivas.filter((p) => p.cargo === cargo);
}

const CAMPO_CARGO: Record<CargoTipo, keyof Asentamiento['cargos']> = {
  gobernador: 'gobernadorId',
  tesorero: 'tesoreroId',
  general: 'generalId',
  maestroObras: 'maestroObrasId',
  sacerdote: 'sacerdoteId',
};

/**
 * Activa una política (Doc 4.4): duración fija, no cancelable antes de tiempo, respeta slots por cargo.
 * El Gobernador tiene pool COMPLETA (cualquier política); el resto solo las de su propio pool.
 */
export function activarPolitica(
  asentamiento: Asentamiento,
  faccion: Faccion,
  cargo: CargoTipo,
  politicaId: string,
  tickActual: number,
  contador = 0
): Asentamiento {
  const def = definicion(politicaId);
  if (cargo !== 'gobernador' && def.cargo !== cargo) {
    throw new PoliticaInvalidaError(`"${def.nombre}" no pertenece al pool de este cargo.`);
  }
  if (!asentamiento.cargos[CAMPO_CARGO[cargo]]) {
    throw new PoliticaInvalidaError('El cargo debe estar ocupado para activar una política en su nombre.');
  }
  if (activasPorCargo(asentamiento, cargo).some((p) => p.politicaId === politicaId)) {
    throw new PoliticaInvalidaError('Esa política ya está activa para este cargo.');
  }
  const limite = slotsDisponibles(cargo, faccion.nivel);
  if (activasPorCargo(asentamiento, cargo).length >= limite) {
    throw new PoliticaInvalidaError(`Sin slots libres para ${cargo} (${limite} máximo con el nivel actual de Facción).`);
  }

  const nueva: PoliticaActiva = {
    id: `politica-${asentamiento.id}-${tickActual}-${contador}`,
    politicaId,
    cargo,
    activadaEnTick: tickActual,
    expiraEnTick: tickActual + POLITICAS.duracionTicksPorDefecto,
  };
  return { ...asentamiento, politicasActivas: [...asentamiento.politicasActivas, nueva] };
}

/** Expira políticas cuyo plazo terminó; no hay cancelación anticipada (Doc 4.4). */
export function avanzarPoliticas(asentamiento: Asentamiento, tickActual: number): { asentamiento: Asentamiento; eventos: string[] } {
  const eventos: string[] = [];
  const vigentes = asentamiento.politicasActivas.filter((p) => {
    const expirada = tickActual >= p.expiraEnTick;
    if (expirada) eventos.push(`Política "${definicion(p.politicaId).nombre}" expira.`);
    return !expirada;
  });
  return { asentamiento: vigentes.length === asentamiento.politicasActivas.length ? asentamiento : { ...asentamiento, politicasActivas: vigentes }, eventos };
}

type CampoFactor =
  | 'factorConsumoComida'
  | 'factorCrecimientoNobleza'
  | 'factorTiempoConstruccion'
  | 'factorComisionExterna'
  | 'factorCostoReclutamiento'
  | 'factorProduccionTrigo'
  | 'factorCapacidadCaravana'
  | 'factorVelocidadCaravana';

function productoFactor(asentamiento: Asentamiento, campo: CampoFactor): number {
  return asentamiento.politicasActivas.reduce((acc, activa) => {
    const def = POLITICA_CATALOGO.find((p) => p.id === activa.politicaId);
    const valor = def ? (def as Record<string, unknown>)[campo] : undefined;
    return typeof valor === 'number' ? acc * valor : acc;
  }, 1);
}

export const factorConsumoComida = (a: Asentamiento): number => productoFactor(a, 'factorConsumoComida');
export const factorCrecimientoNobleza = (a: Asentamiento): number => productoFactor(a, 'factorCrecimientoNobleza');
export const factorTiempoConstruccion = (a: Asentamiento): number => productoFactor(a, 'factorTiempoConstruccion');
export const factorComisionExterna = (a: Asentamiento): number => productoFactor(a, 'factorComisionExterna');
export const factorCostoReclutamiento = (a: Asentamiento): number => productoFactor(a, 'factorCostoReclutamiento');
/** Edicto de Cosecha (Gobernador): multiplica la producción de trigo de todas las Granjas — madera/piedra sin cambios. */
export const factorProduccionTrigo = (a: Asentamiento): number => productoFactor(a, 'factorProduccionTrigo');
/** "Carga Ampliada" (Tesorero): multiplica la capacidad de carga de las caravanas propias del asentamiento. */
export const factorCapacidadCaravana = (a: Asentamiento): number => productoFactor(a, 'factorCapacidadCaravana');
/** "Rutas Rápidas" (Tesorero): multiplica la velocidad de las caravanas propias del asentamiento. */
export const factorVelocidadCaravana = (a: Asentamiento): number => productoFactor(a, 'factorVelocidadCaravana');

/**
 * Campos ADITIVOS (a diferencia de `productoFactor`, que multiplica): suman lo que aporte cada política
 * activa en vez de escalar un factor base. Sirve para bonos de cupo tipo "+N" en vez de "×N" (Doc 4.4).
 */
function sumaFactorPolitica(asentamiento: Asentamiento, campo: string): number {
  return asentamiento.politicasActivas.reduce((acc, activa) => {
    const def = POLITICA_CATALOGO.find((p) => p.id === activa.politicaId);
    const valor = def ? (def as Record<string, unknown>)[campo] : undefined;
    return typeof valor === 'number' ? acc + valor : acc;
  }, 0);
}

/** "Ampliación de Flota" (Tesorero): cupo extra de caravanas propias, sumado al que ya da el nivel de Mercado. */
export const cupoCaravanaExtra = (a: Asentamiento): number => sumaFactorPolitica(a, 'cupoCaravanaExtra');

/**
 * Campos "objetivo" (no multiplicativos): en vez de multiplicar factores, toman el mayor valor propuesto
 * por cualquier política activa. Sirve para políticas como "Protección de Riesgos" (Doc 4.2/4.4 — no es
 * un factor de tasa, es un mínimo a alcanzar antes de permitir cualquier otra auto-construcción).
 */
function valorMaximoPolitica(asentamiento: Asentamiento, campo: 'minimoLenerasPrioritario' | 'minimoGranjasPrioritario'): number {
  return asentamiento.politicasActivas.reduce((max, activa) => {
    const def = POLITICA_CATALOGO.find((p) => p.id === activa.politicaId);
    const valor = def ? (def as Record<string, unknown>)[campo] : undefined;
    return typeof valor === 'number' && valor > max ? valor : max;
  }, 0);
}

/** >0 si el Maestro de Obras activó "Protección de Riesgos": mínimo de Leñeras a priorizar sobre cualquier otra necesidad. */
export const minimoLenerasPrioritario = (a: Asentamiento): number => valorMaximoPolitica(a, 'minimoLenerasPrioritario');
/** >0 si el Maestro de Obras activó "Protección de Riesgos": mínimo de Granjas a priorizar sobre cualquier otra necesidad. */
export const minimoGranjasPrioritario = (a: Asentamiento): number => valorMaximoPolitica(a, 'minimoGranjasPrioritario');
