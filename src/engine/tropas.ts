import type { Asentamiento, Ejercito, Escuadron, Heroe } from '../domain/types';
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
import { esResidente, puedeReclutarEn } from './pertenencia';
import { campamentoDe, conEscuadrones } from './tropa';
import { distancia } from '../world/geometria';

export class ReclutamientoInvalidoError extends Error {}

/** Con qué nace un escuadrón: nivel 1, sin experiencia y sin progresión táctica de Conquest. */
export const PROGRESION_INICIAL: Pick<
  Escuadron,
  'nivel' | 'experiencia' | 'habilidadesDesbloqueadas' | 'formacionesDesbloqueadas' | 'formacionSeleccionada'
> = { nivel: 1, experiencia: 0, habilidadesDesbloqueadas: [], formacionesDesbloqueadas: [], formacionSeleccionada: 0 };

/** Dónde está una escuadra, para decírselo a quien intenta reclutar otra de la misma tropa. */
function dondeEsta(e: Escuadron): string {
  switch (e.contenedor.tipo) {
    case 'campamento':
      return 'tu campamento';
    case 'ejercito':
      return `el ejército ${e.contenedor.ejercitoId}`;
    case 'escolta':
      return `la escolta de la caravana ${e.contenedor.caravanaId}`;
  }
}

/** ¿Se repone AQUÍ? Solo donde está físicamente (Doc 5.16.2): en el campamento de su héroe si reside en esta
 * plaza, o en su columna si está a la puerta. */
function reponibleAqui(e: Escuadron, asentamiento: Asentamiento, ejercitos: readonly Ejercito[]): boolean {
  const contenedor = e.contenedor;
  if (contenedor.tipo === 'campamento') return esResidente(asentamiento, e.heroeId);
  if (contenedor.tipo === 'escolta') return false;
  const columna = ejercitos.find((x) => x.id === contenedor.ejercitoId);
  return !!columna && distancia(columna.posicionActual, asentamiento.posicion) <= MOVIMIENTO.radioPuerta;
}

/**
 * Reclutamiento por equipo (Doc 5.7/5.8): recluta una tropa específica vía Barracón/Galería de tiro, según
 * el nivel interno del edificio, pagando el equipo fabricado en Armería en vez de cobre directo. Pesants Y
 * Artesanos pueden reclutar por este carril (a petición del usuario, ambos edificios reclutan de los dos
 * pools — reemplaza el antiguo reclutamiento directo de Artesanos con cobre a secas, y el de Nobleza vía Gran
 * Fundición, ambos retirados). Una tropa NUNCA cambia de identidad (Doc 5.8, a petición del usuario): "mejorar"
 * no es ascenso automático del mismo escuadrón, es reclutar una tropa DISTINTA y mejor cuando el edificio suba
 * de nivel interno — ver `poderEscuadron` en engine/combate.ts, que aplica el bonus de experiencia sin tocar
 * nunca `tropaId`.
 *
 * Escuadrón de UN héroe, no del asentamiento (Doc 2.5/5.16.2): como mucho UNO por `tropaId` en TODA la partida,
 * tope `tropa.unidadesPorDefecto`. Si ya lo tiene, reclutar solo lo REPONE —paga y añade las unidades que faltan
 * hasta el tope—, y solo donde está: en su campamento o en su columna a la puerta (`reponibleAqui`). Reclutar no
 * es un gate del cargo de General (Doc 2.2 vs 2.5 — 2.5 ganó la ambigüedad: es beneficio de residencia).
 */
