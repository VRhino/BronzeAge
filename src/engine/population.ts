import type { Asentamiento, Poblacion } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { EDIFICIO_CATALOGO, NIVEL_ASENTAMIENTO, POBLACION } from '../constants';

/** Fase A5 — payload de `poblacion.hambruna_muerte` (ver `avanzarNutricionPoblacion`). Los eventos de
 * `crecerPoblacion` ('poblacion.primeros_artesanos'/'poblacion.primeros_nobles') no llevan payload — el
 * mensaje ya lo dice todo, no hay dato adicional que estructurar. */
export interface PayloadHambrunaMuerte {
  muertes: number;
  muertesPesants: number;
  muertesArtesanos: number;
}
import type { RandomFn } from '../worldgen';
import {
  EDIFICIOS_TRANSFORMACION,
  capacidadViviendaArtesanos,
  capacidadViviendaPesants,
  edificiosPorTipoYEstado,
  nutricionPoblacionDe,
  poblacionTotal,
} from './asentamientoQuery';
import { factorConsumoComida, factorCrecimientoNobleza } from './politicas';

/** Incremento entero esperado = actual*tasa, con redondeo estocástico para no estancarse con poblaciones pequeñas. */
function crecimientoEstocastico(actual: number, tasa: number, rng: RandomFn): number {
  const esperado = actual * tasa;
  const base = Math.floor(esperado);
  const resto = esperado - base;
  return base + (rng() < resto ? 1 : 0);
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
export function crecerPoblacion(asentamiento: Asentamiento, rng: RandomFn): { poblacion: Poblacion; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];

  // Hambruna (Doc 4.1, a petición del usuario): el factor de comida ya no es un booleano trigo>0?1:0.2 sino
  // que escala con el medidor de nutrición (`avanzarNutricionPoblacion`, corre antes en el mismo tick — ver
  // simulation.ts) — entre `factorCrecimientoMinimo` (nutrición en 0, hambre sostenida) y 1 (nutrición 100,
  // bien alimentado). `felicidad` sigue aparte como placeholder de política/Sacerdote (Sprint 4+, sin
  // implementar todavía) — nutrición es solo comida, no bienestar general.
  const nutricion = nutricionPoblacionDe(asentamiento);
  const { factorCrecimientoMinimo } = POBLACION.hambre;
  const comidaFactor = factorCrecimientoMinimo + (1 - factorCrecimientoMinimo) * (nutricion / 100);
  const estabilidad = 1;
  const felicidad = 1;

  const capacidadPesants = capacidadViviendaPesants(asentamiento);
  const espacioPesantsFactor = capacidadPesants <= 0 ? 0 : Math.max(0, Math.min(1, 1 - asentamiento.poblacion.pesants / capacidadPesants));
  const tasaPesants = POBLACION.pesants.tasaCrecimientoBase * comidaFactor * espacioPesantsFactor * estabilidad * felicidad;
  const nuevosPesants = crecimientoEstocastico(asentamiento.poblacion.pesants, tasaPesants, rng);

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
      eventos.push({
        codigo: 'poblacion.primeros_artesanos',
        mensaje: 'Los primeros Artesanos se establecen gracias al primer edificio de transformación.',
      });
    } else {
      const tasaArtesanos = POBLACION.artesanos.tasaCrecimientoBase * comidaFactor * espacioArtesanosFactor * estabilidad * felicidad;
      nuevosArtesanos = crecimientoEstocastico(asentamiento.poblacion.artesanos, tasaArtesanos, rng);
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
      eventos.push({ codigo: 'poblacion.primeros_nobles', mensaje: 'Aparecen los primeros Nobles.' });
    } else {
      const tasaNobleza =
        POBLACION.nobleza.tasaCrecimientoBase * comidaFactor * espacioPalacioFactor * estabilidad * felicidad * factorCrecimientoNobleza(asentamiento);
      nuevaNobleza = crecimientoEstocastico(asentamiento.poblacion.nobleza, tasaNobleza, rng);
    }
  }

  // Techo de población por nivel (Doc Fase_0_5 §3.1, a petición del usuario): por encima de este total, la
  // Vivienda/Palacio dejan de dar cupo efectivo aunque tengan capacidad física de sobra — el nivel es lo que
  // abre el techo de habitantes, no solo una llave de edificios. Escala hacia abajo el crecimiento de este
  // tick proporcionalmente entre las 3 clases si juntas se pasarían del techo (nunca purga población ya
  // asentada, solo limita cuánta puede sumarse este tick).
  const techo = NIVEL_ASENTAMIENTO.techoPoblacion[asentamiento.nivel] ?? Infinity;
  const espacioBajoTecho = Math.max(0, techo - poblacionTotal(asentamiento));
  const crecimientoBruto = nuevosPesants + nuevosArtesanos + nuevaNobleza;
  const factorTecho = crecimientoBruto > espacioBajoTecho && crecimientoBruto > 0 ? espacioBajoTecho / crecimientoBruto : 1;

  return {
    poblacion: {
      pesants: asentamiento.poblacion.pesants + Math.floor(nuevosPesants * factorTecho),
      artesanos: asentamiento.poblacion.artesanos + Math.floor(nuevosArtesanos * factorTecho),
      nobleza: asentamiento.poblacion.nobleza + Math.floor(nuevaNobleza * factorTecho),
    },
    eventos,
  };
}

