// Piezas comunes a todos los manejadores de comando: resolver una entidad por id, sustituirla en el estado, y
// el envoltorio que convierte una excepción en rechazo.
//
// Existe por un motivo medido, no estético. Antes de esto, los 31 comandos repetían a mano el mismo trío:
// 24 `try/catch` idénticos, 30 guardas `find` + `return rechazo(...)`, y el mismo
// `asentamientos.map(a => a.id === x.id ? x : a)` copiado en 5 archivos. Esa repetición no era inocente: el
// doc 4 documenta TRES tandas de bugs por olvidarse justo de ella — el `.find(...)!` sistemático que dejaba
// pasar `undefined` al motor (y reventaba con un `TypeError`, que `rechazoDesdeError` RELANZA en vez de
// rechazar), y `romperRelacion` sin `try/catch` dejando escapar un `DiplomaciaInvalidaError` crudo hasta la
// interfaz. Con `comando()` esas dos clases de bug dejan de depender de que quien escribe el manejador se
// acuerde: no hay forma de registrar un comando sin envolverlo.
//
// El precio, dicho claro: los resolutores señalan con una EXCEPCIÓN (`RechazoDominio`) en vez de devolver un
// valor de error, que es control de flujo por excepción en una capa que por lo demás es de funciones puras.
// Queda encerrado dentro de `comando()`, no escapa nunca de un manejador, y es exactamente lo que ya hacía
// `rechazoDesdeError` con los errores del motor — así que el mecanismo es uno, no dos.
import type { Asentamiento, CampamentoBandido, Caravana, Faccion } from '../../domain/types';
import type { GameSessionState } from '../estado';
import { CODIGOS_ERROR, type CodigoError } from './codigosDeError';
import { rechazo, rechazoDesdeError, type ManejadorComando } from './tipos';

/** Rechazo señalado desde dentro de un manejador. Lo captura `comando()` y lo convierte en
 * `ResultadoComando { ok: false }`; nunca sale de ahí. */
export class RechazoDominio extends Error {
  constructor(public readonly codigo: CodigoError) {
    super(codigo);
    this.name = 'RechazoDominio';
  }
}

/** Rechaza el comando en curso desde cualquier punto del manejador. Devuelve `never`, así que sirve también
 * para estrechar tipos (`if (!x) rechazar(...)`). */
export function rechazar(codigo: CodigoError): never {
  throw new RechazoDominio(codigo);
}

/**
 * Envuelve un manejador para que las dos formas de fallo esperables se conviertan en rechazo con código
 * estable: `RechazoDominio` (lo señaló el propio comando, ej. entidad inexistente) y los errores de dominio
 * del motor (vía `rechazoDesdeError`). Cualquier otra excepción se RELANZA — un error inesperado es un bug, y
 * enterrarlo en un "comando rechazado" lo haría indistinguible de un rechazo legítimo.
 *
 * Todo manejador del registro pasa por aquí, incluidos los que hoy no llaman al motor: la uniformidad es
 * justamente lo que hace que no se pueda olvidar al añadir el siguiente.
 */
export function comando<P, R>(cuerpo: ManejadorComando<P, R>): ManejadorComando<P, R> {
  return (estado, mapa, ctx, params) => {
    try {
      return cuerpo(estado, mapa, ctx, params);
    } catch (err) {
      if (err instanceof RechazoDominio) return rechazo(estado, err.codigo);
      return rechazoDesdeError(estado, err);
    }
  };
}

// --- Resolutores: id -> entidad, o rechazo ---
//
// El motor recibe siempre la entidad ya resuelta, nunca un id suelto, así que estos rechazos son de ESTA capa
// y no del modelo de juego (por eso sus códigos viven en `codigosDeError.ts` junto a los del motor, ver ahí).

export function exigirAsentamiento(estado: GameSessionState, asentamientoId: string): Asentamiento {
  const asentamiento = estado.asentamientos.find((a) => a.id === asentamientoId);
  if (!asentamiento) rechazar(CODIGOS_ERROR.asentamientoNoExiste);
  return asentamiento;
}

export function exigirFaccion(estado: GameSessionState, faccionId: string): Faccion {
  const faccion = estado.facciones.find((f) => f.id === faccionId);
  if (!faccion) rechazar(CODIGOS_ERROR.faccionNoExiste);
  return faccion;
}

/** La Facción dueña de un asentamiento. Que no exista es estado corrupto, no un id malo del cliente, pero se
 * trata igual: rechazo con código en vez de un `undefined` que reviente dentro del motor. */
export function exigirFaccionDe(estado: GameSessionState, asentamiento: Asentamiento): Faccion {
  return exigirFaccion(estado, asentamiento.faccionId);
}

export function exigirCaravana(estado: GameSessionState, caravanaId: string): Caravana {
  const caravana = estado.caravanas.find((c) => c.id === caravanaId);
  if (!caravana) rechazar(CODIGOS_ERROR.caravanaNoExiste);
  return caravana;
}

export function exigirCampamento(estado: GameSessionState, campamentoId: string): CampamentoBandido {
  const campamento = estado.campamentosBandidos.find((c) => c.id === campamentoId);
  if (!campamento) rechazar(CODIGOS_ERROR.campamentoNoExiste);
  return campamento;
}

// --- Actualizadores: estado nuevo con una entidad sustituida ---

/** Sustituye un asentamiento por su versión actualizada, dejando el resto intacto. */
export function conAsentamiento(estado: GameSessionState, actualizado: Asentamiento): GameSessionState {
  return { ...estado, asentamientos: estado.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a)) };
}

/** Sustituye VARIOS asentamientos en una sola pasada — los comandos de combate actualizan atacante y
 * defensor a la vez, y encadenar dos `conAsentamiento` copiaría el array dos veces. */
export function conAsentamientos(estado: GameSessionState, actualizados: Asentamiento[]): GameSessionState {
  const porId = new Map(actualizados.map((a) => [a.id, a]));
  return { ...estado, asentamientos: estado.asentamientos.map((a) => porId.get(a.id) ?? a) };
}

/** Sustituye una Facción por su versión actualizada. */
export function conFaccion(estado: GameSessionState, actualizada: Faccion): GameSessionState {
  return { ...estado, facciones: estado.facciones.map((f) => (f.id === actualizada.id ? actualizada : f)) };
}
