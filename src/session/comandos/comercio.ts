// Comandos de comercio: proponer un trueque entre asentamientos, colocar una orden de mercado y componer
// una caravana comercial (revamp, Doc 3.13: casco vacío + carros + animales).
import type { AcuerdoTrueque, AnimalTipo, Caravana, CarroTipo, Escuadron, RecursoTipo } from '../../domain/types';
import type { Instante } from '../../domain/tiempo';
import {
  aceptarTrueque as aceptarTruequeEngine,
  crearCaravanaVacia as crearCaravanaVaciaEngine,
  agregarCarroACaravana as agregarCarroEngine,
  comprarAnimalParaCaravana as comprarAnimalEngine,
  prepararCaravanaManual as prepararCaravanaManualEngine,
  cancelarPreparacionCaravana as cancelarPreparacionEngine,
  moverCarroEntreCaravanas as moverCarroEngine,
  moverCargaCarroAparcada as moverCargaCarroAparcadaEngine,
  enviarCaravanaAlOrigen as enviarCaravanaAlOrigenEngine,
  seleccionarEscoltaCaravana,
  asignarEscoltaACaravana as asignarEscoltaEngine,
  retirarEscoltaDeCaravana as retirarEscoltaEngine,
  proponerTrueque as proponerTruequeEngine,
  rechazarTrueque as rechazarTruequeEngine,
} from '../../engine/trade';
import { esCiudadano } from '../../engine/faccion';
import { alCampamentoPorIds, conEscuadrones, indiceTropa } from '../../engine/tropa';
import { anexarAlHistorialDeOrdenes, colocarOrdenMercado as colocarOrdenMercadoEngine, comerciarEnPlaza as comerciarEnPlazaEngine } from '../../engine/market';
import { capacidadCargaDe } from '../../engine/ejercitos';
import { computeTodasLasZonas } from '../../engine/zones';
import { RED_VACIA } from '../../engine/redCaminos';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAcuerdo, exigirAsentamiento, exigirCaravana, exigirColumnaDe, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento, eventos as construirEventos, type EventoDeComando } from './eventos';

export interface PayloadTruequePropuesto {
  acuerdoId: string;
  asentamientoAId: string;
  asentamientoBId: string;
  lineasA: LineaDeTrueque[];
  lineasB: LineaDeTrueque[];
}
export interface PayloadOrdenColocada {
  ordenId: string;
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: RecursoTipo;
  cantidad: number;
  precioUnitario: number;
}
export interface PayloadRespuestaTrueque {
  acuerdoId: string;
}
export interface PayloadCaravanaConstruida {
  caravanaId: string;
  asentamientoId: string;
}

/** Una línea de lo que ofrece un lado de un trueque: un recurso y cuánto. Un lado puede ofrecer varias (Doc 3.2). */
export interface LineaDeTrueque {
  recurso: RecursoTipo;
  cantidad: number;
}

export interface ParamsProponerTrueque {
  asentamientoAId: string;
  /** Lo que entrega A. */
  lineasA: LineaDeTrueque[];
  asentamientoBId: string;
  /** Lo que entrega B. */
  lineasB: LineaDeTrueque[];
}

/**
 * Ofrece un trueque al otro lado (Doc 3.2). **Solo lo OFRECE**: nace `'propuesto'` y no mueve nada de nadie
 * hasta que el otro contesta con `aceptarTrueque`.
 *
 * Por eso el Camino Comercial ya no se traza aqui sino al aceptar: un camino es infraestructura fisica
 * permanente (Doc 1.6), y hasta 2026-09-07 una propuesta unilateral bastaba para plantarle uno a un vecino
 * que no habia dicho ni si ni no.
 */
export const proponerTrueque = comando<ParamsProponerTrueque, { acuerdoId: string }>((estado, _mapa, ctx, params) => {
  const nuevo = proponerTruequeEngine(
    estado.asentamientos,
    params.asentamientoAId,
    params.asentamientoBId,
    params.lineasA,
    params.lineasB,
    ctx.instante,
    ctx.ids.siguiente()
  );

  const siguiente: GameSessionState = { ...estado, acuerdos: [...estado.acuerdos, nuevo] };
  return exito(
    siguiente,
    construirEventos(ctx, [
      {
        codigo: 'comercio.trueque_propuesto',
        mensaje: `Trueque propuesto: ${nuevo.id}.`,
        payload: { acuerdoId: nuevo.id, ...params } satisfies PayloadTruequePropuesto,
      },
    ]),
    { acuerdoId: nuevo.id }
  );
});

