import type { Asentamiento, CampamentoBandido, Ejercito, Escuadron, Faccion, Heroe, RelacionPolitica, UbicacionHeroe } from '../domain/types';
import { alCampamento, conEscuadrones, sinTropa, type CaravanaConEscolta, type EjercitoConTropa } from './tropa';
import { distancia } from '../world/geometria';
import type { EventoCrudo } from '../domain/eventos';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { RandomFn } from '../worldgen';
import { CAMPAMENTOS_BANDIDOS, MILITAR, NIVEL_FACCION, OCUPACION, REPUTACION, TROPAS_RECLUTABLES } from '../constants';
import { agregarRecurso } from './almacen';
import { aplicarAjustesReputacion } from './reputacion';
import { aplicarAjustesExperiencia, type AjusteExperiencia } from './faccion';
import { multiplicadorDefensivoDeRecintos } from './muralla';
import { CAMPO_CARGO, esResidente, estanAliadas } from './pertenencia';
import { estaOcupado } from './asentamientoQuery';

export class CombateInvalidoError extends Error {}

/** Poder de combate (Doc 5.1: héroe-comandante liderando tropa; el resultado es CÁLCULO, no combate visual, Doc 5.10).
 * `poderBase` sale siempre del catálogo `TROPAS_RECLUTABLES` vía `tropaId` (Doc 5.7/5.8) — toda tropa lo tiene,
 * y la experiencia la mejora sin cambiarla nunca de identidad (Doc 5.8, a petición del usuario). */
export function poderEscuadron(e: Escuadron): number {
  const poderBase = TROPAS_RECLUTABLES.find((t) => t.id === e.tropaId)!.poderBase;
  return poderBase * e.cantidad * (1 + e.experiencia * MILITAR.bonusExperienciaPorPunto);
}

export function poderTotal(escuadrones: readonly Escuadron[], bonusCohesion: boolean): number {
  const suma = escuadrones.reduce((acc, e) => acc + poderEscuadron(e), 0);
  if (!bonusCohesion || escuadrones.length <= 1) return suma;
  // Cohesión entre escuadrones defendiendo juntos (Doc 5.3), abstraída sin formaciones renderizadas (Fase 0).
  return suma * (1 + MILITAR.bonusCohesionPorEscuadronExtra * (escuadrones.length - 1));
}

