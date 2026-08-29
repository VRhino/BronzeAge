import type { Faccion, RelacionPolitica } from '../domain/types';
import { REPUTACION } from '../constants';

export interface AjusteReputacion {
  faccionId: string;
  delta: number;
  razon: string;
}

export function ajustarReputacion(faccion: Faccion, delta: number): Faccion {
  return { ...faccion, reputacion: Math.max(-100, Math.min(100, faccion.reputacion + delta)) };
}

/** Aplica una lista de ajustes puntuales (Doc 2.7: eventos de trueque/diplomacia/guerra) sobre las Facciones. */
export function aplicarAjustesReputacion(facciones: Faccion[], ajustes: AjusteReputacion[]): Faccion[] {
  if (ajustes.length === 0) return facciones;
  const porId = new Map(facciones.map((f) => [f.id, f]));
  for (const { faccionId, delta } of ajustes) {
    const faccion = porId.get(faccionId);
    if (faccion) porId.set(faccionId, ajustarReputacion(faccion, delta));
  }
  return facciones.map((f) => porId.get(f.id)!);
}

/** Decaimiento hacia 0 (Doc 2.7) + trickle por mantener una Alianza activa mucho tiempo. */
export function avanzarReputacion(facciones: Faccion[], relaciones: RelacionPolitica[]): Faccion[] {
  const bonusPorFaccion = new Map<string, number>();
  for (const r of relaciones) {
    if (r.estado !== 'activa' || r.tipo !== 'alianza') continue;
    bonusPorFaccion.set(r.faccionAId, (bonusPorFaccion.get(r.faccionAId) ?? 0) + REPUTACION.bonusPorMinutoAlianzaActiva);
    bonusPorFaccion.set(r.faccionBId, (bonusPorFaccion.get(r.faccionBId) ?? 0) + REPUTACION.bonusPorMinutoAlianzaActiva);
  }

  return facciones.map((f) => {
    let rep = f.reputacion;
    if (rep > 0) rep = Math.max(0, rep - REPUTACION.decaimientoPorMinuto);
    else if (rep < 0) rep = Math.min(0, rep + REPUTACION.decaimientoPorMinuto);
    rep = Math.max(-100, Math.min(100, rep + (bonusPorFaccion.get(f.id) ?? 0)));
    return rep === f.reputacion ? f : { ...f, reputacion: rep };
  });
}

/** Uso 3 (Doc 2.7): restricción del Embajador para proponer alianzas con score muy bajo. */
export function puedeProponerAlianza(faccion: Faccion): boolean {
  return faccion.reputacion >= REPUTACION.umbralBajoParaEmbajador;
}

/** Uso 1 (Doc 2.7): términos de comercio asimétricos — tratar con una Facción poco confiable sale más caro para ELLA. */
export function factorComisionPorReputacion(faccion: Faccion): number {
  return faccion.reputacion <= REPUTACION.umbralBajoParaComision ? REPUTACION.factorComisionPorReputacionBaja : 1;
}
