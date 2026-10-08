// Ejército en preparación dentro de una plaza o de un campamento (Doc 5.14.5): convocar, unirse, separarse, cancelar y partir.
//
// Mientras se prepara nadie ha salido: las escuadras siguen en su campamento y el almacén intacto, así que cancelar o separarse no
// devuelve nada. Al partir, cada integrante sale POR EL MISMO CAMINO de siempre —`salirAlMundo`, `salirDeAsentamiento` o
// `salirDelCampamento`, según resida o no y según el lugar— y las columnas resultantes se funden en un único ejército en la puerta.
// Reutilizar esas salidas es lo que garantiza que tropa, liderazgo, carga y víveres se validen igual que al salir solo; también se
// usan, sin aplicar el resultado, para rechazar al unirse lo que luego no podría salir.
import { MOVIMIENTO } from '../../constants';
import { segundos, sumar } from '../../domain/tiempo';
import type { Convocatoria, LugarConvocatoria, SeleccionDeSalida } from '../../domain/types';
import { MovilizacionInvalidaError, unirseEnCampo as unirseEnCampoEngine } from '../../engine/ejercitos';
import {
  convocatoriaDe,
  integrantesDentro,
  nuevaConvocatoria,
  pedirUnion,
  retirarPeticion,
  cambiarSeleccion,
  separarseDeConvocatoria as separarseEngine,
  unirseAConvocatoria as unirseEngine,
} from '../../engine/convocatoria';
import { esCiudadano } from '../../engine/faccion';
import { esResidente } from '../../engine/pertenencia';
import type { EjercitoConTropa } from '../../engine/tropa';
import { situarHeroes } from '../../engine/ubicacion';
import type { EventoDominio } from '../../domain/eventos';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import type { Mapa } from '../../world/mapa';
import { CODIGOS_ERROR } from './codigosDeError';
import { comando, conColumnas, conTropaDe, exigirAsentamiento, exigirJugador } from './ayudas';
import { evento } from './eventos';
import { salirAlMundo, salirDeAsentamiento, salirDelCampamento } from './presencia';
import { exito, rechazo, type ContextoComando, type ManejadorComando } from './tipos';

export interface PayloadConvocatoria {
  convocatoriaId: string;
  liderId: string;
  /** Quien hace o sufre el hecho. */
  heroeId: string;
  /** Todos los que tienen que enterarse (`heroesNombrados`): el canal `heroe/<id>` de cada uno. */
  heroesIds: string[];
  lugar: LugarConvocatoria;
  /** Hasta cuándo vive la petición (`convocatoria.union_pedida`). */
  expiraEn?: number;
  /** El ejército que sale (`convocatoria.partio`). */
  ejercitoId?: string;
}

export interface ParamsConvocarEjercito extends SeleccionDeSalida {
  heroeId: string;
  politicaDeUnion: 'aceptar' | 'preguntar';
}

export interface ParamsUnirseAConvocatoria extends SeleccionDeSalida {
  heroeId: string;
  convocatoriaId: string;
}

export interface ParamsResponderPeticionDeConvocatoria {
  heroeId: string;
  convocatoriaId: string;
  solicitanteId: string;
  aceptar: boolean;
}

export interface ParamsCambiarSeleccionDeConvocatoria extends SeleccionDeSalida {
  heroeId: string;
}

export interface ParamsDeLaConvocatoria {
  heroeId: string;
}

const convocatoriasDe = (estado: GameSessionState): Convocatoria[] => estado.convocatorias ?? [];

const conConvocatorias = (estado: GameSessionState, convocatorias: Convocatoria[]): GameSessionState => ({ ...estado, convocatorias });

const sustituir = (convocatorias: Convocatoria[], nueva: Convocatoria): Convocatoria[] => convocatorias.map((c) => (c.id === nueva.id ? nueva : c));

function exigirConvocatoria(estado: GameSessionState, convocatoriaId: string): Convocatoria {
  const c = convocatoriasDe(estado).find((x) => x.id === convocatoriaId);
  if (!c) throw new MovilizacionInvalidaError('Ese ejército en preparación ya no existe.');
  return c;
}

const faccionDe = (estado: GameSessionState, heroeId: string): string => estado.facciones.find((f) => esCiudadano(f, heroeId))?.id ?? '';

