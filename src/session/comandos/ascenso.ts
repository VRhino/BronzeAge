// Comando de la subida de nivel de asentamiento (Doc 4.5; decisión del usuario del 2026-09-26). La regla vive en el
// motor (`engine/ascenso.ts`: gates, coste, solvencia, cupo); aquí solo se engancha al estado de la partida. Quién
// puede pedirla —solo el Gobernador residente— lo decide `autorizacion.ts`, igual que el resto de comandos.
// Archivo propio y no una función más en `construccion.ts`: la obra de ascenso no es un `Edificio`, no pasa por la
// cola de construcción.
import { iniciarAscenso } from '../../engine/ascenso';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento } from './ayudas';
import { desdeCrudos } from './eventos';

export interface ParamsSolicitarAscenso {
  asentamientoId: string;
}

/**
 * El Gobernador pide subir el asentamiento al nivel siguiente: el motor comprueba gates, cupo de la Facción,
 * coste y solvencia, cobra el coste entero del almacén del asentamiento y arranca la obra. El nivel sube solo en
 * el tick en que la obra termina (`avanzarAscenso`). Si algo falla, el motor lanza `AscensoInvalidoError` con
 * todos los motivos y el comando se rechaza con `ascenso.invalido`.
 */
export const solicitarAscenso = comando<ParamsSolicitarAscenso, void>((estado, mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const resultado = iniciarAscenso(asentamiento, estado.asentamientos, estado.facciones, mapa, ctx.instante);
  return exito(conAsentamiento(estado, resultado.asentamiento), desdeCrudos(ctx, resultado.eventos, asentamiento.id));
});
