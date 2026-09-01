// Comandos de la muralla: comprometer un recinto, abandonar uno incompleto (Paso 2c) y mejorarlo de nivel
// (Paso 3, `Consideraciones/Murallas_Definicion.md`). Archivo propio y no un par de funciones más en
// `construccion.ts` a propósito — un `Recinto` no es un `Edificio` (no pasa por la cola, no tiene
// `EdificioTipo`, ver `domain/types.ts`), y mezclar los dos volvería confuso cuál de las dos entidades
// gestiona cada comando.
import { comprometerRecintoManualmente, abandonarRecintoManualmente, iniciarMejoraDeRecintoManualmente } from '../../engine/muralla';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento } from './ayudas';
import { evento } from './eventos';
import type { CargoConstructor } from './construccion';

export interface PayloadRecinto {
  asentamientoId: string;
  cargo?: CargoConstructor;
  recintoId?: string;
  nivel?: number;
}

export interface ParamsComprometerRecinto {
  asentamientoId: string;
  cargo: CargoConstructor;
  nivel: number;
}

/**
 * Traza y compromete un recinto: gratis (§8 del doc), congela el anillo y arranca la obra que
 * `avanzarObraDeRecintos` va pagando celda a celda en cada tick (`engine/construction.ts`). Mismo camino que
 * `anadirEdificioManualmente` — Gobernador o Maestro de Obras, gate de nivel de asentamiento.
 */
export const comprometerRecinto = comando<ParamsComprometerRecinto, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const actualizado = comprometerRecintoManualmente(asentamiento, params.cargo, params.nivel, ctx.instante);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.recinto_comprometido',
      mensaje: `${params.cargo} traza y compromete un recinto de nivel ${params.nivel} (gratis; la obra se paga celda a celda).`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, nivel: params.nivel } satisfies PayloadRecinto,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsAbandonarRecinto {
  asentamientoId: string;
  recintoId: string;
}

/**
 * Abandona un recinto INCOMPLETO: se borra entero y su suelo queda libre, sin devolución de materiales (§8).
 * Solo el Gobernador — a diferencia de comprometer, no admite Maestro de Obras (es una decisión de gobierno,
 * no de obra). Un recinto ya terminado no se puede abandonar (`abandonarRecintoManualmente` lo rechaza).
 */
export const abandonarRecinto = comando<ParamsAbandonarRecinto, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const actualizado = abandonarRecintoManualmente(asentamiento, params.recintoId);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.recinto_abandonado',
      mensaje: `El gobernador abandona un recinto incompleto (sin devolución de materiales).`,
      payload: { asentamientoId: asentamiento.id, recintoId: params.recintoId } satisfies PayloadRecinto,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsMejorarRecinto {
  asentamientoId: string;
  cargo: CargoConstructor;
  recintoId: string;
}

/**
 * Empieza a mejorar un recinto de nivel (§7 del doc): gratis, reinicia su obra y a partir de ahí
 * `avanzarObraDeRecintos` la paga celda a celda igual que la construcción original, con la tarifa de mejora.
 * Solo sobre un recinto YA COMPLETO — mismo camino que comprometer: Gobernador o Maestro de Obras.
 */
export const mejorarRecinto = comando<ParamsMejorarRecinto, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const actualizado = iniciarMejoraDeRecintoManualmente(asentamiento, params.cargo, params.recintoId);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.recinto_mejora_iniciada',
      mensaje: `${params.cargo} empieza a mejorar un recinto (gratis; la obra se paga celda a celda).`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, recintoId: params.recintoId } satisfies PayloadRecinto,
      asentamientoId: asentamiento.id,
    }),
  ]);
});
