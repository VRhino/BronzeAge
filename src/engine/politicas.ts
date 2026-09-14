import type { Asentamiento, CargoTipo, Faccion, PoliticaActiva } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { POLITICAS, POLITICA_CATALOGO, type PerfilTrazado } from '../constants';
import { cargoOcupado } from './pertenencia';

/** Fase A5 — payload de `politica.expirada` (ver `avanzarPoliticas`). */
export interface PayloadPoliticaExpirada {
  politicaId: string;
  politicaNombre: string;
  cargo: CargoTipo;
}

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

/**
 * Activa una política (Doc 4.4): duración fija, no cancelable antes de tiempo, respeta slots por cargo.
 * El Gobernador tiene pool COMPLETA (cualquier política); el resto solo las de su propio pool.
 */
export function activarPolitica(
  asentamiento: Asentamiento,
  faccion: Faccion,
  cargo: CargoTipo,
  politicaId: string,
  instante: Instante,
  contador = 0
): Asentamiento {
  const def = definicion(politicaId);
  if (cargo !== 'gobernador' && def.cargo !== cargo) {
    throw new PoliticaInvalidaError(`"${def.nombre}" no pertenece al pool de este cargo.`);
  }
  if (!cargoOcupado(asentamiento, cargo)) {
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
    id: `politica-${asentamiento.id}-${contador}`,
    politicaId,
    cargo,
    activadaEn: instante,
    expiraEn: sumar(instante, minutos(POLITICAS.duracionMinutosPorDefecto)),
  };
  return { ...asentamiento, politicasActivas: [...asentamiento.politicasActivas, nueva] };
}

/** Expira políticas cuyo plazo terminó; no hay cancelación anticipada (Doc 4.4). */
export function avanzarPoliticas(asentamiento: Asentamiento, instante: Instante): { asentamiento: Asentamiento; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const vigentes = asentamiento.politicasActivas.filter((p) => {
    const expirada = instante >= p.expiraEn;
    if (expirada) {
      const def = definicion(p.politicaId);
      eventos.push({
        codigo: 'politica.expirada',
        mensaje: `Política "${def.nombre}" expira.`,
        payload: { politicaId: p.politicaId, politicaNombre: def.nombre, cargo: p.cargo } satisfies PayloadPoliticaExpirada,
      });
    }
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
  | 'factorVelocidadCaravana'
  | 'factorRecaudacion'
  | 'factorCrecimientoPoblacion';

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
/** "Levas de guarnición" (General, Doc 5.15.3): cupo extra de guarnición por héroe residente. */
export const cupoGuarnicionExtra = (a: Asentamiento): number => sumaFactorPolitica(a, 'cupoGuarnicionExtra');

/** "Presión Fiscal" (Tesorero, bloque "economía del oro"): multiplica la recaudación de oro por población
 * (`recaudacionOro`, engine/population.ts). 1 si no hay ninguna activa. */
export const factorRecaudacion = (a: Asentamiento): number => productoFactor(a, 'factorRecaudacion');
/** "Presión Fiscal" (Tesorero): frena el crecimiento de las 3 clases de población — es el downside de subir
 * impuestos, aplicado en `crecerPoblacion` (engine/population.ts) mientras no exista un medidor de felicidad. */
export const factorCrecimientoPoblacion = (a: Asentamiento): number => productoFactor(a, 'factorCrecimientoPoblacion');

/** Campos FLAG (a diferencia de `productoFactor`/`sumaFactorPolitica`/`valorMaximoPolitica`): true si CUALQUIER
 * política activa lo declara `true`, sin escalar ni sumar nada — sirve para políticas de tipo interruptor. */
function algunaPoliticaActiva(asentamiento: Asentamiento, campo: string): boolean {
  return asentamiento.politicasActivas.some((activa) => {
    const def = POLITICA_CATALOGO.find((p) => p.id === activa.politicaId);
    return def ? (def as Record<string, unknown>)[campo] === true : false;
  });
}

/** True si el Maestro de Obras activó "Líneas de Producción": los edificios de transformación nuevos se
 * sitúan cerca de la fuente de sus insumos en vez del primer hueco libre (ver `sitioConcentricoLineaProduccion`,
 * engine/construction.ts). */
export const lineasProduccionPriorizadas = (a: Asentamiento): boolean => algunaPoliticaActiva(a, 'lineasProduccionPriorizadas');

/**
 * Perfil de trazado impuesto por una ordenanza activa del Maestro de Obras (doc trazado §E6.23), o `null` si
 * no hay ninguna — en cuyo caso el asentamiento usa su tradición local (`resolverPerfil`, engine/trazado.ts).
 *
 * Devuelve el PRIMERO que encuentre, no un producto ni una suma: un perfil es una elección discreta, no un
 * factor. En la práctica nunca hay dos, porque las cuatro ordenanzas viven en el único slot de `maestroObras`
 * — pero el Gobernador tiene pool completa y varios slots, así que la ambigüedad es alcanzable y conviene
 * resolverla de forma determinista (orden del catálogo) en vez de dejarla al azar del array.
 */
export function perfilTrazadoDePolitica(asentamiento: Partial<Pick<Asentamiento, 'politicasActivas'>>): PerfilTrazado | null {
  const activas = asentamiento.politicasActivas;
  if (!activas || activas.length === 0) return null;
  for (const def of POLITICA_CATALOGO) {
    const perfil = (def as { perfilTrazado?: PerfilTrazado }).perfilTrazado;
    if (perfil && activas.some((a) => a.politicaId === def.id)) return perfil;
  }
  return null;
}