/** Un evento que avisa a todos los implicados. En una plaza se atribuye a ella; en un campamento, a ninguna (`''`). */
function aviso(
  ctx: ContextoComando,
  codigo: string,
  mensaje: string,
  c: Convocatoria,
  heroeId: string,
  extra: Partial<PayloadConvocatoria> = {},
  avisados: readonly string[] = []
): EventoDominio {
  const heroesIds = [...new Set([c.liderId, heroeId, ...c.integrantes.map((i) => i.heroeId), ...avisados])];
  return evento(ctx, {
    codigo,
    mensaje,
    payload: { convocatoriaId: c.id, liderId: c.liderId, heroeId, heroesIds, lugar: c.lugar, ...extra } satisfies PayloadConvocatoria,
    asentamientoId: c.lugar.tipo === 'asentamiento' ? c.lugar.id : '',
  });
}

// --- Salir del lugar, por el camino de siempre ---

type Salida = { manejador: ManejadorComando<never, { ejercitoId: string }>; params: unknown };

function salidaDe(estado: GameSessionState, lugar: LugarConvocatoria, heroeId: string, sel: SeleccionDeSalida): Salida {
  if (lugar.tipo === 'campamento') return { manejador: salirDelCampamento as never, params: { campamentoId: lugar.id, heroeId, ...sel } };
  const resideAqui = esResidente(exigirAsentamiento(estado, lugar.id), heroeId);
  return resideAqui
    ? { manejador: salirAlMundo as never, params: { asentamientoId: lugar.id, heroeId, ...sel } }
    : { manejador: salirDeAsentamiento as never, params: { asentamientoId: lugar.id, heroeId } };
}

/** Sale de verdad `heroeId` del lugar con esa selección; si no puede, falla con el motivo concreto. */
function salirDelLugar(estado: GameSessionState, mapa: Mapa, ctx: ContextoComando, lugar: LugarConvocatoria, heroeId: string, sel: SeleccionDeSalida) {
  const { manejador, params } = salidaDe(estado, lugar, heroeId, sel);
  const t = (manejador as ManejadorComando<unknown, { ejercitoId: string }>)(estado, mapa, ctx, params);
  if (!t.resultado.ok) throw new MovilizacionInvalidaError(`${heroeId} no puede salir con lo elegido: ${t.resultado.detalleError ?? t.resultado.codigoError}`);
  return { estado: t.estado, ejercitoId: t.resultado.datos!.ejercitoId, eventos: t.resultado.eventos };
}

/** ¿Saldría `heroeId` con esa selección? Se comprueba saliendo en un estado que se descarta. */
function exigirQuePuedeSalir(estado: GameSessionState, mapa: Mapa, ctx: ContextoComando, lugar: LugarConvocatoria, heroeId: string, sel: SeleccionDeSalida): void {
  salirDelLugar(estado, mapa, ctx, lugar, heroeId, sel);
}

// --- Comandos ---

/** Un héroe dentro de una plaza o un campamento convoca un ejército: queda en preparación, sin salir, con la política de unión que fija. */
export const convocarEjercito = comando<ParamsConvocarEjercito, { convocatoriaId: string }>((estado, mapa, ctx, params) => {
  const heroe = exigirJugador(estado, params.heroeId);
  const sel = { escuadronIds: params.escuadronIds, carga: params.carga };
  const nueva = nuevaConvocatoria(convocatoriasDe(estado), heroe, faccionDe(estado, heroe.id), params.politicaDeUnion, sel, `convocatoria-${ctx.ids.siguiente()}`, ctx.instante);
  exigirQuePuedeSalir(estado, mapa, ctx, nueva.lugar, heroe.id, sel);
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, [...convocatoriasDe(estado), nueva]), heroe.id, 'Convoca un ejército.'),
    [aviso(ctx, 'convocatoria.convocada', `${heroe.displayName} convoca un ejército.`, nueva, heroe.id)],
    { convocatoriaId: nueva.id }
  );
});

/**
 * Unirse a un ejército en preparación del lugar donde estás (misma Facción, dentro del mismo sitio), con las escuadras y la carga con
 * las que saldrías. Con política `preguntar` queda una petición para el Líder, que vive el mismo tiempo que en campo (Doc 5.14.1).
 */
export const unirseAConvocatoria = comando<ParamsUnirseAConvocatoria, { unido: boolean }>((estado, mapa, ctx, params) => {
  const heroe = exigirJugador(estado, params.heroeId);
  const c = exigirConvocatoria(estado, params.convocatoriaId);
  const sel = { escuadronIds: params.escuadronIds, carga: params.carga };
  const faccionId = faccionDe(estado, heroe.id);
  const lista = convocatoriasDe(estado);
  exigirQuePuedeSalir(estado, mapa, ctx, c.lugar, heroe.id, sel);

  if (c.politicaDeUnion === 'preguntar') {
    const expiraEn = sumar(ctx.instante, segundos(MOVIMIENTO.vidaPeticionUnionSegundos));
    const conPeticion = pedirUnion(lista, c, heroe, faccionId, sel, ctx.instante, expiraEn);
    return exito(
      conHistorialDeJugador(conConvocatorias(estado, sustituir(lista, conPeticion)), heroe.id, 'Pide unirse a un ejército en preparación.'),
      [aviso(ctx, 'convocatoria.union_pedida', `${heroe.displayName} pide unirse al ejército.`, c, heroe.id, { expiraEn })],
      { unido: false }
    );
  }
  const unida = unirseEngine(lista, c, heroe, faccionId, sel, ctx.instante);
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, sustituir(lista, unida)), heroe.id, 'Se une a un ejército en preparación.'),
    [aviso(ctx, 'convocatoria.unido', `${heroe.displayName} se une al ejército.`, unida, heroe.id)],
    { unido: true }
  );
});

