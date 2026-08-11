import type { Asentamiento, Escuadron } from '../domain/types';
import { MILITAR, TROPAS_RECLUTABLES } from '../constants';
import { descontarRecursos, tieneRecursos } from './almacen';
import { edificiosPorTipoYEstado, poblacionTotal } from './asentamientoQuery';
import { factorCostoReclutamiento } from './politicas';

export class ReclutamientoInvalidoError extends Error {}

/**
 * Reclutamiento por equipo (Doc 5.7/5.8): recluta una tropa específica vía Barracón/Galería de tiro, según
 * el nivel interno del edificio, pagando el equipo fabricado en Armería en vez de cobre directo. Pesants Y
 * Artesanos pueden reclutar por este carril (a petición del usuario, ambos edificios reclutan de los dos
 * pools — reemplaza el antiguo reclutamiento directo de Artesanos con cobre a secas, y el de Nobleza vía Gran
 * Fundición, ambos retirados). Una tropa NUNCA cambia de identidad (Doc 5.8, a petición del usuario): "mejorar"
 * no es ascenso automático del mismo escuadrón, es reclutar una tropa DISTINTA y mejor cuando el edificio suba
 * de nivel interno — ver `poderEscuadron` en engine/combate.ts, que aplica el bonus de veteranía sin tocar
 * nunca `tropaId`.
 *
 * Escuadrón de UN jugador, no del asentamiento (Doc 2.5, a petición del usuario — corrige el bug donde dos
 * jugadores reclutando la misma tropa en el mismo asentamiento se fundían en un solo escuadrón compartido):
 * cada jugador residente (fundador o con casa comprada, ver `Asentamiento.jugadoresFundadoresIds`/
 * `casasCompradas`) tiene como mucho UN escuadrón por `tropaId`, tope `tropa.unidadesPorDefecto`. Reclutar ya
 * no es un gate del cargo de General (Doc 2.2 vs 2.5 — 2.5 ganó la ambigüedad: reclutar es beneficio de
 * ciudadanía/residencia, no de cargo): cualquier residente puede reclutar o reponer SU propio escuadrón.
 * "Reponer bajas" no es un mecanismo aparte: si el jugador ya tiene el escuadrón por debajo del tope, reclutar
 * de nuevo paga y añade solo las unidades que faltan hasta el tope (mismo costo por soldado que reclutar desde
 * cero) en vez de sumar otro bloque completo.
 */
export function reclutarTropa(
  asentamiento: Asentamiento,
  jugadorId: string,
  tropaId: string,
  origen: 'pesants' | 'artesanos',
  tickActual: number,
  contador = 0
): Asentamiento {
  const esResidente = asentamiento.jugadoresFundadoresIds.includes(jugadorId) || asentamiento.casasCompradas.includes(jugadorId);
  if (!esResidente) {
    throw new ReclutamientoInvalidoError('Solo un jugador residente de este asentamiento puede reclutar aquí.');
  }
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) throw new ReclutamientoInvalidoError('La tropa no existe en el catálogo.');

  const existente = asentamiento.escuadrones.find((e) => e.jugadorId === jugadorId && e.tropaId === tropaId);
  const cantidad = tropa.unidadesPorDefecto - (existente?.cantidad ?? 0);
  if (cantidad <= 0) {
    throw new ReclutamientoInvalidoError('Este escuadrón ya está al tope de unidades.');
  }
  if (asentamiento.poblacion[origen] < cantidad) {
    throw new ReclutamientoInvalidoError(`No hay suficientes ${origen} disponibles (hacen falta ${cantidad}).`);
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

  const escuadrones = existente
    ? asentamiento.escuadrones.map((e) => (e.id === existente.id ? { ...e, cantidad: e.cantidad + cantidad } : e))
    : [
        ...asentamiento.escuadrones,
        {
          id: `escuadron-${asentamiento.id}-${tickActual}-${contador}`,
          nombre: `${tropa.nombre} de ${jugadorId}`,
          jugadorId,
          origen,
          cantidad,
          veterania: 0,
          moral: 100,
          tropaId,
        } satisfies Escuadron,
      ];

  return {
    ...asentamiento,
    poblacion: { ...asentamiento.poblacion, [origen]: asentamiento.poblacion[origen] - cantidad },
    almacen: descontarRecursos(asentamiento.almacen, costoTotal),
    escuadrones,
  };
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
