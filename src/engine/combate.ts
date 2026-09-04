import type { Asentamiento, CampamentoBandido, Caravana, Ejercito, Escuadron, Faccion, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { CAMPAMENTOS_BANDIDOS, MILITAR, NIVEL_FACCION, REPUTACION, TROPAS_RECLUTABLES } from '../constants';
import { agregarRecurso } from './almacen';
import { aplicarAjustesReputacion } from './reputacion';
import { aplicarAjustesExperiencia, type AjusteExperiencia } from './faccion';
import { multiplicadorDefensivoDeRecintos } from './muralla';
import { CAMPO_CARGO, estanAliadas } from './pertenencia';

export class CombateInvalidoError extends Error {}

/** Poder de combate (Doc 5.1: héroe-comandante liderando tropa; el resultado es CÁLCULO, no combate visual, Doc 5.10).
 * `poderBase` sale siempre del catálogo `TROPAS_RECLUTABLES` vía `tropaId` (Doc 5.7/5.8) — toda tropa lo tiene,
 * nunca cambia de identidad al ganar veteranía (Doc 5.8, a petición del usuario). */
export function poderEscuadron(e: Escuadron, instante: Instante): number {
  const poderBase = TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)!.poderBase;
  const base = poderBase * e.cantidad;
  const conVeterania = base * (1 + e.veterania * MILITAR.bonusVeteraniaPorPunto);
  const herido = e.heridoHasta !== undefined && instante < e.heridoHasta;
  return herido ? conVeterania * MILITAR.penalizacionHerido : conVeterania;
}

export function poderTotal(escuadrones: readonly Escuadron[], instante: Instante, bonusCohesion: boolean): number {
  const suma = escuadrones.reduce((acc, e) => acc + poderEscuadron(e, instante), 0);
  if (!bonusCohesion || escuadrones.length <= 1) return suma;
  // Cohesión entre escuadrones defendiendo juntos (Doc 5.3), abstraída sin formaciones renderizadas (Fase 0).
  return suma * (1 + MILITAR.bonusCohesionPorEscuadronExtra * (escuadrones.length - 1));
}

function aplicarBajas(escuadrones: Escuadron[], fraccionBajas: number, victoria: boolean, instante: Instante): Escuadron[] {
  return escuadrones.map((e) => {
    const bajas = Math.round(e.cantidad * fraccionBajas);
    const cantidad = Math.max(0, e.cantidad - bajas);
    const veterania = e.veterania + (victoria ? MILITAR.veteraniaGanadaPorVictoria : MILITAR.veteraniaGanadaPorDerrota);
    const heridoHasta = victoria ? e.heridoHasta : sumar(instante, minutos(MILITAR.duracionHeridoMinutos));
    return { ...e, cantidad, veterania, heridoHasta };
  });
}

// --- Eventos estructurados de combate ---
//
// Los comandos de combate son los ÚNICOS eventos de jugador con audiencia restringida de verdad: quién atacó
// a quién, con cuánto poder y con qué resultado no puede viajar indiscriminadamente a todos los clientes. Por
// eso el `payload` lleva los bandos como ids y no solo interpolados en `mensaje` — las proyecciones por
// audiencia de Fase C filtran sobre eso, no parseando castellano.

export interface PayloadCombateResuelto {
  ganador: 'atacante' | 'defensor';
  poderAtacante: number;
  poderDefensor: number;
}
export interface PayloadAsedio {
  atacanteId: string;
  defensorId: string;
  faccionAtacanteId: string;
  faccionDefensoraId: string;
}
export interface PayloadIntercepcion {
  atacanteId: string;
  caravanaId: string;
  /** Fracción del contenido capturada. Ausente si la intercepción falló. */
  fraccionCapturada?: number;
}
export interface PayloadAtaqueCampamento {
  atacanteId: string;
  campamentoId: string;
}

export interface ResultadoCombate {
  ganador: 'atacante' | 'defensor';
  atacantes: Escuadron[];
  defensores: Escuadron[];
  eventos: EventoCrudo[];
}

