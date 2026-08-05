import type { Asentamiento, Escuadron, OrigenTropa } from '../domain/types';
import { ASCENSO_TROPA, MILITAR, RECLUTAMIENTO, TROPA_CATALOGO } from '../constants';
import { descontarRecursos, tieneRecursos } from './almacen';
import { edificiosPorTipoYEstado, poblacionTotal } from './asentamientoQuery';
import { factorCostoReclutamiento } from './politicas';

export class ReclutamientoInvalidoError extends Error {}

/**
 * Reclutamiento (Doc 4.1/5.7): requiere General designado (mando militar, Doc 2.2). Los Pesants nacen
 * Tier 1 y los Artesanos Tier 2 directo; suben de tier veteranizando en combate (ver ASCENSO_TROPA).
 * Nobleza nace Tier 4 (progresión plana, conversión instantánea) y exige Gran Fundición activa.
 */
export function reclutar(
  asentamiento: Asentamiento,
  origen: OrigenTropa,
  cantidad: number,
  tickActual: number,
  contador = 0
): Asentamiento {
  if (!asentamiento.cargos.generalId) {
    throw new ReclutamientoInvalidoError('El asentamiento necesita un General para reclutar tropas.');
  }
  if (cantidad <= 0) throw new ReclutamientoInvalidoError('La cantidad debe ser mayor que 0.');
  if (asentamiento.poblacion[origen] < cantidad) {
    throw new ReclutamientoInvalidoError(`No hay suficientes ${origen} disponibles.`);
  }

  const config = RECLUTAMIENTO[origen];
  if ('requiereEdificio' in config && config.requiereEdificio) {
    if (edificiosPorTipoYEstado(asentamiento, config.requiereEdificio).length === 0) {
      throw new ReclutamientoInvalidoError(`Se necesita ${config.requiereEdificio} activa para reclutar de ${origen}.`);
    }
  }

  const factorCosto = factorCostoReclutamiento(asentamiento);
  const costoTotal = Object.fromEntries(
    Object.entries(config.costo).map(([recurso, cantidadUnitaria]) => [recurso, cantidadUnitaria * cantidad * factorCosto])
  );
  if (!tieneRecursos(asentamiento.almacen, costoTotal)) {
    throw new ReclutamientoInvalidoError('No hay materiales suficientes para el reclutamiento.');
  }

  const tier = config.tierInicial;
  const existente = asentamiento.escuadrones.find((e) => e.origen === origen && e.tier === tier);
  const escuadrones = existente
    ? asentamiento.escuadrones.map((e) => (e.id === existente.id ? { ...e, cantidad: e.cantidad + cantidad } : e))
    : [
        ...asentamiento.escuadrones,
        {
          id: `escuadron-${asentamiento.id}-${tickActual}-${contador}`,
          nombre: `${TROPA_CATALOGO[tier]!.nombre} de ${asentamiento.id}`,
          origen,
          tier,
          cantidad,
          veterania: 0,
          moral: 100,
        } satisfies Escuadron,
      ];

  return {
    ...asentamiento,
    poblacion: { ...asentamiento.poblacion, [origen]: asentamiento.poblacion[origen] - cantidad },
    almacen: descontarRecursos(asentamiento.almacen, costoTotal),
    escuadrones,
  };
}

/** Asciende de tier por veteranía (carril combate real, Doc 4.1/5.5); Nobleza no aplica (progresión plana). */
export function ascenderTierSiCorresponde(escuadron: Escuadron, fundicionActiva: boolean): Escuadron {
  if (escuadron.origen === 'nobleza') return escuadron;
  if (escuadron.tier === 1 && escuadron.veterania >= ASCENSO_TROPA.veteraniaParaTier2) {
    return { ...escuadron, tier: 2 };
  }
  if (escuadron.tier === 2 && escuadron.veterania >= ASCENSO_TROPA.veteraniaParaTier3 && fundicionActiva) {
    return { ...escuadron, tier: 3 };
  }
  return escuadron;
}

/** Mantenimiento (Doc 5.4): consumo de raciones; sin suministro la moral colapsa y desertan permanentemente. */
export function avanzarMantenimientoTropas(asentamiento: Asentamiento): { asentamiento: Asentamiento; eventos: string[] } {
  if (asentamiento.escuadrones.length === 0) return { asentamiento, eventos: [] };
  const eventos: string[] = [];

  const totalSoldados = asentamiento.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
  const racionNecesaria = totalSoldados * MILITAR.racionPorSoldadoPorTick;
  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const factorSuministro = racionNecesaria > 0 ? Math.min(1, trigoDisponible / racionNecesaria) : 1;
  const almacen = descontarRecursos(asentamiento.almacen, { trigo: Math.min(trigoDisponible, racionNecesaria) });

  // El squad (nombre, veteranía) persiste aunque `cantidad` llegue a 0 (Doc 5.4) — se puede rellenar reclutando.
  const escuadrones = asentamiento.escuadrones.map((e) => {
    let moral = e.moral;
    if (factorSuministro >= 1) {
      moral = Math.min(100, moral + MILITAR.regeneracionMoralPorTick);
    } else {
      moral = Math.max(0, moral - MILITAR.degradacionMoralSinRacion * (1 - factorSuministro));
    }
    let cantidad = e.cantidad;
    if (moral <= 0 && cantidad > 0) {
      const desertores = Math.min(cantidad, Math.ceil(cantidad * MILITAR.desercionFraccionPorTickSinMoral));
      cantidad -= desertores;
      if (desertores > 0) eventos.push(`${e.nombre}: ${desertores} desertan por hambre (moral colapsada).`);
    }
    return { ...e, moral, cantidad };
  });

  return { asentamiento: { ...asentamiento, almacen, escuadrones }, eventos };
}

export function poblacionTotalConTropas(asentamiento: Asentamiento): number {
  return poblacionTotal(asentamiento) + asentamiento.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
}