export interface ParamsResponderTrueque {
  acuerdoId: string;
}

/**
 * El lado receptor acepta (Doc 3.2). Es aqui, y no al proponer, donde el acuerdo empieza a obligar. El camino no
 * nace aqui sino con las caravanas que lo recorren (red de caminos, Doc 1.6, `engine/redCaminos.ts`).
 */
export const aceptarTrueque = comando<ParamsResponderTrueque, { acuerdoId: string }>((estado, _mapa, ctx, params) => {
  const acuerdo = aceptarTruequeEngine(exigirAcuerdo(estado, params.acuerdoId), ctx.instante);

  const narrados: EventoDeComando[] = [
    {
      codigo: 'comercio.trueque_aceptado',
      mensaje: `Trueque aceptado: ${acuerdo.id}.`,
      payload: { acuerdoId: acuerdo.id } satisfies PayloadRespuestaTrueque,
    },
  ];

  const siguiente: GameSessionState = { ...estado, acuerdos: conAcuerdo(estado.acuerdos, acuerdo) };
  return exito(siguiente, construirEventos(ctx, narrados), { acuerdoId: acuerdo.id });
});

/** El lado receptor dice que no (Doc 3.2). No traza camino ni mueve nada: solo deja constancia de la respuesta. */
export const rechazarTrueque = comando<ParamsResponderTrueque, { acuerdoId: string }>((estado, _mapa, ctx, params) => {
  const acuerdo = rechazarTruequeEngine(exigirAcuerdo(estado, params.acuerdoId));
  const siguiente: GameSessionState = { ...estado, acuerdos: conAcuerdo(estado.acuerdos, acuerdo) };
  return exito(
    siguiente,
    construirEventos(ctx, [
      {
        codigo: 'comercio.trueque_rechazado',
        mensaje: `Trueque rechazado: ${acuerdo.id}.`,
        payload: { acuerdoId: acuerdo.id } satisfies PayloadRespuestaTrueque,
      },
    ]),
    { acuerdoId: acuerdo.id }
  );
});

function conAcuerdo(acuerdos: readonly AcuerdoTrueque[], acuerdo: AcuerdoTrueque): AcuerdoTrueque[] {
  return acuerdos.map((a) => (a.id === acuerdo.id ? acuerdo : a));
}

export interface ParamsColocarOrdenMercado {
  asentamientoId: string;
  tipo: 'compra' | 'venta';
  recurso: RecursoTipo;
  cantidad: number;
  /** Sin precio, el motor usa el de referencia del mercado. */
  precio?: number;
}

export const colocarOrdenMercado = comando<ParamsColocarOrdenMercado, { ordenId: string }>((estado, _mapa, ctx, params) => {
  const nueva = colocarOrdenMercadoEngine(
    estado.asentamientos,
    params.asentamientoId,
    params.tipo,
    params.recurso,
    params.cantidad,
    ctx.instante,
    params.precio,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = { ...estado, ordenes: [...estado.ordenes, nueva] };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'mercado.orden_colocada',
        mensaje: `Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`,
        payload: {
          ordenId: nueva.id,
          asentamientoId: params.asentamientoId,
          tipo: nueva.tipo,
          // `OrdenMercado.recurso` es `string` en el dominio; `params.recurso` es el MISMO valor ya tipado.
          recurso: params.recurso,
          cantidad: nueva.cantidad,
          precioUnitario: nueva.precioUnitario,
        } satisfies PayloadOrdenColocada,
        asentamientoId: params.asentamientoId,
      }),
    ],
    { ordenId: nueva.id }
  );
});

export interface ParamsComerciarEnPlaza {
  heroeId: string;
  asentamientoId: string;
  ordenId: string;
  /** Cuanto se quiere mover. Se sirve lo que se pueda: el tope real sale de la orden, del almacen de la plaza,
   * de su oro, del carro y de lo que se lleve encima, y ninguno lo ve el cliente entero. */
  cantidad: number;
}

