// Líneas de un acuerdo de trueque (Doc 3.2): cada lado se compromete a entregar una o varias líneas, cada una con su
// recurso, su cantidad pactada y lo ya entregado. Predicados puros, sin reglas de comercio: módulo hoja (solo importa
// `domain/types`) que consumen `trade.ts`, `ejercitos.ts`, la IA de las Facciones NPC y los comandos.
import type { AcuerdoTrueque, LineaTrueque } from '../domain/types';

export type LadoTrueque = 'A' | 'B';

export const lineasDe = (acuerdo: AcuerdoTrueque, lado: LadoTrueque): LineaTrueque[] => (lado === 'A' ? acuerdo.lineasA : acuerdo.lineasB);

/** Lo que le falta a una línea; nunca negativo. */
export const faltanteDe = (linea: LineaTrueque): number => Math.max(0, linea.cantidadTotal - linea.cantidadEntregada);

/** Las líneas de un lado que aún no están saldadas, con lo que falta de cada una. */
export function lineasPendientes(acuerdo: AcuerdoTrueque, lado: LadoTrueque): { recurso: string; faltante: number }[] {
  return lineasDe(acuerdo, lado)
    .map((l) => ({ recurso: l.recurso, faltante: faltanteDe(l) }))
    .filter((l) => l.faltante > 0);
}

export const ladoSaldado = (acuerdo: AcuerdoTrueque, lado: LadoTrueque): boolean => lineasDe(acuerdo, lado).every((l) => faltanteDe(l) <= 0);

export const acuerdoSaldado = (acuerdo: AcuerdoTrueque): boolean => ladoSaldado(acuerdo, 'A') && ladoSaldado(acuerdo, 'B');

/** Cantidad pactada y entregada de todo un lado, sumadas. Sirve para ponderar (score, penalización), no para comparar recursos. */
export function totalesDeLado(acuerdo: AcuerdoTrueque, lado: LadoTrueque): { total: number; entregado: number } {
  const lineas = lineasDe(acuerdo, lado);
  return {
    total: lineas.reduce((a, l) => a + l.cantidadTotal, 0),
    entregado: lineas.reduce((a, l) => a + Math.min(l.cantidadEntregada, l.cantidadTotal), 0),
  };
}

/** Qué fracción (0-1) de lo pactado por un lado no se llegó a entregar. */
export function fraccionIncumplida(acuerdo: AcuerdoTrueque, lado: LadoTrueque): number {
  const { total, entregado } = totalesDeLado(acuerdo, lado);
  return total > 0 ? 1 - entregado / total : 0;
}

/** ¿Algún lado de este acuerdo ofrece (entrega) este recurso a ese asentamiento? `lado` es quien lo DEBE entregar. */
export const ofreceRecurso = (acuerdo: AcuerdoTrueque, lado: LadoTrueque, recurso: string): boolean =>
  lineasDe(acuerdo, lado).some((l) => l.recurso === recurso);

/** El acuerdo con `cantidad` más entregada de `recurso` por ese lado. Un recurso que ninguna línea pide no suma. */
export function conEntrega(acuerdo: AcuerdoTrueque, lado: LadoTrueque, recurso: string, cantidad: number): AcuerdoTrueque {
  const abonar = (lineas: LineaTrueque[]) => lineas.map((l) => (l.recurso === recurso ? { ...l, cantidadEntregada: l.cantidadEntregada + cantidad } : l));
  return lado === 'A' ? { ...acuerdo, lineasA: abonar(acuerdo.lineasA) } : { ...acuerdo, lineasB: abonar(acuerdo.lineasB) };
}
