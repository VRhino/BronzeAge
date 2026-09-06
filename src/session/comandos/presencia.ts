// Presencia del jugador en el mundo (Doc 1.10): salir, entrar y volver a salir.
//
// Estos tres comandos son los únicos que MUEVEN a un jugador entre los tres sitios donde puede estar —dentro
// de una plaza, dentro de una columna, o fuera del mundo—. Todo lo demás lee la ubicación; solo esto la
// escribe (con la excepción de fundar, que te deja dentro de lo que acabas de fundar).
//
// La distinción que los ordena no es "salir/entrar" sino DE DÓNDE y A DÓNDE:
//
//   residencia  --salirAlMundo-->         columna   (con pantalla: eliges tropas y carga)
//   columna     --entrarEnAsentamiento--> plaza     (tu residencia la disuelve; otra la aparca)
//   plaza ajena --salirDeAsentamiento-->  columna   (sin pantalla: sales con lo que tenías)
//
// No hay un cuarto: salir de tu propia residencia SIEMPRE es `salirAlMundo`, porque ahí tienes tu roster
// entero delante y hay algo que elegir.
import { MOVIMIENTO } from '../../constants';
import {
  absorberColumna,
  enLaPuertaDe,
  marcharA as marcharAEngine,
  salirAlMundo as salirAlMundoEngine,
  MovilizacionInvalidaError,
  type ObjetivoEjercito,
} from '../../engine/ejercitos';
import { esResidente } from '../../engine/pertenencia';
import { situarJugadores } from '../../engine/ubicacion';
import { liderazgoComprometido } from '../../engine/liderazgo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, exigirAsentamiento, conAsentamiento } from './ayudas';
import { evento } from './eventos';

export interface ParamsSalirAlMundo {
  asentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
  /** Lo que se lleva del almacén, por recurso. Puede ir vacío: salir con el carro seco es legítimo. */
  carga: Record<string, number>;
}

export interface PayloadSalidaAlMundo {
  ejercitoId: string;
  asentamientoId: string;
  jugadorId: string;
  escuadronIds: string[];
  liderazgoUsado: number;
  /** Total cargado en el carro, sumando recursos. Es la cifra que decide cuánto aguanta fuera. */
  cargaTotal: number;
}

export interface ParamsEntrarEnAsentamiento {
  asentamientoId: string;
  jugadorId: string;
}

export interface PayloadPresencia {
  asentamientoId: string;
  jugadorId: string;
  ejercitoId: string;
}

export interface ParamsSalirDeAsentamiento {
  asentamientoId: string;
  jugadorId: string;
}

/** La columna en la que va este jugador, si va en alguna. Lee `participantes` y no los escuadrones: un
 * viajero sin tropas también va dentro de la suya (Doc 5.12.1). */
function columnaDe(estado: GameSessionState, jugadorId: string) {
  return estado.ejercitos.find((e) => e.participantes.some((p) => p.jugadorId === jugadorId));
}

/** Dónde está su cuerpo, que NO es lo mismo que a qué columna pertenece: quien entra en una plaza ajena
 * sigue siendo participante de la columna que dejó aparcada a la puerta. */
function ubicacionDe(estado: GameSessionState, jugadorId: string) {
  return estado.jugadores.find((j) => j.id === jugadorId)?.ubicacion;
}

/**
 * Salir al mundo desde la residencia (Doc 1.10.2): eliges tropas y carga, y apareces junto a la plaza.
 *
 * Es la única salida con pantalla de equipamiento, y por una razón concreta: es el único sitio donde tienes
 * delante tu roster entero y el almacén. Salir de una plaza ajena es `salirDeAsentamiento` y no elige nada.
 */
