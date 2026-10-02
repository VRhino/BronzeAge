import type { AcuerdoTrueque, Asentamiento, Faccion, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import type { Instante } from '../domain/tiempo';

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
import { ReglaInvalidaError } from './errores';

/** Payload de `rebelionVasallo` — comando de jugador, no tick (ver `session/comandos/diplomacia.ts`). */
export interface PayloadRebelionVasallo {
  relacionId: string;
  faccionSenoraId: string;
  faccionVasallaId: string;
}

export class DiplomaciaInvalidaError extends ReglaInvalidaError {}

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
  tributoCantidadPorMinuto: number,
  instante: Instante,
  contador = 0
): RelacionPolitica {
  validarPar(facciones, faccionSeñoraId, faccionVasallaId, relaciones);
  return {
    id: `vasallaje-${faccionSeñoraId}-${faccionVasallaId}-${contador}`,
    tipo: 'vasallaje',
    faccionAId: faccionSeñoraId,
    faccionBId: faccionVasallaId,
    tributo: { recurso: tributoRecurso, cantidadPorMinuto: tributoCantidadPorMinuto },
    creadoEn: instante,
    estado: 'activa',
  };
}

/** Alianza (Doc 2.3): relación LIBRE y simétrica entre iguales, revocable en cualquier momento. */
export function proponerAlianza(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  faccionAId: string,
  faccionBId: string,
  instante: Instante,
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
    id: `alianza-${faccionAId}-${faccionBId}-${contador}`,
    tipo: 'alianza',
    faccionAId,
    faccionBId,
    creadoEn: instante,
    estado: 'activa',
  };
}

/**
 * Guerra (Doc 2.4.1): libre, sin condición previa. Se arrastra por vasallaje (Doc 2.4): quien declara guerra a un
 * vasallo la declara también a su señor, y a un señor, a todos sus vasallos. Las Facciones del bando contrario que
 * ya tienen alguna relación activa con quien declara (alianza, vasallaje o guerra) no se arrastran; solo falla la
 * declaración si falla el objetivo directo.
 */
export function declararGuerra(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  atacanteId: string,
  objetivoId: string,
  instante: Instante,
  contador = 0
): RelacionPolitica[] {
  validarPar(facciones, atacanteId, objetivoId, relaciones);
  const vasallajes = relaciones.filter((r) => r.estado === 'activa' && r.tipo === 'vasallaje');
  const señorId = vasallajes.find((r) => r.faccionBId === objetivoId)?.faccionAId ?? objetivoId;
  const bando = new Set([señorId, objetivoId, ...vasallajes.filter((r) => r.faccionAId === señorId).map((r) => r.faccionBId)]);
  return [...bando]
    .filter((id) => id !== atacanteId && (id === objetivoId || !existeRelacionActiva(relaciones, atacanteId, id)))
    .map((id) => ({
      id: `guerra-${atacanteId}-${id}-${contador}`,
      tipo: 'guerra' as const,
      faccionAId: atacanteId,
      faccionBId: id,
      creadoEn: instante,
      estado: 'activa' as const,
    }));
}

/**
 * Paz (Doc 2.4.1): mutua. La primera Facción en ofrecerla deja la propuesta sobre la guerra; cuando la ofrece la
 * otra, la guerra se acaba. Devuelve si se firmó.
 */
export function proponerPaz(relaciones: RelacionPolitica[], relacionId: string, faccionId: string): { relaciones: RelacionPolitica[]; firmada: boolean } {
  const guerra = relaciones.find((r) => r.id === relacionId);
  if (!guerra || guerra.tipo !== 'guerra' || guerra.estado !== 'activa') throw new DiplomaciaInvalidaError('No hay una guerra activa con ese id.');
  if (guerra.faccionAId !== faccionId && guerra.faccionBId !== faccionId) throw new DiplomaciaInvalidaError('Esa Facción no está en esa guerra.');
  if (guerra.pazPropuestaPor === faccionId) throw new DiplomaciaInvalidaError('Esa Facción ya ofreció la paz; falta que responda la otra.');
  const firmada = guerra.pazPropuestaPor !== undefined;
  const nueva: RelacionPolitica = firmada ? { ...guerra, estado: 'rota' } : { ...guerra, pazPropuestaPor: faccionId };
  return { relaciones: relaciones.map((r) => (r.id === relacionId ? nueva : r)), firmada };
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
  if (relacion.tipo === 'guerra') throw new DiplomaciaInvalidaError('Una guerra no se rompe: se acaba con la paz (`proponerPaz`).');

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
 * entre asentamientos de ambas Facciones y declara la guerra del vasallo contra su señor y el resto de sus vasallos (Doc 2.4).
 */
export function rebelionVasallo(
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  acuerdos: AcuerdoTrueque[],
  asentamientos: Asentamiento[],
  relacionId: string,
  instante: Instante,
  contador = 0
): { facciones: Faccion[]; relaciones: RelacionPolitica[]; acuerdos: AcuerdoTrueque[]; eventos: EventoCrudo[] } {
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

  const rotas = relaciones.map((r) => (r.id === relacionId ? { ...r, estado: 'rota' as const } : r));
  const guerras = declararGuerra(facciones, rotas, relacion.faccionBId, relacion.faccionAId, instante, contador);

  return {
    facciones: faccionesActualizadas,
    relaciones: [...rotas, ...guerras],
    acuerdos: acuerdosActualizados,
    eventos: [
      {
        codigo: 'diplomacia.rebelion_vasallo',
        mensaje: `Rebelión: el vasallaje ${relacionId} se rompe, se cancelan sus acuerdos comerciales vigentes y empieza la guerra.`,
        payload: {
          relacionId,
          faccionSenoraId: relacion.faccionAId,
          faccionVasallaId: relacion.faccionBId,
        } satisfies PayloadRebelionVasallo,
      },
    ],
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
    const cantidad = Math.min(disponible, relacion.tributo.cantidadPorMinuto);
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
