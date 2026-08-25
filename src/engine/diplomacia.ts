import type { AcuerdoTrueque, Asentamiento, Faccion, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payload de `diplomacia.tributo_pagado` (ver `avanzarTributos`). */
export interface PayloadTributoPagado {
  pagadorId: string;
  señoraId: string;
  recurso: string;
  cantidad: number;
}
import { REPUTACION } from '../constants';
import { agregarRecurso, cantidadDisponible, descontarRecursos } from './almacen';
import { aplicarAjustesReputacion, puedeProponerAlianza } from './reputacion';

export class DiplomaciaInvalidaError extends Error {}

function existeRelacionActiva(relaciones: RelacionPolitica[], aId: string, bId: string): boolean {
  return relaciones.some(
    (r) => r.estado === 'activa' && ((r.faccionAId === aId && r.faccionBId === bId) || (r.faccionAId === bId && r.faccionBId === aId))
  );
}

function validarPar(facciones: Faccion[], aId: string, bId: string, relaciones: RelacionPolitica[]): void {
  if (aId === bId) throw new DiplomaciaInvalidaError('Una Facción no puede relacionarse consigo misma.');
  if (!facciones.some((f) => f.id === aId) || !facciones.some((f) => f.id === bId)) {
    throw new DiplomaciaInvalidaError('Alguna de las Facciones no existe.');
  }
  if (existeRelacionActiva(relaciones, aId, bId)) {
    throw new DiplomaciaInvalidaError('Ya existe una relación activa entre estas Facciones.');
  }
}

/**
 * Vasallaje (Doc 2.4): "protección a cambio de tributo pactado en recursos específicos". Fase 0 acepta la
 * propuesta al instante (sin Embajador interactivo real todavía) y el tributo se paga automáticamente cada tick.
 */
export function proponerVasallaje(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  faccionSeñoraId: string,
  faccionVasallaId: string,
  tributoRecurso: string,
  tributoCantidadPorTick: number,
  tickActual: number,
  contador = 0
): RelacionPolitica {
  validarPar(facciones, faccionSeñoraId, faccionVasallaId, relaciones);
  return {
    id: `vasallaje-${faccionSeñoraId}-${faccionVasallaId}-${tickActual}-${contador}`,
    tipo: 'vasallaje',
    faccionAId: faccionSeñoraId,
    faccionBId: faccionVasallaId,
    tributo: { recurso: tributoRecurso, cantidadPorTick: tributoCantidadPorTick },
    creadoEnTick: tickActual,
    estado: 'activa',
  };
}

/** Alianza (Doc 2.3): relación LIBRE y simétrica entre iguales, revocable en cualquier momento. */
export function proponerAlianza(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  faccionAId: string,
  faccionBId: string,
  tickActual: number,
  contador = 0
): RelacionPolitica {
  validarPar(facciones, faccionAId, faccionBId, relaciones);
  const proponente = facciones.find((f) => f.id === faccionAId)!;
  if (!puedeProponerAlianza(proponente)) {
    throw new DiplomaciaInvalidaError(
      `Reputación demasiado baja (${proponente.reputacion.toFixed(0)}) para que el Embajador proponga alianzas (Doc 2.7).`
    );
  }
  return {
    id: `alianza-${faccionAId}-${faccionBId}-${tickActual}-${contador}`,
    tipo: 'alianza',
    faccionAId,
    faccionBId,
    creadoEnTick: tickActual,
    estado: 'activa',
  };
}

/**
 * Ruptura vía 2 (Doc 2.4): liberación voluntaria por el señor (vasallaje, bonus de reputación para la señora,
 * Doc 2.7) o fin unilateral de una Alianza (penalización de reputación para quien la rompe, `iniciadorFaccionId`).
 */
export function romperRelacion(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  relacionId: string,
  iniciadorFaccionId?: string
): { facciones: Faccion[]; relaciones: RelacionPolitica[] } {
  const relacion = relaciones.find((r) => r.id === relacionId);
  if (!relacion) throw new DiplomaciaInvalidaError('La relación no existe.');

  const ajustes =
    relacion.tipo === 'vasallaje'
      ? [{ faccionId: relacion.faccionAId, delta: REPUTACION.bonusLiberarVasalloVoluntario, razon: 'liberar vasallo voluntariamente' }]
      : [
          {
            faccionId: iniciadorFaccionId ?? relacion.faccionAId,
            delta: REPUTACION.penalizacionRomperAlianza,
            razon: 'romper alianza unilateralmente',
          },
        ];

  return {
    facciones: aplicarAjustesReputacion(facciones, ajustes),
    relaciones: relaciones.map((r) => (r.id === relacionId ? { ...r, estado: 'rota' as const } : r)),
  };
}

/**
 * Ruptura vía 1 (Doc 2.4): rebelión forzada del vasallo. Cancela de inmediato los acuerdos de trueque vigentes
 * entre asentamientos de ambas Facciones (la declaración de guerra automática es mecánica de combate, Sprint 5).
 */
export function rebelionVasallo(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  acuerdos: AcuerdoTrueque[],
  asentamientos: Asentamiento[],
  relacionId: string
): { facciones: Faccion[]; relaciones: RelacionPolitica[]; acuerdos: AcuerdoTrueque[]; eventos: string[] } {
  const relacion = relaciones.find((r) => r.id === relacionId);
  if (!relacion || relacion.tipo !== 'vasallaje') {
    throw new DiplomaciaInvalidaError('La relación no existe o no es un vasallaje.');
  }
  const idsSeñora = new Set(asentamientos.filter((a) => a.faccionId === relacion.faccionAId).map((a) => a.id));
  const idsVasalla = new Set(asentamientos.filter((a) => a.faccionId === relacion.faccionBId).map((a) => a.id));

  const acuerdosActualizados = acuerdos.map((ac) => {
    const cruzaAmbas =
      (idsSeñora.has(ac.asentamientoAId) && idsVasalla.has(ac.asentamientoBId)) ||
      (idsVasalla.has(ac.asentamientoAId) && idsSeñora.has(ac.asentamientoBId));
    return cruzaAmbas && ac.estado === 'activo' ? { ...ac, estado: 'expirado' as const } : ac;
  });

  // "Rebelión de vasallo por incumplimiento del señor" (Doc 2.7): penaliza a la señora (proxy simplificado,
  // Fase 0 no rastrea la causa exacta del incumplimiento, solo que la rebelión ocurrió bajo su liderazgo).
  const faccionesActualizadas = aplicarAjustesReputacion(facciones, [
    { faccionId: relacion.faccionAId, delta: REPUTACION.penalizacionRebelionParaSenora, razon: 'rebelión de vasallo' },
  ]);

  return {
    facciones: faccionesActualizadas,
    relaciones: relaciones.map((r) => (r.id === relacionId ? { ...r, estado: 'rota' as const } : r)),
    acuerdos: acuerdosActualizados,
    eventos: [`Rebelión: el vasallaje ${relacionId} se rompe y se cancelan sus acuerdos comerciales vigentes.`],
  };
}

/** Tributo periódico (Doc 2.4): del asentamiento con más stock del recurso pactado al primer asentamiento del señor. */
export function avanzarTributos(
  relaciones: RelacionPolitica[],
  asentamientos: Asentamiento[]
): { asentamientos: Asentamiento[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const asentamientosPorId = new Map(asentamientos.map((a) => [a.id, { ...a }]));

  for (const relacion of relaciones) {
    if (relacion.estado !== 'activa' || relacion.tipo !== 'vasallaje' || !relacion.tributo) continue;

    const señora = [...asentamientosPorId.values()].filter((a) => a.faccionId === relacion.faccionAId)[0];
    const pagadores = [...asentamientosPorId.values()]
      .filter((a) => a.faccionId === relacion.faccionBId)
      .sort((a, b) => cantidadDisponible(b.almacen, relacion.tributo!.recurso) - cantidadDisponible(a.almacen, relacion.tributo!.recurso));
    const pagador = pagadores[0];
    if (!señora || !pagador) continue;

    const disponible = cantidadDisponible(pagador.almacen, relacion.tributo.recurso);
    const cantidad = Math.min(disponible, relacion.tributo.cantidadPorTick);
    if (cantidad <= 0) continue;

    asentamientosPorId.set(pagador.id, { ...pagador, almacen: descontarRecursos(pagador.almacen, { [relacion.tributo.recurso]: cantidad }) });
    asentamientosPorId.set(señora.id, { ...señora, almacen: agregarRecurso(señora.almacen, relacion.tributo.recurso, cantidad) });
    eventos.push({
      codigo: 'diplomacia.tributo_pagado',
      mensaje: `Tributo: ${pagador.id} paga ${cantidad.toFixed(1)} ${relacion.tributo.recurso} a ${señora.id}.`,
      payload: {
        pagadorId: pagador.id,
        señoraId: señora.id,
        recurso: relacion.tributo.recurso,
        cantidad,
      } satisfies PayloadTributoPagado,
    });
  }

  return { asentamientos: asentamientos.map((a) => asentamientosPorId.get(a.id)!), eventos };
}
