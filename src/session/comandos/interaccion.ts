// Las acciones que la geometría OFRECE y el jugador decide (Doc 5.12.3).
//
// Este módulo es el reverso de una decisión de diseño: hasta ahora, acercarse a un enemigo bastaba para que
// el motor resolviera el choque dentro del tick. Ahora la proximidad solo ABRE opciones —mirar, perseguir,
// atacar— y ninguna ocurre sin que alguien la pida. Un encuentro deja de ser un accidente por pasar cerca.
//
// Los anillos, de fuera hacia dentro, y qué habilita cada uno:
//
//   150 / 80  ves que hay algo, y quién es
//    40       puedes INSPECCIONARLO — y él se entera de que lo miras
//    15       puedes atacarlo
//    10       puedes cruzar la puerta de una plaza (eso vive en `presencia.ts`)
//
// El de 40 es el que hace de esto un juego de dos: la telemetría que la proyección niega a distancia se
// consigue acercándose, y acercarse te delata. Nadie audita al rival desde el sofá.
import {
  asediarPlaza,
  atacarCampamento,
  atacarColumna,
  capacidadCargaDe,
  dejarDePerseguir as dejarDePerseguirEngine,
  inspeccionarCaravana as inspeccionarCaravanaEngine,
  inspeccionarColumna,
  interceptar,
  MovilizacionInvalidaError,
  perseguir as perseguirEngine,
  validarAlcance,
  validarAsedio,
  validarAtaqueAColumna,
  type ComposicionColumna,
  type ContenidoCaravana,
} from '../../engine/ejercitos';
import {
  abrirBatalla,
  aperturaContraCampamento,
  aperturaContraCaravana,
  aperturaContraColumna,
  aperturaDeAsedio,
  bloqueosDe,
  eventosDeBatalla,
  hayHumano,
  idDeBatalla,
  type Apertura,
} from '../batallas';
import { heridosEn, herir } from '../../engine/heroe';
import { conEscolta, indiceTropa, sinEscolta } from '../../engine/tropa';
import type { Asentamiento, Ejercito } from '../../domain/types';
import { minutos, sumar, type Instante } from '../../domain/tiempo';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conColumnas, conTropaDe, exigirAsentamiento, exigirCampamento, exigirCaravana, exigirColumnaDe, exigirEjercito } from './ayudas';
import { desdeCrudos, evento } from './eventos';

/** A qué se puede apuntar desde el menú de interacción: una columna o una caravana. Es la misma forma que usa
 * la persecución, y no por casualidad — se persigue lo que se puede mirar. */
export type ObjetivoDeInteraccion = { tipo: 'ejercito'; id: string } | { tipo: 'caravana'; id: string };

export interface ParamsInspeccionar {
  heroeId: string;
  objetivo: ObjetivoDeInteraccion;
}

export interface PayloadObservado {
  /** Quién mira. Va en el payload porque el aviso es la mitad de la mecánica: el observado tiene derecho a
   * saber que lo miran, y a poder decidir algo al respecto. */
  observadorId: string;
  objetivo: ObjetivoDeInteraccion;
}

/**
 * Mirar de cerca (Doc 5.12.3): qué tropas lleva esa columna y de quién son, o qué carga esa caravana.
 *
 * **No es gratis, y ahí está el diseño.** Hay que meterse en el anillo de 40 —dentro del alcance de una
 * columna que quiera cazarte— y el observado **recibe aviso**. La información que la proyección no regala a
 * distancia se compra acercándose y delatándose.
 *
 * El aviso se emite atribuido a la plaza de origen del observado, que es como esta partida decide audiencias:
 * le llega a su Facción, no al mundo.
 */