export const salirAlMundo = comando<ParamsSalirAlMundo, { ejercitoId: string }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  if (columnaDe(estado, params.jugadorId)) {
    throw new MovilizacionInvalidaError('Ya estás fuera: no se puede salir dos veces.');
  }

  const ejercitoId = `ejercito-${ctx.ids.siguiente()}`;
  const { asentamiento: origen, ejercito } = salirAlMundoEngine(
    asentamiento,
    estado.jugadores.find((j) => j.id === params.jugadorId),
    params.jugadorId,
    params.escuadronIds,
    params.carga,
    ejercitoId,
    ctx.instante
  );

  const cargaTotal = Object.values(ejercito.suministro).reduce((suma, cantidad) => suma + cantidad, 0);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, origen),
    ejercitos: [...estado.ejercitos, ejercito],
    jugadores: situarJugadores(estado.jugadores, [params.jugadorId], { tipo: 'columna', ejercitoId: ejercito.id }),
  };

  const conQue = ejercito.escuadrones.length === 0 ? 'sin tropas' : `con ${ejercito.escuadrones.length} escuadrón(es)`;
  return exito(
    conHistorialDeJugador(siguiente, params.jugadorId, `Sale al mundo desde ${asentamiento.id} ${conQue}.`),
    [
      evento(ctx, {
        codigo: 'jugador.sale_al_mundo',
        mensaje: `Un jugador sale de ${asentamiento.id} ${conQue} y ${cargaTotal} de carga.`,
        payload: {
          ejercitoId: ejercito.id,
          asentamientoId: asentamiento.id,
          jugadorId: params.jugadorId,
          escuadronIds: ejercito.escuadrones.map((e) => e.id),
          liderazgoUsado: liderazgoComprometido(ejercito.escuadrones),
          cargaTotal,
        } satisfies PayloadSalidaAlMundo,
        asentamientoId: asentamiento.id,
      }),
    ],
    { ejercitoId: ejercito.id }
  );
});

/**
 * Entrar en un asentamiento estando en su puerta (Doc 1.10.3). Lo que pasa con la columna depende de dónde
 * entres, y las dos ramas son distintas de verdad:
 *
 * - **Tu residencia**: la columna se DISUELVE. Escuadrones a la guarnición, carro al almacén. Volver a salir
 *   vuelve a pasar por la pantalla de equipamiento, que es lo correcto: en tu casa tienes todo delante.
 * - **Cualquier otra**: se queda APARCADA a la puerta, intacta. Sales con lo que llevabas.
 *
 * De la segunda salen gratis dos casos que si no habría que escribir aparte: si conquistan la plaza mientras
 * estás dentro, tu columna está fuera y la retomas; y si te la destruyen aparcada, sales a pie.
 */
export const entrarEnAsentamiento = comando<ParamsEntrarEnAsentamiento, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const columna = columnaDe(estado, params.jugadorId);
  if (!columna) throw new MovilizacionInvalidaError('No estás en el mundo: no hay puerta que cruzar.');
  // Entrar es un acto INDIVIDUAL y una columna personal es de uno solo, así que disolverla o aparcarla no le
  // hace nada a nadie más. Un Ejército lleva a varios: dejarle cruzar la puerta desde dentro lo disolvería
  // con su gente dentro, y de paso sería una salida encubierta que se salta al Líder (Doc 5.14.2). Para
  // entrar hay que separarse antes, que es un comando con sus propias reglas.
  if (columna.tipo === 'ejercito') {
    throw new MovilizacionInvalidaError('Vas en un ejército: hay que separarse antes de entrar en una plaza.');
  }
  if (!enLaPuertaDe(columna, asentamiento)) {
    throw new MovilizacionInvalidaError(`Hay que estar a menos de ${MOVIMIENTO.radioPuerta} de la plaza para entrar.`);
  }
  // La política de acceso del Gobernador es el paso 5 (Doc 1.10.5). Hasta entonces se entra solo en lo
  // propio, que es MÁS restrictivo que la regla final y no al revés: ninguna partida se acostumbra a algo
  // que luego se prohíba.
  if (asentamiento.faccionId !== columna.faccionId) {
    throw new MovilizacionInvalidaError('Todavía no se puede entrar en una plaza de otra Facción.');
  }

  const esSuResidencia = esResidente(asentamiento, params.jugadorId);
  const siguiente: GameSessionState = esSuResidencia
    ? {
        ...conAsentamiento(estado, absorberColumna(asentamiento, columna, true)),
        ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id),
      }
    : estado;

  return exito(
    conHistorialDeJugador(
      {
        ...siguiente,
        jugadores: situarJugadores(siguiente.jugadores, [params.jugadorId], { tipo: 'asentamiento', asentamientoId: asentamiento.id }),
      },
      params.jugadorId,
      esSuResidencia
        ? `Vuelve a casa en ${asentamiento.id} y su columna se deshace.`
        : `Entra en ${asentamiento.id}; su columna queda a la puerta.`
    ),
    [
      evento(ctx, {
        codigo: 'jugador.entra_en_asentamiento',
        mensaje: esSuResidencia
          ? `Un jugador vuelve a ${asentamiento.id} y su columna se deshace.`
          : `Un jugador entra en ${asentamiento.id} dejando su columna a la puerta.`,
        payload: {
          asentamientoId: asentamiento.id,
          jugadorId: params.jugadorId,
          ejercitoId: columna.id,
        } satisfies PayloadPresencia,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});

/**
 * Salir de una plaza AJENA retomando la columna aparcada (Doc 1.10.3), sin pantalla de equipamiento: sales
 * con lo que llevabas encima.
 *
 * Desde tu residencia esto no aplica y se rechaza a propósito, en vez de hacer lo mismo en silencio: allí hay
 * un roster entero y un almacén que elegir, y ese es `salirAlMundo`.
 */
export const salirDeAsentamiento = comando<ParamsSalirDeAsentamiento, { ejercitoId: string }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  if (esResidente(asentamiento, params.jugadorId)) {
    throw new MovilizacionInvalidaError('De tu propia residencia se sale eligiendo tropas y carga, con `salirAlMundo`.');
  }
  const columna = columnaDe(estado, params.jugadorId);
  if (!columna) throw new MovilizacionInvalidaError('No tienes ninguna columna a la puerta que retomar.');

  return exito(
    conHistorialDeJugador(
      { ...estado, jugadores: situarJugadores(estado.jugadores, [params.jugadorId], { tipo: 'columna', ejercitoId: columna.id }) },
      params.jugadorId,
      `Sale de ${asentamiento.id} y retoma su columna.`
    ),
    [
      evento(ctx, {
        codigo: 'jugador.sale_de_asentamiento',
        mensaje: `Un jugador sale de ${asentamiento.id} y retoma su columna.`,
        payload: {
          asentamientoId: asentamiento.id,
          jugadorId: params.jugadorId,
          ejercitoId: columna.id,
        } satisfies PayloadPresencia,
        asentamientoId: asentamiento.id,
      }),
    ],
    { ejercitoId: columna.id }
  );
});