/**
 * Resolución numérica de combate (Doc 5.2/5.10): "mismo motor" para asedio/mundo abierto/caravanas.
 * PERMADEATH real (Doc 5.4): las bajas son permanentes; sin empates (jitter aleatorio rompe la igualdad).
 */
export function resolverCombate(
  atacantes: Escuadron[],
  defensores: Escuadron[],
  instante: Instante,
  rng: RandomFn,
  multiplicadorDefensor = 1
): ResultadoCombate {
  if (atacantes.length === 0) throw new CombateInvalidoError('El atacante no tiene escuadrones con los que combatir.');
  if (defensores.length === 0) throw new CombateInvalidoError('El defensor no tiene escuadrones con los que combatir.');

  const jitterA = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const jitterD = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const poderA = poderTotal(atacantes, instante, false) * jitterA;
  // El multiplicador de muralla (Paso 3b, `multiplicadorDefensivoDeRecintos`) SOLO llega aquí desde
  // `iniciarAsedio` — `combateCampoAbierto` no lo pasa nunca (1 por defecto): en mundo abierto no hay ningún
  // recinto que atravesar, así que aplicarlo ahí sería un bono de la nada.
  const poderD = poderTotal(defensores, instante, true) * jitterD * multiplicadorDefensor;

  const ganador: 'atacante' | 'defensor' = poderA > poderD ? 'atacante' : 'defensor';
  const ratio = Math.min(poderA, poderD) / Math.max(poderA, poderD, 1);
  // Cuanto más ajustado el combate, más bajas sufre el bando ganador; el perdedor siempre pierde más.
  const bajasGanador = 0.05 + 0.15 * ratio;
  const bajasPerdedor = 0.3 + 0.4 * (1 - ratio);

  const atacantesResultado = aplicarBajas(atacantes, ganador === 'atacante' ? bajasGanador : bajasPerdedor, ganador === 'atacante', instante);
  const defensoresResultado = aplicarBajas(defensores, ganador === 'defensor' ? bajasGanador : bajasPerdedor, ganador === 'defensor', instante);

  return {
    ganador,
    atacantes: atacantesResultado,
    defensores: defensoresResultado,
    eventos: [
      {
        codigo: 'combate.resuelto',
        mensaje: `Combate resuelto: gana el ${ganador} (poder ${poderA.toFixed(0)} vs ${poderD.toFixed(0)}).`,
        payload: { ganador, poderAtacante: poderA, poderDefensor: poderD } satisfies PayloadCombateResuelto,
      },
    ],
  };
}

function seleccionarEscuadrones(asentamiento: Asentamiento, ids: string[]): Escuadron[] {
  const seleccionados = asentamiento.escuadrones.filter((e) => ids.includes(e.id) && e.cantidad > 0);
  if (seleccionados.length === 0) throw new CombateInvalidoError('No hay escuadrones válidos seleccionados.');
  return seleccionados;
}

function reemplazarEscuadrones(asentamiento: Asentamiento, actualizados: Escuadron[]): Escuadron[] {
  const porId = new Map(actualizados.map((e) => [e.id, e]));
  return asentamiento.escuadrones.map((e) => porId.get(e.id) ?? e);
}

/**
 * XP de Facción por JUGADOR, versión mínima (Doc Fase_0_5 §8, a petición del usuario): si 3 jugadores atacan
 * juntos, la Facción recibe 3× la XP de ese evento, no un monto plano por combate. Se calcula al vuelo
 * contando `jugadorId` DISTINTOS entre los escuadrones que participaron en un bando — NO se guarda XP de
 * jugador por separado (eso exigiría una entidad `Jugador` que hoy no existe en el motor; queda pendiente si
 * llega a necesitar un propósito propio más allá de alimentar la XP de Facción).
 *
 * Aplica en los bandos con escuadrones REALES (asedio y campo abierto en ambos lados; campamento de bandidos
 * e interceptar caravana solo en el atacante). La defensa de una caravana NO se multiplica — Doc 3.10: la
 * escolta no está modelada con escuadrones/jugadores reales todavía, es una defensa fija placeholder.
 */
