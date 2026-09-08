import type { Asentamiento, Escuadron } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { MILITAR, MOVIMIENTO, RECLUTAMIENTO_ORO_POR_ESCALON, RESERVA_CONSTRUCCION, TROPAS_RECLUTABLES } from '../constants';

/** Fase A5 — payload de `tropas.desercion` (ver `avanzarMantenimientoTropas`). */
export interface PayloadTropasDesercion {
  escuadronId: string;
  escuadronNombre: string;
  desertores: number;
}
import { descontarRecursos, tieneRecursos } from './almacen';
import { edificiosPorTipoYEstado, poblacionDisponibleParaReclutar } from './asentamientoQuery';
import { consumoComidaPoblacion } from './population';
import { factorCostoReclutamiento } from './politicas';
import { puedeReclutarEn } from './pertenencia';

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
  /** Facción del jugador — para `puedeReclutarEn` (Doc 5.4/5.8, revisión 2026-09-08). Para el NPC siempre es
   * `asentamiento.faccionId`; para un comando, la Facción del actor. */
  faccionDelJugadorId: string,
  tropaId: string,
  origen: 'pesants' | 'artesanos',
  contador = 0
): Asentamiento {
  const permiso = puedeReclutarEn(asentamiento, jugadorId, faccionDelJugadorId);
  if (permiso === 'no') {
    throw new ReclutamientoInvalidoError('No puedes reclutar aquí: ni resides ni es una plaza de tu Facción que lo permita.');
  }
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) throw new ReclutamientoInvalidoError('La tropa no existe en el catálogo.');

  const existente = asentamiento.escuadrones.find((e) => e.jugadorId === jugadorId && e.tropaId === tropaId);
  // Fuera de tu residencia solo REPONES lo que ya tienes aquí (guarnición o columna) — nunca un escuadrón
  // nuevo ni una tropa distinta.
  if (permiso === 'solo_reponer' && !existente) {
    throw new ReclutamientoInvalidoError('Fuera de tu residencia solo puedes reponer un escuadrón que ya tienes aquí, no reclutar uno nuevo.');
  }
  const cantidad = tropa.unidadesPorDefecto - (existente?.cantidad ?? 0);
  if (cantidad <= 0) {
    throw new ReclutamientoInvalidoError('Este escuadrón ya está al tope de unidades.');
  }
  // El pool de reclutamiento NO es la población total: es la población menos la que ya hace falta para cubrir
  // la mano de obra vigente (a petición del usuario, tras encontrar en pruebas asentamientos colapsando porque
  // el NPC reclutaba pesants que ya estaban ocupados produciendo — ver `poblacionDisponibleParaReclutar`,
  // `engine/asentamientoQuery.ts`, que es el mismo número que la UI ya mostraba como "Pool de pesants para
  // reclutamiento" sin que el motor lo hiciera cumplir). Regla del motor: aplica igual a reclutamiento manual y
  // al de la gobernanza NPC.
  const disponible = poblacionDisponibleParaReclutar(asentamiento, origen);
  if (disponible < cantidad) {
    throw new ReclutamientoInvalidoError(
      `No hay suficientes ${origen} disponibles para reclutar sin desproteger la producción (hacen falta ${cantidad}, disponibles ${disponible} tras cubrir mano de obra).`
    );
  }

  const edificio = edificiosPorTipoYEstado(asentamiento, tropa.edificio)[0];
  if (!edificio || (edificio.nivelInterno ?? 1) < tropa.nivelRequerido) {
    throw new ReclutamientoInvalidoError(
      `Se necesita ${tropa.edificio} activo en nivel interno ${tropa.nivelRequerido} para reclutar "${tropa.nombre}".`
    );
  }

  const factorCosto = factorCostoReclutamiento(asentamiento);
  const costoTotal: Record<string, number> = Object.fromEntries(
    Object.entries(tropa.costoEquipo).map(([recurso, cantidadUnitaria]) => [recurso, (cantidadUnitaria ?? 0) * cantidad * factorCosto])
  );
  // Oro por soldado según escalón (Doc 5.8, bloque "economía del oro"): se suma al coste de equipo, SALVO la
  // milicia del Centro Urbano. `factorCostoReclutamiento` ("Leva Forzosa") no lo toca — solo el equipo.
  if (tropa.edificio !== 'centroUrbano') {
    const oroPorSoldado = RECLUTAMIENTO_ORO_POR_ESCALON[tropa.escalon] ?? 0;
    if (oroPorSoldado > 0) costoTotal['oro'] = (costoTotal['oro'] ?? 0) + oroPorSoldado * cantidad;
  }
  if (!tieneRecursos(asentamiento.almacen, costoTotal)) {
    throw new ReclutamientoInvalidoError('No hay recursos suficientes (equipo u oro) para reclutar esta tropa.');
  }

  // Reserva de trigo ANTES de reclutar (a petición del usuario — mano de obra/reclutamiento a futuro con
  // jugadores reales, no solo el NPC): reclutar (o reponer) una tropa exige que el trigo en almacén cubra
  // `horizonteMinutosComida` ticks del consumo YA PROYECTADO CON la tropa nueva sumada — mismo patrón y mismo
  // horizonte que `reservaDinamicaConstruccion` (`engine/mantenimiento.ts`) exige para construir, extendido
  // aquí a trigo. Es una regla del MOTOR, no un heurístico del NPC (como Vivienda: aplica igual a
  // reclutamiento manual y automático) — reemplaza el throttle "1 residente por tick en nivel 1" que antes
  // vivía en `app/npcGobernanza.ts`: en vez de un límite artificial por nivel, el límite real es si el
  // asentamiento tiene margen de comida de verdad. Con la Granja produciendo, el margen crece solo; bajo
  // escasez, se cierra antes de que reclutar termine de romper nada — ver
  // `issues/granjas_no_escalan_con_poblacion.md` y `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md`
  // §"Abierto" para el diagnóstico completo (colapso masivo medido en batch con el throttle viejo).
  const reservaTrigoRequerida = reservaDeTrigo(asentamiento, cantidad * MILITAR.racionPorSoldadoPorMinuto);
  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  if (trigoDisponible < reservaTrigoRequerida) {
    throw new ReclutamientoInvalidoError(
      `No hay reserva de trigo suficiente para sostener esta tropa sin poner en riesgo al asentamiento (hacen falta ${Math.ceil(reservaTrigoRequerida)}, hay ${Math.floor(trigoDisponible)}).`
    );
  }

  const escuadrones = existente
    ? asentamiento.escuadrones.map((e) => (e.id === existente.id ? { ...e, cantidad: e.cantidad + cantidad } : e))
    : [
        ...asentamiento.escuadrones,
        {
          id: `escuadron-${asentamiento.id}-${contador}`,
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

/** Ración de trigo/tick que exige un conjunto de escuadrones, mire quien lo mire (Doc 5.4). El
 * `factorConsumo` lo usa un ejército ESTACIONADO, que consume reducido pero nunca 0 (Doc 5.12.3). */
export function consumoRacionDeEscuadrones(escuadrones: readonly Escuadron[], factorConsumo = 1): number {
  const totalSoldados = escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
  return totalSoldados * MILITAR.racionPorSoldadoPorMinuto * factorConsumo;
}

/** Lo que come una COLUMNA (Doc 5.13): sus soldados más sus jugadores. El sumando por participante es lo que
 * impide que un viajero sin tropas viaje gratis — con cero escuadrones el término de arriba es 0. */
function consumoRacionDeColumna(escuadrones: readonly Escuadron[], participantes: number, factorConsumo = 1): number {
  return consumoRacionDeEscuadrones(escuadrones, factorConsumo) + participantes * MOVIMIENTO.consumoPorParticipante * factorConsumo;
}

/** Ración total de trigo/tick que exigen los escuadrones de la GUARNICIÓN (Doc 5.4) — usada tanto para
 * descontarla como para el "apartado de trigo" mostrado en Mantenimiento (ver `gameStore.mantenimientoInfo`).
 *
 * No hace falta tocarla cuando existan los ejércitos: al salir a campaña los escuadrones se van DE VERDAD de
 * `asentamiento.escuadrones` (Doc 5.4), así que esto ya cuenta solo lo que se quedó en casa, que es
 * exactamente lo que la regla pide. */
export function consumoRacionTropas(asentamiento: Asentamiento): number {
  return consumoRacionDeEscuadrones(asentamiento.escuadrones);
}

/**
 * Trigo que un asentamiento NO puede tocar: lo que su gente —población y guarnición— come durante
 * `RESERVA_CONSTRUCCION.horizonteMinutosComida` minutos de mundo.
 *
 * Escrita aquí una sola vez porque son ya TRES las puertas que dan al almacén y todas tienen que medir con la
 * misma vara: la auto-construcción (`reservaDinamicaConstruccion`), el reclutamiento (justo abajo) y la carga
 * del carro de un ejército que sale de campaña (`engine/ejercitos.ts`, Doc 5.13). Antes la fórmula estaba
 * copiada en las dos primeras, y la tercera habría sido la copia número tres.
 *
 * `consumoExtraPorMinuto` proyecta bocas que TODAVÍA no existen — la tropa que se está a punto de reclutar —,
 * que es lo que distingue "¿me queda margen?" de "¿me quedará margen después de esto?".
 */
export function reservaDeTrigo(asentamiento: Asentamiento, consumoExtraPorMinuto = 0): number {
  const porMinuto = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento) + consumoExtraPorMinuto;
  return porMinuto * RESERVA_CONSTRUCCION.horizonteMinutosComida;
}

/**
 * La regla del hambre, escrita UNA vez (Doc 5.4): se come de la despensa que se le pase, y lo que no alcanza
 * se paga en moral; a moral 0 hay deserción permanente.
 *
 * Es deliberadamente ignorante de DÓNDE está la comida: recibe unos escuadrones y un montón de trigo. Eso es
 * lo que permite que la guarnición (que come del almacén del asentamiento) y un ejército en campaña (que come
 * de su carro de suministros, Doc 5.13) compartan curva, constantes y evento sin duplicar nada — el carro no
 * es un subsistema paralelo, es un segundo llamador de esta función.
 *
 * `factorConsumo` < 1 para un ejército estacionado (Doc 5.12.3). No hay parámetro de "consecuencia": el
 * usuario cerró que dispersión y deserción son lo mismo, así que el resultado del hambre es idéntico en
 * ambos sitios.
 *
 * Devuelve el trigo REALMENTE consumido para que el llamador lo descuente de donde corresponda.
 */
export function avanzarRacion(
  escuadrones: readonly Escuadron[],
  trigoDisponible: number,
  factorConsumo = 1,
  /** Jugadores dentro de la columna, que también comen (Doc 5.13). 0 para la guarnición, que no lleva a
   * nadie — sus jugadores comen de la población del asentamiento, no de una ración de campaña. */
  participantes = 0
): { escuadrones: Escuadron[]; trigoConsumido: number; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const racionNecesaria = consumoRacionDeColumna(escuadrones, participantes, factorConsumo);
  const factorSuministro = racionNecesaria > 0 ? Math.min(1, trigoDisponible / racionNecesaria) : 1;
  const trigoConsumido = Math.min(trigoDisponible, racionNecesaria);

  // El squad (nombre, veteranía) persiste aunque `cantidad` llegue a 0 (Doc 5.4) — se puede rellenar reclutando.
  const actualizados = escuadrones.map((e) => {
    let moral = e.moral;
    if (factorSuministro >= 1) {
      moral = Math.min(100, moral + MILITAR.regeneracionMoralPorMinuto);
    } else {
      moral = Math.max(0, moral - MILITAR.degradacionMoralSinRacion * (1 - factorSuministro));
    }
    let cantidad = e.cantidad;
    if (moral <= 0 && cantidad > 0) {
      const desertores = Math.min(cantidad, Math.ceil(cantidad * MILITAR.desercionFraccionPorMinutoSinMoral));
      cantidad -= desertores;
      if (desertores > 0) {
        eventos.push({
          codigo: 'tropas.desercion',
          mensaje: `${e.nombre}: ${desertores} desertan por hambre (moral colapsada).`,
          payload: { escuadronId: e.id, escuadronNombre: e.nombre, desertores } satisfies PayloadTropasDesercion,
        });
      }
    }
    return { ...e, moral, cantidad };
  });

  return { escuadrones: actualizados, trigoConsumido, eventos };
}

/** Mantenimiento de la GUARNICIÓN (Doc 5.4): envoltorio de `avanzarRacion` sobre el almacén del asentamiento.
 * El ejército en campaña usará la misma función con su carro (Doc 5.13). */
export function avanzarMantenimientoTropas(asentamiento: Asentamiento): { asentamiento: Asentamiento; eventos: EventoCrudo[] } {
  if (asentamiento.escuadrones.length === 0) return { asentamiento, eventos: [] };

  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const { escuadrones, trigoConsumido, eventos } = avanzarRacion(asentamiento.escuadrones, trigoDisponible);
  const almacen = descontarRecursos(asentamiento.almacen, { trigo: trigoConsumido });

  return { asentamiento: { ...asentamiento, almacen, escuadrones }, eventos };
}