/** Bajas PERMANENTES (Doc 5.16.2) y la experiencia que deja el combate numérico. */
export function aplicarBajas(escuadrones: readonly Escuadron[], fraccionBajas: number, victoria: boolean): Escuadron[] {
  return escuadrones.map((e) => {
    const bajas = Math.round(e.cantidad * fraccionBajas);
    const experiencia = e.experiencia + (victoria ? MILITAR.experienciaGanadaPorVictoria : MILITAR.experienciaGanadaPorDerrota);
    return { ...e, cantidad: Math.max(0, e.cantidad - bajas), experiencia };
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
  rng: RandomFn,
  multiplicadorDefensor = 1
): ResultadoCombate {
  if (atacantes.length === 0) throw new CombateInvalidoError('El atacante no tiene escuadrones con los que combatir.');
  if (defensores.length === 0) throw new CombateInvalidoError('El defensor no tiene escuadrones con los que combatir.');

  const jitterA = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const jitterD = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const poderA = poderTotal(atacantes, false) * jitterA;
  // El multiplicador de muralla (Paso 3b, `multiplicadorDefensivoDeRecintos`) SOLO llega aquí desde
  // `iniciarAsedio` — un encuentro en mundo abierto no lo pasa nunca (1 por defecto): ahí no hay ningún
  // recinto que atravesar, así que aplicarlo ahí sería un bono de la nada.
  const poderD = poderTotal(defensores, true) * jitterD * multiplicadorDefensor;

  const ganador: 'atacante' | 'defensor' = poderA > poderD ? 'atacante' : 'defensor';
  const ratio = Math.min(poderA, poderD) / Math.max(poderA, poderD, 1);
  // Cuanto más ajustado el combate, más bajas sufre el bando ganador; el perdedor siempre pierde más.
  const bajasGanador = 0.05 + 0.15 * ratio;
  const bajasPerdedor = 0.3 + 0.4 * (1 - ratio);

  const atacantesResultado = aplicarBajas(atacantes, ganador === 'atacante' ? bajasGanador : bajasPerdedor, ganador === 'atacante');
  const defensoresResultado = aplicarBajas(defensores, ganador === 'defensor' ? bajasGanador : bajasPerdedor, ganador === 'defensor');

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

function seleccionarEscuadrones(tropa: readonly Escuadron[], ids: readonly string[]): Escuadron[] {
  const seleccionados = tropa.filter((e) => ids.includes(e.id) && e.cantidad > 0);
  if (seleccionados.length === 0) throw new CombateInvalidoError('No hay escuadrones válidos seleccionados.');
  return seleccionados;
}

/**
 * XP de Facción por JUGADOR, versión mínima (Doc Fase_0_5 §8, a petición del usuario): si 3 jugadores atacan
 * juntos, la Facción recibe 3× la XP de ese evento, no un monto plano por combate. Se calcula al vuelo
 * contando `heroeId` DISTINTOS entre los escuadrones que participaron en un bando — NO se guarda XP de
 * jugador por separado (eso exigiría una entidad `Jugador` que hoy no existe en el motor; queda pendiente si
 * llega a necesitar un propósito propio más allá de alimentar la XP de Facción).
 *
 * Aplica en los bandos con escuadrones REALES (asedio y campo abierto en ambos lados; campamento de bandidos
 * e interceptar caravana solo en el atacante). La defensa de una caravana NO se multiplica — Doc 3.10: la
 * escolta no está modelada con escuadrones/jugadores reales todavía, es una defensa fija placeholder.
 */
function jugadoresParticipantes(escuadrones: Escuadron[]): number {
  return new Set(escuadrones.map((e) => e.heroeId)).size;
}

/**
 * Lo que le pasa a un asentamiento AL SER CONQUISTADO (Doc 5.4 /
 * `Consideraciones/Ocupacion_Post_Conquista_Definicion.md`). La ciudad cambia de dueño entera:
 *
 * - **Queda SIN guarnición** (Doc 5.15.5): nadie la guarnece solo por haberla ganado. Los conquistadores
 *   siguen fuera con su columna; si quieren defenderla, tienen que pasar a residir en ella.
 * - **Los antiguos residentes dejan de serlo**, y con ellos caen los cargos locales. Adónde van —y que su
 *   campamento quede a 0— lo decide `desalojarResidentes`, que el llamador aplica con el mundo delante.
 * - **Saqueo determinista** (sin `RandomFn` — esta función es pura): `pesants`/`artesanos` pierden
 *   `OCUPACION.fraccionSaqueoPoblacion` (nobleza intacta, huye/negocia); `OCUPACION.fraccionEdificiosDanados`
 *   de los edificios `activo` —por orden de id, exentos Centro Urbano + la 1ª Granja y la 1ª Leñera activas—
 *   pasan a `en_cola` marcados `danado` (§3: sin comida ni madera el saqueo es una sentencia); cada recinto
 *   completo pierde `floor(OCUPACION.fraccionDanoMuralla × celdas.length)` de `avance` (la muralla no cae,
 *   deja de dar el multiplicador pleno hasta repararse por la vía normal de obra).
 * - **`medidorMantenimiento: 100`** y **`ocupacionHasta`** — abre la ventana de ocupación (§2.4): inmune a un
 *   nuevo asedio, recaudación y crecimiento reducidos, mantenimiento congelado, tiempo fijo.
 */
export function aplicarConquista(defensor: Asentamiento, faccionConquistadoraId: string, instante: Instante): Asentamiento {
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

  return {
    ...defensor,
    faccionId: faccionConquistadoraId,
    heroesFundadoresIds: [],
    casasCompradas: [],
    cargos,
    poblacion,
    edificios,
    recintos,
    medidorMantenimiento: 100,
    ocupacionHasta: sumar(instante, minutos(OCUPACION.duracionMinutos)),
  };
}

/**
 * Lo que pasa cuando cae una plaza (Doc 5.15.5), con `conquistado` tal como era ANTES de la conquista:
 *
 *  - **Nadie se queda dentro de una plaza enemiga** (decisión del usuario, 2026-09-14). Quien estaba dentro sale junto
 *    a ella: el visitante, a la columna que dejó aparcada; el resto, a una columna propia, estacionada en la plaza y
 *    con el carro vacío, que lleva las escuadras con las que defendió y sobrevivieron (`lucharon`). Un residente
 *    herido no defendió, así que sale solo.
 *  - Todo lo demás del campamento de los residentes —guarnición incluida— queda a 0 y se va con ellos al asentamiento
 *    más cercano de su Facción, donde pasan a residir. Sin ninguno, quedan huérfanos, con sus escuadras a 0 pero
 *    suyas, con su nivel y experiencia. Lo que cada uno llevaba fuera, en su columna o de escolta, no se toca.
 *
 * Las columnas nuevas vuelven como vistas con la tropa puesta, y los héroes ya con esa tropa en su columna.
 * `ponytail:` el traslado no mira el cupo de viviendas del destino; sin él, un desalojado se quedaría sin casa por un
 * número. Si hace falta tope, el sitio es este.
 */
export function desalojarResidentes(
  conquistado: Asentamiento,
  asentamientos: readonly Asentamiento[],
  heroes: readonly Heroe[],
  /** Las columnas del mundo: la aparcada de un visitante es donde vuelve. */
  ejercitos: readonly Ejercito[],
  /** Las escuadras que defendieron en persona (los loadouts de los que estaban dentro), sin la guarnición. */
  lucharon: ReadonlySet<string>,
  instante: Instante
): { asentamientos: Asentamiento[]; heroes: Heroe[]; columnas: EjercitoConTropa[] } {
  const residentes = heroes.filter((h) => esResidente(conquistado, h.id));
  const refugio = asentamientos
    .filter((a) => a.id !== conquistado.id && a.faccionId === conquistado.faccionId)
    .sort((a, b) => distancia(a.posicion, conquistado.posicion) - distancia(b.posicion, conquistado.posicion) || (a.id < b.id ? -1 : 1))[0];

  const columnas: EjercitoConTropa[] = [];
  const ubicaciones = new Map<string, UbicacionHeroe>();
  for (const h of heroes.filter((h) => h.ubicacion.tipo === 'asentamiento' && h.ubicacion.asentamientoId === conquistado.id)) {
    const aparcada = ejercitos.find((e) => e.participantes.some((p) => p.heroeId === h.id));
    if (aparcada) {
      ubicaciones.set(h.id, { tipo: 'columna', ejercitoId: aparcada.id });
      continue;
    }
    const escuadrones = h.escuadrones.filter((e) => lucharon.has(e.id) && e.contenedor.tipo === 'campamento' && e.cantidad > 0);
    const id = `${conquistado.id}-salida-${h.id}-${instante}`;
    columnas.push({
      id,
      faccionId: esResidente(conquistado, h.id) ? conquistado.faccionId : '',
      origenAsentamientoId: refugio?.id ?? '',
      participantes: [{ heroeId: h.id, unidoEn: instante }],
      tipo: 'personal',
      liderId: h.id,
      politicaDeUnion: 'rechazar',
      escuadronIds: escuadrones.map((e) => e.id),
      escuadrones,
      suministro: {},
      caravanasAdjuntasIds: [],
      objetivo: { tipo: 'punto', punto: conquistado.posicion },
      ruta: [],
      progreso: 0,
      posicionActual: conquistado.posicion,
      estado: 'estacionado',
    });
    ubicaciones.set(h.id, { tipo: 'columna', ejercitoId: id });
  }

  const fuera = columnas.flatMap((c) => sinTropa(c).tropa);
  const salen = new Set(fuera.map((e) => e.id));
  const sinCampamento = residentes.flatMap((h) =>
    h.escuadrones
      .filter((e) => e.contenedor.tipo === 'campamento' && !salen.has(e.id))
      .map((e) => ({ ...e, cantidad: 0, enGuarnicion: false }))
  );
  return {
    asentamientos: refugio
      ? asentamientos.map((a) => (a.id === refugio.id ? { ...a, casasCompradas: [...a.casasCompradas, ...residentes.map((h) => h.id)] } : a))
      : [...asentamientos],
    heroes: conEscuadrones(heroes, [...sinCampamento, ...fuera]).map((h) => {
      const ubicacion = ubicaciones.get(h.id);
      return ubicacion ? { ...h, ubicacion } : h;
    }),
    columnas,
  };
}

/**
 * Asedio de asentamientos (Doc 5.2.1): mortalidad severa, sin instancia visual (Fase 0 = cálculo). La conquista
 * exacta tras ganar el asedio queda PENDIENTE en el diseño (Preguntas_Abiertas) — Fase 0 asume CAPTURA directa
 * (reasignación de Facción), la opción más simple de las citadas ahí (captura/destrucción/vasallaje automático).
 */
export function iniciarAsedio(
  atacante: Asentamiento,
  /** El campamento del atacante (`campamentoDe`): de ahí salen los escuadrones elegidos. */
  tropaAtacante: readonly Escuadron[],
  defensor: Asentamiento,
  /** El campamento del defensor: su guarnición entera, hoy (`enGuarnicion` llega en la fase 3). */
  tropaDefensora: readonly Escuadron[],
  escuadronIdsAtacantes: string[],
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): { defensor: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; conquistado: boolean; tropa: Escuadron[] } {
  if (atacante.faccionId === defensor.faccionId) {
    throw new CombateInvalidoError('No se puede asediar un asentamiento de la propia Facción.');
  }
  if (!atacante.cargos.generalId) throw new CombateInvalidoError('El atacante necesita un General para asediar.');

  // Ventana de ocupación (Ocupacion §2.4): una plaza recién conquistada es INMUNE a un nuevo asedio hasta que
  // el reloj vence. Rebota sin combate y sin tocar el RNG — la guarnición instalada sana y se repone en paz.
  if (estaOcupado(defensor, instante)) {
    return {
      defensor,
      facciones,
      tropa: [],
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

  const escuadronesAtacantes = seleccionarEscuadrones(tropaAtacante, escuadronIdsAtacantes);
  const escuadronesDefensores = seleccionarEscuadrones(tropaDefensora, tropaDefensora.map((e) => e.id));

  // Paso 3b (§16 del doc de murallas): la razón de ser de toda la mecánica — un asedio contra un recinto
  // cerrado es mucho más caro para el atacante, y tanto más cuantas menos puertas tenga el defensor.
  const multiplicadorMuralla = multiplicadorDefensivoDeRecintos(defensor.recintos ?? []);
  const resultado = resolverCombate(escuadronesAtacantes, escuadronesDefensores, rng, multiplicadorMuralla);

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

  // Conquistar no mueve a nadie (Doc 5.15.5): los atacantes siguen en su campamento con sus bajas y la plaza
  // queda sin guarnición. A los residentes derrotados los desaloja el llamador (`desalojarResidentes`).
  return {
    defensor: conquistado ? aplicarConquista(defensor, atacante.faccionId, instante) : defensor,
    facciones: faccionesFinal,
    eventos,
    conquistado,
    tropa: [...resultado.atacantes, ...resultado.defensores],
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
 *
 * Es el ataque DESDE UNA PLAZA y hoy solo lo usan los NPC (`npcGobernanza.ts`), hasta que se defina su
 * comportamiento en el mundo (Mecánicas §36). El jugador ataca con una columna que llegue (Doc 1.9,
 * `atacarCampamentoConColumna`).
 */
export function atacarCampamentoBandidos(
  atacante: Asentamiento,
  /** El campamento del atacante (`campamentoDe`): de ahí salen los escuadrones elegidos. */
  tropa: readonly Escuadron[],
  escuadronIdsAtacantes: string[],
  campamento: CampamentoBandido,
  facciones: Faccion[],
  rng: RandomFn
): { atacante: Asentamiento; facciones: Faccion[]; eventos: EventoCrudo[]; campamentoDestruido: boolean; tropa: Escuadron[] } {
  const escuadrones = seleccionarEscuadrones(tropa, escuadronIdsAtacantes);
  const { gana, escuadrones: escuadronesActualizados } = choqueContraCampamento(escuadrones, campamento, rng);

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
    atacante: { ...atacante, almacen },
    facciones: faccionesFinal,
    eventos,
    campamentoDestruido: gana,
    tropa: escuadronesActualizados,
  };
}

/** El choque contra un campamento de bandidos (Doc 1.9): poder con jitter contra su `poder` fijo, con las bajas de
 * siempre. Lo comparten el ataque desde una plaza (NPC) y el de una columna. */
function choqueContraCampamento(escuadrones: Escuadron[], campamento: CampamentoBandido, rng: RandomFn): { gana: boolean; escuadrones: Escuadron[] } {
  const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const gana = poderTotal(escuadrones, false) * jitter > campamento.poder;
  return { gana, escuadrones: aplicarBajas(escuadrones, gana ? 0.05 : 0.25, gana) };
}

/**
 * Una columna ataca el campamento de bandidos que tiene delante (Doc 1.9). Con números hasta que exista la batalla de
 * Unity (Doc 5.15.6): los soldados que lleva contra el `poder` fijo del campamento. Si gana, el campamento cae y su
 * recompensa va al carro, hasta donde quepa; lo que no cabe se pierde, como el botín de una caravana. Recibe solo lo
 * que combate: sin las escuadras de los heridos, que aparta quien llama.
 */
export function atacarCampamentoConColumna(
  ejercito: EjercitoConTropa,
  campamento: CampamentoBandido,
  facciones: Faccion[],
  capacidadCarga: number,
  rng: RandomFn
): { ejercito: EjercitoConTropa; destruido: boolean; facciones: Faccion[]; eventos: EventoCrudo[] } {
  const vivos = ejercito.escuadrones.filter((e) => e.cantidad > 0);
  if (vivos.length === 0) throw new CombateInvalidoError('La columna no lleva soldados con los que atacar.');
  const choque = choqueContraCampamento(vivos, campamento, rng);
  const porId = new Map(choque.escuadrones.map((e) => [e.id, e]));

  let suministro = ejercito.suministro;
  if (choque.gana) {
    let libre = Math.max(0, capacidadCarga - Object.values(suministro).reduce((x, y) => x + y, 0));
    for (const [recurso, cantidad] of Object.entries(CAMPAMENTOS_BANDIDOS.recompensa)) {
      const cabe = Math.min(cantidad ?? 0, libre);
      if (cabe <= 0) continue;
      suministro = { ...suministro, [recurso]: (suministro[recurso] ?? 0) + cabe };
      libre -= cabe;
    }
  }

  const payload: PayloadAtaqueCampamento = { atacanteId: ejercito.id, campamentoId: campamento.id };
  return {
    ejercito: { ...ejercito, escuadrones: ejercito.escuadrones.map((e) => porId.get(e.id) ?? e), suministro },
    destruido: choque.gana,
    facciones: aplicarAjustesExperiencia(facciones, [
      { faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.combate * jugadoresParticipantes(vivos), razon: 'combate (campamento de bandidos)' },
    ]),
    eventos: [
      choque.gana
        ? { codigo: 'combate.campamento_destruido', mensaje: `La columna ${ejercito.id} destruye el campamento de bandidos ${campamento.id}.`, payload }
        : { codigo: 'combate.ataque_campamento_fallido', mensaje: `La columna ${ejercito.id} falla el ataque al campamento de bandidos ${campamento.id}.`, payload },
    ],
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
 * Gane o no, el ejército sigue en campo y acampa (`estacionado`): conquistar no lo convierte en guarnición
 * (Doc 5.15.5). A los residentes derrotados los desaloja el llamador (`desalojarResidentes`).
 */
export function asediarConEjercito(
  ejercito: EjercitoConTropa,
  defensor: Asentamiento,
  /** El campamento del defensor: su guarnición entera, hoy (`enGuarnicion` llega en la fase 3). */
  tropaDefensora: readonly Escuadron[],
  facciones: Faccion[],
  relaciones: RelacionPolitica[],
  instante: Instante,
  rng: RandomFn
): {
  ejercito: EjercitoConTropa;
  defensor: Asentamiento;
  facciones: Faccion[];
  eventos: EventoCrudo[];
  conquistado: boolean;
  /** El campamento del defensor tras el combate. */
  tropaDefensora: Escuadron[];
} {
  const defensores = tropaDefensora.filter((e) => e.cantidad > 0);
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
      tropaDefensora: [],
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
      defensor: cae ? aplicarConquista(defensor, ejercito.faccionId, instante) : defensor,
      facciones: cae
        ? aplicarAjustesExperiencia(facciones, [
            { faccionId: ejercito.faccionId, delta: NIVEL_FACCION.xp.conquista, razon: 'conquista' },
          ])
        : facciones,
      eventos,
      conquistado: cae,
      tropaDefensora: [],
    };
  }

  const resultado = resolverCombate(atacantes, defensores, rng, multiplicadorDefensivoDeRecintos(defensor.recintos ?? []));
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
  const ejercitoTrasCombate: EjercitoConTropa = {
    ...ejercito,
    escuadrones: ejercito.escuadrones.map((e) => idsAtacantes.get(e.id) ?? e),
  };
  return {
    ejercito: ejercitoTrasCombate,
    defensor: conquistado ? aplicarConquista(defensor, ejercito.faccionId, instante) : defensor,
    facciones: aplicarAjustesExperiencia(conReputacion, ajustesXp),
    eventos,
    conquistado,
    tropaDefensora: resultado.defensores,
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
  a: EjercitoConTropa,
  b: EjercitoConTropa,
  facciones: Faccion[],
  rng: RandomFn
): { a: EjercitoConTropa; b: EjercitoConTropa; facciones: Faccion[]; eventos: EventoCrudo[] } {
  const vivosA = a.escuadrones.filter((e) => e.cantidad > 0);
  const vivosB = b.escuadrones.filter((e) => e.cantidad > 0);
  const resultado = resolverCombate(vivosA, vivosB, rng);

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
  ejercito: EjercitoConTropa,
  caravana: CaravanaConEscolta,
  capacidadCarga: number,
  rng: RandomFn
): {
  ejercito: EjercitoConTropa;
  capturada: boolean;
  eventos: EventoCrudo[];
  /** La caravana tras el combate: `null` si fue capturada (se elimina), o con la escolta actualizada si aguantó. */
  caravana: CaravanaConEscolta | null;
  /** La escolta sin héroe de una caravana capturada: a 0 y de vuelta al campamento de su héroe (Doc 5.15.4). */
  escoltaPerdida: Escuadron[];
} {
  const vivos = ejercito.escuadrones.filter((e) => e.cantidad > 0);
  const conEscolta = (caravana.escolta?.length ?? 0) > 0;
  const defensa = conEscolta ? poderTotal(caravana.escolta!, true) : MILITAR.defensaBaseCaravana;
  const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
  const gana = poderTotal(vivos, false) * jitter > defensa;

  const fraccionBajas = gana ? 0.05 : 0.25;
  const conBajas = aplicarBajas(vivos, fraccionBajas, gana);
  const porId = new Map(conBajas.map((e) => [e.id, e]));

  // La escolta sin héroe sufre bajas (Doc 3.13.6), y si la caravana cae queda a 0 (Doc 5.15.4).
  const escoltaTrasCombate = conEscolta ? aplicarBajas(caravana.escolta!, gana ? 0.25 : 0.05, !gana) : undefined;

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
    escoltaPerdida: gana && escoltaTrasCombate ? alCampamento(escoltaTrasCombate.map((e) => ({ ...e, cantidad: 0 }))) : [],
  };
}
