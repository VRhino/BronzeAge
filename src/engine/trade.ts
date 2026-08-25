import type { AcuerdoTrueque, Asentamiento, CaminoComercial, Caravana, Faccion, Point, ZonaInfluencia } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payloads de los eventos de este subsistema (ver `avanzarCaravanas`/`asignarCaravanasATrueque`). */
export interface PayloadCaravanaLlega {
  caravanaId: string;
  tipo: Caravana['tipo'];
  origenId: string;
  destinoId: string;
  contenido: Record<string, number>;
  comision: number;
}
export interface PayloadPeaje {
  destinoId: string;
  monto: number;
  controladorId: string;
  cantidadChokepoints: number;
}
export interface PayloadTruequeCumplido {
  acuerdoId: string;
  asentamientoAId: string;
  asentamientoBId: string;
}
export interface PayloadTruequeExpirado {
  acuerdoId: string;
}
export interface PayloadCaravanaSale {
  origenId: string;
  destinoId: string;
  cantidad: number;
  recurso: string;
}
import { ASIGNACION_CARAVANA, CARAVANA_CATALOGO, CHOKEPOINTS_PEAJE, COMISION, REPUTACION, TRUEQUE } from '../constants';
import { COSTE_MOVIMIENTO } from '../worldgen';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { agregarRecurso, cantidadDisponible, descontarRecursos, tieneRecursos } from './almacen';
import { buscarCamino } from './caminos';
import { chokepointsDePeajeEnRuta } from './chokepoints';
import { calcularPrecioReferencia } from './market';
import { cupoCaravanas, puedeCrearCaravana, ticksCooldownCaravanaRestantes, tieneMercadoActivo } from './asentamientoQuery';
import { avanzarPosicionEnRuta } from './movimiento';
import { factorCapacidadCaravana, factorComisionExterna, factorVelocidadCaravana } from './politicas';
import { aplicarAjustesReputacion, factorComisionPorReputacion, type AjusteReputacion } from './reputacion';

