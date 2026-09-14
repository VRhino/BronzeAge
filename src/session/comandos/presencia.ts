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
import type { Asentamiento } from '../../domain/types';
import { guarnecer as guarnecerEngine, marcharA as marcharAEngine, salirAlMundo as salirAlMundoEngine, type ObjetivoEjercito } from '../../engine/ejercitos';
import { conVeto } from '../../engine/pertenencia';
import { conFotoTomadaPor, cruzarLaPuerta, retomarColumna, situarHeroes } from '../../engine/ubicacion';
import { liderazgoComprometido } from '../../engine/liderazgo';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito } from './tipos';
import { comando, exigirAsentamiento, exigirColumnaDe, exigirJugador, conAsentamiento } from './ayudas';
import { evento } from './eventos';

export interface ParamsSalirAlMundo {
  asentamientoId: string;
  heroeId: string;
  escuadronIds: string[];
  /** Lo que se lleva del almacén, por recurso. Puede ir vacío: salir con el carro seco es legítimo. */
  carga: Record<string, number>;
}

export interface PayloadSalidaAlMundo {
  ejercitoId: string;
  asentamientoId: string;
  heroeId: string;
  escuadronIds: string[];
  liderazgoUsado: number;
  /** Total cargado en el carro, sumando recursos. Es la cifra que decide cuánto aguanta fuera. */
  cargaTotal: number;
}

export interface ParamsEntrarEnAsentamiento {
  asentamientoId: string;
  heroeId: string;
}

export interface PayloadPresencia {
  asentamientoId: string;
  heroeId: string;
  ejercitoId: string;
}

export interface ParamsSalirDeAsentamiento {
  asentamientoId: string;
  heroeId: string;
}

export interface ParamsGuarnecer {
  asentamientoId: string;
  heroeId: string;
}

export interface PayloadGuarnecer {
  asentamientoId: string;
  ejercitoId: string;
  escuadrones: number;
  caravanasAparcadas: string[];
}

/**
 * Salir al mundo desde la residencia (Doc 1.10.2): eliges tropas y carga, y apareces junto a la plaza.
 *
 * Es la única salida con pantalla de equipamiento, y por una razón concreta: es el único sitio donde tienes
 * delante tu roster entero y el almacén. Salir de una plaza ajena es `salirDeAsentamiento` y no elige nada.
 */