export const inspeccionar = comando<ParamsInspeccionar, ComposicionColumna | ContenidoCaravana>((estado, _mapa, ctx, params) => {
  const observador = exigirColumnaDe(estado, params.heroeId);

  if (params.objetivo.tipo === 'ejercito') {
    const objetivo = exigirEjercito(estado, params.objetivo.id);
    const composicion = inspeccionarColumna(observador, conTropaDe(estado, objetivo));

    return exito(
      conHistorialDeJugador(estado, params.heroeId, `Inspecciona la columna ${objetivo.id}.`),
      [
        evento(ctx, {
          codigo: 'columna.observada',
          mensaje: `Alguien se ha acercado a mirar la columna ${objetivo.id}.`,
          payload: { observadorId: params.heroeId, objetivo: params.objetivo } satisfies PayloadObservado,
          // Atribuido al OBSERVADO, no al que mira: el aviso es para quien lo sufre.
          asentamientoId: objetivo.origenAsentamientoId,
        }),
      ],
      composicion
    );
  }

  const caravana = exigirCaravana(estado, params.objetivo.id);
  // Escoltada = por un ejército (Doc 5.13.3) o por escuadrones cedidos sin héroe (Doc 3.13.4).
  const escoltada =
    estado.ejercitos.some((e) => e.caravanasAdjuntasIds.includes(caravana.id)) || (caravana.escoltaIds?.length ?? 0) > 0;
  const contenido = inspeccionarCaravanaEngine(observador, caravana, escoltada);

  return exito(
    conHistorialDeJugador(estado, params.heroeId, `Inspecciona la caravana ${caravana.id}.`),
    [
      evento(ctx, {
        codigo: 'caravana.observada',
        mensaje: `Alguien se ha acercado a mirar la caravana ${caravana.id}.`,
        payload: { observadorId: params.heroeId, objetivo: params.objetivo } satisfies PayloadObservado,
        asentamientoId: caravana.origenAsentamientoId,
      }),
    ],
    contenido
  );
});

/** `atacar` apunta además a un campamento de bandidos (Doc 1.9) y a una plaza, que se asedia (Doc 5.12.4): no se
 * miran ni se persiguen. */
export type ObjetivoDeAtaque = ObjetivoDeInteraccion | { tipo: 'campamento'; id: string } | { tipo: 'asentamiento'; id: string };

export interface ParamsAtacar {
  heroeId: string;
  objetivo: ObjetivoDeAtaque;
}

export interface ParamsPerseguir {
  heroeId: string;
  objetivo: ObjetivoDeInteraccion;
}

export interface ParamsDejarDePerseguir {
  heroeId: string;
}

export interface PayloadPersecucion {
  ejercitoId: string;
  heroeId: string;
  objetivo?: ObjetivoDeInteraccion;
}

/** Un héroe herido no entra en batallas ni persigue (Doc 5.16.4). Devuelve los heridos de ahora, que el motor necesita. */
function exigirSano(estado: GameSessionState, heroeId: string, ahora: Instante): Set<string> {
  const heridos = heridosEn(estado.heroes, ahora);
  if (heridos.has(heroeId)) throw new MovilizacionInvalidaError('Estás herido: no puedes entrar en batalla ni perseguir.');
  return heridos;
}

/**
 * Atacar lo que tienes delante, a distancia de choque (Doc 5.12.3). Sustituye al combate que el tick
 * resolvía solo por geometría: acercarse ya no basta.
 *
 * Los héroes del bando derrotado quedan **heridos** (Doc 5.16.4) y pierde la mitad de su carro, sea viajero o
 * Ejército. Un herido no ataca, sus escuadras no combaten, y a una columna de solo heridos no se la puede tocar.
 *
 * Un campamento de bandidos también se ataca así, con la columna que llega a él (Doc 1.9): si cae, su recompensa
 * va al carro y se agenda su reaparición. Y una plaza de otra Facción: atacarla es asediarla, una sola batalla
 * cuando se ordena, porque llegar a ella solo es acampar (Doc 5.12.4).
 *
 * Con servidores de batalla y algún héroe humano, el combate no se resuelve aquí: se abre una batalla de Unity y se
 * devuelve su `battleId` (Doc 5.10, doc 02 §3.1).
 */