export class TruequeInvalidoError extends Error {}
export class CaravanaInvalidaError extends Error {}

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Propone un acuerdo de trueque entre dos asentamientos (Doc 3.2). "Funciona en ambas direcciones": cada lado
 * se compromete a entregar su propio recurso. En Fase 0 se acepta al proponerse (el flujo real de "el Tesorero
 * de B acepta" requiere un jugador interactivo real, fuera de alcance del prototipo), quedando "activo" ya mismo.
 */
export function proponerTrueque(
  asentamientos: Asentamiento[],
  asentamientoAId: string,
  asentamientoBId: string,
  recursoA: string,
  recursoB: string,
  cantidadTotalA: number,
  cantidadTotalB: number,
  tickActual: number,
  contador = 0
): AcuerdoTrueque {
  if (asentamientoAId === asentamientoBId) {
    throw new TruequeInvalidoError('Un asentamiento no puede acordar trueque consigo mismo.');
  }
  const a = asentamientos.find((s) => s.id === asentamientoAId);
  const b = asentamientos.find((s) => s.id === asentamientoBId);
  if (!a || !b) throw new TruequeInvalidoError('Alguno de los asentamientos no existe.');
  if (cantidadTotalA <= 0 || cantidadTotalB <= 0) {
    throw new TruequeInvalidoError('Las cantidades pactadas deben ser mayores que 0.');
  }

  return {
    id: `trueque-${asentamientoAId}-${asentamientoBId}-${tickActual}-${contador}`,
    asentamientoAId,
    asentamientoBId,
    recursoA,
    recursoB,
    cantidadTotalA,
    cantidadTotalB,
    cantidadEntregadaA: 0,
    cantidadEntregadaB: 0,
    creadoEnTick: tickActual,
    expiraEnTick: tickActual + TRUEQUE.plazoTicksPorDefecto,
    estado: 'activo',
  };
}

/**
 * Construye una caravana comercial propia (ampliación de comercio, a petición del usuario): a diferencia del
 * resto de tipos de caravana (todavía efímeros), esta es un activo PERSISTENTE que el asentamiento conserva y
 * cuenta contra su `cupoCaravanas` (Mercado + política "Ampliación de Flota", ver asentamientoQuery.ts) hasta
 * que se pierda capturada en combate (Doc 3.10) — no se puede desmantelar voluntariamente (confirmado con el
 * usuario). Nace 'disponible', parada en el propio asentamiento, lista para que `asignarCaravanasATrueque`
 * la asigne a un envío.
 */
export function construirCaravanaComercial(
  asentamiento: Asentamiento,
  caravanasExistentes: Caravana[],
  tickActual: number,
  contador = 0
): { asentamiento: Asentamiento; caravana: Caravana } {
  if (!tieneMercadoActivo(asentamiento)) {
    throw new CaravanaInvalidaError('El asentamiento necesita un Mercado activo para construir caravanas.');
  }
  if (!puedeCrearCaravana(asentamiento, tickActual)) {
    throw new CaravanaInvalidaError(
      `Cooldown de creación de caravanas: faltan ${ticksCooldownCaravanaRestantes(asentamiento, tickActual)} ticks para poder crear otra desde este asentamiento.`
    );
  }
  const cupo = cupoCaravanas(asentamiento);
  const propias = caravanasExistentes.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === asentamiento.id).length;
  if (propias >= cupo) {
    throw new CaravanaInvalidaError(`Cupo de caravanas alcanzado (${propias}/${cupo}).`);
  }
  const costo = CARAVANA_CATALOGO.comercial.costoConstruccion as Partial<Record<string, number>>;
  if (!tieneRecursos(asentamiento.almacen, costo)) {
    throw new CaravanaInvalidaError('No hay recursos suficientes para construir la caravana.');
  }
  const almacen = descontarRecursos(asentamiento.almacen, costo);
  const caravana: Caravana = {
    id: `caravana-comercial-${asentamiento.id}-${tickActual}-${contador}`,
    tipo: 'comercial',
    origenAsentamientoId: asentamiento.id,
    contenido: {},
    posicionActual: asentamiento.posicion,
    progreso: 0,
    estado: 'disponible',
  };
  return { asentamiento: { ...asentamiento, almacen, ultimaCaravanaCreadaEnTick: tickActual }, caravana };
}

/** Tasa base (Doc 3.5); si es externa, el Tesorero del destino (quien cobra la comisión) puede modularla. */
function tasaComision(origen: Asentamiento, destino: Asentamiento): number {
  if (origen.faccionId === destino.faccionId) return COMISION.tasaMismaFaccion;
  return COMISION.tasaExterna * factorComisionExterna(destino);
}

function bonusPorDistancia(dist: number): number {
  const proporcion = Math.min(1, dist / COMISION.distanciaParaBonusMax);
  return 1 + proporcion * (COMISION.bonusPorDistanciaMax - 1);
}