export function reclutarTropa(
  asentamiento: Asentamiento,
  heroes: readonly Heroe[],
  /** Las columnas del mundo: una escuadra que va en una solo se repone con la columna a la puerta. */
  ejercitos: readonly Ejercito[],
  heroeId: string,
  /** Facción del jugador — para `puedeReclutarEn` (Doc 5.4/5.8, revisión 2026-09-08). Para el NPC siempre es
   * `asentamiento.faccionId`; para un comando, la Facción del actor. */
  faccionDelJugadorId: string,
  tropaId: string,
  origen: 'pesants' | 'artesanos',
  contador = 0
): { asentamiento: Asentamiento; heroes: Heroe[] } {
  const permiso = puedeReclutarEn(asentamiento, heroeId, faccionDelJugadorId);
  if (permiso === 'no') {
    throw new ReclutamientoInvalidoError('No puedes reclutar aquí: ni resides ni es una plaza de tu Facción que lo permita.');
  }
  const tropa = TROPAS_RECLUTABLES.find((t) => t.id === tropaId);
  if (!tropa) throw new ReclutamientoInvalidoError('La tropa no existe en el catálogo.');
  const heroe = heroes.find((h) => h.id === heroeId);
  if (!heroe) throw new ReclutamientoInvalidoError('Ese héroe no existe.');

  const existente = heroe.escuadrones.find((e) => e.tropaId === tropaId);
  if (existente && !reponibleAqui(existente, asentamiento, ejercitos)) {
    throw new ReclutamientoInvalidoError(`Ya tienes una escuadra de ${tropa.nombre}: está en ${dondeEsta(existente)}. Solo puedes reponerla donde está.`);
  }
  // Fuera de tu residencia solo REPONES lo que ya tienes aquí (tu columna a la puerta) — nunca un escuadrón
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
  const reservaTrigoRequerida = reservaDeTrigo(
    asentamiento,
    consumoRacionDeEscuadrones(campamentoDe(asentamiento, heroes)),
    cantidad * MILITAR.racionPorSoldadoPorMinuto
  );
  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  if (trigoDisponible < reservaTrigoRequerida) {
    throw new ReclutamientoInvalidoError(
      `No hay reserva de trigo suficiente para sostener esta tropa sin poner en riesgo al asentamiento (hacen falta ${Math.ceil(reservaTrigoRequerida)}, hay ${Math.floor(trigoDisponible)}).`
    );
  }

  const escuadron: Escuadron = existente
    ? { ...existente, cantidad: existente.cantidad + cantidad }
    : {
        id: `escuadron-${asentamiento.id}-${contador}`,
        nombre: `${tropa.nombre} de ${heroeId}`,
        heroeId,
        origen,
        cantidad,
        ...PROGRESION_INICIAL,
        moral: 100,
        tropaId,
        contenedor: { tipo: 'campamento' },
        enGuarnicion: false,
      };

  return {
    asentamiento: {
      ...asentamiento,
      poblacion: { ...asentamiento.poblacion, [origen]: asentamiento.poblacion[origen] - cantidad },
      almacen: descontarRecursos(asentamiento.almacen, costoTotal),
    },
    heroes: conEscuadrones(heroes, [escuadron]),
  };
}

/** Ración de trigo/tick que exige un conjunto de escuadrones, mire quien lo mire (Doc 5.4) — la guarnición de
 * una plaza es su campamento (`campamentoDe`). El `factorConsumo` lo usa un ejército ESTACIONADO, que consume
 * reducido pero nunca 0 (Doc 5.12.3). */
export function consumoRacionDeEscuadrones(escuadrones: readonly Escuadron[], factorConsumo = 1): number {
  const totalSoldados = escuadrones.reduce((acc, e) => acc + e.cantidad, 0);
  return totalSoldados * MILITAR.racionPorSoldadoPorMinuto * factorConsumo;
}

/** Lo que come una COLUMNA (Doc 5.13): sus soldados más sus jugadores. El sumando por participante es lo que
 * impide que un viajero sin tropas viaje gratis — con cero escuadrones el término de arriba es 0. */
function consumoRacionDeColumna(escuadrones: readonly Escuadron[], participantes: number, factorConsumo = 1): number {
  return consumoRacionDeEscuadrones(escuadrones, factorConsumo) + participantes * MOVIMIENTO.consumoPorParticipante * factorConsumo;
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
 * `consumoTropasPorMinuto` es la ración de su campamento (`consumoRacionDeEscuadrones(campamentoDe(...))`): las
 * escuadras viven en sus héroes, no en la plaza. `consumoExtraPorMinuto` proyecta bocas que TODAVÍA no existen
 * —la tropa que se está a punto de reclutar—, que es lo que distingue "¿me queda margen?" de "¿me quedará?".
 */
export function reservaDeTrigo(asentamiento: Asentamiento, consumoTropasPorMinuto: number, consumoExtraPorMinuto = 0): number {
  const porMinuto = consumoComidaPoblacion(asentamiento) + consumoTropasPorMinuto + consumoExtraPorMinuto;
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

  // El squad (nombre, experiencia) persiste aunque `cantidad` llegue a 0 (Doc 5.4) — se puede rellenar reclutando.
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

/** Mantenimiento de la GUARNICIÓN (Doc 5.4): el campamento de la plaza come de su almacén. El ejército en campaña
 * usa la misma `avanzarRacion` con su carro (Doc 5.13). */
export function avanzarMantenimientoTropas(
  asentamiento: Asentamiento,
  campamento: readonly Escuadron[]
): { asentamiento: Asentamiento; campamento: Escuadron[]; eventos: EventoCrudo[] } {
  if (campamento.length === 0) return { asentamiento, campamento: [], eventos: [] };

  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const { escuadrones, trigoConsumido, eventos } = avanzarRacion(campamento, trigoDisponible);
  const almacen = descontarRecursos(asentamiento.almacen, { trigo: trigoConsumido });

  return { asentamiento: { ...asentamiento, almacen }, campamento: escuadrones, eventos };
}