export interface ParamsMarcharA {
  jugadorId: string;
  objetivo: ObjetivoEjercito;
}

export interface PayloadMarchaFijada {
  ejercitoId: string;
  jugadorId: string;
  objetivo: ObjetivoEjercito;
  /** `true` si la columna ya iba a algún sitio: es un cambio de rumbo, no una salida. Lo distingue el
   * cliente para narrarlo, y el log para que "clic, clic, clic" no parezca tres campañas. */
  rectifica: boolean;
}

/**
 * Fijar o rectificar el destino de tu columna (Doc 5.12.1): clic en un punto y la miniatura se pone en
 * camino, tantas veces como quieras.
 *
 * Es la mitad que le faltaba a `salirAlMundo`: al salir apareces junto a la plaza SIN destino, y esto es lo
 * que te pone en marcha. Solo funciona yendo en columna personal — si vas en un ejército el rumbo se acordó
 * al salir y su única salida es cancelar (Doc 5.12.6).
 */
export const marcharA = comando<ParamsMarcharA, { ejercitoId: string }>((estado, mapa, ctx, params) => {
  const columna = columnaDe(estado, params.jugadorId);
  if (!columna) throw new MovilizacionInvalidaError('No estás en el mundo: no hay a dónde marchar.');
  if (ubicacionDe(estado, params.jugadorId)?.tipo !== 'columna') {
    throw new MovilizacionInvalidaError('Estás dentro de una plaza: hay que salir antes de ponerse en marcha.');
  }

  const rectifica = columna.estado === 'marchando';
  const enMarcha = marcharAEngine(columna, params.objetivo, estado.asentamientos, mapa);
  const aDonde = params.objetivo.tipo === 'asentamiento' ? params.objetivo.id : 'un punto del mapa';

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === enMarcha.id ? enMarcha : e)) },
      params.jugadorId,
      rectifica ? `Cambia de rumbo hacia ${aDonde}.` : `Se pone en marcha hacia ${aDonde}.`
    ),
    [
      evento(ctx, {
        codigo: 'jugador.marcha_fijada',
        mensaje: rectifica ? `Una columna cambia de rumbo hacia ${aDonde}.` : `Una columna se pone en marcha hacia ${aDonde}.`,
        payload: { ejercitoId: enMarcha.id, jugadorId: params.jugadorId, objetivo: params.objetivo, rectifica } satisfies PayloadMarchaFijada,
        asentamientoId: enMarcha.origenAsentamientoId,
      }),
    ],
    { ejercitoId: enMarcha.id }
  );
});