export interface PayloadComercioEnPlaza {
  heroeId: string;
  asentamientoId: string;
  ordenId: string;
  recurso: RecursoTipo;
  cantidad: number;
  valor: number;
  comision: number;
  /** El héroe y la plaza son de Facciones distintas: solo ese comercio alimenta los mercados de mercenarios (Doc 1.9b). */
  entreFacciones: boolean;
}

/**
 * **El mostrador** (Doc 3.3): el jugador toma una orden de la plaza donde esta, con su columna en la puerta.
 *
 * Es el unico camino por el que la mercancia de una orden cambia de manos desde que el emparejamiento
 * automatico entre plazas desaparecio (`Consideraciones/Comercio_Fisico_Definicion.md`). Toda la regla vive en
 * el motor; aqui solo se resuelven la columna, la plaza y la orden, y se recomponen las tres listas.
 */
export const comerciarEnPlaza = comando<ParamsComerciarEnPlaza, { cantidad: number; valor: number; comision: number }>(
  (estado, _mapa, ctx, params) => {
    const columna = exigirColumnaDe(estado, params.heroeId);
    const plaza = exigirAsentamiento(estado, params.asentamientoId);
    const orden = estado.ordenes.find((o) => o.id === params.ordenId) ?? estado.historialOrdenes?.find((o) => o.id === params.ordenId);
    if (!orden) rechazar(CODIGOS_ERROR.ordenNoExiste);

    const resultado = comerciarEnPlazaEngine(
      columna,
      params.heroeId,
      plaza,
      orden,
      params.cantidad,
      capacidadCargaDe(columna, estado.caravanas),
      ctx.instante
    );

    const siguiente: GameSessionState = {
      ...conAsentamiento(estado, resultado.plaza),
      ejercitos: estado.ejercitos.map((e) => (e.id === resultado.ejercito.id ? resultado.ejercito : e)),
      // La que se acaba de cumplir sale de las que están en pie.
      ...(resultado.orden.estado === 'activa'
        ? { ordenes: estado.ordenes.map((o) => (o.id === resultado.orden.id ? resultado.orden : o)) }
        : {
            ordenes: estado.ordenes.filter((o) => o.id !== resultado.orden.id),
            historialOrdenes: anexarAlHistorialDeOrdenes(estado.historialOrdenes, [resultado.orden]),
          }),
    };

    const sentido = orden.tipo === 'venta' ? 'compra' : 'vende';
    return exito(
      siguiente,
      [
        evento(ctx, {
          codigo: 'mercado.comercio_en_plaza',
          mensaje: `Un jugador ${sentido} ${resultado.cantidad.toFixed(1)} ${orden.recurso} en el mercado de ${plaza.id} por ${resultado.valor.toFixed(1)} oro (comisión ${resultado.comision.toFixed(1)}).`,
          payload: {
            heroeId: params.heroeId,
            asentamientoId: plaza.id,
            ordenId: orden.id,
            recurso: orden.recurso as RecursoTipo,
            cantidad: resultado.cantidad,
            valor: resultado.valor,
            comision: resultado.comision,
            entreFacciones: estado.facciones.some((f) => esCiudadano(f, params.heroeId)) && !estado.facciones.some((f) => f.id === plaza.faccionId && esCiudadano(f, params.heroeId)),
          } satisfies PayloadComercioEnPlaza,
          asentamientoId: plaza.id,
        }),
      ],
      { cantidad: resultado.cantidad, valor: resultado.valor, comision: resultado.comision }
    );
  }
);

export interface ParamsCrearCaravana {
  asentamientoId: string;
}

/**
 * Crea una caravana comercial VACÍA (revamp, Doc 3.13.2): cuenta contra el cupo del Mercado pero no puede
 * viajar hasta que se le añadan carros (`agregarCarroCaravana`) y animales (`comprarAnimalCaravana`). El
 * coste está en las piezas.
 */