function jugadoresParticipantes(escuadrones: Escuadron[]): number {
  return new Set(escuadrones.map((e) => e.jugadorId)).size;
}

/**
 * Lo que le pasa a un asentamiento AL SER CONQUISTADO (Doc 5.4). La ciudad cambia de dueño entera:
 *
 * - **La guarnición cae a CERO unidades, pero los escuadrones NO desaparecen** (decisión del usuario,
 *   2026-09-04): conservan nombre, tropa, dueño y **veteranía**. Es la misma regla que el resto del juego ya
 *   aplica al aniquilar un regimiento (Doc 5.4: "el SQUAD persiste aunque el regimiento sea aniquilado — se
 *   puede rellenar con nuevos reclutas conservando el progreso") y la misma que deja volver a casa a los
 *   estandartes de un ejército deshecho por hambre. Lo que se pierde son los hombres, no la unidad.
 *
 *   Y **nada de eso pasa al conquistador**: los escuadrones son del Jugador, no del asentamiento — tropas
 *   personales de otro, no botín transferible. Antes se heredaban con sus unidades intactas, que es
 *   exactamente lo que el Doc 5.4 prohíbe.
 * - **Los antiguos residentes dejan de serlo**, y con ellos caen los cargos locales. De ahí sale el estado
 *   HUÉRFANO (Doc 5.4): quien estuviera de campaña conserva los escuadrones que lleva encima pero se queda
 *   sin sitio donde volver, reabastecer ni reclutar. No hace falta guardar ese estado en ninguna parte —
 *   "huérfano" es no residir en ningún asentamiento, y eso ya se deriva de `esResidente`.
 *
 *   Se les retira también a los que estaban EN CASA (decisión del usuario, 2026-09-04): la alternativa era
 *   dejarlos como residentes de una ciudad de la Facción enemiga, con todo lo que la residencia habilita.
 *
 * Los cascarones a cero se quedan EN el asentamiento conquistado, que es el único sitio donde el modelo sabe
 * guardar un escuadrón que no marcha. Ni su dueño puede rellenarlos (ya no reside ahí) ni el conquistador
 * puede usarlos (`movilizarEjercito` rechaza los de otro jugador), así que quedan congelados con su
 * veteranía a la espera de que la mecánica del huérfano decida cómo se recuperan.
 *
 * Lo que NO toca: población, edificios, almacén ni murallas. Conquistar entrega "un asentamiento completo y
 * en funcionamiento" (Doc 5.12.4) — ese es el premio que hace que atacar compense.
 */
export function aplicarConquista(defensor: Asentamiento, faccionConquistadoraId: string): Asentamiento {
  const cargos = { ...defensor.cargos };
  for (const campo of Object.values(CAMPO_CARGO)) cargos[campo] = null;
  return {
    ...defensor,
    faccionId: faccionConquistadoraId,
    escuadrones: defensor.escuadrones.map((e) => ({ ...e, cantidad: 0 })),
    jugadoresFundadoresIds: [],
    casasCompradas: [],
    cargos,
  };
}

/**
 * Asedio de asentamientos (Doc 5.2.1): mortalidad severa, sin instancia visual (Fase 0 = cálculo). La conquista
 * exacta tras ganar el asedio queda PENDIENTE en el diseño (Preguntas_Abiertas) — Fase 0 asume CAPTURA directa
 * (reasignación de Facción), la opción más simple de las citadas ahí (captura/destrucción/vasallaje automático).
 */
