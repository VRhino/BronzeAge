import type { Asentamiento, Caravana, Faccion, Point } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { CARAVANA_CATALOGO, EDIFICIO_CATALOGO, FUNDACION } from '../constants';
import { agregarRecurso, descontarRecursos, tieneRecursos } from './almacen';
import { avanzarPosicionEnRuta } from './movimiento';
import { posicionLibreParaFundar } from './zones';
import { calcularCapFundacion } from './faccion';
import { fundarAsentamiento, FundacionInvalidaError } from './settlement';
import { nivelActualDe } from './asentamientoQuery';

export class ExpansionInvalidaError extends Error {}

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** Coste total de una Caravana de Fundación (Doc 1.8): materiales iniciales + edificios de arranque
 * (Centro Urbano no cuesta nada) + madera extra por fabricar la caravana en sí. */
export function costoCaravanaFundacion(): Partial<Record<string, number>> {
  const costo: Partial<Record<string, number>> = { ...FUNDACION.materialesIniciales };
  for (const [recurso, cantidad] of Object.entries(EDIFICIO_CATALOGO.granja.costo)) {
    costo[recurso] = (costo[recurso] ?? 0) + cantidad;
  }
  for (const [recurso, cantidad] of Object.entries(EDIFICIO_CATALOGO.vivienda.costo)) {
    costo[recurso] = (costo[recurso] ?? 0) + cantidad * FUNDACION.viviendasIniciales;
  }
  costo.madera = (costo.madera ?? 0) + FUNDACION.costoMaderaExtraCaravana;
  return costo;
}

/** Facción dueña de la caravana, resuelta a través de su asentamiento de origen (la propia Caravana no
 * guarda faccionId). Undefined si el origen ya no existe (asentamiento colapsado en tránsito). */
function faccionDeCaravana(caravana: Caravana, asentamientos: Asentamiento[]): string | undefined {
  return asentamientos.find((a) => a.id === caravana.origenAsentamientoId)?.faccionId;
}

/** Nº de "asentamientos efectivos" de una Facción a efectos del Cap de Fundación (Doc 1.7/1.8): los ya
 * fundados MÁS las Caravanas de Fundación propias todavía en tránsito (Doc 1.8: el cupo se reserva al
 * lanzar, no al llegar). */
function asentamientosEfectivos(faccionId: string, asentamientos: Asentamiento[], caravanas: Caravana[]): number {
  const propios = asentamientos.filter((a) => a.faccionId === faccionId).length;
  const enTransito = caravanas.filter(
    (c) => c.tipo === 'construccion' && c.destinoPosicion && faccionDeCaravana(c, asentamientos) === faccionId
  ).length;
  return propios + enTransito;
}

/**
 * Lanza una Caravana de Fundación (Doc 1.8): gates de nivel≥2 + coste completo, reserva de inmediato un
 * cupo del Cap de Fundación de la Facción, y lleva consigo a ciudadanos YA EXISTENTES de esa Facción (hasta
 * el máximo de fundación grupal) para fundar al llegar — decisión confirmada con el usuario, no jugadores
 * nuevos inventados.
 */
export function lanzarCaravanaFundacion(
  mapa: Mapa,
  origen: Asentamiento,
  faccion: Faccion,
  destino: Point,
  asentamientosExistentes: Asentamiento[],
  caravanasExistentes: Caravana[],
  numJugadores: number,
  tickActual: number,
  contador = 0
): { origenActualizado: Asentamiento; caravana: Caravana } {
  if (origen.faccionId !== faccion.id) {
    throw new ExpansionInvalidaError('El asentamiento de origen no pertenece a esta Facción.');
  }
  // nivelActual (Doc Fase_0_5 §6.2), no nivelAlcanzado: un origen degradado por debajo de nivel 2 no puede
  // lanzar una Caravana de Fundación hasta recuperarse, aunque haya llegado a nivel 2 alguna vez.
  if (nivelActualDe(origen) < 2) {
    throw new ExpansionInvalidaError('El asentamiento de origen debe estar en nivel 2 como mínimo para lanzar una Caravana de Fundación.');
  }
  if (!posicionLibreParaFundar(destino, asentamientosExistentes)) {
    throw new ExpansionInvalidaError('El destino cae dentro de una zona de influencia existente.');
  }

  const cap = calcularCapFundacion(faccion.nivel);
  const efectivos = asentamientosEfectivos(faccion.id, asentamientosExistentes, caravanasExistentes);
  if (efectivos >= cap) {
    throw new ExpansionInvalidaError(`Cap de fundación alcanzado (${efectivos}/${cap} en nivel ${faccion.nivel}, incluyendo caravanas ya en tránsito).`);
  }

  const costo = costoCaravanaFundacion();
  if (!tieneRecursos(origen.almacen, costo)) {
    throw new ExpansionInvalidaError('El asentamiento de origen no tiene recursos suficientes para la Caravana de Fundación.');
  }

  const n = Math.min(FUNDACION.maxJugadoresFundacionGrupal, Math.max(1, numJugadores || 1));
  const jugadoresFundadoresIds = faccion.ciudadanosIds.slice(0, n);
  if (jugadoresFundadoresIds.length === 0) {
    throw new ExpansionInvalidaError('La Facción no tiene ciudadanos disponibles para fundar el nuevo asentamiento.');
  }

  const caravana: Caravana = {
    id: `caravana-fundacion-${origen.id}-${tickActual}-${contador}`,
    tipo: 'construccion',
    origenAsentamientoId: origen.id,
    contenido: costo as Record<string, number>,
    posicionActual: origen.posicion,
    progreso: 0,
    destinoPosicion: destino,
    jugadoresFundadoresIds,
    // Ruta calculada al lanzar (Fase 0.3, ver `world/rutas.ts`): rodea terreno costoso en vez de ir en
    // línea recta hacia el punto de fundación elegido.
    ruta: calcularRuta(mapa, origen.posicion, destino),
  };

  return { origenActualizado: { ...origen, almacen: descontarRecursos(origen.almacen, costo) }, caravana };
}