export const crearCaravana = comando<ParamsCrearCaravana, { caravanaId: string }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  const { asentamiento: actualizado, caravana } = crearCaravanaVaciaEngine(
    asentamiento,
    estado.caravanas,
    ctx.instante,
    ctx.ids.siguiente()
  );
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, actualizado),
    caravanas: [...estado.caravanas, caravana],
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_construida',
        mensaje: `Crea una caravana comercial vacía (${caravana.id}).`,
        payload: { caravanaId: caravana.id, asentamientoId: params.asentamientoId } satisfies PayloadCaravanaConstruida,
        asentamientoId: params.asentamientoId,
      }),
    ],
    { caravanaId: caravana.id }
  );
});

function conCaravana(estado: GameSessionState, actualizada: Caravana): GameSessionState {
  return { ...estado, caravanas: estado.caravanas.map((c) => (c.id === actualizada.id ? actualizada : c)) };
}

export interface ParamsAgregarCarro {
  caravanaId: string;
  tipoCarro: CarroTipo;
}

/** Fabrica un carro y lo añade a una caravana disponible (Doc 3.13.2). Básico → Mercado; reforzado → Carpintería. */
export const agregarCarroCaravana = comando<ParamsAgregarCarro, { carros: number }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const asentamiento = exigirAsentamiento(estado, caravana.origenAsentamientoId);
  const r = agregarCarroEngine(caravana, asentamiento, params.tipoCarro);
  const siguiente: GameSessionState = { ...conAsentamiento(conCaravana(estado, r.caravana), r.asentamiento) };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_carro_agregado',
        mensaje: `Añade un carro ${params.tipoCarro} a la caravana ${caravana.id} (${r.caravana.carros!.length} en total).`,
        payload: { caravanaId: caravana.id, tipoCarro: params.tipoCarro, carros: r.caravana.carros!.length },
        asentamientoId: asentamiento.id,
      }),
    ],
    { carros: r.caravana.carros!.length }
  );
});

export interface ParamsComprarAnimal {
  caravanaId: string;
  carroIndice: number;
  tipoAnimal: AnimalTipo;
}

/** Compra un animal y lo engancha a un carro sin tracción de una caravana disponible (Doc 3.13.2). */
export const comprarAnimalCaravana = comando<ParamsComprarAnimal, { caravanaId: string }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const asentamiento = exigirAsentamiento(estado, caravana.origenAsentamientoId);
  const r = comprarAnimalEngine(caravana, asentamiento, params.carroIndice, params.tipoAnimal);
  const siguiente: GameSessionState = { ...conAsentamiento(conCaravana(estado, r.caravana), r.asentamiento) };
  return exito(
    siguiente,
    [
      // Cuenta para el logro de `cria_caballar` (Doc 6.6): lo suma `exito` desde el evento.
      evento(ctx, {
        codigo: 'comercio.caravana_animal_comprado',
        mensaje: `Compra un ${params.tipoAnimal} para el carro ${params.carroIndice} de la caravana ${caravana.id}.`,
        payload: { caravanaId: caravana.id, carroIndice: params.carroIndice, tipoAnimal: params.tipoAnimal },
        asentamientoId: asentamiento.id,
      }),
    ],
    { caravanaId: caravana.id }
  );
});

export interface ParamsReservarCaravana {
  caravanaId: string;
  reservada: boolean;
}

/** Marca o desmarca una caravana como reservada para envíos manuales (Doc 3.13.5) — fuera del reparto automático. */
export const reservarCaravana = comando<ParamsReservarCaravana, { reservada: boolean }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const siguiente = conCaravana(estado, { ...caravana, reservadaManual: params.reservada });
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_reserva',
        mensaje: `La caravana ${caravana.id} ${params.reservada ? 'queda reservada para envíos manuales' : 'vuelve al reparto automático'}.`,
        payload: { caravanaId: caravana.id, reservada: params.reservada },
        asentamientoId: caravana.origenAsentamientoId,
      }),
    ],
    { reservada: params.reservada }
  );
});