export function iniciarAsedio(
  atacante: Asentamiento,
  defensor: Asentamiento,
  escuadronIdsAtacantes: string[],
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): { atacante: Asentamiento; defensor: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; conquistado: boolean } {
  if (atacante.faccionId === defensor.faccionId) {
    throw new CombateInvalidoError('No se puede asediar un asentamiento de la propia Facción.');
  }
  if (!atacante.cargos.generalId) throw new CombateInvalidoError('El atacante necesita un General para asediar.');

  const escuadronesAtacantes = seleccionarEscuadrones(atacante, escuadronIdsAtacantes);
  const escuadronesDefensores = seleccionarEscuadrones(defensor, defensor.escuadrones.map((e) => e.id));

  // Paso 3b (§16 del doc de murallas): la razón de ser de toda la mecánica — un asedio contra un recinto
  // cerrado es mucho más caro para el atacante, y tanto más cuantas menos puertas tenga el defensor.
  const multiplicadorMuralla = multiplicadorDefensivoDeRecintos(defensor.recintos ?? []);
  const resultado = resolverCombate(escuadronesAtacantes, escuadronesDefensores, instante, rng, multiplicadorMuralla);

  const conquistado = resultado.ganador === 'atacante';
  const payloadAsedio: PayloadAsedio = {
    atacanteId: atacante.id,
    defensorId: defensor.id,
    faccionAtacanteId: atacante.faccionId,
    faccionDefensoraId: defensor.faccionId,
  };
  const eventos: EventoCrudo[] = [
    ...resultado.eventos,
    conquistado
      ? { codigo: 'combate.asedio_conquista', mensaje: `${atacante.id} conquista ${defensor.id}.`, payload: payloadAsedio }
      : { codigo: 'combate.asedio_resistido', mensaje: `${defensor.id} resiste el asedio de ${atacante.id}.`, payload: payloadAsedio },
  ];

  // Atacar a un Aliado sin romper la relación antes es la penalización MÁS SEVERA de reputación (Doc 2.7).
  const faccionesConReputacion = estanAliadas(relaciones, atacante.faccionId, defensor.faccionId)
    ? aplicarAjustesReputacion(facciones, [
        { faccionId: atacante.faccionId, delta: REPUTACION.penalizacionAtacarAliado, razon: 'atacar a un Aliado' },
      ])
    : facciones;

  // Doc Fase_0_5 §8: participación en combate otorga XP de Facción a ambos bandos; conquistar suma además el
  // bonus de conquista al atacante. Nota: el cupo de asentamientos por nivel (§5) NO se re-evalúa aquí — un
  // asentamiento conquistado conserva su `nivel` sin verificar cupo del conquistador, mismo criterio que
  // `CAP_FUNDACION_POR_NIVEL` (Doc 1.7, no aplica a conquista) — punto abierto #2 del documento de diseño.
  const ajustesXp: AjusteExperiencia[] = [
    {
      faccionId: atacante.faccionId,
      delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(escuadronesAtacantes),
      razon: 'combate (asedio)',
    },
    {
      faccionId: defensor.faccionId,
      delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(escuadronesDefensores),
      razon: 'combate (asedio)',
    },
  ];
  if (conquistado) ajustesXp.push({ faccionId: atacante.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' });
  const faccionesFinal = aplicarAjustesExperiencia(faccionesConReputacion, ajustesXp);

  return {
    atacante: { ...atacante, escuadrones: reemplazarEscuadrones(atacante, resultado.atacantes) },
    defensor: conquistado
      ? aplicarConquista(defensor, atacante.faccionId)
      : { ...defensor, escuadrones: reemplazarEscuadrones(defensor, resultado.defensores) },
    facciones: faccionesFinal,
    eventos,
    conquistado,
  };
}

/** Mundo abierto (Doc 5.2.2): choque de patrullas/ejércitos sin cambio de territorio; el perdedor queda "Herido". */
export function combateCampoAbierto(
  asentamientoA: Asentamiento,
  escuadronIdsA: string[],
  asentamientoB: Asentamiento,
  escuadronIdsB: string[],
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): { asentamientoA: Asentamiento; asentamientoB: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[] } {
  const escuadronesA = seleccionarEscuadrones(asentamientoA, escuadronIdsA);
  const escuadronesB = seleccionarEscuadrones(asentamientoB, escuadronIdsB);
  const resultado = resolverCombate(escuadronesA, escuadronesB, instante, rng);

  const faccionesConReputacion = estanAliadas(relaciones, asentamientoA.faccionId, asentamientoB.faccionId)
    ? aplicarAjustesReputacion(facciones, [
        { faccionId: asentamientoA.faccionId, delta: REPUTACION.penalizacionAtacarAliado, razon: 'atacar a un Aliado' },
      ])
    : facciones;

  const faccionesFinal = aplicarAjustesExperiencia(faccionesConReputacion, [
    {
      faccionId: asentamientoA.faccionId,
      delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(escuadronesA),
      razon: 'combate (campo abierto)',
    },
    {
      faccionId: asentamientoB.faccionId,
      delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(escuadronesB),
      razon: 'combate (campo abierto)',
    },
  ]);

  return {
    asentamientoA: { ...asentamientoA, escuadrones: reemplazarEscuadrones(asentamientoA, resultado.atacantes) },
    asentamientoB: { ...asentamientoB, escuadrones: reemplazarEscuadrones(asentamientoB, resultado.defensores) },
    facciones: faccionesFinal,
    eventos: resultado.eventos,
  };
}

/**
 * Defensa/intercepción de caravanas (Doc 3.10): escolta no modelada individualmente en Fase 0 (sin jugadores
 * reales escoltando) — se usa una defensa base fija como placeholder. Captura al 50% del contenido si gana.
 */
export function interceptarCaravana(
  atacante: Asentamiento,
  escuadronIdsAtacantes: string[],
  caravana: Caravana,
  instante: Instante,
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  rng: RandomFn
): { atacante: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; caravanaCapturada: boolean } {
  if (!atacante.cargos.generalId) throw new CombateInvalidoError('El atacante necesita un General para interceptar.');
  const escuadrones = seleccionarEscuadrones(atacante, escuadronIdsAtacantes);
  const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const poderAtacante = escuadrones.reduce((acc, e) => acc + poderEscuadron(e, instante), 0) * jitter;
  const gana = poderAtacante > MILITAR.defensaBaseCaravana;

  const fraccionBajas = gana ? 0.05 : 0.25;
  const escuadronesActualizados = aplicarBajas(escuadrones, fraccionBajas, gana, instante);

  let almacen = atacante.almacen;
  const eventos: EventoCrudo[] = [];
  if (gana) {
    for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
      almacen = agregarRecurso(almacen, recurso, cantidad * MILITAR.umbralCapturaCaravana);
    }
    eventos.push({
      codigo: 'combate.caravana_interceptada',
      mensaje: `${atacante.id} intercepta la caravana ${caravana.id} y captura ${MILITAR.umbralCapturaCaravana * 100}% de su carga.`,
      payload: {
        atacanteId: atacante.id,
        caravanaId: caravana.id,
        fraccionCapturada: MILITAR.umbralCapturaCaravana,
      } satisfies PayloadIntercepcion,
    });
  } else {
    eventos.push({
      codigo: 'combate.intercepcion_fallida',
      mensaje: `${atacante.id} falla la intercepción de la caravana ${caravana.id} y sufre bajas.`,
      payload: { atacanteId: atacante.id, caravanaId: caravana.id } satisfies PayloadIntercepcion,
    });
  }

  // Doc Fase_0_5 §8: ataque de caravana otorga XP al atacante; defensa de caravana otorga XP a la Facción
  // dueña de la caravana (resuelta vía `origenAsentamientoId`, ausente solo en partidas antiguas sin dueño).
  const duenoId = asentamientos.find((a) => a.id === caravana.origenAsentamientoId)?.faccionId;
  const ajustesXp: AjusteExperiencia[] = [
    {
      faccionId: atacante.faccionId,
      delta: NIVEL_FACCION.xp.ataqueCaravana * jugadoresParticipantes(escuadrones),
      razon: 'ataque a caravana',
    },
  ];
  if (duenoId && duenoId !== atacante.faccionId) {
    // Sin multiplicar (a diferencia del atacante): la escolta no tiene escuadrones/jugadores reales todavía.
    ajustesXp.push({ faccionId: duenoId, delta: NIVEL_FACCION.xp.defensaCaravana, razon: 'defensa de caravana' });
  }

  return {
    atacante: { ...atacante, almacen, escuadrones: reemplazarEscuadrones(atacante, escuadronesActualizados) },
    facciones: aplicarAjustesExperiencia(facciones, ajustesXp),
    eventos,
    caravanaCapturada: gana,
  };
}

/**
 * Ataque de un jugador a un campamento de bandidos (Doc 1.9, a petición del usuario) — mismo patrón que el
 * resto del combate (Doc 5.2/5.10): poder de los escuadrones elegidos, con jitter, contra el `poder` fijo
 * del campamento (placeholder, ver `CAMPAMENTOS_BANDIDOS`, constants.ts). Si gana, el campamento se destruye
 * (el llamador debe quitarlo del estado, ver `campamentoDestruido`) y entrega una recompensa fija en
 * recursos; si pierde, los escuadrones sufren bajas (mismo `aplicarBajas` que el resto del combate) y el
 * campamento sigue en pie.
 *
 * SIN gate de General (corrección — a diferencia de asediar/interceptar, ver `iniciarAsedio`/
 * `interceptarCaravana`): un campamento bandido es una amenaza NPC de mundo abierto, no una acción de
 * guerra entre Facciones que necesite coordinación de mando. Cualquier escuadrón propio elegido (de
 * cualquier jugador residente, Doc 2.5) puede atacarlo — basta con vencerlo en combate.
 */
export function atacarCampamentoBandidos(
  atacante: Asentamiento,
  escuadronIdsAtacantes: string[],
  campamento: CampamentoBandido,
  instante: Instante,
  facciones: Faccion[],
  rng: RandomFn
): { atacante: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; campamentoDestruido: boolean } {
  const escuadrones = seleccionarEscuadrones(atacante, escuadronIdsAtacantes);
  const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const poderAtacante = poderTotal(escuadrones, instante, false) * jitter;
  const gana = poderAtacante > campamento.poder;

  const fraccionBajas = gana ? 0.05 : 0.25;
  const escuadronesActualizados = aplicarBajas(escuadrones, fraccionBajas, gana, instante);

  let almacen = atacante.almacen;
  const eventos: EventoCrudo[] = [];
  const payloadCampamento: PayloadAtaqueCampamento = { atacanteId: atacante.id, campamentoId: campamento.id };
  if (gana) {
    for (const [recurso, cantidad] of Object.entries(CAMPAMENTOS_BANDIDOS.recompensa)) {
      if (cantidad) almacen = agregarRecurso(almacen, recurso, cantidad);
    }
    eventos.push({
      codigo: 'combate.campamento_destruido',
      mensaje: `${atacante.id} destruye el campamento de bandidos ${campamento.id} y obtiene botín.`,
      payload: payloadCampamento,
    });
  } else {
    eventos.push({
      codigo: 'combate.ataque_campamento_fallido',
      mensaje: `${atacante.id} falla el ataque al campamento de bandidos ${campamento.id} y sufre bajas.`,
      payload: payloadCampamento,
    });
  }

  // Doc Fase_0_5 §8: combate contra un campamento NPC otorga XP igual que contra otra Facción — no hay
  // defensor de Facción rival al que dar XP (el campamento no es una Facción).
  const faccionesFinal = aplicarAjustesExperiencia(facciones, [
    {
      faccionId: atacante.faccionId,
      delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(escuadrones),
      razon: 'combate (campamento de bandidos)',
    },
  ]);

  return {
    atacante: { ...atacante, almacen, escuadrones: reemplazarEscuadrones(atacante, escuadronesActualizados) },
    facciones: faccionesFinal,
    eventos,
    campamentoDestruido: gana,
  };
}

/**
 * Asedio disparado por la LLEGADA de un ejército a un asentamiento ajeno (Doc 5.12, Paso 7).
 *
 * Es el mismo combate que `iniciarAsedio` con dos diferencias que importan:
 *
 *  1. **El atacante es un ejército, no un asentamiento.** Sus escuadrones ya salieron de casa y viajan con él,
 *     así que no hay que seleccionarlos ni exigir General: la decisión de quién sale se tomó al movilizar.
 *  2. **Si no hay defensores, no hay combate.** Doc 5.12.4: un asentamiento cuyos jugadores se llevaron todo
 *     queda indefenso y cae sin pelear. Y esta rama **no consume aleatoriedad**, que no es un detalle: una
 *     partida donde nadie asedia una plaza defendida hace exactamente las mismas llamadas al RNG que antes de
 *     existir el Paso 7, y el guardián de determinismo sigue verde sin tocarlo.
 *
 * El ejército NO entra en la ciudad al conquistarla: se queda acampado donde está (`avanzarEjercitos` lo pasa
 * a `estacionado`). Meter sus escuadrones en la guarnición del sitio los convertiría en tropa apostada en un
 * asentamiento donde su jugador no reside, que es justo la incoherencia que `aplicarConquista` deshace.
 */
export function asediarConEjercito(
  ejercito: Ejercito,
  defensor: Asentamiento,
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): { ejercito: Ejercito; defensor: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; conquistado: boolean } {
  const defensores = defensor.escuadrones.filter((e) => e.cantidad > 0);
  const atacantes = ejercito.escuadrones.filter((e) => e.cantidad > 0);

  const payload: PayloadAsedio = {
    atacanteId: ejercito.id,
    defensorId: defensor.id,
    faccionAtacanteId: ejercito.faccionId,
    faccionDefensoraId: defensor.faccionId,
  };

  // Plaza desguarnecida: cae sin combate y sin tocar el RNG (Doc 5.12.4).
  if (defensores.length === 0 || atacantes.length === 0) {
    const cae = defensores.length === 0 && atacantes.length > 0;
    const eventos: EventoCrudo[] = [
      cae
        ? {
            codigo: 'combate.asedio_conquista',
            mensaje: `${defensor.id} cae sin un solo defensor en pie ante el ejército ${ejercito.id}.`,
            payload,
          }
        : {
            codigo: 'combate.asedio_resistido',
            mensaje: `El ejército ${ejercito.id} llega a ${defensor.id} sin nadie con quien combatir.`,
            payload,
          },
    ];
    return {
      ejercito,
      defensor: cae ? aplicarConquista(defensor, ejercito.faccionId) : defensor,
      facciones: cae
        ? aplicarAjustesExperiencia(facciones, [
            { faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' },
          ])
        : facciones,
      eventos,
      conquistado: cae,
    };
  }

  const resultado = resolverCombate(
    atacantes,
    defensores,
    instante,
    rng,
    multiplicadorDefensivoDeRecintos(defensor.recintos ?? [])
  );
  const conquistado = resultado.ganador === 'atacante';

  const eventos: EventoCrudo[] = [
    ...resultado.eventos,
    conquistado
      ? { codigo: 'combate.asedio_conquista', mensaje: `El ejército ${ejercito.id} conquista ${defensor.id}.`, payload }
      : { codigo: 'combate.asedio_resistido', mensaje: `${defensor.id} resiste el asedio del ejército ${ejercito.id}.`, payload },
  ];

  // Atacar a un Aliado sin romper la relación antes es la penalización MÁS SEVERA de reputación (Doc 2.7) —
  // misma regla que por el camino del comando, para que no dependa de por dónde llegue el asedio.
  const conReputacion = estanAliadas(relaciones, ejercito.faccionId, defensor.faccionId)
    ? aplicarAjustesReputacion(facciones, [
        { faccionId: ejercito.faccionId, delta: REPUTACION.penalizacionAtacarAliado, razon: 'atacar a un Aliado' },
      ])
    : facciones;

  const ajustesXp: AjusteExperiencia[] = [
    { faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(atacantes), razon: 'combate (asedio)' },
    { faccionId: defensor.faccionId, delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(defensores), razon: 'combate (asedio)' },
  ];
  if (conquistado) ajustesXp.push({ faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' });

  const idsAtacantes = new Map(resultado.atacantes.map((e) => [e.id, e]));
  return {
    ejercito: { ...ejercito, escuadrones: ejercito.escuadrones.map((e) => idsAtacantes.get(e.id) ?? e) },
    defensor: conquistado
      ? aplicarConquista(defensor, ejercito.faccionId)
      : { ...defensor, escuadrones: reemplazarEscuadrones(defensor, resultado.defensores) },
    facciones: aplicarAjustesExperiencia(conReputacion, ajustesXp),
    eventos,
    conquistado,
  };
}
