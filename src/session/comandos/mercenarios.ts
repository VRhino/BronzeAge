// Comandos de los campamentos de mercenarios (Doc 1.9b). Residir en uno está con el resto de cambios de residencia
// (`cargos.ts`); aquí, lo que se hace en él: reclutar, comprar y el fondo de refundación.
import { reclutarEnCampamento as reclutarEngine, tecnologiasDelCampamento, type PagarCon } from '../../engine/reclutamientoMercenario';
import { esCiudadano } from '../../engine/faccion';
import { comprarEnCampamento as comprarEngine } from '../../engine/mercadoMercenario';
import { aportarARefundacion as aportarEngine, comprarCaravanaDeRefundacion as comprarCaravanaEngine, retirarDeRefundacion as retirarEngine } from '../../engine/refundacion';
import { ALMACEN_PERSONAL } from '../../constants';
import type { CampamentoMercenarios, Point } from '../../domain/types';
import { exito } from './tipos';
import { comando, exigirFaccion, exigirJugador, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import type { GameSessionState } from '../estado';
import { evento } from './eventos';
import { campamentoDeResidente } from '../../engine/mercenarios';
import { columnaDe, enLaPuertaDelCampamento } from '../../engine/ejercitos';

/** ¿Está el héroe en ese campamento (D75)? Dentro, o con su columna en la puerta. */
function exigirEn(estado: GameSessionState, heroeId: string, campamento: CampamentoMercenarios): void {
  const heroe = estado.heroes.find((h) => h.id === heroeId);
  if (heroe?.ubicacion.tipo === 'mercenarios' && heroe.ubicacion.campamentoId === campamento.id) return;
  const columna = columnaDe(estado.ejercitos, heroeId);
  if (columna && enLaPuertaDelCampamento(columna, campamento)) return;
  rechazar(CODIGOS_ERROR.campamentoLejos);
}

/**
 * Las acciones del campamento se hacen en él (D75): dentro, o con la columna en su puerta. Su campamento es donde reside; si no
 * reside en ninguno, el motor ya rechaza con su motivo.
 */
function exigirEnSuCampamento(estado: GameSessionState, heroeId: string): void {
  const campamento = campamentoDeResidente(estado.campamentosMercenarios, heroeId);
  if (campamento) exigirEn(estado, heroeId, campamento);
}

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
  exigirEnSuCampamento(estado, heroe.id);
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

export interface ParamsComprarEnCampamento {
  recurso: string;
  cantidad: number;
  /** Dónde compra: por defecto, donde reside. Cualquier otro campamento solo le vende trigo para repostar (D44). */
  campamentoId?: string;
}

export interface PayloadCompradoEnCampamento {
  campamentoId: string;
  heroeId: string;
  recurso: string;
  cantidad: number;
  oro: number;
}

/**
 * Compra en el mercado del campamento donde reside el actor (Doc 1.9b): paga con el oro de su almacén personal y recibe en él. El oro
 * cobrado se destruye. Sirve lo que puede (stock, oro, sitio) y falla si no puede servir nada.
 */
export const comprarEnCampamento = comando<ParamsComprarEnCampamento, { cantidad: number; oro: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  const campamento = params.campamentoId
    ? estado.campamentosMercenarios.find((c) => c.id === params.campamentoId)
    : campamentoDeResidente(estado.campamentosMercenarios, heroe.id);
  if (!campamento) rechazar(params.campamentoId ? CODIGOS_ERROR.campamentoDesconocido : CODIGOS_ERROR.mercenariosInvalido);
  exigirEn(estado, heroe.id, campamento);
  const columna = columnaDe(estado.ejercitos, heroe.id);
  const r = comprarEngine(campamento, heroe, columna, estado.asentamientos, params.recurso, params.cantidad, ctx.instante);
  return exito(
    {
      ...estado,
      campamentosMercenarios: estado.campamentosMercenarios.map((c) => (c.id === campamento.id ? r.campamento : c)),
      heroes: estado.heroes.map((h) => (h.id === heroe.id ? r.heroe : h)),
      ejercitos: r.columna ? estado.ejercitos.map((e) => (e.id === r.columna!.id ? r.columna! : e)) : estado.ejercitos,
    },
    [
      evento(ctx, {
        codigo: 'mercenarios.comprado',
        mensaje: `${heroe.displayName} compra ${r.cantidad} ${params.recurso} en ${campamento.id} por ${r.oro} de oro.`,
        payload: { campamentoId: campamento.id, heroeId: heroe.id, recurso: params.recurso, cantidad: r.cantidad, oro: r.oro } satisfies PayloadCompradoEnCampamento,
      }),
    ],
    { cantidad: r.cantidad, oro: r.oro }
  );
});

