// Comandos de comercio: proponer un trueque entre asentamientos, colocar una orden de mercado y componer
// una caravana comercial (revamp, Doc 3.13: casco vacío + carros + animales).
import type { AcuerdoTrueque, AnimalTipo, Caravana, CarroTipo, RecursoTipo } from '../../domain/types';
import {
  aceptarTrueque as aceptarTruequeEngine,
  crearCaravanaVacia as crearCaravanaVaciaEngine,
  agregarCarroACaravana as agregarCarroEngine,
  comprarAnimalParaCaravana as comprarAnimalEngine,
  proponerTrueque as proponerTruequeEngine,
  rechazarTrueque as rechazarTruequeEngine,
} from '../../engine/trade';
import { colocarOrdenMercado as colocarOrdenMercadoEngine, comerciarEnPlaza as comerciarEnPlazaEngine } from '../../engine/market';
import { capacidadCargaDe } from '../../engine/ejercitos';
import { asegurarCaminoComercial } from '../../engine/caminos';
import type { GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, conAsentamiento, exigirAcuerdo, exigirAsentamiento, exigirCaravana, exigirColumnaDe, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { evento, eventos as construirEventos, type EventoDeComando } from './eventos';

export interface PayloadTruequePropuesto {
  acuerdoId: string;
  asentamientoAId: string;
  asentamientoBId: string;
  recursoA: RecursoTipo;
  cantidadA: number;
  recursoB: RecursoTipo;
  cantidadB: number;
}
export interface PayloadCaminoComercial {
  asentamientoAId: string;
  asentamientoBId: string;
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

export interface ParamsProponerTrueque {
  asentamientoAId: string;
  recursoA: RecursoTipo;
  cantidadA: number;
  asentamientoBId: string;
  recursoB: RecursoTipo;
  cantidadB: number;
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
    params.recursoA,
    params.recursoB,
    params.cantidadA,
    params.cantidadB,
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
 * El lado receptor acepta (Doc 3.2). Es aqui, y no al proponer, donde el acuerdo empieza a obligar y donde
 * nace el Camino Comercial del par (Doc 1.6): la relacion comercial existe cuando los dos han dicho que si.
 */
export const aceptarTrueque = comando<ParamsResponderTrueque, { acuerdoId: string }>((estado, mapa, ctx, params) => {
  const acuerdo = aceptarTruequeEngine(exigirAcuerdo(estado, params.acuerdoId), ctx.instante);

  const narrados: EventoDeComando[] = [
    {
      codigo: 'comercio.trueque_aceptado',
      mensaje: `Trueque aceptado: ${acuerdo.id}.`,
      payload: { acuerdoId: acuerdo.id } satisfies PayloadRespuestaTrueque,
    },
  ];

  // Los asentamientos existen seguro llegados aqui (el acuerdo los referencia y `proponerTrueque` los
  // resolvio al crearlo); estos `exigir` estan por no reintroducir un `find` sin guarda.
  const a = exigirAsentamiento(estado, acuerdo.asentamientoAId);
  const b = exigirAsentamiento(estado, acuerdo.asentamientoBId);
  const previos = estado.caminos.length;
  const caminos = asegurarCaminoComercial(estado.caminos, mapa, a, b);
  if (caminos.length > previos) {
    narrados.push({
      codigo: 'comercio.camino_creado',
      mensaje: `Nuevo camino comercial entre ${a.id} y ${b.id}.`,
      payload: { asentamientoAId: a.id, asentamientoBId: b.id } satisfies PayloadCaminoComercial,
    });
  }

  const siguiente: GameSessionState = { ...estado, acuerdos: conAcuerdo(estado.acuerdos, acuerdo), caminos };
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
  jugadorId: string;
  asentamientoId: string;
  ordenId: string;
  /** Cuanto se quiere mover. Se sirve lo que se pueda: el tope real sale de la orden, del almacen de la plaza,
   * de su oro, del carro y de lo que se lleve encima, y ninguno lo ve el cliente entero. */
  cantidad: number;
}

export interface PayloadComercioEnPlaza {
  jugadorId: string;
  asentamientoId: string;
  ordenId: string;
  recurso: RecursoTipo;
  cantidad: number;
  valor: number;
  comision: number;
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
    const columna = exigirColumnaDe(estado, params.jugadorId);
    const plaza = exigirAsentamiento(estado, params.asentamientoId);
    const orden = estado.ordenes.find((o) => o.id === params.ordenId);
    if (!orden) rechazar(CODIGOS_ERROR.ordenNoExiste);

    const resultado = comerciarEnPlazaEngine(
      columna,
      params.jugadorId,
      plaza,
      orden,
      params.cantidad,
      capacidadCargaDe(columna, estado.caravanas),
      ctx.instante
    );

    const siguiente: GameSessionState = {
      ...conAsentamiento(estado, resultado.plaza),
      ejercitos: estado.ejercitos.map((e) => (e.id === resultado.ejercito.id ? resultado.ejercito : e)),
      ordenes: estado.ordenes.map((o) => (o.id === resultado.orden.id ? resultado.orden : o)),
    };

    const sentido = orden.tipo === 'venta' ? 'compra' : 'vende';
    return exito(
      siguiente,
      [
        evento(ctx, {
          codigo: 'mercado.comercio_en_plaza',
          mensaje: `Un jugador ${sentido} ${resultado.cantidad.toFixed(1)} ${orden.recurso} en el mercado de ${plaza.id} por ${resultado.valor.toFixed(1)} oro (comisión ${resultado.comision.toFixed(1)}).`,
          payload: {
            jugadorId: params.jugadorId,
            asentamientoId: plaza.id,
            ordenId: orden.id,
            recurso: orden.recurso as RecursoTipo,
            cantidad: resultado.cantidad,
            valor: resultado.valor,
            comision: resultado.comision,
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