/** El Líder contesta una petición. Al aceptar se revalida todo: quien pidió tiene que seguir dentro y poder salir con lo que ofreció. */
export const responderPeticionDeConvocatoria = comando<ParamsResponderPeticionDeConvocatoria, { unido: boolean }>((estado, mapa, ctx, params) => {
  const c = exigirConvocatoria(estado, params.convocatoriaId);
  const lista = convocatoriasDe(estado);
  const { convocatoria: sinPeticion, peticion } = retirarPeticion(c, params.heroeId, params.solicitanteId, ctx.instante);

  if (!params.aceptar) {
    return exito(
      conHistorialDeJugador(conConvocatorias(estado, sustituir(lista, sinPeticion)), params.heroeId, 'Rechaza a un jugador en su ejército en preparación.'),
      [aviso(ctx, 'convocatoria.union_rechazada', 'El Líder rechaza a un jugador.', c, params.solicitanteId)],
      { unido: false }
    );
  }
  const solicitante = exigirJugador(estado, params.solicitanteId);
  const sel = { escuadronIds: peticion.escuadronIds, carga: peticion.carga };
  exigirQuePuedeSalir(estado, mapa, ctx, c.lugar, solicitante.id, sel);
  const unida = unirseEngine(lista, sinPeticion, solicitante, faccionDe(estado, solicitante.id), sel, ctx.instante);
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, sustituir(lista, unida)), solicitante.id, 'Es aceptado en un ejército en preparación.'),
    [aviso(ctx, 'convocatoria.unido', 'El Líder acepta a un jugador.', unida, solicitante.id)],
    { unido: true }
  );
});

/**
 * Un integrante (también el Líder) cambia SU PROPIA selección —escuadras, en el orden en que quiere que entren en combate, y carga— sin
 * separarse. Se valida igual que al unirse: saldría con eso. Los demás lo ven al instante (`convocatoria.seleccion_cambiada`).
 */
export const cambiarSeleccionDeConvocatoria = comando<ParamsCambiarSeleccionDeConvocatoria, void>((estado, mapa, ctx, params) => {
  const c = convocatoriaDe(convocatoriasDe(estado), params.heroeId);
  if (!c) throw new MovilizacionInvalidaError('No estás en ningún ejército en preparación.');
  const sel = { escuadronIds: params.escuadronIds, carga: params.carga };
  exigirQuePuedeSalir(estado, mapa, ctx, c.lugar, params.heroeId, sel);
  const cambiada = cambiarSeleccion(c, params.heroeId, sel);
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, sustituir(convocatoriasDe(estado), cambiada)), params.heroeId, 'Cambia lo que saca su ejército en preparación.'),
    [aviso(ctx, 'convocatoria.seleccion_cambiada', 'Un integrante cambia lo que saca el ejército.', cambiada, params.heroeId)]
  );
});

/** Un integrante se va mientras se prepara: sigue dentro del lugar y no hay nada que devolver. El Líder no se separa, cancela. */
export const separarseDeConvocatoria = comando<ParamsDeLaConvocatoria, void>((estado, _mapa, ctx, params) => {
  const c = convocatoriaDe(convocatoriasDe(estado), params.heroeId);
  if (!c) throw new MovilizacionInvalidaError('No estás en ningún ejército en preparación.');
  const resto = separarseEngine(c, params.heroeId);
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, sustituir(convocatoriasDe(estado), resto)), params.heroeId, 'Se separa de un ejército en preparación.'),
    [aviso(ctx, 'convocatoria.separado', 'Un integrante se separa del ejército en preparación.', c, params.heroeId)]
  );
});

