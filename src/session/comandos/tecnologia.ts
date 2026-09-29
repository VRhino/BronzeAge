// Comando de la adopción de tecnología (Doc 6.5). La regla vive en el motor (`engine/tecnologia.ts`: Rey en la
// capital, tecnología aparecida, tarifa de su Era); aquí solo se engancha al estado de la partida. Que quien lo pide
// sea el Rey lo decide `autorizacion.ts`.
import type { TecnologiaId } from '../../domain/types';
import { adoptarTecnologia as adoptarTecnologiaEngine } from '../../engine/tecnologia';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirFaccion } from './ayudas';
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