export const atacar = comando<ParamsAtacar, { battleId: string } | undefined>((estado, _mapa, ctx, params) => {
  const heridos = exigirSano(estado, params.heroeId, ctx.instante);
  const atacante = exigirColumnaDe(estado, params.heroeId);

  if (ctx.batallasEnUnity) {
    const apertura = aperturaDeAtaque(estado, atacante, params, heridos, ctx.instante);
    if (hayHumano(apertura)) {
      const { estado: conBatalla, batalla } = abrirBatalla(estado, apertura, ctx.instante, idDeBatalla(estado.gameId, ctx.ids.siguiente()));
      return exito(
        conHistorialDeJugador(conBatalla, params.heroeId, `Abre la batalla ${batalla.id}.`),
        eventosDeBatalla(conBatalla, batalla, 'batalla.abierta', 'Empieza una batalla.').map((e) => evento(ctx, e)),
        { battleId: batalla.id }
      );
    }
  }

  if (params.objetivo.tipo === 'asentamiento') {
    const plaza = plazaAsediable(estado, atacante, params.objetivo.id, heridos, ctx.instante);
    const asedio = asediarPlaza(
      conTropaDe(estado, atacante),
      plaza,
      { asentamientos: estado.asentamientos, ejercitos: estado.ejercitos, heroes: estado.heroes, facciones: [...estado.facciones], relaciones: estado.relaciones },
      heridos,
      ctx.instante,
      ctx.rng
    );
    const siguiente = conColumnas(
      { ...estado, asentamientos: asedio.asentamientos, heroes: asedio.heroes, facciones: asedio.facciones },
      [asedio.ejercito, ...asedio.columnas]
    );
    return exito(
      conHistorialDeJugador(siguiente, params.heroeId, `Asedia ${plaza.id}.`),
      asedio.eventos.map((e) => evento(ctx, typeof e === 'string' ? { codigo: 'legado', mensaje: e } : e))
    );
  }

  if (params.objetivo.tipo === 'campamento') {
    const campamento = exigirCampamento(estado, params.objetivo.id);
    const asalto = atacarCampamento(conTropaDe(estado, atacante), campamento, [...estado.facciones], capacidadCargaDe(atacante, estado.caravanas), heridos, ctx.rng);
    const trasAsalto = conColumnas(estado, [asalto.ejercito]);
    const siguiente: GameSessionState = {
      ...trasAsalto,
      facciones: asalto.facciones,
      heroes: herir(trasAsalto.heroes, asalto.vencidos, ctx.instante),
      // Al destruirlo se agenda su reaparición; el spawn en sí lo evalúa el tick (`avanzarSpawnBandidos`).
      campamentosBandidos: asalto.destruido ? estado.campamentosBandidos.filter((c) => c.id !== campamento.id) : estado.campamentosBandidos,
      bandidosProximoSpawnEn: asalto.destruido ? sumar(ctx.instante, minutos(CAMPAMENTOS_BANDIDOS.respawnMinutos)) : estado.bandidosProximoSpawnEn,
    };
    return exito(
      conHistorialDeJugador(siguiente, params.heroeId, `Ataca el campamento de bandidos ${campamento.id}.`),
      desdeCrudos(ctx, asalto.eventos, atacante.origenAsentamientoId)
    );
  }

  if (params.objetivo.tipo === 'ejercito') {
    const defensor = exigirEjercito(estado, params.objetivo.id);
    const choque = atacarColumna(
      conTropaDe(estado, atacante),
      conTropaDe(estado, defensor),
      [...estado.facciones],
      estado.relaciones,
      estado.caravanas,
      heridos,
      ctx.rng
    );

    const trasChoque = conColumnas(estado, [choque.atacante, choque.defensor]);
    const siguiente: GameSessionState = { ...trasChoque, facciones: choque.facciones, heroes: herir(trasChoque.heroes, choque.vencidos, ctx.instante) };

    return exito(
      conHistorialDeJugador(siguiente, params.heroeId, `Ataca a la columna ${defensor.id}.`),
      // El combate se narra a los DOS hogares: el que lo sufre tiene tanto derecho a saberlo como el que lo
      // ordena, y sin la segunda atribución el atacado se enteraría por las bajas.
      [
        ...desdeCrudos(ctx, choque.eventos, atacante.origenAsentamientoId),
        ...desdeCrudos(ctx, choque.eventos, defensor.origenAsentamientoId),
      ]
    );
  }

  const caravana = exigirCaravana(estado, params.objetivo.id);
  const emboscada = interceptar(
    conTropaDe(estado, atacante),
    conEscolta(caravana, indiceTropa(estado.heroes)),
    capacidadCargaDe(atacante, estado.caravanas),
    heridos,
    ctx.rng
  );
  // La escolta vuelve a su héroe: la de una caravana capturada, a 0 y al campamento (Doc 5.15.4).
  const queda = emboscada.caravana ? sinEscolta(emboscada.caravana) : undefined;
  const conAtacante = conColumnas(estado, [emboscada.ejercito], [...emboscada.escoltaPerdida, ...(queda?.tropa ?? [])]);
  const siguiente: GameSessionState = {
    ...conAtacante,
    heroes: herir(conAtacante.heroes, emboscada.vencidos, ctx.instante),
    caravanas: queda
      ? conAtacante.caravanas.map((c) => (c.id === caravana.id ? queda.caravana : c))
      : conAtacante.caravanas.filter((c) => c.id !== caravana.id),
  };

  return exito(
    conHistorialDeJugador(siguiente, params.heroeId, `Intercepta la caravana ${caravana.id}.`),
    [
      ...desdeCrudos(ctx, emboscada.eventos, atacante.origenAsentamientoId),
      ...desdeCrudos(ctx, emboscada.eventos, caravana.origenAsentamientoId),
    ]
  );
});