/** El Líder cancela: la convocatoria desaparece y todos siguen dentro del lugar, con todo en su sitio. */
export const cancelarConvocatoria = comando<ParamsDeLaConvocatoria, void>((estado, _mapa, ctx, params) => {
  const c = convocatoriaDe(convocatoriasDe(estado), params.heroeId);
  if (!c) throw new MovilizacionInvalidaError('No estás en ningún ejército en preparación.');
  if (c.liderId !== params.heroeId) throw new MovilizacionInvalidaError('Solo el Líder cancela el ejército: los demás se separan.');
  return exito(
    conHistorialDeJugador(conConvocatorias(estado, convocatoriasDe(estado).filter((x) => x.id !== c.id)), params.heroeId, 'Cancela su ejército en preparación.'),
    [aviso(ctx, 'convocatoria.cancelada', 'El Líder cancela el ejército en preparación.', c, params.heroeId, {}, c.peticiones?.map((p) => p.heroeId))]
  );
});

/**
 * El Líder parte: salen todos los que siguen dentro, juntos, como UN ejército quieto en la puerta del lugar, con el Líder de la
 * convocatoria al volante y su política de unión (Doc 5.14.5). Todo o nada: si alguien ya no puede salir con lo que eligió, nadie sale
 * y se dice quién y por qué.
 */
export const partirConvocatoria = comando<ParamsDeLaConvocatoria, { ejercitoId: string }>((estado, mapa, ctx, params) => {
  const c = convocatoriaDe(convocatoriasDe(estado), params.heroeId);
  if (!c) throw new MovilizacionInvalidaError('No estás en ningún ejército en preparación.');
  if (c.liderId !== params.heroeId) throw new MovilizacionInvalidaError('Solo el Líder hace partir al ejército.');
  const dentro = integrantesDentro(c, estado.heroes);
  if (dentro[0]?.heroeId !== c.liderId) throw new MovilizacionInvalidaError('El Líder ya no está dentro del lugar.');

  let actual = estado;
  const columnas: { heroeId: string; ejercitoId: string }[] = [];
  const hechos: EventoDominio[] = [];
  for (const i of dentro) {
    const s = salirDelLugar(actual, mapa, ctx, c.lugar, i.heroeId, i);
    actual = s.estado;
    hechos.push(...s.eventos);
    columnas.push({ heroeId: i.heroeId, ejercitoId: s.ejercitoId });
  }

  // La columna del Líder se convierte en el ejército; las demás se funden en ella, a su misma altura (todas salen por la misma puerta).
  const base = actual.ejercitos.find((e) => e.id === columnas[0]!.ejercitoId)!;
  let ejercito: EjercitoConTropa = {
    ...conTropaDe(actual, base),
    tipo: 'ejercito',
    politicaDeUnion: c.politicaDeUnion,
    ...(c.lugar.tipo === 'campamento' ? { origenCampamentoId: c.lugar.id } : {}),
  };
  actual = conColumnas(actual, [ejercito]);
  for (const { heroeId, ejercitoId } of columnas.slice(1)) {
    const columna = actual.ejercitos.find((e) => e.id === ejercitoId)!;
    ejercito = unirseEnCampoEngine(ejercito, { ...conTropaDe(actual, columna), posicionActual: ejercito.posicionActual }, ctx.instante);
    actual = conColumnas({ ...actual, ejercitos: actual.ejercitos.filter((e) => e.id !== ejercitoId) }, [ejercito]);
    actual = { ...actual, heroes: situarHeroes(actual.heroes, [heroeId], { tipo: 'columna', ejercitoId: ejercito.id }) };
  }

  const final = exito(
    conHistorialDeJugador(conConvocatorias(actual, convocatoriasDe(actual).filter((x) => x.id !== c.id)), params.heroeId, `Parte con su ejército ${ejercito.id}.`),
    [aviso(ctx, 'convocatoria.partio', `El ejército ${ejercito.id} parte con ${dentro.length} héroe(s).`, c, params.heroeId, { ejercitoId: ejercito.id })],
    { ejercitoId: ejercito.id }
  );
  // Los hechos de cada salida ya están en el estado; se devuelven también al llamador, que es quien los difunde.
  return { ...final, resultado: { ...final.resultado, eventos: [...hechos, ...final.resultado.eventos] } } as typeof final;
});

/**
 * Quien está en un ejército en preparación no sale por su cuenta: saldría de la convocatoria sin avisar a nadie. Envuelve los comandos
 * que sacan a un héroe de su lugar para que se pase antes por separarse o cancelar. `partirConvocatoria` usa los manejadores sin envolver.
 */
export function sinConvocatoria<P extends { heroeId: string }, R>(manejador: ManejadorComando<P, R>): ManejadorComando<P, R> {
  return (estado, mapa, ctx, params) =>
    convocatoriaDe(convocatoriasDe(estado), params.heroeId)
      ? rechazo(estado, CODIGOS_ERROR.movilizacionInvalida, 'Estás en un ejército en preparación: sepárate o cancélalo antes de salir por tu cuenta.')
      : manejador(estado, mapa, ctx, params);
}