export interface ParamsFondoRefundacion {
  recurso: string;
  cantidad: number;
}

export interface PayloadFondoRefundacion {
  campamentoId: string;
  heroeId: string;
  faccionId: string;
  recurso: string;
  cantidad: number;
  sentido: 'aporta' | 'retira';
}

/** La Facción del actor: la de la que es ciudadano. Sin ninguna, no hay refundación posible. */
function faccionDelActor(estado: GameSessionState, heroeId: string) {
  const faccion = estado.facciones.find((f) => esCiudadano(f, heroeId));
  if (!faccion) rechazar(CODIGOS_ERROR.faccionNoPerteneces);
  return faccion;
}

/** Aporta del almacén personal al fondo de refundación del campamento donde reside (Doc 1.9b). Solo una Facción sin asentamientos. */
export const aportarARefundacion = comando<ParamsFondoRefundacion, { movido: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  exigirEnSuCampamento(estado, heroe.id);
  const faccion = faccionDelActor(estado, heroe.id);
  const r = aportarEngine(estado.campamentosMercenarios, estado.heroes, faccion, estado.asentamientos, heroe.id, params.recurso, params.cantidad);
  const campamentoId = r.campamentos.find((c) => c.residentesIds.includes(heroe.id))!.id;
  return exito(
    { ...estado, campamentosMercenarios: r.campamentos, heroes: r.heroes },
    [
      evento(ctx, {
        codigo: 'mercenarios.fondo_refundacion',
        mensaje: `${heroe.displayName} aporta ${r.movido} ${params.recurso} al fondo de refundación de ${faccion.nombre}.`,
        payload: { campamentoId, heroeId: heroe.id, faccionId: faccion.id, recurso: params.recurso, cantidad: r.movido, sentido: 'aporta' } satisfies PayloadFondoRefundacion,
      }),
    ],
    { movido: r.movido }
  );
});

/** Retira lo aportado por el actor del fondo, de vuelta a su almacén personal. */
export const retirarDeRefundacion = comando<ParamsFondoRefundacion, { movido: number }>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  exigirEnSuCampamento(estado, heroe.id);
  const faccion = faccionDelActor(estado, heroe.id);
  const r = retirarEngine(estado.campamentosMercenarios, estado.heroes, heroe.id, params.recurso, params.cantidad, ALMACEN_PERSONAL.capacidad);
  const campamentoId = r.campamentos.find((c) => c.residentesIds.includes(heroe.id))!.id;
  return exito(
    { ...estado, campamentosMercenarios: r.campamentos, heroes: r.heroes },
    [
      evento(ctx, {
        codigo: 'mercenarios.fondo_refundacion',
        mensaje: `${heroe.displayName} retira ${r.movido} ${params.recurso} del fondo de refundación.`,
        payload: { campamentoId, heroeId: heroe.id, faccionId: faccion.id, recurso: params.recurso, cantidad: r.movido, sentido: 'retira' } satisfies PayloadFondoRefundacion,
      }),
    ],
    { movido: r.movido }
  );
});

export interface ParamsComprarCaravanaDeRefundacion {
  destino: Point;
}

export interface PayloadCaravanaDeRefundacion {
  caravanaId: string;
  campamentoId: string;
  faccionId: string;
  destino: Point;
}

/** Compra la Caravana de Fundación al 75 % con el fondo del campamento (Doc 1.9b): sale de él hacia `destino`, con el actor de fundador. */
export const comprarCaravanaDeRefundacion = comando<ParamsComprarCaravanaDeRefundacion, { caravanaId: string }>((estado, mapa, ctx, params) => {
  const heroe = exigirJugador(estado, ctx.actor);
  exigirEnSuCampamento(estado, heroe.id);
  const faccion = exigirFaccion(estado, faccionDelActor(estado, heroe.id).id);
  const r = comprarCaravanaEngine(
    estado.campamentosMercenarios,
    faccion,
    estado.asentamientos,
    estado.caravanas,
    mapa,
    heroe.id,
    params.destino,
    ctx.ids.siguiente()
  );
  return exito(
    { ...estado, campamentosMercenarios: r.campamentos, caravanas: [...estado.caravanas, r.caravana] },
    [
      evento(ctx, {
        codigo: 'mercenarios.caravana_refundacion',
        mensaje: `${faccion.nombre} compra una Caravana de Fundación en ${r.caravana.origenCampamentoId} hacia (${Math.round(params.destino.x)}, ${Math.round(params.destino.y)}).`,
        payload: { caravanaId: r.caravana.id, campamentoId: r.caravana.origenCampamentoId!, faccionId: faccion.id, destino: params.destino } satisfies PayloadCaravanaDeRefundacion,
      }),
    ],
    { caravanaId: r.caravana.id }
  );
});