export interface ParamsPrepararCaravana {
  caravanaId: string;
  heroeId: string;
  destinoAsentamientoId: string;
  /** Mapa recurso -> cantidad: qué se carga del almacén del origen, hasta la capacidad de la caravana. */
  carga: Record<string, number>;
  /** Escuadrones del jugador que van de escolta sin héroe (Doc 3.13.4), hasta el cupo de Liderazgo del Mercado. */
  escoltaEscuadronIds?: string[];
  /** Hora de mundo (`Instante`, ms) a la que sale: programada. Con todo reservado desde ya. Ausente = sale al acabar la preparación. */
  salirEn?: number;
}

/**
 * Lanza una caravana comercial a mano (Doc 3.13.3): elige carga, destino y una escolta opcional (Doc 3.13.4).
 * La caravana pasa por `'preparando'` en el origen —tanto más tiempo cuantos más carros— y al terminar sale
 * sola en el tick. `cancelarCaravana` la revierte mientras siga preparándose. Con `salirEn` queda programada: espera
 * en el origen, con carga, carros y escolta reservados desde ya, hasta esa hora.
 */
export const prepararCaravana = comando<ParamsPrepararCaravana, { caravanaId: string; preparaHasta?: number }>(
  (estado, mapa, ctx, params) => {
    const caravana = exigirCaravana(estado, params.caravanaId);
    const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId);
    const destino = exigirAsentamiento(estado, params.destinoAsentamientoId);

    const jugador = estado.heroes.find((j) => j.id === params.heroeId);
    const escolta = seleccionarEscoltaCaravana(origen, jugador, params.escoltaEscuadronIds ?? []);

    const territorio = { red: estado.red ?? RED_VACIA, asentamientos: estado.asentamientos, zonas: computeTodasLasZonas(estado.asentamientos) };
    const r = prepararCaravanaManualEngine(caravana, origen, destino, params.carga, escolta, escoltaCedida(estado, caravana), mapa, territorio, ctx.instante, params.salirEn as Instante | undefined);
    const siguiente: GameSessionState = {
      ...conAsentamiento(conCaravana(estado, r.caravana), r.asentamiento),
      heroes: conEscuadrones(estado.heroes, r.tropa),
      red: r.red,
    };
    return exito(
      siguiente,
      [
        evento(ctx, {
          codigo: 'comercio.caravana_preparando',
          mensaje: `La caravana ${caravana.id} carga para ${destino.id}${
            escolta.length > 0 ? ` con ${escolta.length} escuadrón(es) de escolta` : ''
          } y ${r.caravana.estado === 'preparando' ? 'se prepara' : 'sale ya'}.`,
          payload: { caravanaId: caravana.id, destinoId: destino.id, estado: r.caravana.estado, escolta: escolta.length },
          asentamientoId: origen.id,
        }),
      ],
      { caravanaId: caravana.id, preparaHasta: r.caravana.preparaHasta }
    );
  }
);

/** Las escuadras que la caravana ya tiene cedidas, de cualquier héroe. */
function escoltaCedida(estado: GameSessionState, caravana: Caravana): Escuadron[] {
  const tropa = indiceTropa(estado.heroes);
  return (caravana.escoltaIds ?? []).flatMap((id) => tropa.get(id) ?? []);
}

export interface ParamsAsignarEscolta {
  caravanaId: string;
  heroeId: string;
  /** Escuadrones propios, en su campamento y fuera de la guarnición. */
  escuadronIds: string[];
}

/**
 * Cede escuadrones a una caravana comercial parada en su origen (Doc 3.13.4), sin lanzarla: el reparto automático la manda ya escoltada.
 * Cada residente cede los suyos; el cupo (puntos de Liderazgo según el Mercado) es de la caravana y lo comparten. No gastan el Liderazgo de quien presta.
 */
export const asignarEscolta = comando<ParamsAsignarEscolta, { caravanaId: string; escolta: number }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId);
  const jugador = estado.heroes.find((j) => j.id === params.heroeId);
  const escolta = seleccionarEscoltaCaravana(origen, jugador, params.escuadronIds);
  const r = asignarEscoltaEngine(caravana, origen, escolta, escoltaCedida(estado, caravana));
  const siguiente: GameSessionState = { ...conCaravana(estado, r.caravana), heroes: conEscuadrones(estado.heroes, r.tropa) };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_escolta_cedida',
        mensaje: `${jugador!.displayName} cede ${escolta.length} escuadrón(es) de escolta a la caravana ${caravana.id}.`,
        payload: { caravanaId: caravana.id, heroeId: params.heroeId, escuadronIds: escolta.map((e) => e.id), escoltaTotal: r.caravana.escoltaIds?.length ?? 0 },
        asentamientoId: origen.id,
      }),
    ],
    { caravanaId: caravana.id, escolta: r.caravana.escoltaIds?.length ?? 0 }
  );
});

