// Comandos de tecnología: la adopción (Doc 6.5) y la compra a un Aeda (Doc 6.7). La regla vive en el motor (`engine/tecnologia.ts`: Rey en la
// capital, tecnología aparecida, tarifa de su Era); aquí solo se engancha al estado de la partida. Que quien lo pide
// sea el Rey lo decide `autorizacion.ts`.
import type { TecnologiaId } from '../../domain/types';
import { aedaEn } from '../../engine/aedas';
import { abandonarEpica as abandonarEpicaEngine, empezarEpica as empezarEpicaEngine, RESIDENTES_VACIOS } from '../../engine/aedasResidentes';
import { adoptarTecnologia as adoptarTecnologiaEngine, VentaInvalidaError, venderTecnologia } from '../../engine/tecnologia';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAsentamiento, exigirFaccion, exigirFaccionDe } from './ayudas';
import { desdeCrudos } from './eventos';

export interface ParamsAdoptarTecnologia {
  faccionId: string;
  tecnologiaId: TecnologiaId;
}

export const adoptarTecnologia = comando<ParamsAdoptarTecnologia, void>((estado, _mapa, ctx, params) => {
  const faccion = exigirFaccion(estado, params.faccionId);
  const r = adoptarTecnologiaEngine(estado.tecnologia, faccion, params.tecnologiaId, estado);
  return exito({ ...conAsentamiento(estado, r.capital), tecnologia: r.tecnologia }, desdeCrudos(ctx, r.eventos, r.capital.id));
});

export interface ParamsComprarTecnologiaAeda {
  asentamientoId: string;
  tecnologiaId: TecnologiaId;
}

/** Compra a un Aeda itinerante detenido en la plaza. Que quien lo pide sea el Rey o el Gobernador de la plaza, presente, lo decide `autorizacion.ts`. */
export const comprarTecnologiaAeda = comando<ParamsComprarTecnologiaAeda, void>((estado, _mapa, ctx, params) => {
  const plaza = exigirAsentamiento(estado, params.asentamientoId);
  const aeda = aedaEn(estado.aedas ?? [], plaza.id);
  if (!aeda) throw new VentaInvalidaError('No hay ningún Aeda en esta plaza.');
  const r = venderTecnologia(estado.tecnologia, exigirFaccionDe(estado, plaza), plaza, aeda.id, params.tecnologiaId, ctx.instante);
  return exito({ ...conAsentamiento(estado, r.plaza), tecnologia: r.tecnologia }, desdeCrudos(ctx, r.eventos, plaza.id));
});

export interface ParamsEmpezarEpica {
  asentamientoId: string;
  aedaId: string;
  tecnologiaId: TecnologiaId;
}

/** Un Aeda residente empieza la épica de una tecnología (Doc 6.7). Quién lo pide lo decide `autorizacion.ts`. */
export const empezarEpica = comando<ParamsEmpezarEpica, void>((estado, _mapa, ctx, params) => {
  const plaza = exigirAsentamiento(estado, params.asentamientoId);
  const r = empezarEpicaEngine(estado.aedasResidentes ?? RESIDENTES_VACIOS, estado.tecnologia, plaza, params.aedaId, params.tecnologiaId);
  return exito({ ...estado, aedasResidentes: r.estado }, desdeCrudos(ctx, r.eventos, plaza.id));
});

export interface ParamsAbandonarEpica {
  asentamientoId: string;
  aedaId: string;
}

export const abandonarEpica = comando<ParamsAbandonarEpica, void>((estado, _mapa, ctx, params) => {
  const plaza = exigirAsentamiento(estado, params.asentamientoId);
  const r = abandonarEpicaEngine(estado.aedasResidentes ?? RESIDENTES_VACIOS, plaza, params.aedaId);
  return exito({ ...estado, aedasResidentes: r.estado }, desdeCrudos(ctx, r.eventos, plaza.id));
});
