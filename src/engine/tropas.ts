import type { Asentamiento, Escuadron, OrigenTropa } from '../domain/types';
import { ASCENSO_TROPA, MILITAR, RECLUTAMIENTO, TROPA_CATALOGO, TROPAS_RECLUTABLES } from '../constants';
import { descontarRecursos, tieneRecursos } from './almacen';
import { edificiosPorTipoYEstado, poblacionTotal } from './asentamientoQuery';
import { factorCostoReclutamiento } from './politicas';

export class ReclutamientoInvalidoError extends Error {}

/**
 * Reclutamiento (Doc 4.1/5.7): requiere General designado (mando militar, Doc 2.2). Artesanos nacen Tier 2
 * directo; suben de tier veteranizando en combate (ver ASCENSO_TROPA). Nobleza nace Tier 4 (progresión plana,
 * conversión instantánea) y exige Gran Fundición activa. Pesants ya NO recluta aquí — ver `reclutarTropa`
 * (reclutamiento por equipo vía Barracón/Galería de tiro, rediseño Doc 5.7/5.8).
 */
export function reclutar(
  asentamiento: Asentamiento,
  origen: Exclude<OrigenTropa, 'pesants'>,
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

/**
 * Reclutamiento por equipo (Doc 5.7/5.8, rediseño): recluta una tropa específica vía Barracón/Galería de
 * tiro, según el nivel interno del edificio, pagando el equipo fabricado en Armería en vez de cobre directo.
 * Siempre origen Pesants, tier 1 fijo — "mejorar" no es ascenso automático por veteranía (ver
 * `ascenderTierSiCorresponde`), es reclutar una tropa mejor cuando el edificio suba de nivel interno.
 */
export function reclutarTropa(
  asentamiento: Asentamiento,
  tropaId: string,
  cantidad: number,
  tickActual: number,
  contador = 0
): Asentamiento {
  if (!asentamiento.cargos.generalId) {
    throw new ReclutamientoInvalidoError('El asentamiento necesita un General para reclutar tropas.');
  }
  if (cantidad <= 0) throw new ReclutamientoInvalidoError('La cantidad debe ser mayor que 0.');
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) throw new ReclutamientoInvalidoError('La tropa no existe en el catálogo.');
  if (asentamiento.poblacion.pesants < cantidad) {
    throw new ReclutamientoInvalidoError('No hay suficientes pesants disponibles.');
  }

  const edificio = edificiosPorTipoYEstado(asentamiento, tropa.edificio)[0];
  if (!edificio || (edificio.nivelInterno ?? 1) < tropa.nivelRequerido) {
    throw new ReclutamientoInvalidoError(
      `Se necesita ${tropa.edificio} activo en nivel interno ${tropa.nivelRequerido} para reclutar "${tropa.nombre}".`
    );
  }

  const factorCosto = factorCostoReclutamiento(asentamiento);
  const costoTotal = Object.fromEntries(
    Object.entries(tropa.costoEquipo).map(([recurso, cantidadUnitaria]) => [recurso, (cantidadUnitaria ?? 0) * cantidad * factorCosto])
  );
  if (!tieneRecursos(asentamiento.almacen, costoTotal)) {
    throw new ReclutamientoInvalidoError('No hay equipo suficiente para reclutar esta tropa.');
  }

  const existente = asentamiento.escuadrones.find((e) => e.tropaId === tropaId);
  const escuadrones = existente
    ? asentamiento.escuadrones.map((e) => (e.id === existente.id ? { ...e, cantidad: e.cantidad + cantidad } : e))
    : [
        ...asentamiento.escuadrones,
        {
          id: `escuadron-${asentamiento.id}-${tickActual}-${contador}`,
          nombre: `${tropa.nombre} de ${asentamiento.id}`,
          origen: 'pesants' as const,
          tier: 1 as const,
          cantidad,
          veterania: 0,
          moral: 100,
          tropaId,
        } satisfies Escuadron,
      ];

  return {
    ...asentamiento,
    poblacion: { ...asentamiento.poblacion, pesants: asentamiento.poblacion.pesants - cantidad },
    almacen: descontarRecursos(asentamiento.almacen, costoTotal),
    escuadrones,
  };
}

/** Asciende de tier por veteranía (carril combate real, Doc 4.1/5.5); Nobleza no aplica (progresión plana).
 * Tropas de equipo (Barracón/Galería, `tropaId` presente) tampoco ascienden así — "mejorar" es reclutar una
 * tropa mejor cuando el edificio suba de nivel interno (Doc 5.8 PENDIENTE, resuelto en el rediseño). */
export function ascenderTierSiCorresponde(escuadron: Escuadron, fundicionActiva: boolean): Escuadron {
  if (escuadron.origen === 'nobleza' || escuadron.tropaId) return escuadron;
  if (escuadron.tier === 1 && escuadron.veterania >= ASCENSO_TROPA.veteraniaParaTier2) {
    return { ...escuadron, tier: 2 };
  }
  if (escuadron.tier === 2 && escuadron.veterania >= ASCENSO_TROPA.veteraniaParaTier3 && fundicionActiva) {
    return { ...escuadron, tier: 3 };
  }
  return escuadron;
}

/** Ración total de trigo/tick que exigen los escuadrones activos (Doc 5.4) — usada tanto para descontarla
 * aquí como para el "apartado de trigo" mostrado en Mantenimiento (ver `gameStore.mantenimientoInfo`). */
export function consumoRacionTropas(asentamiento: Asentamiento): number {
  const totalSoldados = asentamiento.escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
  return totalSoldados * MILITAR.racionPorSoldadoPorTick;
}

/** Mantenimiento (Doc 5.4): consumo de raciones; sin suministro la moral colapsa y desertan permanentemente. */
export function avanzarMantenimientoTropas(asentamiento: Asentamiento): { asentamiento: Asentamiento; eventos: string[] } {
  if (asentamiento.escuadrones.length === 0) return { asentamiento, eventos: [] };
  const eventos: string[] = [];

  const racionNecesaria = consumoRacionTropas(asentamiento);
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