export interface ParamsQuitarEscolta {
  caravanaId: string;
  heroeId: string;
  /** Cuáles de los suyos retira; ausente = todos los que tenga cedidos a esa caravana. */
  escuadronIds?: string[];
}

/** Retira de la escolta de una caravana parada en su origen los escuadrones del héroe; vuelven a su campamento. Nadie retira los de otro. */
export const quitarEscolta = comando<ParamsQuitarEscolta, { caravanaId: string; retirados: number }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const jugador = estado.heroes.find((j) => j.id === params.heroeId);
  const suyos = (jugador?.escuadrones ?? []).filter((e) => (caravana.escoltaIds ?? []).includes(e.id)).map((e) => e.id);
  const ids = params.escuadronIds ?? suyos;
  if (ids.length === 0 || !ids.every((id) => suyos.includes(id))) rechazar(CODIGOS_ERROR.comercioCaravanaInvalida);

  const r = retirarEscoltaEngine(caravana, ids);
  const siguiente: GameSessionState = { ...conCaravana(estado, r.caravana), heroes: alCampamentoPorIds(estado.heroes, r.escoltaLiberada) };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_escolta_retirada',
        mensaje: `${jugador!.displayName} retira ${ids.length} escuadrón(es) de la escolta de la caravana ${caravana.id}.`,
        payload: { caravanaId: caravana.id, heroeId: params.heroeId, escuadronIds: ids },
        asentamientoId: caravana.origenAsentamientoId,
      }),
    ],
    { caravanaId: caravana.id, retirados: ids.length }
  );
});

export interface ParamsCancelarCaravana {
  caravanaId: string;
}

/** Cancela una caravana que se está preparando y devuelve la carga al almacén (Doc 3.13.3). */
export const cancelarCaravana = comando<ParamsCancelarCaravana, { caravanaId: string }>((estado, _mapa, ctx, params) => {
  const caravana = exigirCaravana(estado, params.caravanaId);
  const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId);
  const r = cancelarPreparacionEngine(caravana, origen);
  const siguiente: GameSessionState = {
    ...conAsentamiento(conCaravana(estado, r.caravana), r.asentamiento),
    heroes: alCampamentoPorIds(estado.heroes, r.escoltaLiberada),
  };
  return exito(
    siguiente,
    [
      evento(ctx, {
        codigo: 'comercio.caravana_cancelada',
        mensaje: `Se cancela la preparación de la caravana ${caravana.id}; la carga vuelve al almacén.`,
        payload: { caravanaId: caravana.id },
        asentamientoId: origen.id,
      }),
    ],
    { caravanaId: caravana.id }
  );
});

export interface ParamsMoverCarro {
  desdeCaravanaId: string;
  haciaCaravanaId: string;
  carroIndice: number;
}

export interface ParamsMoverCargaCaravanaAparcada {
  heroeId: string;
  caravanaId: string;
  /** La plaza que hospeda la caravana aparcada (donde está su carro). */
  asentamientoId: string;
  recurso: RecursoTipo;
  cantidad: number;
  sentido: 'cargar' | 'descargar';
}

/**
 * Carga o descarga el carro de una caravana `'aparcada'` (Ocupacion §2.3d) contra el almacén de la plaza que
 * la hospeda. Toda la regla vive en el motor; aquí solo se resuelven la caravana y la plaza.
 */
