import type { Asentamiento, Poblacion } from '../domain/types';
import { POBLACION } from '../constants';
import { capacidadArtesanos, capacidadHabitacional, edificiosPorTipoYEstado, poblacionTotal } from './asentamientoQuery';
import { factorConsumoComida, factorCrecimientoNobleza } from './politicas';

/** Incremento entero esperado = actual*tasa, con redondeo estocástico para no estancarse con poblaciones pequeñas. */
function crecimientoEstocastico(actual: number, tasa: number): number {
  const esperado = actual * tasa;
  const base = Math.floor(esperado);
  const resto = esperado - base;
  return base + (Math.random() < resto ? 1 : 0);
}

/**
 * Fórmula base de Pesants (Doc 4.1): comida disponible + vivienda disponible + estabilidad + felicidad.
 * Estabilidad y felicidad no están modeladas aún (Sprint 4+ políticas/Sacerdote) — se usan como 1 (neutro).
 */
export function crecerPoblacion(asentamiento: Asentamiento): { poblacion: Poblacion; eventos: string[] } {
  const eventos: string[] = [];
  const capacidadVivienda = capacidadHabitacional(asentamiento);
  const totalActual = poblacionTotal(asentamiento);
  const espacioLibreFactor = capacidadVivienda <= 0 ? 0 : Math.max(0, Math.min(1, 1 - totalActual / capacidadVivienda));

  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const comidaFactor = trigoDisponible > 0 ? 1 : 0.2;
  const estabilidad = 1;
  const felicidad = 1;

  const tasaPesants = POBLACION.pesants.tasaCrecimientoBase * comidaFactor * espacioLibreFactor * estabilidad * felicidad;
  const nuevosPesants = crecimientoEstocastico(asentamiento.poblacion.pesants, tasaPesants);

  // Rediseño de progreso (Fase 0, Doc 4.1/4.2.1): el tope ya no depende de un edificio genérico "Taller" —
  // es la suma de trabajadoresRequeridos de los edificios de transformación activos (Curtiduría/Armería/
  // Fundición/Carpintería, según su nivel interno). Sin ninguno activo, no crecen artesanos nuevos.
  const capacidadArtesanosActual = capacidadArtesanos(asentamiento);
  let nuevosArtesanos = 0;
  if (capacidadArtesanosActual > asentamiento.poblacion.artesanos && espacioLibreFactor > 0) {
    nuevosArtesanos = Math.max(
      1,
      crecimientoEstocastico(capacidadArtesanosActual - asentamiento.poblacion.artesanos, POBLACION.artesanos.tasaCrecimientoBase)
    );
    if (nuevosArtesanos > 0 && asentamiento.poblacion.artesanos === 0) {
      eventos.push('Los primeros Artesanos se establecen gracias a los primeros edificios de transformación.');
    }
  }

  // Nobleza (Doc 4.1, rediseño de progreso Fase 0): además del mínimo de ciudadanos ya existente, ahora
  // también exige Palacio construido (Doc 4.2.1 — "desbloquea la aparición de la población noble"). Sin
  // Palacio, ningún número de ciudadanos hace aparecer Nobleza. `casasCompradas` ya incluye a los fundadores
  // (reciben casa automática al fundar, Doc 2.5) además de quienes compraron casa después, así que basta con
  // su longitud. Sacerdote puede acelerar el crecimiento vía política.
  let nuevaNobleza = 0;
  const ciudadanosEnAsentamiento = asentamiento.casasCompradas.length;
  const tienePalacio = edificiosPorTipoYEstado(asentamiento, 'palacio').length > 0;
  const cumpleRequisitoNobleza = ciudadanosEnAsentamiento >= POBLACION.nobleza.minCiudadanos && tienePalacio;
  if (cumpleRequisitoNobleza && espacioLibreFactor > 0) {
    if (asentamiento.poblacion.nobleza === 0) {
      nuevaNobleza = 1;
      eventos.push('Aparecen los primeros Nobles.');
    } else {
      nuevaNobleza = crecimientoEstocastico(
        asentamiento.poblacion.nobleza,
        POBLACION.nobleza.tasaCrecimientoBase * factorCrecimientoNobleza(asentamiento)
      );
    }
  }

  return {
    poblacion: {
      pesants: asentamiento.poblacion.pesants + nuevosPesants,
      artesanos: asentamiento.poblacion.artesanos + nuevosArtesanos,
      nobleza: asentamiento.poblacion.nobleza + nuevaNobleza,
    },
    eventos,
  };
}

/** Consumo de comida (Doc 4.1 implícito): cada habitante consume trigo por tick. Racionamiento (Sacerdote) lo reduce. */
export function consumirComida(asentamiento: Asentamiento): Asentamiento {
  const consumo = poblacionTotal(asentamiento) * POBLACION.consumoComidaPorHabitante * factorConsumoComida(asentamiento);
  const trigo = asentamiento.almacen['trigo'];
  if (!trigo) return asentamiento;
  return {
    ...asentamiento,
    almacen: {
      ...asentamiento.almacen,
      trigo: { ...trigo, cantidad: Math.max(0, trigo.cantidad - consumo) },
    },
  };
}