export const salirAlMundo = comando<ParamsSalirAlMundo, { ejercitoId: string }>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const { asentamiento: origen, ejercito } = salirAlMundoEngine(
    asentamiento,
    estado.heroes.find((j) => j.id === params.heroeId),
    params.heroeId,
    params.escuadronIds,
    params.carga,
    estado.ejercitos,
    `ejercito-${ctx.ids.siguiente()}`,
    ctx.instante
  );

  const cargaTotal = Object.values(ejercito.suministro).reduce((suma, cantidad) => suma + cantidad, 0);
  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, origen),
    ejercitos: [...estado.ejercitos, ejercito],
    // Al cruzar la puerta hacia fuera se congela lo que estaba viendo de dentro (Doc 1.10.1). La foto se toma
    // del asentamiento YA sin las tropas ni la carga que se lleva: es lo que deja atrás, no lo que había
    // antes de hacer la maleta.
    heroes: conFotoTomadaPor(
      situarHeroes(estado.heroes, [params.heroeId], { tipo: 'columna', ejercitoId: ejercito.id }),
      params.heroeId,
      origen,
      ctx.instante
    ),
  };

  const conQue = ejercito.escuadrones.length === 0 ? 'sin tropas' : `con ${ejercito.escuadrones.length} escuadrón(es)`;
  return exito(
    conHistorialDeJugador(siguiente, params.heroeId, `Sale al mundo desde ${asentamiento.id} ${conQue}.`),
    [
      evento(ctx, {
        codigo: 'jugador.sale_al_mundo',
        mensaje: `Un jugador sale de ${asentamiento.id} ${conQue} y ${cargaTotal} de carga.`,
        payload: {
          ejercitoId: ejercito.id,
          asentamientoId: asentamiento.id,
          heroeId: params.heroeId,
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
  const columna = exigirColumnaDe(estado, params.heroeId);
  const cruce = cruzarLaPuerta(columna, asentamiento, params.heroeId, estado.relaciones);

  const esSuResidencia = cruce.disuelveColumna;
  const siguiente: GameSessionState = esSuResidencia
    ? { ...conAsentamiento(estado, cruce.asentamiento), ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id) }
    : estado;

  return exito(
    conHistorialDeJugador(
      {
        ...siguiente,
        heroes: situarHeroes(siguiente.heroes, [params.heroeId], { tipo: 'asentamiento', asentamientoId: asentamiento.id }),
      },
      params.heroeId,
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
          heroeId: params.heroeId,
          ejercitoId: columna.id,
        } satisfies PayloadPresencia,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});

/**
 * `guarnecer` (Ocupacion §2.3): marchar un EJÉRCITO a una plaza de tu Facción y volcar la tropa en su
 * guarnición. El ejército se consume; los jugadores quedan DENTRO de la plaza que acaban de reforzar (17.1) —
 * salen luego con `movilizarEjercito` (el gate ya lo permite: "escuadrones tuyos ya posados aquí").
 *
 * Las caravanas adjuntas no se pierden: pasan a `'aparcada'` en la plaza (§2.3d). No es `entrarEnAsentamiento`
 * —ese exige columna personal y aparca la columna intacta— sino la vía de un ejército para acabar en una
 * plaza propia sin conquistarla.
 */
export const guarnecer = comando<ParamsGuarnecer, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const columna = exigirColumnaDe(estado, params.heroeId);

  const r = guarnecerEngine(asentamiento, columna, estado.caravanas);
  const aparcadasPorId = new Map(r.caravanasAparcadas.map((c) => [c.id, c]));
  const heroesDeLaColumna = columna.participantes.map((p) => p.heroeId);

  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, r.asentamiento),
    ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id),
    caravanas: estado.caravanas.map((c) => aparcadasPorId.get(c.id) ?? c),
    heroes: situarHeroes(estado.heroes, heroesDeLaColumna, { tipo: 'asentamiento', asentamientoId: asentamiento.id }),
  };

  const conCaravanas = r.caravanasAparcadas.length > 0 ? ` y ${r.caravanasAparcadas.length} caravana(s) quedan aparcadas` : '';
  return exito(
    conHistorialDeJugador(
      siguiente,
      params.heroeId,
      `Guarnece ${asentamiento.id}: ${columna.escuadrones.length} escuadrón(es) a la guarnición${conCaravanas}.`
    ),
    [
      evento(ctx, {
        codigo: 'ejercito.guarnece',
        mensaje: `El ejército ${columna.id} guarnece ${asentamiento.id}: ${columna.escuadrones.length} escuadrón(es) se suman a su guarnición.`,
        payload: {
          asentamientoId: asentamiento.id,
          ejercitoId: columna.id,
          escuadrones: columna.escuadrones.length,
          caravanasAparcadas: r.caravanasAparcadas.map((c) => c.id),
        } satisfies PayloadGuarnecer,
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
  retomarColumna(asentamiento, params.heroeId);
  const columna = exigirColumnaDe(estado, params.heroeId);

  return exito(
    conHistorialDeJugador(
      { ...estado, heroes: situarHeroes(estado.heroes, [params.heroeId], { tipo: 'columna', ejercitoId: columna.id }) },
      params.heroeId,
      `Sale de ${asentamiento.id} y retoma su columna.`
    ),
    [
      evento(ctx, {
        codigo: 'jugador.sale_de_asentamiento',
        mensaje: `Un jugador sale de ${asentamiento.id} y retoma su columna.`,
        payload: {
          asentamientoId: asentamiento.id,
          heroeId: params.heroeId,
          ejercitoId: columna.id,
        } satisfies PayloadPresencia,
        asentamientoId: asentamiento.id,
      }),
    ],
    { ejercitoId: columna.id }
  );
});

export interface ParamsMarcharA {
  heroeId: string;
  objetivo: ObjetivoEjercito;
}

export interface PayloadMarchaFijada {
  ejercitoId: string;
  heroeId: string;
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
  const columna = exigirColumnaDe(estado, params.heroeId);
  const jugador = exigirJugador(estado, params.heroeId);

  const rectifica = columna.estado === 'marchando';
  const enMarcha = marcharAEngine(columna, jugador, params.objetivo, estado.asentamientos, mapa);
  const aDonde = params.objetivo.tipo === 'asentamiento' ? params.objetivo.id : 'un punto del mapa';

  return exito(
    conHistorialDeJugador(
      { ...estado, ejercitos: estado.ejercitos.map((e) => (e.id === enMarcha.id ? enMarcha : e)) },
      params.heroeId,
      rectifica ? `Cambia de rumbo hacia ${aDonde}.` : `Se pone en marcha hacia ${aDonde}.`
    ),
    [
      evento(ctx, {
        codigo: 'jugador.marcha_fijada',
        mensaje: rectifica ? `Una columna cambia de rumbo hacia ${aDonde}.` : `Una columna se pone en marcha hacia ${aDonde}.`,
        payload: { ejercitoId: enMarcha.id, heroeId: params.heroeId, objetivo: params.objetivo, rectifica } satisfies PayloadMarchaFijada,
        asentamientoId: enMarcha.origenAsentamientoId,
      }),
    ],
    { ejercitoId: enMarcha.id }
  );
});

export interface ParamsFijarPoliticaDeAcceso {
  asentamientoId: string;
  heroeId: string;
  politica: NonNullable<Asentamiento['politicaDeAcceso']>;
}

export interface ParamsVetarJugador {
  asentamientoId: string;
  heroeId: string;
  vetadoId: string;
  /** `false` para levantar el veto. Un comando y no dos: vetar y perdonar son el mismo interruptor. */
  vetar: boolean;
}

export interface PayloadPuerta {
  asentamientoId: string;
  politica: NonNullable<Asentamiento['politicaDeAcceso']>;
}

export interface PayloadVeto {
  asentamientoId: string;
  vetadoId: string;
  vetado: boolean;
}

/**
 * El Gobernador decide quién cruza su puerta (Doc 1.10.5).
 *
 * No expira, a diferencia de las políticas de Doc 4.4: una puerta que se abre sola a las dos horas y media
 * no es una puerta. Por eso vive en el asentamiento y no en `politicasActivas`.
 */
export const fijarPoliticaDeAcceso = comando<ParamsFijarPoliticaDeAcceso, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  return exito(
    conHistorialDeJugador(
      conAsentamiento(estado, { ...asentamiento, politicaDeAcceso: params.politica }),
      params.heroeId,
      `Fija la puerta de ${asentamiento.id} en ${params.politica}.`
    ),
    [
      evento(ctx, {
        codigo: 'asentamiento.puerta_fijada',
        mensaje: `${asentamiento.id} pasa a estar ${params.politica === 'abierto' ? 'abierta a todos' : `en ${params.politica}`}.`,
        payload: { asentamientoId: asentamiento.id, politica: params.politica } satisfies PayloadPuerta,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});

/**
 * Veta (o perdona) a un jugador concreto por encima de la política (Doc 1.10.5).
 *
 * Es lo que hace útil tener la plaza abierta: se abre a todos MENOS a esos. **A un residente no se le veta**
 * — nadie se queda fuera de su propia casa, y echar a un vecino es el exilio (Doc 2.8), que es otra cosa y
 * pasa por otra puerta.
 */
export const vetarJugador = comando<ParamsVetarJugador, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);

  return exito(
    conHistorialDeJugador(
      conAsentamiento(estado, conVeto(asentamiento, params.vetadoId, params.vetar)),
      params.heroeId,
      params.vetar ? `Veta a un jugador en ${asentamiento.id}.` : `Levanta un veto en ${asentamiento.id}.`
    ),
    [
      evento(ctx, {
        codigo: 'asentamiento.veto',
        mensaje: params.vetar ? `${asentamiento.id} cierra su puerta a un jugador.` : `${asentamiento.id} levanta un veto.`,
        payload: { asentamientoId: asentamiento.id, vetadoId: params.vetadoId, vetado: params.vetar } satisfies PayloadVeto,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});
