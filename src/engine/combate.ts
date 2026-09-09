import type { Asentamiento, CampamentoBandido, Caravana, Ejercito, Escuadron, Faccion, RelacionPolitica } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { CAMPAMENTOS_BANDIDOS, MILITAR, NIVEL_FACCION, OCUPACION, REPUTACION, TROPAS_RECLUTABLES } from '../constants';
import { agregarRecurso } from './almacen';
import { aplicarAjustesReputacion } from './reputacion';
import { aplicarAjustesExperiencia, type AjusteExperiencia } from './faccion';
import { multiplicadorDefensivoDeRecintos } from './muralla';
import { CAMPO_CARGO, estanAliadas } from './pertenencia';
import { estaOcupado } from './asentamientoQuery';

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

export function aplicarBajas(escuadrones: readonly Escuadron[], fraccionBajas: number, victoria: boolean, instante: Instante): Escuadron[] {
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
  // `iniciarAsedio` — un encuentro en mundo abierto no lo pasa nunca (1 por defecto): ahí no hay ningún
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
 * Lo que le pasa a un asentamiento AL SER CONQUISTADO (Doc 5.4 /
 * `Consideraciones/Ocupacion_Post_Conquista_Definicion.md`). La ciudad cambia de dueño entera:
 *
 * - **La guarnición del conquistado la forman los escuadrones del CONQUISTADOR** (`guarnicionEntrante`): el
 *   ejército conquistador —o los escuadrones seleccionados del asentamiento atacante— se vuelca dentro y su
 *   carro al almacén (`suministroEntrante`, capado por capacidad como cualquier depósito). NUNCA queda a 0:
 *   es el arreglo del ping-pong (antes caía sin un defensor cada vez que pasaba un ejército). Los escuadrones
 *   siguen siendo de sus jugadores, que NO residen aquí — guarnición de no-residentes (§2.3b): defienden,
 *   comen del trigo del asentamiento, su dueño los repone y re-moviliza.
 * - **Los cascarones congelados de los desalojados NO se quedan**: salen (huérfanos, Doc 5.4). El llamador ya
 *   no los pasa — antes se quedaban a 0 y dejaban la plaza indefensa.
 * - **Los antiguos residentes dejan de serlo**, y con ellos caen los cargos locales (HUÉRFANO, Doc 5.4 — no
 *   se guarda: es no residir en ningún sitio, se deriva de `esResidente`).
 * - **Saqueo determinista** (sin `RandomFn` — esta función es pura): `pesants`/`artesanos` pierden
 *   `OCUPACION.fraccionSaqueoPoblacion` (nobleza intacta, huye/negocia); `OCUPACION.fraccionEdificiosDanados`
 *   de los edificios `activo` —por orden de id, exentos Centro Urbano + la 1ª Granja y la 1ª Leñera activas—
 *   pasan a `en_cola` marcados `danado` (§3: sin comida ni madera el saqueo es una sentencia); cada recinto
 *   completo pierde `floor(OCUPACION.fraccionDanoMuralla × celdas.length)` de `avance` (la muralla no cae,
 *   deja de dar el multiplicador pleno hasta repararse por la vía normal de obra).
 * - **`medidorMantenimiento: 100`** y **`ocupacionHasta`** — abre la ventana de ocupación (§2.4): inmune a un
 *   nuevo asedio, recaudación y crecimiento reducidos, mantenimiento congelado, tiempo fijo.
 */
export function aplicarConquista(
  defensor: Asentamiento,
  faccionConquistadoraId: string,
  guarnicionEntrante: readonly Escuadron[],
  suministroEntrante: Record<string, number>,
  instante: Instante
): Asentamiento {
  const cargos = { ...defensor.cargos };
  for (const campo of Object.values(CAMPO_CARGO)) cargos[campo] = null;

  const queda = 1 - OCUPACION.fraccionSaqueoPoblacion;
  const poblacion = {
    pesants: Math.floor(defensor.poblacion.pesants * queda),
    artesanos: Math.floor(defensor.poblacion.artesanos * queda),
    nobleza: defensor.poblacion.nobleza,
  };

  // Saqueo de edificios: orden por id (determinista), exentos Centro Urbano y la primera Granja/Leñera activas.
  const activos = defensor.edificios
    .filter((e) => e.estado === 'activo')
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  const exento = new Set<string>();
  for (const e of activos) if (e.tipo === 'centroUrbano') exento.add(e.id);
  for (const tipo of ['granja', 'lenera'] as const) {
    const primero = activos.find((e) => e.tipo === tipo);
    if (primero) exento.add(primero.id);
  }
  const danables = activos.filter((e) => !exento.has(e.id));
  const aDanar = new Set(
    danables.slice(0, Math.ceil(OCUPACION.fraccionEdificiosDanados * danables.length)).map((e) => e.id)
  );
  const edificios = defensor.edificios.map((e) =>
    aDanar.has(e.id) ? { ...e, estado: 'en_cola' as const, danado: true, completaEn: undefined } : e
  );

  // Saqueo de murallas: cada recinto completo pierde integridad; la reparación es la obra normal de recintos.
  const recintos = defensor.recintos?.map((r) => {
    if (r.avance < r.celdas.length - 1) return r;
    const perdida = Math.floor(OCUPACION.fraccionDanoMuralla * r.celdas.length);
    return { ...r, avance: Math.max(-1, r.avance - perdida) };
  });

  const almacen = Object.entries(suministroEntrante).reduce(
    (acc, [recurso, cantidad]) => agregarRecurso(acc, recurso, cantidad),
    defensor.almacen
  );

  return {
    ...defensor,
    faccionId: faccionConquistadoraId,
    escuadrones: [...guarnicionEntrante],
    jugadoresFundadoresIds: [],
    casasCompradas: [],
    cargos,
    poblacion,
    edificios,
    recintos,
    almacen,
    medidorMantenimiento: 100,
    ocupacionHasta: sumar(instante, minutos(OCUPACION.duracionMinutos)),
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

  // Ventana de ocupación (Ocupacion §2.4): una plaza recién conquistada es INMUNE a un nuevo asedio hasta que
  // el reloj vence. Rebota sin combate y sin tocar el RNG — la guarnición instalada sana y se repone en paz.
  if (estaOcupado(defensor, instante)) {
    return {
      atacante,
      defensor,
      facciones,
      eventos: [
        {
          codigo: 'combate.asedio_resistido',
          mensaje: `${defensor.id} está bajo ocupación reciente y rechaza el asedio de ${atacante.id} sin combatir.`,
          payload: {
            atacanteId: atacante.id,
            defensorId: defensor.id,
            faccionAtacanteId: atacante.faccionId,
            faccionDefensoraId: defensor.faccionId,
          } satisfies PayloadAsedio,
        },
      ],
      conquistado: false,
    };
  }

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

  // Al conquistar, los escuadrones seleccionados MARCHAN a guarnecer la plaza tomada y salen de la del
  // atacante (Ocupacion §2.2: mismo principio que un ejército absorbido, sin ejército de por medio).
  const idsSeleccionados = new Set(escuadronesAtacantes.map((e) => e.id));
  return {
    atacante: {
      ...atacante,
      escuadrones: conquistado
        ? atacante.escuadrones.filter((e) => !idsSeleccionados.has(e.id))
        : reemplazarEscuadrones(atacante, resultado.atacantes),
    },
    defensor: conquistado
      ? aplicarConquista(defensor, atacante.faccionId, resultado.atacantes, {}, instante)
      : { ...defensor, escuadrones: reemplazarEscuadrones(defensor, resultado.defensores) },
    facciones: faccionesFinal,
    eventos,
    conquistado,
  };
}

// `combateCampoAbierto` e `interceptarCaravana` VIVÍAN AQUÍ y se retiraron en el Paso 11 del movimiento de
// ejércitos (2026-09-04), junto con sus comandos. Lo que hacían no se ha perdido: son ahora
// `encuentroEntreEjercitos` e `interceptarCaravanaConEjercito` (más abajo), disparadas por la GEOMETRÍA desde
// `resolverEncuentros` (engine/ejercitos.ts) en vez de por una orden del jugador — se manda un ejército y el
// choque ocurre donde tenga que ocurrir (Doc 5.12.3).
//
// Retirarlos no era solo limpieza: `interceptarCaravana` resolvía contra una defensa base FIJA, que es
// exactamente lo que la escolta sustituyó (Doc 5.13.3). Mantener los dos habría dejado dos reglas distintas
// para el mismo hecho según por dónde se entrara.

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
 * Al CONQUISTAR, el ejército SE VUELVE la guarnición de la plaza tomada (Ocupacion §2.2): sus escuadrones y
 * su carro se vuelcan dentro vía `aplicarConquista` y `ejercitoConsumido: true` le dice a `avanzarEjercitos`
 * que lo suelte sin evento `disuelto`. Si resiste, el ejército sigue en campo y acampa (`estacionado`).
 */
export function asediarConEjercito(
  ejercito: Ejercito,
  defensor: Asentamiento,
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): {
  ejercito: Ejercito;
  defensor: Asentamiento;
  facciones: Faccion[];
  eventos: EventoCrudo[];
  conquistado: boolean;
  /** El ejército se volcó en la guarnición del conquistado — `avanzarEjercitos` no debe conservarlo. */
  ejercitoConsumido: boolean;
} {
  const defensores = defensor.escuadrones.filter((e) => e.cantidad > 0);
  const atacantes = ejercito.escuadrones.filter((e) => e.cantidad > 0);

  const payload: PayloadAsedio = {
    atacanteId: ejercito.id,
    defensorId: defensor.id,
    faccionAtacanteId: ejercito.faccionId,
    faccionDefensoraId: defensor.faccionId,
  };

  // Ventana de ocupación (Ocupacion §2.4): inmune a un nuevo asedio. Rebota sin combate ni RNG; el ejército
  // acampa (`avanzarEjercitos` lo pasa a `estacionado`) y el Paso 10 decide qué hace un rival ahí plantado.
  if (estaOcupado(defensor, instante)) {
    return {
      ejercito,
      defensor,
      facciones,
      eventos: [
        {
          codigo: 'combate.asedio_resistido',
          mensaje: `${defensor.id} está bajo ocupación reciente: el ejército ${ejercito.id} no puede asediarla todavía.`,
          payload,
        },
      ],
      conquistado: false,
      ejercitoConsumido: false,
    };
  }

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
      defensor: cae
        ? aplicarConquista(defensor, ejercito.faccionId, ejercito.escuadrones, ejercito.suministro, instante)
        : defensor,
      facciones: cae
        ? aplicarAjustesExperiencia(facciones, [
            { faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' },
          ])
        : facciones,
      eventos,
      conquistado: cae,
      ejercitoConsumido: cae,
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
  const ejercitoTrasCombate: Ejercito = {
    ...ejercito,
    escuadrones: ejercito.escuadrones.map((e) => idsAtacantes.get(e.id) ?? e),
  };
  return {
    ejercito: ejercitoTrasCombate,
    defensor: conquistado
      ? aplicarConquista(defensor, ejercito.faccionId, ejercitoTrasCombate.escuadrones, ejercito.suministro, instante)
      : { ...defensor, escuadrones: reemplazarEscuadrones(defensor, resultado.defensores) },
    facciones: aplicarAjustesExperiencia(conReputacion, ajustesXp),
    eventos,
    conquistado,
    ejercitoConsumido: conquistado,
  };
}

/**
 * Choque en campo abierto entre dos ejércitos que se han cruzado (Doc 5.2.2 / 5.12.3, Paso 10).
 *
 * Nadie lo ordena: sale de la geometría. Por eso NO hay atacante ni defensor en el sentido del comando viejo
 * —los dos iban a lo suyo— y el bonus de cohesión defensiva no se aplica a ninguno; la ventaja de defender es
 * de quien está en una plaza, no de quien se topa con otro en un camino.
 *
 * Los aliados no se cruzan: el llamador ya los excluye. Sin eso, dos columnas amigas que compartieran ruta se
 * masacrarían solas cada tick, que es lo contrario de lo que una alianza significa.
 */
export function encuentroEntreEjercitos(
  a: Ejercito,
  b: Ejercito,
  facciones: Faccion[],
  instante: Instante,
  rng: RandomFn
): { a: Ejercito; b: Ejercito; facciones: Faccion[]; eventos: EventoCrudo[] } {
  const vivosA = a.escuadrones.filter((e) => e.cantidad > 0);
  const vivosB = b.escuadrones.filter((e) => e.cantidad > 0);
  const resultado = resolverCombate(vivosA, vivosB, instante, rng);

  const porId = <T extends { id: string }>(lista: T[]) => new Map(lista.map((x) => [x.id, x]));
  const actualizadosA = porId(resultado.atacantes);
  const actualizadosB = porId(resultado.defensores);

  const eventos: EventoCrudo[] = [
    ...resultado.eventos,
    {
      codigo: 'combate.encuentro',
      mensaje: `Los ejércitos ${a.id} y ${b.id} se cruzan y combaten: gana ${resultado.ganador === 'atacante' ? a.id : b.id}.`,
      payload: {
        ejercitoAId: a.id,
        ejercitoBId: b.id,
        faccionAId: a.faccionId,
        faccionBId: b.faccionId,
        ganadorId: resultado.ganador === 'atacante' ? a.id : b.id,
      } satisfies PayloadEncuentroEjercitos,
    },
  ];

  const faccionesFinal = aplicarAjustesExperiencia(facciones, [
    { faccionId: a.faccionId, delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(vivosA), razon: 'combate (encuentro)' },
    { faccionId: b.faccionId, delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(vivosB), razon: 'combate (encuentro)' },
  ]);

  return {
    a: { ...a, escuadrones: a.escuadrones.map((e) => actualizadosA.get(e.id) ?? e) },
    b: { ...b, escuadrones: b.escuadrones.map((e) => actualizadosB.get(e.id) ?? e) },
    facciones: faccionesFinal,
    eventos,
  };
}

/** Fase A5 — payload de `combate.encuentro` (Doc 5.2.2). */
export interface PayloadEncuentroEjercitos {
  ejercitoAId: string;
  ejercitoBId: string;
  faccionAId: string;
  faccionBId: string;
  ganadorId: string;
}

/** Fase A5 — payload de `combate.caravana_interceptada_por_ejercito` (Doc 3.10 / 5.2.3). */
export interface PayloadInterceptacionEjercito {
  ejercitoId: string;
  caravanaId: string;
  capturada: boolean;
  botin: Record<string, number>;
}

/**
 * Un ejército alcanza una caravana enemiga sin escolta y la embosca (Doc 3.10 / 5.2.3, Paso 10).
 *
 * Es la misma resolución asimétrica de siempre —poder del atacante contra `defensaBaseCaravana`, captura del
 * 50% de la carga, la caravana se elimina si cae— pero disparada por la geometría en vez de por un comando, y
 * con un atacante que es una columna en el mapa y no un asentamiento.
 *
 * **Dónde va el botín es una decisión que el canon no cerraba** (2026-09-04): los comandos viejos lo metían en
 * el almacén del asentamiento atacante, y un ejército no tiene almacén. Va a su CARGA, con dos consecuencias
 * que lo hacen coherente con el resto: cabe solo lo que quepa —el resto se pierde, saquear no es gratis— y
 * llega a casa por la vía que ya existe, porque el sobrante del carro vuelve al almacén de origen al
 * replegarse (Doc 5.13). El carro sigue sin poder descargarse en ruta, así que esto no lo convierte en un
 * transporte de mercancías: para eso están las caravanas adjuntas.
 */
export function interceptarCaravanaConEjercito(
  ejercito: Ejercito,
  caravana: Caravana,
  capacidadCarga: number,
  instante: Instante,
  rng: RandomFn
): {
  ejercito: Ejercito;
  capturada: boolean;
  eventos: EventoCrudo[];
  /** La caravana tras el combate: `null` si fue capturada (se elimina), o con la escolta actualizada si aguantó. */
  caravana: Caravana | null;
  /** Escolta sin héroe (Doc 3.13.4) que vuelve a la guarnición del origen — solo si fue capturada. */
  escoltaDevuelta: Escuadron[];
} {
  const vivos = ejercito.escuadrones.filter((e) => e.cantidad > 0);
  const conEscolta = (caravana.escolta?.length ?? 0) > 0;
  const defensa = conEscolta ? poderTotal(caravana.escolta!, instante, true) : MILITAR.defensaBaseCaravana;
  const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const gana = poderTotal(vivos, instante, false) * jitter > defensa;

  const fraccionBajas = gana ? 0.05 : 0.25;
  const conBajas = aplicarBajas(vivos, fraccionBajas, gana, instante);
  const porId = new Map(conBajas.map((e) => [e.id, e]));

  // La escolta sin héroe sufre bajas y vuelve a casa con el debuff de derrota (Doc 3.13.6) — lo que se pierde
  // son la carga y los carros, no la tropa.
  const escoltaTrasCombate = conEscolta ? aplicarBajas(caravana.escolta!, gana ? 0.25 : 0.05, !gana, instante) : undefined;

  let suministro = ejercito.suministro;
  const botin: Record<string, number> = {};
  if (gana) {
    let libre = Math.max(0, capacidadCarga - Object.values(suministro).reduce((x, y) => x + y, 0));
    for (const [recurso, cantidad] of Object.entries(caravana.contenido)) {
      const cabe = Math.min(cantidad * MILITAR.umbralCapturaCaravana, libre);
      if (cabe <= 0) continue;
      suministro = { ...suministro, [recurso]: (suministro[recurso] ?? 0) + cabe };
      botin[recurso] = cabe;
      libre -= cabe;
    }
  }

  const eventos: EventoCrudo[] = [
    {
      codigo: 'combate.caravana_interceptada_por_ejercito',
      mensaje: gana
        ? `El ejército ${ejercito.id} embosca y destruye la caravana ${caravana.id}${
            Object.keys(botin).length > 0
              ? `, con un botín de ${Object.entries(botin)
                  .map(([r, c]) => `${c.toFixed(0)} ${r}`)
                  .join(', ')}`
              : ' (sin sitio en el carro para el botín)'
          }.`
        : `La caravana ${caravana.id} se zafa del ejército ${ejercito.id}.`,
      payload: { ejercitoId: ejercito.id, caravanaId: caravana.id, capturada: gana, botin } satisfies PayloadInterceptacionEjercito,
    },
  ];

  return {
    ejercito: { ...ejercito, escuadrones: ejercito.escuadrones.map((e) => porId.get(e.id) ?? e), suministro },
    capturada: gana,
    eventos,
    caravana: gana ? null : escoltaTrasCombate ? { ...caravana, escolta: escoltaTrasCombate } : caravana,
    escoltaDevuelta: gana && escoltaTrasCombate ? escoltaTrasCombate : [],
  };
}
