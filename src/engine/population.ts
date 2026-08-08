import type { Asentamiento, Poblacion } from '../domain/types';
import { EDIFICIO_CATALOGO, POBLACION } from '../constants';
import {
  EDIFICIOS_TRANSFORMACION,
  capacidadViviendaArtesanos,
  capacidadViviendaPesants,
  edificiosPorTipoYEstado,
  poblacionTotal,
} from './asentamientoQuery';
import { factorConsumoComida, factorCrecimientoNobleza } from './politicas';

/** Incremento entero esperado = actual*tasa, con redondeo estocástico para no estancarse con poblaciones pequeñas. */
function crecimientoEstocastico(actual: number, tasa: number): number {
  const esperado = actual * tasa;
  const base = Math.floor(esperado);
  const resto = esperado - base;
  return base + (Math.random() < resto ? 1 : 0);
}

/**
 * Crecimiento de población (Doc 4.1) — rediseño a petición del usuario: las 3 clases usan la MISMA fórmula
 * proporcional (comida disponible × cupo libre × estabilidad × felicidad × tasa propia), diferenciadas solo
 * por su tasa (Pesants > Artesanos > Nobleza, ver POBLACION en constants.ts) y por su condición de primera
 * aparición y su cupo. Estabilidad y felicidad no están modeladas aún (Sprint 4+ políticas/Sacerdote) — se
 * usan como 1 (neutro).
 *
 * Cupos: rediseño a petición del usuario (ver Correcciones) — Pesants y Artesanos tenían un único cupo de
 * Vivienda compartido, y como Pesants crece ~2.4x más rápido, acaparaba el cupo entero y dejaba a Artesanos
 * varado indefinidamente en 1 unidad (verificado 7600 ticks sin moverse — no era un bug, consecuencia fiel
 * del pool compartido). Cada Vivienda ahora da cupos SEPARADOS por clase (`capacidadViviendaPesants`/
 * `capacidadViviendaArtesanos`, ver constants.ts `EDIFICIO_CATALOGO.vivienda`) — ya no compiten entre sí.
 * Nobleza sigue con su propio cupo aparte, la `capacidadNobles` del Palacio.
 */
export function crecerPoblacion(asentamiento: Asentamiento): { poblacion: Poblacion; eventos: string[] } {
  const eventos: string[] = [];

  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const comidaFactor = trigoDisponible > 0 ? 1 : 0.2;
  const estabilidad = 1;
  const felicidad = 1;

  const capacidadPesants = capacidadViviendaPesants(asentamiento);
  const espacioPesantsFactor = capacidadPesants <= 0 ? 0 : Math.max(0, Math.min(1, 1 - asentamiento.poblacion.pesants / capacidadPesants));
  const tasaPesants = POBLACION.pesants.tasaCrecimientoBase * comidaFactor * espacioPesantsFactor * estabilidad * felicidad;
  const nuevosPesants = crecimientoEstocastico(asentamiento.poblacion.pesants, tasaPesants);

  // Artesanos: no aparece ninguno hasta el primer edificio de transformación activo (Fundición/Curtiduría/
  // Armería/Carpintería) — a partir de ahí crecen con la misma fórmula que Pesants, a su propia tasa (más
  // lenta) y con su propio cupo de Vivienda (separado del de Pesants). `trabajadoresRequeridos` de esos
  // edificios sigue existiendo, pero ahora solo escala la PRODUCCIÓN (`ratioManoObraArtesanos`), ya no
  // limita cuánta población de Artesanos puede aparecer.
  const hayEdificioTransformacion = EDIFICIOS_TRANSFORMACION.some((tipo) => edificiosPorTipoYEstado(asentamiento, tipo).length > 0);
  const capacidadArtesanosVivienda = capacidadViviendaArtesanos(asentamiento);
  const espacioArtesanosFactor =
    capacidadArtesanosVivienda <= 0 ? 0 : Math.max(0, Math.min(1, 1 - asentamiento.poblacion.artesanos / capacidadArtesanosVivienda));
  let nuevosArtesanos = 0;
  if (hayEdificioTransformacion && espacioArtesanosFactor > 0) {
    if (asentamiento.poblacion.artesanos === 0) {
      nuevosArtesanos = 1;
      eventos.push('Los primeros Artesanos se establecen gracias al primer edificio de transformación.');
    } else {
      const tasaArtesanos = POBLACION.artesanos.tasaCrecimientoBase * comidaFactor * espacioArtesanosFactor * estabilidad * felicidad;
      nuevosArtesanos = crecimientoEstocastico(asentamiento.poblacion.artesanos, tasaArtesanos);
    }
  }

  // Nobleza (Doc 4.1): condición de aparición sin cambios — ciudadanía mínima (`casasCompradas` ya incluye a
  // los fundadores, Doc 2.5) y Palacio construido. A partir de ahí, misma fórmula proporcional a la tasa más
  // lenta de las 3, con su propio cupo (`capacidadNobles` del Palacio) en vez del de Vivienda. Sacerdote
  // puede acelerar el crecimiento vía política.
  const palaciosActivos = edificiosPorTipoYEstado(asentamiento, 'palacio').length;
  const capacidadNobleza = palaciosActivos * EDIFICIO_CATALOGO.palacio.capacidadNobles;
  const espacioPalacioFactor = capacidadNobleza <= 0 ? 0 : Math.max(0, Math.min(1, 1 - asentamiento.poblacion.nobleza / capacidadNobleza));
  const ciudadanosEnAsentamiento = asentamiento.casasCompradas.length;
  const cumpleRequisitoNobleza = ciudadanosEnAsentamiento >= POBLACION.nobleza.minCiudadanos && palaciosActivos > 0;
  let nuevaNobleza = 0;
  if (cumpleRequisitoNobleza && espacioPalacioFactor > 0) {
    if (asentamiento.poblacion.nobleza === 0) {
      nuevaNobleza = 1;
      eventos.push('Aparecen los primeros Nobles.');
    } else {
      const tasaNobleza =
        POBLACION.nobleza.tasaCrecimientoBase * comidaFactor * espacioPalacioFactor * estabilidad * felicidad * factorCrecimientoNobleza(asentamiento);
      nuevaNobleza = crecimientoEstocastico(asentamiento.poblacion.nobleza, tasaNobleza);
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

/** Consumo de comida de la población (Doc 4.1 implícito): tasa FIJA por habitante (`consumoComidaPorHabitante`)
 * sumada por el total de habitantes (pesants+artesanos+nobleza); Racionamiento (Sacerdote) la reduce. Usada
 * tanto para descontarla en `consumirComida` como para el "apartado de trigo" de Mantenimiento (ver
 * `gameStore.mantenimientoInfo`), que suma esto con `consumoRacionTropas` (engine/tropas.ts). */
export function consumoComidaPoblacion(asentamiento: Asentamiento): number {
  return poblacionTotal(asentamiento) * POBLACION.consumoComidaPorHabitante * factorConsumoComida(asentamiento);
}

/** Consumo de comida (Doc 4.1 implícito): cada habitante consume trigo por tick. Racionamiento (Sacerdote) lo reduce. */
export function consumirComida(asentamiento: Asentamiento): Asentamiento {
  const consumo = consumoComidaPoblacion(asentamiento);
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
