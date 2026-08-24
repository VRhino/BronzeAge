// Comandos de cargos y ciudadanía: Rey y Embajador (Facción), cargos locales de asentamiento, compra de casa
// y activación de políticas. Agrupados en un archivo porque comparten la misma forma —localizar la entidad,
// delegar en el motor, registrar— y separarlos en cinco archivos de 25 líneas sería ruido sin beneficio.
//
// ⚠️ Corrección respecto a `GameStore`: allí estos comandos hacían `.find(...)!` sobre Facción/asentamiento.
// Si la entidad no existía, el `!` dejaba pasar `undefined` y el motor reventaba con un `TypeError` — un
// error NO de dominio, que `rechazoDesdeError` relanzaría y tumbaría el comando en vez de rechazarlo. Aquí se
// comprueba antes y se devuelve un código de rechazo propio.
import type { CargoTipo } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { asignarCargoLocal as asignarCargoLocalEngine, asignarEmbajador as asignarEmbajadorEngine, asignarRey as asignarReyEngine } from '../../engine/cargos';
import { comprarCasa as comprarCasaEngine } from '../../engine/faccion';
import { activarPolitica as activarPoliticaEngine } from '../../engine/politicas';
import { conHistorialDeJugador, eventoLegado, type GameSessionState } from '../estado';
import { exito, rechazo, rechazoDesdeError, type ContextoComando, type TransicionComando } from './tipos';

/** Códigos propios de esta capa: rechazos por entidad inexistente, que el motor no cubre porque nunca llega a
 * verlos (recibe la entidad ya resuelta). */
const FACCION_NO_EXISTE = 'faccion.no_existe';
const ASENTAMIENTO_NO_EXISTE = 'asentamiento.no_existe';

export interface ParamsAsignarCargoFaccion {
  faccionId: string;
  jugadorId: string;
}

/** Rey y Embajador comparten todo salvo la función del motor y el nombre del cargo en los mensajes. */
function asignarCargoDeFaccion(
  estado: GameSessionState,
  ctx: ContextoComando,
  params: ParamsAsignarCargoFaccion,
  nombreCargo: string,
  aplicar: (faccion: Parameters<typeof asignarReyEngine>[0], jugadorId: string) => ReturnType<typeof asignarReyEngine>
): TransicionComando<void> {
  const faccion = estado.facciones.find((f) => f.id === params.faccionId);
  if (!faccion) return rechazo(estado, FACCION_NO_EXISTE);

  try {
    const actualizada = aplicar(faccion, params.jugadorId);
    let siguiente: GameSessionState = {
      ...estado,
      facciones: estado.facciones.map((f) => (f.id === actualizada.id ? actualizada : f)),
    };
    siguiente = conHistorialDeJugador(siguiente, params.jugadorId, `Nombrado ${nombreCargo} de ${faccion.nombre}.`);
    const evento = eventoLegado(ctx.momento, estado.tick, `${faccion.nombre}: ${params.jugadorId} es el nuevo ${nombreCargo}.`);
    return exito(siguiente, [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export function asignarRey(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsAsignarCargoFaccion): TransicionComando<void> {
  return asignarCargoDeFaccion(estado, ctx, params, 'Rey', asignarReyEngine);
}

export function asignarEmbajador(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsAsignarCargoFaccion): TransicionComando<void> {
  return asignarCargoDeFaccion(estado, ctx, params, 'Embajador', asignarEmbajadorEngine);
}

export interface ParamsAsignarCargoLocal {
  asentamientoId: string;
  cargo: CargoTipo;
  jugadorId: string;
}

export function asignarCargoLocal(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsAsignarCargoLocal
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) return rechazo(estado, FACCION_NO_EXISTE);

  try {
    const actualizado = asignarCargoLocalEngine(asentamiento, faccion, params.cargo, params.jugadorId);
    let siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)),
    };
    siguiente = conHistorialDeJugador(siguiente, params.jugadorId, `Asignado como ${params.cargo} en ${asentamiento.id}.`);
    const evento = eventoLegado(ctx.momento, estado.tick, `${asentamiento.id}: ${params.jugadorId} asignado como ${params.cargo}.`, asentamiento.id);
    return exito(siguiente, [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsComprarCasa {
  asentamientoId: string;
  jugadorId: string;
}

export function comprarCasa(estado: GameSessionState, _mapa: Mapa, ctx: ContextoComando, params: ParamsComprarCasa): TransicionComando<void> {
  try {
    // A diferencia del resto, este comando del motor resuelve el asentamiento por su cuenta y lanza
    // `FaccionInvalidaError` si no existe — no hace falta comprobarlo antes.
    const resultado = comprarCasaEngine(estado.facciones, estado.asentamientos, params.asentamientoId, params.jugadorId);
    let siguiente: GameSessionState = {
      ...estado,
      facciones: resultado.facciones,
      asentamientos: estado.asentamientos.map((a) => (a.id === resultado.asentamiento.id ? resultado.asentamiento : a)),
    };
    siguiente = conHistorialDeJugador(siguiente, params.jugadorId, `Compra casa en ${params.asentamientoId} y obtiene ciudadanía.`);
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${params.jugadorId} compra casa en ${params.asentamientoId} y obtiene ciudadanía.`,
      resultado.asentamiento.id
    );
    return exito(siguiente, [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}

export interface ParamsActivarPolitica {
  asentamientoId: string;
  cargo: CargoTipo;
  politicaId: string;
}

export function activarPolitica(
  estado: GameSessionState,
  _mapa: Mapa,
  ctx: ContextoComando,
  params: ParamsActivarPolitica
): TransicionComando<void> {
  const asentamiento = estado.asentamientos.find((a) => a.id === params.asentamientoId);
  if (!asentamiento) return rechazo(estado, ASENTAMIENTO_NO_EXISTE);
  const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) return rechazo(estado, FACCION_NO_EXISTE);

  try {
    const actualizado = activarPoliticaEngine(asentamiento, faccion, params.cargo, params.politicaId, estado.tick, ctx.ids.siguiente());
    const siguiente: GameSessionState = {
      ...estado,
      asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)),
    };
    const evento = eventoLegado(
      ctx.momento,
      estado.tick,
      `${asentamiento.id}: política "${params.politicaId}" activada por ${params.cargo}.`,
      asentamiento.id
    );
    return exito(siguiente, [evento]);
  } catch (err) {
    return rechazoDesdeError(estado, err);
  }
}
