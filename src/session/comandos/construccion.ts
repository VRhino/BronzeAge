// Comandos de construcción y gestión del asentamiento: cola de obras (añadir, quitar, reordenar), mejora
// forzada, pausa de la auto-construcción, reserva manual del Tesorero y renombrado.
//
// ⚠️ Los cuatro últimos (`alternarAutoConstruccion`, `calibrarReservaManual`, `renombrarAsentamiento`) ni
// siquiera tenían try/catch en `GameStore`, y aun así usaban `.find(...)!`: con un id inexistente el
// TypeError saltaba al leer `asentamiento.id` para el mensaje de log. Aquí devuelven un rechazo.
import type { EdificioTipo, RecursoTipo } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import {
  anadirEdificioManualmente as anadirEdificioManualmenteEngine,
  mejorarEdificioManualmente as mejorarEdificioManualmenteEngine,
  moverEnCola as moverEnColaEngine,
  quitarDeCola as quitarDeColaEngine,
  reclamosDeFuentes,
} from '../../engine/construction';
import { computeTodasLasZonas } from '../../engine/zones';
import { encontrarCapital } from '../../engine/mantenimiento';
import { eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';

const ASENTAMIENTO_NO_EXISTE = 'asentamiento.no_existe';
const FACCION_NO_EXISTE = 'faccion.no_existe';

/** Cargos con autoridad sobre la cola de construcción (Doc 4.2). */
export type CargoConstructor = 'gobernador' | 'maestroObras';

/** Sustituye un asentamiento por su versión actualizada, dejando el resto intacto. */
function conAsentamiento(estado: GameSessionState, actualizado: GameSessionState['asentamientos'][number]): GameSessionState {
  return { ...estado, asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)) };
}

export interface ParamsAnadirEdificio {
  asentamientoId: string;
  cargo: CargoConstructor;
  tipo: EdificioTipo;
}

export function anadirEdificioManualmente(
  estado: GameSessionState,
  mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsAnadirEdificio
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) return rechazo(estado, FACCION_NO_EXISTE);

  try {
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
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${asentamiento.id}: ${params.cargo} añade ${params.tipo} a la cola (pagado).`,
      asentamiento.id
    );
    return exito(conAsentamiento(estado, actualizado), [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsQuitarDeCola {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
}

export function quitarDeCola(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsQuitarDeCola
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  try {
    const actualizado = quitarDeColaEngine(asentamiento, params.cargo, params.edificioId);
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${asentamiento.id}: ${params.cargo} quita un proyecto de la cola (recursos devueltos).`,
      asentamiento.id
    );
    return exito(conAsentamiento(estado, actualizado), [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsMoverEnCola {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
  direccion: 'arriba' | 'abajo';
}

export function moverEnCola(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsMoverEnCola): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  try {
    const actualizado = moverEnColaEngine(asentamiento, params.cargo, params.edificioId, params.direccion);
    const evento = eventoLegado(ctx.momento, estado.tick, `${asentamiento.id}: ${params.cargo} reordena la cola de construcción.`, asentamiento.id);
    return exito(conAsentamiento(estado, actualizado), [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsMejorarEdificio {
  asentamientoId: string;
  cargo: CargoConstructor;
  edificioId: string;
}

export function mejorarEdificioAhora(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsMejorarEdificio
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  try {
    const capital = encontrarCapital(asentamiento.faccionId, estado.asentamientos);
    const actualizado = mejorarEdificioManualmenteEngine(asentamiento, params.cargo, params.edificioId, capital);
    const evento = eventoLegado(ctx.momento, estado.tick, `${asentamiento.id}: ${params.cargo} fuerza la mejora de un edificio.`, asentamiento.id);
    return exito(conAsentamiento(estado, actualizado), [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsAlternarAutoConstruccion {
  asentamientoId: string;
  /** `true` pausa la auto-construcción; `false` la reanuda. */
  pausada: boolean;
}

/**
 * Unifica los dos métodos separados de `GameStore` (`pausarAutoConstruccion`/`reanudarAutoConstruccion`) en
 * un solo comando con bandera, igual que `alternarFaccionNpc`. Es la misma intención con dos valores, y
 * mantener dos handlers idénticos salvo un booleano sería duplicación. El adaptador mapea ambos métodos aquí.
 *
 * Idempotente: pedir el estado en el que ya está no muta ni versiona.
 */
export function alternarAutoConstruccion(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsAlternarAutoConstruccion
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  if ((asentamiento.autoConstruccionPausada ?? false) === params.pausada) {
    return { estado, resultado: { ok: true, eventos: [], version: estado.version } };
  }

  const actualizado = { ...asentamiento, autoConstruccionPausada: params.pausada };
  const evento = eventoLegado(
    ctx.momento,
    estado.tick,
    `${asentamiento.id}: auto-construcción ${params.pausada ? 'pausada' : 'reanudada'}.`,
    asentamiento.id
  );
  return exito(conAsentamiento(estado, actualizado), [evento]);
}

export interface ParamsCalibrarReservaManual {
  asentamientoId: string;
  recurso: RecursoTipo;
  valor: number;
}

/** Reserva de recursos que la auto-construcción no puede tocar (Doc 4.2). Requiere Tesorero asignado. */
export function calibrarReservaManual(
  estado: GameSessionState,
  _mapa: Mapa,
  _ctx: ContextoComando,
  params: ParamsCalibrarReservaManual
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  if (!asentamiento.cargos.tesoreroId) return rechazo(estado, 'reserva.sin_tesorero');

  const limpio = Math.max(0, Math.min(999, Math.round(params.valor)));
  const actualizado = { ...asentamiento, reservaManual: { ...asentamiento.reservaManual, [params.recurso]: limpio } };
  // Sin evento a propósito: `GameStore` tampoco registraba nada aquí — es un ajuste de calibración que se
  // toca repetidamente con un slider, y anotar cada paso llenaría el log de ruido.
  return exito(conAsentamiento(estado, actualizado), []);
}

export interface ParamsRenombrarAsentamiento {
  asentamientoId: string;
  nombre: string;
}

export function renombrarAsentamiento(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsRenombrarAsentamiento
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);

  // Nombre vacío = volver a mostrar el id (`Asentamiento.nombre` es opcional, ver domain/types.ts).
  const nombreLimpio = params.nombre.trim();
  const actualizado = { ...asentamiento, nombre: nombreLimpio || undefined };
  const evento = eventoLegado(
    ctx.momento,
    estado.tick,
    `${asentamiento.id}: renombrado a "${nombreLimpio || asentamiento.id}".`,
    asentamiento.id
  );
  return exito(conAsentamiento(estado, actualizado), [evento]);
}
