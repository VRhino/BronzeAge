// Comandos de los campamentos de mercenarios (Doc 1.9b). Residir en uno está con el resto de cambios de residencia
// (`cargos.ts`); aquí, lo que se hace dentro: reclutar.
import { reclutarEnCampamento as reclutarEngine, tecnologiasDelCampamento, type PagarCon } from '../../engine/reclutamientoMercenario';
import { esCiudadano } from '../../engine/faccion';
import { exito } from './tipos';
import { comando, exigirJugador } from './ayudas';
import { evento } from './eventos';

export interface ParamsReclutarEnCampamento {
  tropaId: string;
  /** Con qué oro paga: su almacén personal (por defecto) o, siendo Líder de una columna a la puerta, el carro. */
  pagarCon?: PagarCon;
}

export interface PayloadReclutadoEnCampamento {
  campamentoId: string;
  heroeId: string;
  tropaId: string;
  cantidad: number;
  oro: number;
}

/**
 * Recluta o repone una tropa en el campamento donde reside el actor (Doc 1.9b). La regla vive en el motor; aquí solo se resuelve
 * qué Facciones cuentan como humanas vivas (las que no gobierna la IA y conservan algún asentamiento) para la tecnología del
 * campamento, y la Facción del héroe para el precio.
 */
export const reclutarEnCampamento = comando<ParamsReclutarEnCampamento, { cantidad: number; oro: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const faccion = estado.facciones.find((f) => esCiudadano(f, heroe.id));
  const tieneAsentamientos = (faccionId: string) => estado.asentamientos.some((a) => a.faccionId === faccionId);
  const humanasVivas = estado.facciones.filter((f) => !estado.faccionesNpcIds.includes(f.id) && tieneAsentamientos(f.id));
  const adoptadas = tecnologiasDelCampamento(estado.tecnologia, humanasVivas, ctx.instante);

  const r = reclutarEngine(
    estado.campamentosMercenarios,
    estado.heroes,
    estado.ejercitos,
    heroe.id,
    params.tropaId,
    adoptadas,
    faccion,
    faccion ? tieneAsentamientos(faccion.id) : false,
    params.pagarCon ?? 'almacenPersonal',
    ctx.instante,
    ctx.ids.siguiente()
  );
  const campamentoId = r.campamentos.find((c) => c.residentesIds.includes(heroe.id))!.id;
  return exito(
    { ...estado, campamentosMercenarios: r.campamentos, heroes: r.heroes, ejercitos: r.ejercitos },
    [
      evento(ctx, {
        codigo: 'mercenarios.reclutado',
        mensaje: `${heroe.displayName} recluta ${r.cantidad} de ${params.tropaId} en ${campamentoId} por ${r.oro} de oro.`,
        payload: { campamentoId, heroeId: heroe.id, tropaId: params.tropaId, cantidad: r.cantidad, oro: r.oro } satisfies PayloadReclutadoEnCampamento,
      }),
    ],
    { cantidad: r.cantidad, oro: r.oro }
  );
});