/** La batalla que abriría este ataque, validado igual que el combate con números. */
function aperturaDeAtaque(estado: GameSessionState, atacante: Ejercito, params: ParamsAtacar, heridos: ReadonlySet<string>, ahora: Instante): Apertura {
  const { objetivo } = params;
  if (objetivo.tipo === 'asentamiento') {
    return aperturaDeAsedio(estado, atacante, plazaAsediable(estado, atacante, objetivo.id, heridos, ahora), params.heroeId, heridos);
  }
  if (objetivo.tipo === 'campamento') {
    const campamento = exigirCampamento(estado, objetivo.id);
    validarAlcance(atacante, campamento.posicion, heridos, 'atacar');
    return aperturaContraCampamento(estado, atacante, campamento, params.heroeId, heridos);
  }
  if (objetivo.tipo === 'ejercito') {
    const defensor = exigirEjercito(estado, objetivo.id);
    validarAtaqueAColumna(atacante, defensor, estado.relaciones, heridos);
    return aperturaContraColumna(estado, atacante, defensor, params.heroeId, heridos);
  }
  const caravana = exigirCaravana(estado, objetivo.id);
  validarAlcance(atacante, caravana.posicionActual, heridos, 'interceptar');
  return aperturaContraCaravana(estado, atacante, caravana, params.heroeId, heridos);
}

/** La plaza que se va a asediar, si se puede. Una que ya está en una batalla no se asedia otra vez: se espera a la
 * puerta, o se une uno a esa batalla (Doc 5.15.1). */
function plazaAsediable(estado: GameSessionState, atacante: Ejercito, plazaId: string, heridos: ReadonlySet<string>, ahora: Instante): Asentamiento {
  const plaza = exigirAsentamiento(estado, plazaId);
  if (bloqueosDe(estado, ahora).asentamientos.has(plaza.id)) {
    throw new MovilizacionInvalidaError('Esa plaza ya está en una batalla: se espera a la puerta, o se une uno a ella.');
  }
  validarAsedio(atacante, plaza, heridos, ahora);
  return plaza;
}

/**
 * Ir a por alguien (Doc 5.12.3). No es un destino sino un objetivo que se mueve: la ruta se recalcula cada
 * tick hacia donde esté, y al alcanzarlo, a 15, se ofrece atacar (`columna.presa_alcanzada`).
 *
 * Termina de tres formas: alcanzándolo, rectificando el rumbo con `marcharA` o soltándolo. Un herido no persigue,
 * y a una columna de solo heridos no se la puede perseguir (Doc 5.16.4).
 */
export const perseguir = comando<ParamsPerseguir, void>((estado, _mapa, ctx, params) => {
  const heridos = exigirSano(estado, params.heroeId, ctx.instante);
  const columna = exigirColumnaDe(estado, params.heroeId);
  // Que el objetivo exista lo comprueba aquí y no el motor: es una entidad que buscar, no una regla.
  let presa: Ejercito | undefined;
  if (params.objetivo.tipo === 'ejercito') presa = exigirEjercito(estado, params.objetivo.id);
  else exigirCaravana(estado, params.objetivo.id);

  const cazando = perseguirEngine(columna, params.objetivo, heridos, presa);

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === cazando.id ? cazando : e)) },
      params.heroeId,
      `Sale en persecución de ${params.objetivo.id}.`
    ),
    [
      evento(ctx, {
        codigo: 'columna.persecucion_iniciada',
        mensaje: `Una columna sale en persecución de ${params.objetivo.id}.`,
        payload: { ejercitoId: cazando.id, heroeId: params.heroeId, objetivo: params.objetivo } satisfies PayloadPersecucion,
        asentamientoId: cazando.origenAsentamientoId,
      }),
    ]
  );
});

/** Soltar la presa. Lo hace también cualquier `marcharA`: elegir destino nuevo es dejar de ir detrás. */
export const dejarDePerseguir = comando<ParamsDejarDePerseguir, void>((estado, _mapa, ctx, params) => {
  const columna = exigirColumnaDe(estado, params.heroeId);
  const suelta = dejarDePerseguirEngine(columna);

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === suelta.id ? suelta : e)) },
      params.heroeId,
      'Abandona la persecución.'
    ),
    [
      evento(ctx, {
        codigo: 'columna.persecucion_abandonada',
        mensaje: 'Una columna abandona la persecución.',
        payload: { ejercitoId: suelta.id, heroeId: params.heroeId } satisfies PayloadPersecucion,
        asentamientoId: suelta.origenAsentamientoId,
      }),
    ]
  );
});
