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
import type { GrupoPuerta } from '../../domain/types';
import {
  entrarEnCampamento as entrarEnCampamentoEngine,
  guarnecer as guarnecerEngine,
  marcharA as marcharAEngine,
  salirAlMundo as salirAlMundoEngine,
  salirDelCampamento as salirDelCampamentoEngine,
  type ObjetivoEjercito,
} from '../../engine/ejercitos';
import { esCiudadano } from '../../engine/faccion';
import { conVeto } from '../../engine/pertenencia';
import { conFotoTomadaPor, cruzarLaPuerta, retomarColumna, situarHeroes } from '../../engine/ubicacion';
import { liderazgoComprometido } from '../../engine/liderazgo';
import { conEscuadrones } from '../../engine/tropa';
import { conHistorialDeJugador, type GameSessionState } from '../estado';
import { exito, sinCambios } from './tipos';
import { campamentoEn, comando, conColumnas, conTropaDe, exigirAsentamiento, exigirColumnaDe, exigirJugador, conAsentamiento, rechazar } from './ayudas';
import { CODIGOS_ERROR } from './codigosDeError';
import { columnaDe } from '../../engine/ejercitos';
import { conMomento, evento } from './eventos';
import { volverAlMundo } from '../../engine/presencia';
import { PRESENCIA } from '../../constants';
import { instante } from '../../domain/tiempo';

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
    campamentoEn(estado, asentamiento),
    estado.heroes.find((j) => j.id === params.heroeId),
    params.heroeId,
    params.escuadronIds,
    params.carga,
    estado.ejercitos,
    `ejercito-${ctx.ids.siguiente()}`,
    ctx.instante
  );

  const cargaTotal = Object.values(ejercito.suministro).reduce((suma, cantidad) => suma + cantidad, 0);
  const conColumna = conColumnas(conAsentamiento(estado, origen), [ejercito]);
  const siguiente: GameSessionState = {
    ...conColumna,
    // Al cruzar la puerta hacia fuera se congela lo que estaba viendo de dentro (Doc 1.10.1). La foto se toma
    // del asentamiento YA sin las tropas ni la carga que se lleva: es lo que deja atrás, no lo que había
    // antes de hacer la maleta.
    heroes: conFotoTomadaPor(
      situarHeroes(conColumna.heroes, [params.heroeId], { tipo: 'columna', ejercitoId: ejercito.id }),
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
  const cruce = cruzarLaPuerta(conTropaDe(estado, columna), asentamiento, params.heroeId, estado.relaciones);

  const esSuResidencia = cruce.disuelveColumna;
  const siguiente: GameSessionState = esSuResidencia
    ? {
        ...conAsentamiento(estado, cruce.asentamiento),
        ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id),
        heroes: conEscuadrones(estado.heroes, cruce.tropa),
      }
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
 * `guarnecer` (Ocupacion §2.3): un EJÉRCITO entra en la plaza de su Facción donde residen todos los que van en
 * él y se deshace: la tropa vuelve a sus campamentos y el carro al almacén (decisión del usuario 2026-09-14,
 * ver `guarnecer` en el motor). Los jugadores quedan DENTRO.
 *
 * Las caravanas adjuntas no se pierden: pasan a `'aparcada'` en la plaza (§2.3d). No es `entrarEnAsentamiento`
 * —ese exige columna personal— sino la vía de un ejército para volver a casa sin replegarse.
 */
export const guarnecer = comando<ParamsGuarnecer, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const columna = exigirColumnaDe(estado, params.heroeId);

  const r = guarnecerEngine(asentamiento, conTropaDe(estado, columna), estado.caravanas);
  const aparcadasPorId = new Map(r.caravanasAparcadas.map((c) => [c.id, c]));
  const heroesDeLaColumna = columna.participantes.map((p) => p.heroeId);

  const siguiente: GameSessionState = {
    ...conAsentamiento(estado, r.asentamiento),
    ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id),
    caravanas: estado.caravanas.map((c) => aparcadasPorId.get(c.id) ?? c),
    heroes: situarHeroes(conEscuadrones(estado.heroes, r.tropa), heroesDeLaColumna, { tipo: 'asentamiento', asentamientoId: asentamiento.id }),
  };

  const conCaravanas = r.caravanasAparcadas.length > 0 ? ` y ${r.caravanasAparcadas.length} caravana(s) quedan aparcadas` : '';
  return exito(
    conHistorialDeJugador(
      siguiente,
      params.heroeId,
      `Guarnece ${asentamiento.id}: ${columna.escuadronIds.length} escuadrón(es) vuelven a su campamento${conCaravanas}.`
    ),
    [
      evento(ctx, {
        codigo: 'ejercito.guarnece',
        mensaje: `El ejército ${columna.id} guarnece ${asentamiento.id}: ${columna.escuadronIds.length} escuadrón(es) vuelven a su campamento.`,
        payload: {
          asentamientoId: asentamiento.id,
          ejercitoId: columna.id,
          escuadrones: columna.escuadronIds.length,
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

export interface ParamsFijarPuerta {
  asentamientoId: string;
  /** Quien la fija: el Gobernador de la plaza o el Rey de su Facción. */
  heroeId: string;
  /** Grupos a los que se cierra la puerta; los que no figuran, entran. La propia Facción no es un grupo. */
  cerradaA: GrupoPuerta[];
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
  cerradaA: GrupoPuerta[];
}

export interface PayloadVeto {
  asentamientoId: string;
  vetadoId: string;
  vetado: boolean;
}

/**
 * Gobernador o Rey deciden a qué grupos se les cierra la puerta (el exilio, Doc 1.10.5 y 2.8).
 *
 * No expira, a diferencia de las políticas de Doc 4.4: una puerta que se abre sola a las dos horas y media
 * no es una puerta. Por eso vive en el asentamiento y no en `politicasActivas`. Quién puede es autorización
 * (`autorizacion.ts`); el comando solo fija el estado, que es único: lo que cambie uno lo ve el otro.
 */
export const fijarPuerta = comando<ParamsFijarPuerta, void>((estado, _mapa, ctx, params) => {
  const asentamiento = exigirAsentamiento(estado, params.asentamientoId);
  const cerradaA = [...new Set(params.cerradaA)];

  return exito(
    conHistorialDeJugador(
      conAsentamiento(estado, { ...asentamiento, puertaCerradaA: cerradaA }),
      params.heroeId,
      `Fija la puerta de ${asentamiento.id}: cerrada a ${cerradaA.join(', ') || 'nadie'}.`
    ),
    [
      evento(ctx, {
        codigo: 'asentamiento.puerta_fijada',
        mensaje: cerradaA.length ? `${asentamiento.id} cierra su puerta a ${cerradaA.join(', ')}.` : `${asentamiento.id} abre su puerta a todos.`,
        payload: { asentamientoId: asentamiento.id, cerradaA } satisfies PayloadPuerta,
        asentamientoId: asentamiento.id,
      }),
    ]
  );
});

/**
 * Veta (o perdona) a un jugador concreto por encima de los grupos (Doc 1.10.5).
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

export interface ParamsPresenciaEnElMundo {
  heroeId: string;
}

/**
 * Desconectarse (Doc 1.10.6): el héroe sigue en el mundo 2:30 —moviéndose como iba— y después sale con su tropa y su
 * carro (`salirDelMundo`, lo aplica el tick). Lo pide el cliente al cerrarse, o el runner de bots al acabar su sesión.
 */
export const desconectarse = comando<ParamsPresenciaEnElMundo, void>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, params.heroeId);
  if (heroe.fuera || heroe.desconectaEn !== undefined) return sinCambios(estado);
  const desconectaEn = instante(ctx.instante + PRESENCIA.retardoDesconexionMs);
  return exito({ ...estado, heroes: estado.heroes.map((h) => (h.id === heroe.id ? { ...h, desconectaEn } : h)) }, [
    evento(ctx, { codigo: 'jugador.se_desconecta', mensaje: `${heroe.displayName} se desconecta.`, payload: { heroeId: heroe.id, desconectaEn } }),
  ]);
});

/**
 * Conectarse (Doc 1.10.6): si aún no había salido, se queda; si estaba fuera, reaparece donde quedó —en su plaza, o en
 * su punto con su columna, sus escuadras y su carro (`volverAlMundo`)—.
 */
export const conectarse = comando<ParamsPresenciaEnElMundo, void>((estado, _mapa, ctx, params) => {
  const heroe = exigirJugador(estado, params.heroeId);
  if (!heroe.fuera) {
    if (heroe.desconectaEn === undefined) return sinCambios(estado);
    return exito({ ...estado, heroes: estado.heroes.map((h) => (h.id === heroe.id ? { ...h, desconectaEn: undefined } : h)) }, []);
  }
  const r = volverAlMundo(estado, heroe.id, `ejercito-${ctx.ids.siguiente()}`, ctx.instante);
  return exito({ ...estado, heroes: r.heroes, ejercitos: r.ejercitos }, conMomento(ctx, r.eventos));
});

export interface ParamsCampamentoMercenarios {
  campamentoId: string;
  heroeId: string;
}

export interface ParamsSalirDelCampamento extends ParamsCampamentoMercenarios {
  /** Solo para quien reside: la tropa de su campamento que se lleva y lo que carga desde su almacén personal (D76). */
  escuadronIds: string[];
  carga: Record<string, number>;
}

function exigirCampamentoMercenarios(estado: GameSessionState, campamentoId: string) {
  const campamento = estado.campamentosMercenarios.find((c) => c.id === campamentoId);
  if (!campamento) rechazar(CODIGOS_ERROR.campamentoDesconocido);
  return campamento;
}

/**
 * Entrar en un campamento de mercenarios con la columna en su puerta (D76, D77): en el propio, la columna se deshace —tropa
 * al campamento, carro al almacén personal, lo que no quepa sigue en el carro aparcado—; en otro, entra con su columna.
 */
export const entrarEnCampamento = comando<ParamsCampamentoMercenarios, void>((estado, _mapa, ctx, params) => {
  const campamento = exigirCampamentoMercenarios(estado, params.campamentoId);
  const heroe = exigirJugador(estado, params.heroeId);
  const columna = exigirColumnaDe(estado, params.heroeId);
  const r = entrarEnCampamentoEngine(campamento, heroe, conTropaDe(estado, columna));
  const sinColumna = { ...estado, ejercitos: estado.ejercitos.filter((e) => e.id !== columna.id) };
  const conTropa = r.columna ? conColumnas(estado, [r.columna], r.tropa) : { ...sinColumna, heroes: conEscuadrones(sinColumna.heroes, r.tropa) };
  const heroes = conTropa.heroes.map((h) => (h.id === heroe.id ? { ...r.heroe, escuadrones: h.escuadrones } : h));
  const campamentosMercenarios = conTropa.campamentosMercenarios.map((c) => (c.id === campamento.id ? r.campamento : c));
  return exito(conHistorialDeJugador({ ...conTropa, heroes, campamentosMercenarios }, heroe.id, `Entra en ${campamento.id}.`), [
    evento(ctx, { codigo: 'jugador.entra_en_campamento', mensaje: `${heroe.displayName} entra en ${campamento.id}.`, payload: { campamentoId: campamento.id, heroeId: heroe.id } }),
  ]);
});

/** Salir de un campamento de mercenarios (D76, D77): su columna aparece en la puerta. */
export const salirDelCampamento = comando<ParamsSalirDelCampamento, { ejercitoId: string }>((estado, _mapa, ctx, params) => {
  const campamento = exigirCampamentoMercenarios(estado, params.campamentoId);
  const heroe = exigirJugador(estado, params.heroeId);
  const aparcada = columnaDe(estado.ejercitos, heroe.id);
  const faccionId = estado.facciones.find((f) => esCiudadano(f, heroe.id))?.id ?? '';
  const r = salirDelCampamentoEngine(campamento, heroe, aparcada, params.escuadronIds, params.carga, faccionId, `ejercito-${ctx.ids.siguiente()}`, ctx.instante);
  const conColumna = conColumnas(estado, [r.columna]);
  const heroes = conColumna.heroes.map((h) => (h.id === heroe.id ? { ...h, ubicacion: r.heroe.ubicacion, almacenPersonal: r.heroe.almacenPersonal, racionEn: r.heroe.racionEn } : h));
  return exito(
    conHistorialDeJugador({ ...conColumna, heroes }, heroe.id, `Sale de ${campamento.id}.`),
    [evento(ctx, { codigo: 'jugador.sale_de_campamento', mensaje: `${heroe.displayName} sale de ${campamento.id}.`, payload: { campamentoId: campamento.id, heroeId: heroe.id, ejercitoId: r.columna.id } })],
    { ejercitoId: r.columna.id }
  );
});