export const moverCargaCaravanaAparcada = comando<ParamsMoverCargaCaravanaAparcada, { movido: number }>(
  (estado, _mapa, ctx, params) => {
    const caravana = exigirCaravana(estado, params.caravanaId);
    const plaza = exigirAsentamiento(estado, params.asentamientoId);
    if (caravana.posicionActual.x !== plaza.posicion.x || caravana.posicionActual.y !== plaza.posicion.y) {
      rechazar(CODIGOS_ERROR.caravanaNoAparcadaAqui);
    }
    const r = moverCargaCarroAparcadaEngine(caravana, plaza, params.recurso, params.cantidad, params.sentido);
    const antes = caravana.contenido[params.recurso] ?? 0;
    const despues = r.caravana.contenido[params.recurso] ?? 0;
    const movido = Math.abs(despues - antes);
    return exito(
      conCaravana(conAsentamiento(estado, r.plaza), r.caravana),
      [
        evento(ctx, {
          codigo: 'comercio.carga_caravana_aparcada',
          mensaje: `La caravana ${caravana.id} ${params.sentido === 'cargar' ? 'carga' : 'descarga'} ${movido.toFixed(0)} ${params.recurso} en ${plaza.id}.`,
          payload: { caravanaId: caravana.id, asentamientoId: plaza.id, recurso: params.recurso, cantidad: movido, sentido: params.sentido },
          asentamientoId: plaza.id,
        }),
      ],
      { movido }
    );
  }
);

export interface ParamsEnviarCaravanaAlOrigen {
  heroeId: string;
  caravanaId: string;
  /** La plaza que hospeda la caravana aparcada. */
  asentamientoId: string;
}

/**
 * Envía una caravana `'aparcada'` (Ocupacion §2.3d) de vuelta a su origen. Vacía: aparece allí al instante.
 * Con carga: viaja el mapa (`'retornando'`) y vuelca en el almacén del origen al llegar.
 */
export const enviarCaravanaAlOrigen = comando<ParamsEnviarCaravanaAlOrigen, { caravanaId: string; enTransito: boolean }>(
  (estado, mapa, ctx, params) => {
    const caravana = exigirCaravana(estado, params.caravanaId);
    const anfitriona = exigirAsentamiento(estado, params.asentamientoId);
    const origen = exigirAsentamiento(estado, caravana.origenAsentamientoId);
    if (caravana.posicionActual.x !== anfitriona.posicion.x || caravana.posicionActual.y !== anfitriona.posicion.y) {
      rechazar(CODIGOS_ERROR.caravanaNoAparcadaAqui);
    }
    const r = enviarCaravanaAlOrigenEngine(caravana, anfitriona, origen, mapa);
    const enTransito = r.caravana.estado === 'retornando';
    return exito(
      conCaravana(estado, r.caravana),
      [
        evento(ctx, {
          codigo: 'comercio.caravana_enviada_al_origen',
          mensaje: enTransito
            ? `La caravana ${caravana.id} sale de ${anfitriona.id} de vuelta a ${origen.id} con su carga.`
            : `La caravana ${caravana.id} vuelve vacía a ${origen.id}.`,
          payload: { caravanaId: caravana.id, origenId: origen.id, anfitrionaId: anfitriona.id, enTransito },
          asentamientoId: origen.id,
        }),
      ],
      { caravanaId: caravana.id, enTransito }
    );
  }
);

/** Mueve un carro (con su animal) entre dos caravanas disponibles del mismo asentamiento (Doc 3.13.5). */
export const moverCarroCaravana = comando<ParamsMoverCarro, { desdeCarros: number; haciaCarros: number }>(
  (estado, _mapa, ctx, params) => {
    const desde = exigirCaravana(estado, params.desdeCaravanaId);
    const hacia = exigirCaravana(estado, params.haciaCaravanaId);
    const r = moverCarroEngine(desde, hacia, params.carroIndice);
    const siguiente = conCaravana(conCaravana(estado, r.desde), r.hacia);
    return exito(
      siguiente,
      [
        evento(ctx, {
          codigo: 'comercio.caravana_carro_movido',
          mensaje: `Mueve un carro de la caravana ${desde.id} a la ${hacia.id}.`,
          payload: { desdeCaravanaId: desde.id, haciaCaravanaId: hacia.id },
          asentamientoId: desde.origenAsentamientoId,
        }),
      ],
      { desdeCarros: r.desde.carros!.length, haciaCarros: r.hacia.carros!.length }
    );
  }
);
