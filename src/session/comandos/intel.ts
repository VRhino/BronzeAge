// Comandos de la intel de las tabernas (Doc 5.12.10): comprar una Mirada o un Informe de plaza. La regla vive en el motor
// (`engine/intel.ts`); aquí se resuelve DÓNDE se compra y CON QUÉ ORO. Una taberna de plaza paga con el almacén de esa plaza; la de un
// campamento de mercenarios, con el oro de botín del héroe. Que quien compra en una plaza sea el Rey, el Embajador o el Gobernador,
// presente en ella, lo decide `autorizacion.ts`.
import type { Asentamiento, CampamentoMercenarios, Faccion, Heroe, InformePlaza, MiradaIntel, Point } from '../../domain/types';
import { esCiudadano } from '../../engine/faccion';
import { cupoMiradas, edificiosPorTipoYEstado } from '../../engine/asentamientoQuery';
import { descontarRecursos, tieneRecursos } from '../../engine/almacen';
import { INTEL } from '../../constants';
import { IntelInvalidaError, levantarInforme, miradasActivasDe, comprarMirada as comprarMiradaEngine, validarInforme } from '../../engine/intel';
import { MEMORIA_VACIA } from '../../engine/memoria';
import { distancia } from '../../world/geometria';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { CODIGOS_ERROR } from './codigosDeError';
import { comando, conAsentamiento, exigirAsentamiento, exigirJugador, rechazar } from './ayudas';
import { evento } from './eventos';
import { exigirEn } from './mercenarios';

/** Dónde se compra: la taberna de una plaza propia o la de un campamento de mercenarios. */
export type OrigenDeIntel = { tipo: 'asentamiento'; id: string } | { tipo: 'campamento'; id: string };

export interface ParamsComprarMirada {
  origen: OrigenDeIntel;
  centro: Point;
}

export interface ParamsComprarInformePlaza {
  origen: OrigenDeIntel;
  asentamientoId: string;
}

/** Quién compra y con qué: la taberna, su Facción y cómo se cobra (devuelve el estado con el oro ya descontado). */
interface Compra {
  origenId: string;
  posicion: Point;
  faccion: Faccion;
  cupo: number;
  cobrar: (estado: GameSessionState, precio: number) => GameSessionState;
}

function resolverCompra(estado: GameSessionState, actor: string, origen: OrigenDeIntel): Compra {
  if (origen.tipo === 'asentamiento') {
    const plaza = exigirAsentamiento(estado, origen.id);
    const faccion = estado.facciones.find((f) => f.id === plaza.faccionId);
    if (!faccion) rechazar(CODIGOS_ERROR.faccionNoExiste);
    if (edificiosPorTipoYEstado(plaza, 'taberna').length === 0) throw new IntelInvalidaError('Esta plaza no tiene taberna.');
    return {
      origenId: plaza.id,
      posicion: plaza.posicion,
      faccion,
      cupo: cupoMiradas(plaza),
      cobrar: (e, precio) => {
        const actual = e.asentamientos.find((a) => a.id === plaza.id) as Asentamiento;
        if (!tieneRecursos(actual.almacen, { oro: precio })) throw new IntelInvalidaError(`El almacén de ${plaza.nombre ?? plaza.id} no tiene los ${precio} de oro que cuesta.`);
        return conAsentamiento(e, { ...actual, almacen: descontarRecursos(actual.almacen, { oro: precio }) });
      },
    };
  }
  const campamento = estado.campamentosMercenarios.find((c: CampamentoMercenarios) => c.id === origen.id);
  if (!campamento) rechazar(CODIGOS_ERROR.campamentoDesconocido);
  const heroe = exigirJugador(estado, actor);
  exigirEn(estado, heroe.id, campamento);
  const faccion = estado.facciones.find((f) => esCiudadano(f, heroe.id));
  if (!faccion) throw new IntelInvalidaError('La intel se compra para una Facción: hay que pertenecer a una.');
  return {
    origenId: campamento.id,
    posicion: campamento.posicion,
    faccion,
    cupo: INTEL.campamento.cupoMiradas,
    cobrar: (e, precio) => {
      const botin = heroe.oroDeBotin ?? 0;
      if (botin < precio) throw new IntelInvalidaError(`No tienes los ${precio} de oro que cuesta (tienes ${botin}).`);
      return { ...e, heroes: e.heroes.map((h: Heroe) => (h.id === heroe.id ? { ...h, oroDeBotin: botin - precio } : h)) };
    },
  };
}

/** Los ojos propios contra los que se mide la distancia del precio: las plazas y columnas de la Facción y la propia taberna. */
function ojosPropios(estado: GameSessionState, faccionId: string, desde: Point): Point[] {
  return [
    desde,
    ...estado.asentamientos.filter((a) => a.faccionId === faccionId).map((a) => a.posicion),
    ...estado.ejercitos.filter((e) => e.faccionId === faccionId).map((e) => e.posicionActual),
  ];
}

/** Compra una Mirada: un ojo sobre un punto del mapa durante un rato (Doc 5.12.10). Devuelve la Mirada. */
export const comprarMirada = comando<ParamsComprarMirada, MiradaIntel>((estado, mapa, ctx, params) => {
  const compra = resolverCompra(estado, ctx.actor, params.origen);
  const r = comprarMiradaEngine(estado.miradasIntel ?? [], {
    id: `mirada-${ctx.ids.siguiente()}`,
    faccionId: compra.faccion.id,
    origenId: compra.origenId,
    cupo: compra.cupo,
    centro: params.centro,
    ojosPropios: ojosPropios(estado, compra.faccion.id, compra.posicion),
    limites: mapa.limites,
    ahora: ctx.instante,
  });
  const pagado = compra.cobrar(estado, r.precio);
  return exito({ ...pagado, miradasIntel: r.miradas }, [], r.mirada);
});

/** Compra el Informe de una plaza ajena: su layout y su defensa en este instante (Doc 5.12.10). Avisa, sin firma, a la Facción espiada. */
export const comprarInformePlaza = comando<ParamsComprarInformePlaza, InformePlaza>((estado, _mapa, ctx, params) => {
  const compra = resolverCompra(estado, ctx.actor, params.origen);
  const objetivo = exigirAsentamiento(estado, params.asentamientoId);
  const memoria = estado.memoriaPorFaccion[compra.faccion.id] ?? MEMORIA_VACIA;
  const enUnaMirada = miradasActivasDe(estado.miradasIntel ?? [], new Set([compra.faccion.id]), ctx.instante).some((m) => distancia(m.centro, objetivo.posicion) <= m.radio);
  const precio = validarInforme(objetivo, compra.faccion.id, memoria.asentamientos[objetivo.id] !== undefined || enUnaMirada, memoria.informes?.[objetivo.id], ctx.instante);
  const pagado = compra.cobrar(estado, precio);
  const informe = levantarInforme(objetivo, estado.heroes, ctx.instante);
  return exito(
    {
      ...pagado,
      memoriaPorFaccion: { ...pagado.memoriaPorFaccion, [compra.faccion.id]: { ...memoria, informes: { ...memoria.informes, [objetivo.id]: informe } } },
    },
    [
      evento(ctx, {
        codigo: 'asentamiento.informe_pedido',
        mensaje: `Alguien ha pedido el plano y la defensa de ${objetivo.nombre ?? objetivo.id}.`,
        payload: { asentamientoId: objetivo.id },
        // A la plaza espiada: el aviso es para su Facción, sin decir quién.
        asentamientoId: objetivo.id,
      }),
    ],
    informe
  );
});