/** Avanza caravanas en tránsito: movimiento y, al llegar, entrega + comisión con bonificación por distancia (Doc 3.8). */
function avanzarCaravanas(
  caravanas: Caravana[],
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  zonas: readonly ZonaInfluencia[],
  asentamientosPorId: Map<string, Asentamiento>,
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  facciones: Faccion[],
  eventos: EventoCrudo[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const restantes: Caravana[] = [];

  for (const caravana of caravanas) {
    if (!caravana.destinoAsentamientoId) {
      // Sin destino todavía: o es una Caravana de Fundación (Doc 1.8, destino = punto del mapa, la avanza
      // `avanzarCaravanasFundacion` en engine/expansion.ts, no esta función), o es una caravana comercial
      // propia 'disponible' (ampliación de comercio, construida pero sin asignar todavía, Doc 3.2). Ambas se
      // dejan pasar sin tocar — una disponible no se mueve hasta que `asignarCaravanasATrueque` la asigne.
      restantes.push(caravana);
      continue;
    }
    const origen = asentamientosPorId.get(caravana.origenAsentamientoId);
    const destino = asentamientosPorId.get(caravana.destinoAsentamientoId);
    if (!origen || !destino) continue; // asentamiento desaparecido (fuera de alcance de Fase 0 aún)

    // Retorno real tras entregar (a petición del usuario: una caravana NUNCA se teletransporta) — recorre la
    // MISMA ruta que la llevó a `destino`, pero en sentido inverso, de vuelta a `origen` (su asentamiento de
    // origen permanente, ver `estado` en domain/types.ts). `origenAsentamientoId`/`destinoAsentamientoId` NO
    // se tocan durante el retorno (siguen siendo origen real / destino real de la entrega ya hecha) — solo se
    // invierten los puntos de inicio/fin del movimiento de este tick.
    const retornando = caravana.estado === 'retornando';
    const puntoInicio = retornando ? destino.posicion : origen.posicion;
    const puntoFin = retornando ? origen.posicion : destino.posicion;

    // Distancia en línea recta: base de la bonificación por distancia de la comisión (más abajo) — NO de
    // cuántos ticks tarda la caravana, que depende de la longitud real de la polilínea de `ruta` (puede
    // rodear terreno costoso).
    const distanciaTotal = Math.max(1, distancia(puntoInicio, puntoFin));
    const velocidadBase = CARAVANA_CATALOGO[caravana.tipo].velocidad;
    // "Rutas Rápidas" (Tesorero, ampliación de comercio) solo aplica a la flota comercial propia — no a
    // Caravanas de Fundación ni a los tipos todavía sin uso real (militar/contrabando, Doc 3.6).
    const velocidad = caravana.tipo === 'comercial' ? velocidadBase * factorVelocidadCaravana(origen) : velocidadBase;

    // Avance real por coste de terreno, sobre la longitud de la polilínea calculada al lanzar la caravana
    // (ver `asignarCaravanasATrueque`) — toda caravana en movimiento (con `destinoAsentamientoId`) trae `ruta`.
    // Bonus de Camino Comercial (Doc 1.6): si existe un camino ya construido para este par de asentamientos,
    // la caravana lo está siguiendo (ver `asignarCaravanasATrueque`, que reusa su polilínea como `ruta`) —
    // todo el trayecto cuenta como "sobre el camino". Simétrico en ambos sentidos: el mismo camino sirve para
    // ir y volver.
    const enCaminoComercial = buscarCamino(caminos, origen.id, destino.id) !== undefined;
    const factorCosteExtra = enCaminoComercial ? COSTE_MOVIMIENTO.factorCamino : 1;
    const avance = avanzarPosicionEnRuta(mapa, caravana.ruta!, caravana.progreso, velocidad, factorCosteExtra);
    const progreso = avance.progreso;
    const posicionActual = avance.posicion;

    if (progreso < 1) {
      restantes.push({ ...caravana, progreso, posicionActual });
      continue;
    }

    if (retornando) {
      // Llegada de vuelta a `origen`: disponible de nuevo para un nuevo envío — sin entrega, comisión ni
      // peaje (viaje vacío, no hay contenido que cobrar). `ruta` se limpia: la siguiente asignación calcula
      // una nueva desde `origen`.
      restantes.push({
        ...caravana,
        estado: 'disponible',
        contenido: {},
        destinoAsentamientoId: undefined,
        origenAcuerdoId: undefined,
        ladoAcuerdo: undefined,
        progreso: 0,
        posicionActual: origen.posicion,
        ruta: undefined,
      });
      continue;
    }

    // Llegada: entrega el contenido, cobra comisión de comercio con bonificación por distancia.
    let almacenDestino = destino.almacen;
    let valorTotal = 0;
    for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
      valorTotal += cantidad * calcularPrecioReferencia(recurso, [...asentamientosPorId.values()]);
      almacenDestino = agregarRecurso(almacenDestino, recurso, cantidad);
    }
    // Reputación (Doc 2.7, uso 1): tratar con una Facción de origen poco confiable deja peores términos —
    // el destino extrae más comisión de esa entrega.
    const faccionOrigen = facciones.find((f) => f.id === origen.faccionId);
    const factorReputacion = faccionOrigen ? factorComisionPorReputacion(faccionOrigen) : 1;
    const comision = valorTotal * tasaComision(origen, destino) * bonusPorDistancia(distanciaTotal) * factorReputacion;
    almacenDestino = agregarRecurso(almacenDestino, 'oro', comision);
    asentamientosPorId.set(destino.id, { ...destino, almacen: almacenDestino });

    eventos.push({
      codigo: 'comercio.caravana_llega',
      mensaje: `Caravana ${caravana.tipo} de ${origen.id} llega a ${destino.id}: entrega ${Object.entries(caravana.contenido)
        .map(([r, c]) => `${c.toFixed(0)} ${r}`)
        .join(', ')} (comisión +${comision.toFixed(1)} oro).`,
      payload: {
        caravanaId: caravana.id,
        tipo: caravana.tipo,
        origenId: origen.id,
        destinoId: destino.id,
        contenido: caravana.contenido,
        comision,
      } satisfies PayloadCaravanaLlega,
    });

    // Peaje de chokepoints (Doc 1.5, Fase 0.3): chokepoints controlados por una Facción rival que la ruta
    // atraviesa cobran al DESTINO en el momento de la entrega — ver `engine/chokepoints.ts`.
    const peajes = chokepointsDePeajeEnRuta(mapa.listarChokepoints(), zonas, caravana.ruta!, origen.faccionId, asentamientosPorId);
    if (peajes.length > 0) {
      const chokepointsPorControlador = new Map<string, number>();
      for (const p of peajes) chokepointsPorControlador.set(p.controladorId, (chokepointsPorControlador.get(p.controladorId) ?? 0) + 1);

      for (const [controladorId, cantidadChokepoints] of chokepointsPorControlador) {
        if (controladorId === destino.id) continue; // el propio destino domina el paso: no se cobra a sí mismo
        const controlador = asentamientosPorId.get(controladorId);
        if (!controlador) continue;
        const monto = Math.min(cantidadChokepoints * CHOKEPOINTS_PEAJE.oro, cantidadDisponible(almacenDestino, 'oro'));
        if (monto <= 0) continue;
        almacenDestino = descontarRecursos(almacenDestino, { oro: monto });
        asentamientosPorId.set(controlador.id, { ...controlador, almacen: agregarRecurso(controlador.almacen, 'oro', monto) });
        eventos.push({
          codigo: 'comercio.peaje',
          mensaje: `Peaje: ${destino.id} paga ${monto.toFixed(1)} oro a ${controlador.id} por ${cantidadChokepoints} chokepoint(s) en la ruta.`,
          payload: { destinoId: destino.id, monto, controladorId: controlador.id, cantidadChokepoints } satisfies PayloadPeaje,
        });
      }
      asentamientosPorId.set(destino.id, { ...destino, almacen: almacenDestino });
    }

    if (caravana.origenAcuerdoId && caravana.ladoAcuerdo) {
      const acuerdo = acuerdosPorId.get(caravana.origenAcuerdoId);
      if (acuerdo) {
        const entregado = Object.values(caravana.contenido)[0] ?? 0;
        const actualizado: AcuerdoTrueque =
          caravana.ladoAcuerdo === 'A'
            ? { ...acuerdo, cantidadEntregadaA: acuerdo.cantidadEntregadaA + entregado }
            : { ...acuerdo, cantidadEntregadaB: acuerdo.cantidadEntregadaB + entregado };
        if (actualizado.cantidadEntregadaA >= actualizado.cantidadTotalA && actualizado.cantidadEntregadaB >= actualizado.cantidadTotalB) {
          actualizado.estado = 'cumplido';
          eventos.push({
            codigo: 'comercio.trueque_cumplido',
            mensaje: `Trueque ${acuerdo.id} cumplido entre ${acuerdo.asentamientoAId} y ${acuerdo.asentamientoBId}.`,
            payload: {
              acuerdoId: acuerdo.id,
              asentamientoAId: acuerdo.asentamientoAId,
              asentamientoBId: acuerdo.asentamientoBId,
            } satisfies PayloadTruequeCumplido,
          });
          const faccionA = asentamientosPorId.get(acuerdo.asentamientoAId)?.faccionId;
          const faccionB = asentamientosPorId.get(acuerdo.asentamientoBId)?.faccionId;
          if (faccionA) ajustesReputacion.push({ faccionId: faccionA, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
          if (faccionB) ajustesReputacion.push({ faccionId: faccionB, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
        }
        acuerdosPorId.set(acuerdo.id, actualizado);
      }
    }

    if (caravana.tipo === 'comercial') {
      // Flota propia (ampliación de comercio): en vez de desaparecer O de reaparecer instantáneamente en
      // `origen` (bug corregido a petición del usuario), arranca el viaje de vuelta real — misma `ruta`
      // invertida, ver `retornando` arriba — y solo al completarlo vuelve a 'disponible'. Es un activo
      // persistente y con costo (`construirCaravanaComercial`), no un objeto de un solo uso como el resto de
      // tipos de caravana.
      restantes.push({
        ...caravana,
        estado: 'retornando',
        contenido: {},
        origenAcuerdoId: undefined,
        ladoAcuerdo: undefined,
        progreso: 0,
        posicionActual: destino.posicion,
        ruta: [...caravana.ruta!].reverse(),
      });
    }
    // Resto de tipos (militar/contrabando, sin uso real todavía, Doc 3.6; Caravana de Fundación no llega
    // aquí, ver arriba): comportamiento sin cambios, la caravana se pierde al entregar.
  }

  return restantes;
}

interface LadoPendiente {
  acuerdo: AcuerdoTrueque;
  lado: 'A' | 'B';
  origenId: string;
  destinoId: string;
  recurso: string;
  total: number;
  entregado: number;
}

/**
 * Score de prioridad (ampliación de comercio, a petición del usuario) para decidir, cuando hay menos
 * caravanas disponibles que envíos pendientes en un mismo asentamiento, cuál se sirve primero. Tres factores
 * normalizados a 0-100 y ponderados (ver `ASIGNACION_CARAVANA`, constants.ts):
 * - Urgencia por expiración: cuánto del plazo del acuerdo ya se consumió.
 * - Urgencia por volumen: qué fracción del total pactado sigue pendiente.
 * - Cercanía: destinos más cercanos rinden más envíos por caravana disponible (round-trip más corto).
 */
function scoreAsignacion(l: LadoPendiente, origen: Asentamiento, destino: Asentamiento, tickActual: number): number {
  const plazoTotal = l.acuerdo.expiraEnTick - l.acuerdo.creadoEnTick;
  const ticksRestantes = Math.max(0, l.acuerdo.expiraEnTick - tickActual);
  const urgenciaExpiracion = plazoTotal > 0 ? 100 * (1 - ticksRestantes / plazoTotal) : 100;
  const pendiente = l.total - l.entregado;
  const urgenciaVolumen = l.total > 0 ? 100 * Math.min(1, pendiente / l.total) : 0;
  const dist = distancia(origen.posicion, destino.posicion);
  const cercania = 100 * (1 - Math.min(1, dist / ASIGNACION_CARAVANA.distanciaReferencia));
  return (
    urgenciaExpiracion * ASIGNACION_CARAVANA.pesoUrgenciaExpiracion +
    urgenciaVolumen * ASIGNACION_CARAVANA.pesoUrgenciaVolumen +
    cercania * ASIGNACION_CARAVANA.pesoCercania
  );
}

/**
 * Asigna caravanas 'disponibles' de la flota propia a los lados pendientes de trueque (ampliación de
 * comercio, a petición del usuario — "solo simulación", Doc 3.2: en el diseño objetivo el jugador elige la
 * caravana, la carga y la escolta a mano; esto es el sustituto automático de Fase 0). Ya NO crea caravanas de
 * la nada como antes — si un asentamiento no tiene ninguna disponible este tick, el envío simplemente espera.
 * Cuando compiten varios envíos por menos caravanas de las que hacen falta, se asignan por `scoreAsignacion`
 * descendente, caravana por caravana, hasta agotar el pool disponible de ese asentamiento.
 */
function asignarCaravanasATrueque(
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  asentamientosPorId: Map<string, Asentamiento>,
  caravanas: Caravana[],
  tickActual: number,
  eventos: EventoCrudo[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const pendientesPorOrigen = new Map<string, LadoPendiente[]>();

  for (const acuerdo of acuerdosPorId.values()) {
    if (acuerdo.estado !== 'activo') continue;
    if (tickActual >= acuerdo.expiraEnTick) {
      acuerdosPorId.set(acuerdo.id, { ...acuerdo, estado: 'expirado' });
      eventos.push({
        codigo: 'comercio.trueque_expirado',
        mensaje: `Trueque ${acuerdo.id} expiró sin completarse.`,
        payload: { acuerdoId: acuerdo.id } satisfies PayloadTruequeExpirado,
      });
      // Incumplir un acuerdo aceptado penaliza reputación (Doc 2.7) — solo al lado que no entregó su cupo.
      const faccionA = asentamientosPorId.get(acuerdo.asentamientoAId)?.faccionId;
      const faccionB = asentamientosPorId.get(acuerdo.asentamientoBId)?.faccionId;
      if (faccionA && acuerdo.cantidadEntregadaA < acuerdo.cantidadTotalA) {
        ajustesReputacion.push({ faccionId: faccionA, delta: REPUTACION.penalizacionTruequeIncumplido, razon: 'trueque incumplido' });
      }
      if (faccionB && acuerdo.cantidadEntregadaB < acuerdo.cantidadTotalB) {
        ajustesReputacion.push({ faccionId: faccionB, delta: REPUTACION.penalizacionTruequeIncumplido, razon: 'trueque incumplido' });
      }
      continue;
    }

    const lados: LadoPendiente[] = [
      { acuerdo, lado: 'A', origenId: acuerdo.asentamientoAId, destinoId: acuerdo.asentamientoBId, recurso: acuerdo.recursoA, total: acuerdo.cantidadTotalA, entregado: acuerdo.cantidadEntregadaA },
      { acuerdo, lado: 'B', origenId: acuerdo.asentamientoBId, destinoId: acuerdo.asentamientoAId, recurso: acuerdo.recursoB, total: acuerdo.cantidadTotalB, entregado: acuerdo.cantidadEntregadaB },
    ];

    for (const l of lados) {
      if (l.entregado >= l.total) continue;
      const yaEnTransito = caravanas.some((c) => c.origenAcuerdoId === acuerdo.id && c.ladoAcuerdo === l.lado && c.estado === 'en_transito');
      if (yaEnTransito) continue;
      const arr = pendientesPorOrigen.get(l.origenId) ?? [];
      arr.push(l);
      pendientesPorOrigen.set(l.origenId, arr);
    }
  }

  const caravanasPorId = new Map(caravanas.map((c) => [c.id, c]));

  for (const [origenId, pendientes] of pendientesPorOrigen) {
    const origen = asentamientosPorId.get(origenId);
    if (!origen) continue;

    const disponibles = caravanas.filter((c) => c.tipo === 'comercial' && c.estado === 'disponible' && c.origenAsentamientoId === origenId);
    if (disponibles.length === 0) continue;

    const conScore = pendientes
      .map((l) => {
        const destino = asentamientosPorId.get(l.destinoId);
        return destino ? { l, destino, score: scoreAsignacion(l, origen, destino, tickActual) } : null;
      })
      .filter((x): x is { l: LadoPendiente; destino: Asentamiento; score: number } => x !== null)
      .sort((a, b) => b.score - a.score);

    let idx = 0;
    for (const { l, destino } of conScore) {
      if (idx >= disponibles.length) break;
      const caravana = disponibles[idx]!;

      const disponibleStock = cantidadDisponible(origen.almacen, l.recurso);
      const pendiente = l.total - l.entregado;
      const capacidad = CARAVANA_CATALOGO.comercial.capacidad * factorCapacidadCaravana(origen);
      const cantidad = Math.min(disponibleStock, pendiente, capacidad);
      if (cantidad <= 0) continue; // sin stock suficiente todavía: la caravana se reintenta el siguiente tick, no consume su turno

      idx++;
      // Ruta calculada al lanzar (Fase 0.3): rodea terreno costoso en vez de ir en línea recta — ver
      // `world/rutas.ts`. Si ya existe un Camino Comercial para este par (Doc 1.6, ver `engine/caminos.ts`),
      // reusa su polilínea en vez de recalcular — es literalmente "seguir el camino ya construido", y es lo
      // que le da el bonus de velocidad en `avanzarCaravanas` (mismo par -> mismo camino encontrado).
      // `buscarCamino` empareja el par en CUALQUIER orden pero devuelve `puntos` siempre en el orden en que
      // se guardó (`asentamientoAId` -> `asentamientoBId`) — bug corregido a petición del usuario: cuando el
      // camino se reutiliza en el sentido CONTRARIO (esta caravana va de B a A), había que invertir la
      // polilínea; si no, `progreso: 0` caía en el extremo del camino más cercano al DESTINO en vez del
      // propio origen, y la caravana "saltaba" allí en su primer paso de movimiento (ver `avanzarPosicionEnRuta`,
      // que siempre mide el progreso desde `ruta[0]` hacia adelante).
      const caminoExistente = buscarCamino(caminos, origen.id, destino.id);
      const rutaOrientada = caminoExistente
        ? caminoExistente.asentamientoAId === origen.id
          ? caminoExistente.puntos
          : [...caminoExistente.puntos].reverse()
        : undefined;
      asentamientosPorId.set(origen.id, { ...origen, almacen: descontarRecursos(origen.almacen, { [l.recurso]: cantidad }) });
      caravanasPorId.set(caravana.id, {
        ...caravana,
        estado: 'en_transito',
        destinoAsentamientoId: l.destinoId,
        contenido: { [l.recurso]: cantidad },
        origenAcuerdoId: l.acuerdo.id,
        ladoAcuerdo: l.lado,
        posicionActual: origen.posicion,
        progreso: 0,
        ruta: rutaOrientada ?? calcularRuta(mapa, origen.posicion, destino.posicion),
      });
      eventos.push({
        codigo: 'comercio.caravana_sale',
        mensaje: `Caravana comercial de ${origenId} sale hacia ${destino.id} con ${cantidad.toFixed(0)} ${l.recurso}.`,
        payload: { origenId, destinoId: destino.id, cantidad, recurso: l.recurso } satisfies PayloadCaravanaSale,
      });
    }
  }

  return [...caravanasPorId.values()];
}

export function avanzarComercio(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  caravanas: Caravana[],
  acuerdos: AcuerdoTrueque[],
  mapa: Mapa,
  caminos: readonly CaminoComercial[],
  zonas: readonly ZonaInfluencia[],
  tickActual: number
): { asentamientos: Asentamiento[]; facciones: Faccion[]; caravanas: Caravana[]; acuerdos: AcuerdoTrueque[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const ajustesReputacion: AjusteReputacion[] = [];
  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, { ...a }]));
  const acuerdosPorId = new Map(acuerdos.map((a) => [a.id, a]));

  const trasMovimiento = avanzarCaravanas(caravanas, mapa, caminos, zonas, asentamientosPorId, acuerdosPorId, facciones, eventos, ajustesReputacion);
  const trasAsignacion = asignarCaravanasATrueque(mapa, caminos, acuerdosPorId, asentamientosPorId, trasMovimiento, tickActual, eventos, ajustesReputacion);

  return {
    asentamientos: asentamientos.map((a) => asentamientosPorId.get(a.id)!),
    facciones: aplicarAjustesReputacion(facciones, ajustesReputacion),
    caravanas: trasAsignacion,
    acuerdos: [...acuerdosPorId.values()],
    eventos,
  };
}
