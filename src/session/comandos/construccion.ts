// Comandos de construcción y gestión del asentamiento: cola de obras (añadir, quitar, reordenar), mejora
// forzada, pausa de la auto-construcción, reserva manual del Tesorero y renombrado.
//
// Todos son de alcance local: sus eventos llevan `asentamientoId`, que es lo que permite a una proyección de
// Fase C mandárselos solo a quien tenga visibilidad sobre ese asentamiento.
import type { EdificioTipo, RecursoTipo } from '../../domain/types';
import {
  anadirEdificioManualmente as anadirEdificioManualmenteEngine,
  mejorarEdificioManualmente as mejorarEdificioManualmenteEngine,
  moverEnCola as moverEnColaEngine,
  quitarDeCola as quitarDeColaEngine,
  reclamosDeFuentes,
} from '../../engine/construction';
import { computeTodasLasZonas } from '../../engine/zones';
import { encontrarCapital } from '../../engine/mantenimiento';
import { exito, sinCambios } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento, exigirFaccionDe, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento } from './eventos';

/** Cargos con autoridad sobre la cola de construcción (Doc 4.2). */
export type CargoConstructor = 'gobernador' | 'maestroObras';

/** Mismo mecanismo de exhaustividad que `RECURSOS_TIPO`/`EDIFICIOS_TIPO` en `domain/types.ts`. */
const TODOS_LOS_CARGOS_CONSTRUCTOR: Record<CargoConstructor, true> = { gobernador: true, maestroObras: true };
export const CARGOS_CONSTRUCTOR = Object.keys(TODOS_LOS_CARGOS_CONSTRUCTOR) as CargoConstructor[];

export interface PayloadColaConstruccion {
  asentamientoId: string;
  cargo: CargoConstructor;
  /** Presente salvo en el reordenado, donde el edificio movido no cambia de estado. */
  edificioTipo?: EdificioTipo;
  edificioId?: string;
}
export interface PayloadAutoConstruccion {
  asentamientoId: string;
  pausada: boolean;
}
export interface PayloadRenombrado {
  asentamientoId: string;
  nombre?: string;
}

export interface ParamsAnadirEdificio {
  asentamientoId: string;
  cargo: CargoConstructor;
  tipo: EdificioTipo;
}

export const anadirEdificioManualmente = comando<ParamsAnadirEdificio, void>((estado, mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const faccion = exigirFaccionDe(estado, asentamiento);

  // La zona de influencia y los reclamos de fuentes se recalculan aquí, igual que hacía `GameStore`: el
  // motor necesita saber dónde puede colocar y qué yacimientos están ya tomados por OTROS asentamientos.
  const zona = computeTodasLasZonas(estado.asentamientos).find((z) => z.asentamientoId === asentamiento.id);
  const capital = encontrarCapital(asentamiento.faccionId, estado.asentamientos);
  const reclamos = reclamosDeFuentes(estado.asentamientos);

  const actualizado = anadirEdificioManualmenteEngine(
    asentamiento,
    faccion,
    params.cargo,
    params.tipo,
    zona?.poligono ?? [],
    mapa,
    capital,
    reclamos,
    ctx.ids.siguiente()
  );
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.encolada',
      mensaje: `${params.cargo} añade ${params.tipo} a la cola (pagado).`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, edificioTipo: params.tipo } satisfies PayloadColaConstruccion,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsQuitarDeCola {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
}

export const quitarDeCola = comando<ParamsQuitarDeCola, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const actualizado = quitarDeColaEngine(asentamiento, params.cargo, params.edificioId);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.desencolada',
      mensaje: `${params.cargo} quita un proyecto de la cola (recursos devueltos).`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, edificioId: params.edificioId } satisfies PayloadColaConstruccion,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsMoverEnCola {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
  direccion: 'arriba' | 'abajo';
}

export const moverEnCola = comando<ParamsMoverEnCola, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const actualizado = moverEnColaEngine(asentamiento, params.cargo, params.edificioId, params.direccion);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.cola_reordenada',
      mensaje: `${params.cargo} reordena la cola de construcción.`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, edificioId: params.edificioId } satisfies PayloadColaConstruccion,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsMejorarEdificio {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
}

export const mejorarEdificioAhora = comando<ParamsMejorarEdificio, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const capital = encontrarCapital(asentamiento.faccionId, estado.asentamientos);
  const actualizado = mejorarEdificioManualmenteEngine(asentamiento, params.cargo, params.edificioId, capital);
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.mejora_forzada',
      mensaje: `${params.cargo} fuerza la mejora de un edificio.`,
      payload: { asentamientoId: asentamiento.id, cargo: params.cargo, edificioId: params.edificioId } satisfies PayloadColaConstruccion,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsAlternarAutoConstruccion {
  asentamientoId: string;
  /** `true` pausa la auto-construcción; `false` la reanuda. */
  pausada: boolean;
}

/**
 * Unifica los dos métodos separados de `GameStore` (`pausarAutoConstruccion`/`reanudarAutoConstruccion`) en
 * un solo comando con bandera. Es la misma intención con dos valores, y
 * mantener dos handlers idénticos salvo un booleano sería duplicación. El adaptador mapea ambos métodos aquí.
 *
 * Idempotente: pedir el estado en el que ya está no muta ni versiona.
 */
export const alternarAutoConstruccion = comando<ParamsAlternarAutoConstruccion, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  if ((asentamiento.autoConstruccionPausada ?? false) === params.pausada) return sinCambios(estado);

  const actualizado = { ...asentamiento, autoConstruccionPausada: params.pausada };
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'construccion.auto_alternada',
      mensaje: `Auto-construcción ${params.pausada ? 'pausada' : 'reanudada'}.`,
      payload: { asentamientoId: asentamiento.id, pausada: params.pausada } satisfies PayloadAutoConstruccion,
      asentamientoId: asentamiento.id,
    }),
  ]);
});

export interface ParamsCalibrarReservaManual {
  asentamientoId: string;
  recurso: RecursoTipo;
  valor: number;
}

/** Reserva de recursos que la auto-construcción no puede tocar (Doc 4.2). Requiere Tesorero asignado. */
export const calibrarReservaManual = comando<ParamsCalibrarReservaManual, void>((estado, _mapa, _ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  if (!asentamiento.cargos.tesoreroId) rechazar(CODIGOS_ERROR.reservaSinTesorero);

  const limpio = Math.max(0, Math.min(999, Math.round(params.valor)));
  const actualizado = { ...asentamiento, reservaManual: { ...asentamiento.reservaManual, [params.recurso]: limpio } };
  // Sin evento a propósito: es un ajuste de calibración que se toca repetidamente con un slider, y anotar
  // cada paso llenaría el log de ruido (mismo criterio que tenía `GameStore`).
  return exito(conAsentamiento(estado, actualizado), []);
});

export interface ParamsRenombrarAsentamiento {
  asentamientoId: string;
  nombre: string;
}

export const renombrarAsentamiento = comando<ParamsRenombrarAsentamiento, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  // Nombre vacío = volver a mostrar el id (`Asentamiento.nombre` es opcional, ver domain/types.ts).
  const nombreLimpio = params.nombre.trim();
  const actualizado = { ...asentamiento, nombre: nombreLimpio || undefined };
  return exito(conAsentamiento(estado, actualizado), [
    evento(ctx, {
      codigo: 'asentamiento.renombrado',
      mensaje: `Renombrado a "${nombreLimpio || asentamiento.id}".`,
      payload: { asentamientoId: asentamiento.id, nombre: nombreLimpio || undefined } satisfies PayloadRenombrado,
      asentamientoId: asentamiento.id,
    }),
  ]);
});