/** Desarma una Caravana de Fundación en tránsito y reembolsa su contenido íntegro al asentamiento de origen. */
export function desarmarCaravanaFundacion(origen: Asentamiento, caravana: Caravana): Asentamiento {
  if (caravana.tipo !== 'construccion' || !caravana.destinoPosicion) {
    throw new ExpansionInvalidaError('Esa caravana no es una Caravana de Fundación.');
  }
  if (caravana.origenAsentamientoId !== origen.id) {
    throw new ExpansionInvalidaError('La caravana no pertenece a este asentamiento.');
  }
  if (caravana.progreso >= 1) {
    throw new ExpansionInvalidaError('La caravana ya llegó a destino.');
  }
  let almacen = origen.almacen;
  for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
    almacen = agregarRecurso(almacen, recurso, cantidad);
  }
  return { ...origen, almacen };
}

/**
 * Avanza las Caravanas de Fundación en tránsito (Doc 1.8): mismo cálculo de movimiento que las caravanas
 * comerciales (`avanzarCaravanas`, engine/trade.ts), pero hacia un punto del mapa en vez de un asentamiento
 * existente. Al llegar, funda el nuevo asentamiento — si por un caso de borde no previsto por el diseño
 * (p. ej. el nivel de Facción bajó en tránsito y el cap ya no alcanza) `fundarAsentamiento` lo rechaza, la
 * caravana se pierde sin reembolso, igual que una caravana interceptada.
 */
export function avanzarCaravanasFundacion(
  caravanas: Caravana[],
  mapa: Mapa,
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  tickActual: number
): { caravanas: Caravana[]; asentamientos: Asentamiento[]; facciones: Faccion[]; eventos: string[] } {
  const eventos: string[] = [];
  const restantes: Caravana[] = [];
  let asentamientosActuales = asentamientos;
  let faccionesActuales = facciones;

  for (const caravana of caravanas) {
    if (caravana.tipo !== 'construccion' || !caravana.destinoPosicion) {
      restantes.push(caravana);
      continue;
    }

    const origen = asentamientosActuales.find((a) => a.id === caravana.origenAsentamientoId);
    if (!origen) {
      eventos.push(`La Caravana de Fundación ${caravana.id} se pierde: su asentamiento de origen ya no existe.`);
      continue;
    }

    const velocidad = CARAVANA_CATALOGO.construccion.velocidad;

    // Con `ruta` (Fase 0.3, calculada al lanzar — ver `lanzarCaravanaFundacion`): avance real por coste de
    // terreno. Sin `ruta` (partidas guardadas antes de Fase 0.3): línea recta, comportamiento sin cambios.
    let progreso: number;
    let posicionActual: Point;
    if (caravana.ruta && caravana.ruta.length >= 2) {
      const avance = avanzarPosicionEnRuta(mapa, caravana.ruta, caravana.progreso, velocidad);
      progreso = avance.progreso;
      posicionActual = avance.posicion;
    } else {
      const distanciaTotal = Math.max(1, distancia(origen.posicion, caravana.destinoPosicion));
      progreso = Math.min(1, caravana.progreso + velocidad / distanciaTotal);
      posicionActual = {
        x: origen.posicion.x + (caravana.destinoPosicion.x - origen.posicion.x) * progreso,
        y: origen.posicion.y + (caravana.destinoPosicion.y - origen.posicion.y) * progreso,
      };
    }

    if (progreso < 1) {
      restantes.push({ ...caravana, progreso, posicionActual });
      continue;
    }

    try {
      const resultado = fundarAsentamiento(
        mapa,
        faccionesActuales,
        origen.faccionId,
        caravana.destinoPosicion,
        caravana.jugadoresFundadoresIds ?? [],
        asentamientosActuales,
        tickActual
      );
      asentamientosActuales = [...asentamientosActuales, resultado.asentamiento];
      faccionesActuales = resultado.facciones;
      eventos.push(`La Caravana de Fundación ${caravana.id} llega y funda ${resultado.asentamiento.id}.`);
    } catch (err) {
      const razon = err instanceof FundacionInvalidaError ? err.message : String(err);
      eventos.push(`La Caravana de Fundación ${caravana.id} llega pero no puede fundar (${razon}) — se pierde.`);
    }
  }

  return { caravanas: restantes, asentamientos: asentamientosActuales, facciones: faccionesActuales, eventos };
}
