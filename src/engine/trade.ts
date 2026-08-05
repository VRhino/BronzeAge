import type { AcuerdoTrueque, Asentamiento, Caravana, Faccion, Point } from '../domain/types';
import { CARAVANA_CATALOGO, COMISION, REPUTACION, TRUEQUE } from '../constants';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { calcularPrecioReferencia } from './market';
import { factorComisionExterna } from './politicas';
import { aplicarAjustesReputacion, factorComisionPorReputacion, type AjusteReputacion } from './reputacion';

export class TruequeInvalidoError extends Error {}

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
  asentamientosPorId: Map<string, Asentamiento>,
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  facciones: Faccion[],
  eventos: string[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const restantes: Caravana[] = [];

  for (const caravana of caravanas) {
    const origen = asentamientosPorId.get(caravana.origenAsentamientoId);
    const destino = asentamientosPorId.get(caravana.destinoAsentamientoId);
    if (!origen || !destino) continue; // asentamiento desaparecido (fuera de alcance de Fase 0 aún)

    const distanciaTotal = Math.max(1, distancia(origen.posicion, destino.posicion));
    const velocidad = CARAVANA_CATALOGO[caravana.tipo].velocidad;
    const progreso = Math.min(1, caravana.progreso + velocidad / distanciaTotal);

    if (progreso < 1) {
      restantes.push({
        ...caravana,
        progreso,
        posicionActual: {
          x: origen.posicion.x + (destino.posicion.x - origen.posicion.x) * progreso,
          y: origen.posicion.y + (destino.posicion.y - origen.posicion.y) * progreso,
        },
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

    eventos.push(
      `Caravana ${caravana.tipo} de ${origen.id} llega a ${destino.id}: entrega ${Object.entries(caravana.contenido)
        .map(([r, c]) => `${c.toFixed(0)} ${r}`)
        .join(', ')} (comisión +${comision.toFixed(1)} oro).`
    );

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
          eventos.push(`Trueque ${acuerdo.id} cumplido entre ${acuerdo.asentamientoAId} y ${acuerdo.asentamientoBId}.`);
          const faccionA = asentamientosPorId.get(acuerdo.asentamientoAId)?.faccionId;
          const faccionB = asentamientosPorId.get(acuerdo.asentamientoBId)?.faccionId;
          if (faccionA) ajustesReputacion.push({ faccionId: faccionA, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
          if (faccionB) ajustesReputacion.push({ faccionId: faccionB, delta: REPUTACION.bonusTruequeCumplido, razon: 'trueque cumplido' });
        }
        acuerdosPorId.set(acuerdo.id, actualizado);
      }
    }
  }

  return restantes;
}

/** Despacha nuevas caravanas para acuerdos activos que aún tengan cupo pendiente y stock disponible en origen. */
function despacharTrueques(
  acuerdosPorId: Map<string, AcuerdoTrueque>,
  asentamientosPorId: Map<string, Asentamiento>,
  caravanasEnTransito: Caravana[],
  tickActual: number,
  eventos: string[],
  ajustesReputacion: AjusteReputacion[]
): Caravana[] {
  const nuevas: Caravana[] = [];
  let contador = 0;

  for (const acuerdo of acuerdosPorId.values()) {
    if (acuerdo.estado !== 'activo') continue;
    if (tickActual >= acuerdo.expiraEnTick) {
      acuerdosPorId.set(acuerdo.id, { ...acuerdo, estado: 'expirado' });
      eventos.push(`Trueque ${acuerdo.id} expiró sin completarse.`);
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

    const lados: Array<{ lado: 'A' | 'B'; origenId: string; destinoId: string; recurso: string; total: number; entregado: number }> = [
      { lado: 'A', origenId: acuerdo.asentamientoAId, destinoId: acuerdo.asentamientoBId, recurso: acuerdo.recursoA, total: acuerdo.cantidadTotalA, entregado: acuerdo.cantidadEntregadaA },
      { lado: 'B', origenId: acuerdo.asentamientoBId, destinoId: acuerdo.asentamientoAId, recurso: acuerdo.recursoB, total: acuerdo.cantidadTotalB, entregado: acuerdo.cantidadEntregadaB },
    ];

    for (const l of lados) {
      if (l.entregado >= l.total) continue;
      const enTransito = caravanasEnTransito.some((c) => c.origenAcuerdoId === acuerdo.id && c.ladoAcuerdo === l.lado);
      if (enTransito) continue;

      const origen = asentamientosPorId.get(l.origenId);
      if (!origen) continue;
      const disponible = cantidadDisponible(origen.almacen, l.recurso);
      const pendiente = l.total - l.entregado;
      const capacidad = CARAVANA_CATALOGO.comercial.capacidad;
      const cantidad = Math.min(disponible, pendiente, capacidad);
      if (cantidad <= 0) continue;

      asentamientosPorId.set(origen.id, { ...origen, almacen: descontarRecursos(origen.almacen, { [l.recurso]: cantidad }) });
      nuevas.push({
        id: `caravana-${acuerdo.id}-${l.lado}-${tickActual}-${contador++}`,
        tipo: 'comercial',
        origenAsentamientoId: l.origenId,
        destinoAsentamientoId: l.destinoId,
        contenido: { [l.recurso]: cantidad },
        posicionActual: origen.posicion,
        progreso: 0,
        origenAcuerdoId: acuerdo.id,
        ladoAcuerdo: l.lado,
      });
      eventos.push(`Caravana comercial parte de ${l.origenId} hacia ${l.destinoId} con ${cantidad.toFixed(0)} ${l.recurso}.`);
    }
  }

  return nuevas;
}

export function avanzarComercio(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  caravanas: Caravana[],
  acuerdos: AcuerdoTrueque[],
  tickActual: number
): { asentamientos: Asentamiento[]; facciones: Faccion[]; caravanas: Caravana[]; acuerdos: AcuerdoTrueque[]; eventos: string[] } {
  const eventos: string[] = [];
  const ajustesReputacion: AjusteReputacion[] = [];
  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, { ...a }]));
  const acuerdosPorId = new Map(acuerdos.map((a) => [a.id, a]));

  const enTransitoTrasEntrega = avanzarCaravanas(caravanas, asentamientosPorId, acuerdosPorId, facciones, eventos, ajustesReputacion);
  const nuevas = despacharTrueques(acuerdosPorId, asentamientosPorId, enTransitoTrasEntrega, tickActual, eventos, ajustesReputacion);

  return {
    asentamientos: asentamientos.map((a) => asentamientosPorId.get(a.id)!),
    facciones: aplicarAjustesReputacion(facciones, ajustesReputacion),
    caravanas: [...enTransitoTrasEntrega, ...nuevas],
    acuerdos: [...acuerdosPorId.values()],
    eventos,
  };
}