/** Consumo de comida de la población (Doc 4.1 implícito): tasa FIJA por habitante (`consumoComidaPorHabitante`)
 * sumada por el total de habitantes (pesants+artesanos+nobleza); Racionamiento (Sacerdote) la reduce. Usada
 * tanto para descontarla en `avanzarNutricionPoblacion` como para el "apartado de trigo" de Mantenimiento (ver
 * `gameStore.mantenimientoInfo`), que suma esto con `consumoRacionTropas` (engine/tropas.ts). */
export function consumoComidaPoblacion(asentamiento: Asentamiento): number {
  return poblacionTotal(asentamiento) * POBLACION.consumoComidaPorHabitante * factorConsumoComida(asentamiento);
}

/**
 * Consumo de comida + hambruna (Doc 4.1, negativo por no sostener trigo — a petición del usuario, espejo de
 * `avanzarMantenimientoTropas` en engine/tropas.ts): descuenta lo que el trigo alcance a cubrir del consumo
 * del tick (nunca negativo) y actualiza `nutricionPoblacion` según la fracción cubierta — sube si el pago fue
 * íntegro, baja proporcional al déficit si no. Mientras la nutrición se mantiene en
 * `POBLACION.hambre.umbralMuertePorHambre` (0), cada tick cuesta una fracción de pesants+artesanos
 * (`fraccionMuertePorTickHambre`) — la nobleza queda protegida ("los nobles comen primero"), igual que
 * `nivel`/población ya asentada nunca se purga por un solo bache de Mantenimiento (Doc §6.2).
 */
export function avanzarNutricionPoblacion(asentamiento: Asentamiento): { asentamiento: Asentamiento; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const { hambre } = POBLACION;

  const consumo = consumoComidaPoblacion(asentamiento);
  const trigo = asentamiento.almacen['trigo'];
  const trigoDisponible = trigo?.cantidad ?? 0;
  const factorSuministro = consumo > 0 ? Math.min(1, trigoDisponible / consumo) : 1;

  const almacen = trigo
    ? { ...asentamiento.almacen, trigo: { ...trigo, cantidad: Math.max(0, trigoDisponible - consumo) } }
    : asentamiento.almacen;

  const nutricionPrevia = nutricionPoblacionDe(asentamiento);
  const nutricionPoblacion =
    factorSuministro >= 1
      ? Math.min(100, nutricionPrevia + hambre.regeneracionPorTick)
      : Math.max(0, nutricionPrevia - hambre.degradacionSinComida * (1 - factorSuministro));

  let poblacion = asentamiento.poblacion;
  if (nutricionPoblacion <= hambre.umbralMuertePorHambre) {
    const afectados = poblacion.pesants + poblacion.artesanos;
    if (afectados > 0) {
      const muertes = Math.min(afectados, Math.ceil(afectados * hambre.fraccionMuertePorTickHambre));
      const muertesPesants = Math.round(muertes * (poblacion.pesants / afectados));
      const muertesArtesanos = muertes - muertesPesants;
      poblacion = {
        ...poblacion,
        pesants: poblacion.pesants - muertesPesants,
        artesanos: poblacion.artesanos - muertesArtesanos,
      };
      eventos.push({
        codigo: 'poblacion.hambruna_muerte',
        mensaje: `${muertes} habitantes mueren de hambre por falta sostenida de trigo.`,
        payload: { muertes, muertesPesants, muertesArtesanos } satisfies PayloadHambrunaMuerte,
      });
    }
  }

  return { asentamiento: { ...asentamiento, almacen, poblacion, nutricionPoblacion }, eventos };
}
