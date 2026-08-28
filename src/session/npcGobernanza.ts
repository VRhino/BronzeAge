// NPC de gobernanza: juega el rol de Gobernador/Tesorero/Rey que un jugador humano jugaría manualmente
// (Doc 3.2/2.2). Vive en la capa de aplicación (`src/app/`), NUNCA dentro del motor: acoplamiento CERO con
// `src/engine/*` — solo consume sus funciones PÚBLICAS, exactamente igual que lo hace `GameStore` cuando el
// jugador pulsa un botón. El motor no sabe que este archivo existe y jamás lo llama solo (ni siquiera con
// `SIMULACION_AUTO_COMERCIO.activo = 1`, que solo cubre la parte de comercio interno de una Facción).
//
// Dos consumidores, mismo comportamiento:
//
// 1. PARTIDA REAL — `GameStore.avanzarTick` lo llama con `faccionesIds` = las Facciones que el jugador ha
//    marcado como "controlada por NPC" desde la pestaña Facción. Sin ese filtro el NPC gobernaría también
//    las Facciones del jugador, que es justo lo que no debe pasar.
// 2. SCRIPTS DE BATCH — `scripts/run-batch-sim.ts` y escenarios temporales lo llaman SIN `faccionesIds`,
//    para que gobierne el mundo entero (ver Consideraciones/Diarios_Simulaciones_Batch/*.md).
//
// En ambos casos el patrón es el mismo, y siempre DESPUÉS del tick del motor:
//
//   const contexto = { tick, momento, rng };                        // ver ContextoSimulacion
//   estado = avanzarSimulacion(estado, mapa, contexto);             // motor real, sin tocar
//   estado = avanzarNpcGobernanza(estado, mapa, contexto, cfg).estado;
//
// `avanzarNpcGobernanza` no genera mundo, no funda el asentamiento inicial de una Facción y no escribe
// archivos — solo decide, con las funciones PÚBLICAS del motor, qué haría ese NPC en ese tick.
//
// Diseño, decisiones y limitaciones: `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md`.

import type { AcuerdoTrueque, Asentamiento, Caravana, CampamentoBandido, EdificioTipo, Faccion, Point, RecursoTipo } from '../domain/types';
import type { Mapa } from '../world/mapa';
import type { RandomFn } from '../worldgen';
import type { ContextoSimulacion, EstadoSimulacion } from '../engine/simulation';
import { avanzarAutoComercioSimulado } from '../engine/simulacionAutoComercio';
import { reclutarTropa, ReclutamientoInvalidoError } from '../engine/tropas';
import { atacarCampamentoBandidos, CombateInvalidoError } from '../engine/combate';
import { lanzarCaravanaFundacion, costoCaravanaFundacion, ExpansionInvalidaError } from '../engine/expansion';
import {
  nivelActualDe,
  tieneMercadoActivo,
  cupoCaravanas,
  edificiosPorTipoYEstado,
  hayProyectoPendiente,
  nutricionPoblacionDe,
} from '../engine/asentamientoQuery';
import { tieneRecursos } from '../engine/almacen';
import { asignarCargoLocal, CargoInvalidoError } from '../engine/cargos';
import { anadirEdificioManualmente, reclamosDeFuentes, ConstruccionManualInvalidaError } from '../engine/construction';
import { construirCaravanaComercial, proponerTrueque, CaravanaInvalidaError, TruequeInvalidoError } from '../engine/trade';
import { computeTodasLasZonas } from '../engine/zones';
import { calcularCostoMantenimiento, encontrarCapital } from '../engine/mantenimiento';
import { evaluarViabilidadFundacion, fundarAsentamiento, FundacionInvalidaError } from '../engine/settlement';
import { CAMPAMENTOS_BANDIDOS, TROPAS_RECLUTABLES } from '../constants';

/**
 * Reserva mínima de madera antes de reclutar (a petición del usuario, tras diagnosticar el colapso masivo de
 * nivel 1 — ver Consideraciones/Diarios_Simulaciones_Batch/Diario_Simulaciones_Batch_10x_Diagnostico_Colapso_Madera.md):
 * un solo reclutamiento de `milicia_lanceros` (25 unidades × 2 madera = 50) agotaba de un golpe el stock
 * inicial completo (`FUNDACION.materialesIniciales.madera = 50`), sin dejar nada para Mantenimiento — y
 * `reclutarTropa` (motor) no respeta ninguna reserva, a diferencia de la auto-construcción
 * (`puedeIniciarConstruccion`). Este NPC ahora impone su PROPIA prudencia en dos frentes: (1) reserva 150 de
 * madera vía `reservaManual` (protege esa madera de la auto-construcción) y (2) no intenta reclutar en un
 * asentamiento si su almacén de madera no llega a 150 — ninguno de los dos cambia el motor.
 */
const RESERVA_MADERA_ANTES_DE_RECLUTAR = 150;

/**
 * Dos gates de prudencia propios del NPC antes de atacar campamentos de bandidos (ver
 * `atacarCampamentosCercanos`) — a petición del usuario: "el ataque no debe ser todo el rato sino solo cuando
 * es posible sin perjudicar el asentamiento". Prudencia propia del NPC, no una regla del motor: atacar es una
 * decisión deliberada y visible que un jugador humano puede querer tomar aun arriesgado (una defensa
 * desesperada, por ejemplo); lo que hace falta frenar es que un GUION siga atacando sin criterio.
 *
 * `UMBRAL_NUTRICION_ANTES_DE_ATACAR` — no atacar si la nutrición civil ya está comprometida (mismo umbral que
 * `RESERVA_MADERA_ANTES_DE_RECLUTAR`).
 *
 * `SALUD_ESCUADRONES_ANTES_DE_ATACAR` — no atacar si los escuadrones YA existentes están por debajo de esta
 * fracción de su tamaño nominal (`cantidad` actual ÷ `unidadesPorDefecto` de su tropa, promediado): no mandar
 * a pelear otra vez a un ejército que ya salió herido de la última pelea, dándole tiempo a reponerse (ahora
 * gateado por la reserva de trigo de `reclutarTropa`, `engine/tropas.ts`) antes de arriesgarlo de nuevo.
 *
 * LOS DOS SON CORRECTOS COMO REGLA, PERO NINGUNO RESUELVE EL COLAPSO MASIVO MEDIDO EN BATCH, y hay que dejarlo
 * anotado para no repetir la prueba: en un mundo de 20 Facciones, 11/20 colapsan hacia el tick 300 tanto SIN
 * estos gates como CON ellos a cualquier umbral razonable probado (nutrición 50 y 95; salud 0.6 y hasta 0.95,
 * casi exigiendo curación completa) — resultados IDÉNTICOS en `colapsados`/`nivelesAsentamiento['2']`/`vivos`
 * en los tres casos. Solo un umbral IMPOSIBLE de cumplir (nutrición 101, salud 1.5 — atacar nunca) cambia el
 * resultado de verdad (colapsos bajan de 11/20 a 7/20, nivel 2 estable sube de 6 a 9), y eso no es un gate
 * utilizable, es apagar la función.
 *
 * La razón: el daño no viene de mandar un ejército YA HERIDO a pelear otra vez (eso es justo lo que
 * `SALUD_ESCUADRONES_ANTES_DE_ATACAR` evita, y no cambió el resultado) — viene de que el PRIMER combate de
 * cada uno de los ~20 asentamientos, prácticamente simultáneo desde el tick 1 (tick 100: ya 20 campamentos
 * destruidos, tropas cayendo de su pico inicial), inflige de golpe el daño agregado que dispara el colapso.
 * Un gate reactivo —que solo puede mirar estado YA dañado— no tiene nada a lo que reaccionar todavía en ese
 * primer ataque: `saludEscuadrones` da 1.0 antes de la primera pelea de cada uno, así que ningún umbral
 * finito lo detiene. Queda como pregunta abierta en
 * `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto" — probablemente hace falta pausar el
 * PRIMER combate hasta que el asentamiento tenga cierta madurez (nivel, población, ticks desde la fundación),
 * no seguir ajustando el umbral de un gate reactivo.
 */
const UMBRAL_NUTRICION_ANTES_DE_ATACAR = 50;
const SALUD_ESCUADRONES_ANTES_DE_ATACAR = 0.6;

/** Tamaño actual de los escuadrones del asentamiento como fracción de su tamaño nominal combinado — 1 si no
 * hay ninguno (nada que proteger, el chequeo de `escuadrones.length === 0` de arriba ya filtra ese caso antes
 * de llegar aquí). */
function saludEscuadrones(asentamiento: Asentamiento): number {
  let cantidadTotal = 0;
  let nominalTotal = 0;
  for (const escuadron of asentamiento.escuadrones) {
    const tropa = TROPAS_RECLUTABLES.find((t) => t.id === escuadron.tropaId);
    if (!tropa) continue;
    cantidadTotal += escuadron.cantidad;
    nominalTotal += tropa.unidadesPorDefecto;
  }
  return nominalTotal <= 0 ? 1 : cantidadTotal / nominalTotal;
}

/**
 * Trueque de SUPERVIVENCIA (a petición del usuario, replanteo del trueque automático tras el mismo
 * diagnóstico de arriba): a diferencia del trueque de especialización que este archivo ya delega en el motor
 * (`avanzarAutoComercioSimulado`, `engine/simulacionAutoComercio.ts` — solo DENTRO de la misma Facción, y
 * excluye madera/trigo del lado del déficit a propósito, asumiendo que "casi todo asentamiento puede
 * producirlos por sí mismo"), este paso persigue específicamente lo que Mantenimiento necesita
 * (`calcularCostoMantenimiento`: madera siempre, +piedra desde nivel 2, +oro desde nivel 3) — y lo busca en
 * CUALQUIER asentamiento de CUALQUIER Facción que tenga excedente, no solo los propios: la idea es
 * sobrevivir, no competir. "Comprar" aquí es siempre vía trueque (`proponerTrueque`, que el motor ya soporta
 * entre Facciones distintas sin restricción — ver `tasaComision` en `engine/trade.ts`), nunca una compra con
 * dinero de la nada.
 */
const RECURSOS_MANTENIMIENTO: RecursoTipo[] = ['madera', 'piedra', 'oro'];
/** Recursos "crudos" que este NPC ofrece como moneda de pago — excluye intermedios de crafting (lingotes,
 * cuero, armas...): son producción especializada, no moneda genérica. Oro se evalúa APARTE y solo se ofrece
 * si ningún otro recurso califica (a petición del usuario: "con oro como último recurso"). */
const RECURSOS_MONEDA_SUPERVIVENCIA: RecursoTipo[] = ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'livestock'];
/** Un recurso solo se ofrece como pago (o se acepta como excedente de un socio) si supera esta fracción de
 * su capacidad — mismo concepto que `SIMULACION_AUTO_COMERCIO.colchonExcedente` (motor), para que ni quien
 * pide ni quien da se queden sin su propio margen de seguridad. */
const COLCHON_EXCEDENTE_SUPERVIVENCIA = 0.3;
/** Cantidad pactada por lado en cada trueque de supervivencia propuesto — mismo orden de magnitud que
 * `SIMULACION_AUTO_COMERCIO.cantidadPorTrueque` (30) en el motor. */
const CANTIDAD_TRUEQUE_SUPERVIVENCIA = 30;
/** Cuántos ticks de costo de Mantenimiento por delante hace falta tener cubiertos para NO considerarse en
 * riesgo — un trueque tarda en construirse (Mercado/caravana) y viajar, así que hay que pedir ayuda ANTES de
 * quedarse en 0 (para entonces ya sería tarde: el medidor empezaría a degradar sin nada que pagar). */
const TICKS_ANTICIPACION_SUPERVIVENCIA = 20;

export interface StatsNpcGobernanza {
  reclutamientosExitosos: number;
  truequesSupervivenciaPropuestos: number;
  campamentosDestruidos: number;
  campamentosAtacadosSinExito: number;
  caravanasFundacionLanzadas: number;
}

export interface ResultadoNpcGobernanza {
  estado: EstadoSimulacion;
  eventos: string[];
  stats: StatsNpcGobernanza;
  /** Siguiente número de secuencia libre para ids (ver `ConfigNpcGobernanza.contadorInicial`). */
  contadorFinal: number;
}

/** Residentes de un asentamiento (Doc 2.5): fundadores + quien compró casa — cualquiera puede reclutar. */
function residentesDe(asentamiento: Asentamiento): string[] {
  return [...new Set([...asentamiento.jugadoresFundadoresIds, ...asentamiento.casasCompradas])];
}

/**
 * Gobernador+Tesorero+reserva para CUALQUIER asentamiento, sin importar cuántos asentamientos tenga su
 * Facción. `avanzarAutoComercioSimulado` (`engine/simulacionAutoComercio.ts`, sin tocar) solo actúa sobre
 * Facciones con 2+ asentamientos propios — fue escrito para el escenario de comercio intra-Facción del
 * batch anterior — así que una Facción que arranca con UN solo asentamiento (este escenario: 100 Facciones
 * × 1) nunca recibiría Gobernador/Tesorero/reserva por esa vía. Este paso cubre ese caso, con el MISMO
 * patrón (primer fundador para ambos cargos, `asignarCargoLocal`) que ya usa el módulo de comercio.
 *
 * La reserva se calibra para el costo de la Caravana de FUNDACIÓN (`costoCaravanaFundacion`,
 * `engine/expansion.ts`) — a diferencia de la reserva del módulo de comercio (solo madera, atada a la
 * caravana comercial), aquí hace falta reservar TODOS los recursos que esa caravana consume, porque es la
 * caravana relevante para la expansión que pide este escenario (punto 5 del usuario original: "dejar
 * material suficiente para la creación de las caravanas").
 *
 * La reserva de la Caravana de Fundación SOLO se activa una vez el asentamiento ya está en
 * `nivelActual >= 2` — el gate real de `lanzarCaravanaFundacion` (por debajo de eso, expandir no es ni legal
 * todavía). `reservaManual` bloquea TODA auto-construcción nueva por debajo de su umbral
 * (`puedeIniciarConstruccion`, `engine/construction.ts` — se SUMA a la reserva dinámica de Mantenimiento, no
 * la reemplaza), así que reservar de entrada el costo completo de la caravana (varios cientos de
 * madera/piedra/trigo/oro) desde el tick de fundación congelaría la Vivienda/Granja/Leñera que un
 * asentamiento recién fundado necesita para sobrevivir la ventana crítica de los primeros ~100 ticks — sería
 * peor el remedio que la enfermedad. Un Tesorero real reservaría para la expansión solo cuando esta ya está
 * al alcance, no desde el día uno.
 *
 * La reserva BASE de madera (`RESERVA_MADERA_ANTES_DE_RECLUTAR`), en cambio, sí se activa desde que hay
 * Tesorero, sin esperar a nivel 2 — su propósito no es proteger una expansión futura, es proteger el
 * Mantenimiento del propio asentamiento (que a nivel 1 solo cobra madera) frente a la auto-construcción.
 */
function asegurarGobernanzaBase(asentamiento: Asentamiento, facciones: Faccion[]): Asentamiento {
  const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
  if (!faccion) return asentamiento;
  let actual = asentamiento;
  const fundador = actual.jugadoresFundadoresIds[0];

  if (!actual.cargos.gobernadorId && fundador) {
    try {
      actual = asignarCargoLocal(actual, faccion, 'gobernador', fundador);
    } catch (err) {
      if (!(err instanceof CargoInvalidoError)) throw err;
    }
  }
  if (!actual.cargos.tesoreroId && fundador) {
    try {
      actual = asignarCargoLocal(actual, faccion, 'tesorero', fundador);
    } catch (err) {
      if (!(err instanceof CargoInvalidoError)) throw err;
    }
  }

  if (actual.cargos.tesoreroId) {
    let reservaManual = actual.reservaManual;
    let cambio = false;

    if ((reservaManual?.madera ?? 0) < RESERVA_MADERA_ANTES_DE_RECLUTAR) {
      reservaManual = { ...reservaManual, madera: RESERVA_MADERA_ANTES_DE_RECLUTAR };
      cambio = true;
    }

    if (nivelActualDe(actual) >= 2) {
      const costo = costoCaravanaFundacion();
      for (const [recurso, cantidad] of Object.entries(costo)) {
        if (cantidad === undefined) continue;
        const key = recurso as RecursoTipo;
        if ((reservaManual?.[key] ?? 0) < cantidad) {
          reservaManual = { ...reservaManual, [key]: cantidad };
          cambio = true;
        }
      }
    }

    if (cambio) actual = { ...actual, reservaManual };
  }

  return actual;
}

/**
 * Al menos `GRANJAS_MINIMAS_ANTES_DE_COMERCIO` Granjas (en cualquier estado — activa, en obra o en cola,
 * cuentan igual: lo que importa es que ya están pagadas y en camino) antes de dejar pasar al asentamiento a
 * infraestructura comercial (paso siguiente en `avanzarNpcGobernanza`). A petición del usuario, tras
 * confirmar en batch (`issues/granjas_no_escalan_con_poblacion.md`) que ninguna de 89 facciones construía
 * nunca una segunda Granja: seguridad alimentaria antes que comercio.
 *
 * Usa CONSTRUCCIÓN MANUAL (`anadirEdificioManualmente`), no el disparador automático del motor
 * (`evaluarNecesidades`, `engine/construction.ts`) — y la elección no es cosmética. El disparador automático
 * respeta `reservaDinamicaConstruccion` **sumada a `asentamiento.reservaManual`** (ver comentario en
 * `construction.ts` junto a esa suma), así que la madera que este mismo archivo reserva para Mantenimiento
 * (`RESERVA_MADERA_ANTES_DE_RECLUTAR`, ver `asegurarGobernanzaBase`) queda fuera de su alcance. La
 * construcción MANUAL calcula su propia reserva SIN esa suma — está exenta a propósito (mismo comentario en
 * `construction.ts`) — así que es el único camino por el que el NPC puede gastar esa madera reservada en una
 * Granja, en vez de dejarla parada protegiendo un Mantenimiento que de nada sirve si la población se muere
 * de hambre antes de llegar a pagarlo.
 */
const GRANJAS_MINIMAS_ANTES_DE_COMERCIO = 2;

function asegurarGranjasMinimas(
  asentamiento: Asentamiento,
  faccion: Faccion,
  zonaPoligono: ReturnType<typeof computeTodasLasZonas>[number]['poligono'],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReturnType<typeof reclamosDeFuentes>,
  contador: number
): { asentamiento: Asentamiento; aseguradas: boolean } {
  if (!asentamiento.cargos.gobernadorId) return { asentamiento, aseguradas: false };

  const totalGranjas = asentamiento.edificios.filter((e) => e.tipo === 'granja').length;
  if (totalGranjas >= GRANJAS_MINIMAS_ANTES_DE_COMERCIO) return { asentamiento, aseguradas: true };

  try {
    const actual = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'granja', zonaPoligono, mapa, capital, reclamos, contador);
    // +1 aquí, no releer `actual.edificios`: es exactamente lo que acabamos de añadir, y evita otra pasada
    // de filtro solo para confirmar lo que ya sabemos.
    return { asentamiento: actual, aseguradas: totalGranjas + 1 >= GRANJAS_MINIMAS_ANTES_DE_COMERCIO };
  } catch (err) {
    // Sin madera todavía (o sin sitio): no hay nada que hacer este tick. `aseguradas: false` retiene al
    // asentamiento en este paso — no pasa a infraestructura comercial hasta que la próxima madera disponible
    // (o la siguiente, o la que haga falta) complete las `GRANJAS_MINIMAS_ANTES_DE_COMERCIO`.
    if (!(err instanceof ConstruccionManualInvalidaError)) throw err;
    return { asentamiento, aseguradas: false };
  }
}

/**
 * Mercado + al menos una caravana comercial propia para CUALQUIER asentamiento con Gobernador (requisito de
 * `anadirEdificioManualmente`) — necesario para que un trueque (de supervivencia o de especialización)
 * pueda de verdad ENTREGAR algo, no solo pactarlo (Doc 3.2: "si no hay ninguna [caravana] disponible, el
 * envío simplemente espera", ver `asignarCaravanasATrueque` en `engine/trade.ts`). Mismo patrón que
 * `asegurarInfraestructuraComercial` (privada dentro de `engine/simulacionAutoComercio.ts`), pero sin
 * esperar a que la Facción tenga 2+ asentamientos propios — ese módulo del motor solo corre para Facciones
 * ya establecidas, y este escenario parte de Facciones de 1. Respeta la reserva de madera
 * (`RESERVA_MADERA_ANTES_DE_RECLUTAR`, vía `puedeIniciarConstruccion` en el motor) igual que cualquier otra
 * construcción manual — el Mercado (100 madera) solo se construye una vez hay margen de sobra sobre esa
 * reserva.
 */
function asegurarInfraestructuraComercial(
  asentamiento: Asentamiento,
  faccion: Faccion,
  caravanas: Caravana[],
  zonaPoligono: ReturnType<typeof computeTodasLasZonas>[number]['poligono'],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReturnType<typeof reclamosDeFuentes>,
  tickActual: number,
  contador: number
): { asentamiento: Asentamiento; caravanaNueva?: Caravana } {
  if (!asentamiento.cargos.gobernadorId) return { asentamiento };

  if (!tieneMercadoActivo(asentamiento)) {
    try {
      const actual = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'mercado', zonaPoligono, mapa, capital, reclamos, contador);
      return { asentamiento: actual };
    } catch (err) {
      if (!(err instanceof ConstruccionManualInvalidaError)) throw err;
      return { asentamiento };
    }
  }

  const propias = caravanas.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === asentamiento.id).length;
  if (propias < cupoCaravanas(asentamiento)) {
    try {
      const resultado = construirCaravanaComercial(asentamiento, caravanas, tickActual, contador);
      return { asentamiento: resultado.asentamiento, caravanaNueva: resultado.caravana };
    } catch (err) {
      if (!(err instanceof CaravanaInvalidaError)) throw err;
    }
  }

  return { asentamiento };
}

/** Devuelve `true` si `tipo` ya está activo, en construcción o en cola — evita reintentar
 * `anadirEdificioManualmente` en cada tick sobre un edificio que ya se está gestionando. */
function tieneOEnCurso(asentamiento: Asentamiento, tipo: EdificioTipo): boolean {
  return edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo);
}

/**
 * Barracón + Galería de tiro a mano para CUALQUIER asentamiento con Gobernador (issue
 * `issues/nivel_3_inalcanzable_sin_jugador_humano.md`): la política de auto-construcción que antes los
 * gateaba se retiró sin sustituto (`avanzarConstruccion`, Paso 3: "Barracón/Galería de tiro/Palacio/Mercado
 * ya NO se auto-detectan aquí"), y el gate de nivel 3 (`NIVEL_ASENTAMIENTO.requisitos[3]`) exige los 5
 * edificios de transformación+militar A LA VEZ, sin margen (`edificiosMinimo`) — sin esto, ningún
 * asentamiento sin jugador humano pulsando el botón podía llegar nunca a nivel 3. Mismo patrón que
 * `asegurarInfraestructuraComercial`: un edificio por llamada, respetando `requisitoNivelAsentamientoConstruccion`
 * (nivel 2 para ambos) y la reserva de mantenimiento vía `anadirEdificioManualmente` — si el asentamiento
 * todavía no llega a nivel 2, o no hay sitio o fondos, el intento simplemente no hace nada este tick.
 */
function asegurarNucleoMilitar(
  asentamiento: Asentamiento,
  faccion: Faccion,
  zonaPoligono: ReturnType<typeof computeTodasLasZonas>[number]['poligono'],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReturnType<typeof reclamosDeFuentes>,
  contador: number
): Asentamiento {
  if (!asentamiento.cargos.gobernadorId) return asentamiento;

  const faltante: EdificioTipo | undefined = !tieneOEnCurso(asentamiento, 'barracon')
    ? 'barracon'
    : !tieneOEnCurso(asentamiento, 'galeriaDeTiro')
      ? 'galeriaDeTiro'
      : undefined;
  if (!faltante) return asentamiento;

  try {
    return anadirEdificioManualmente(asentamiento, faccion, 'gobernador', faltante, zonaPoligono, mapa, capital, reclamos, contador);
  } catch (err) {
    if (!(err instanceof ConstruccionManualInvalidaError)) throw err;
    return asentamiento;
  }
}

function fraccionDisponible(asentamiento: Asentamiento, recurso: RecursoTipo): number {
  const item = asentamiento.almacen[recurso];
  if (!item || item.capacidad <= 0) return 0;
  return item.cantidad / item.capacidad;
}

/** Recursos de Mantenimiento en riesgo para este asentamiento: el stock actual no llega a cubrir
 * `TICKS_ANTICIPACION_SUPERVIVENCIA` ticks del costo que Mantenimiento le va a cobrar (`calcularCostoMantenimiento`,
 * que ya decide solo qué recursos aparecen según el nivel — madera siempre, +piedra/+oro desde nivel 2/3). */
function recursosMantenimientoEnRiesgo(asentamiento: Asentamiento, capital: Asentamiento | undefined): RecursoTipo[] {
  const costo = calcularCostoMantenimiento(asentamiento, capital);
  const riesgo: RecursoTipo[] = [];
  for (const recurso of RECURSOS_MANTENIMIENTO) {
    const necesario = (costo[recurso] ?? 0) * TICKS_ANTICIPACION_SUPERVIVENCIA;
    if (necesario <= 0) continue;
    if ((asentamiento.almacen[recurso]?.cantidad ?? 0) < necesario) riesgo.push(recurso);
  }
  return riesgo;
}

/** Mejor recurso para PAGAR un trueque de supervivencia: el de mayor excedente entre los que no son ni el
 * recurso que se está pidiendo ni uno que este mismo asentamiento tiene también en riesgo (no vender lo que
 * a uno mismo le falta) — oro solo se ofrece si ningún otro recurso califica ("como último recurso"). */
function mejorRecursoDePagoSupervivencia(asentamiento: Asentamiento, recursoBuscado: RecursoTipo, enRiesgo: RecursoTipo[]): RecursoTipo | undefined {
  let mejor: RecursoTipo | undefined;
  let mejorFraccion = COLCHON_EXCEDENTE_SUPERVIVENCIA;
  for (const recurso of RECURSOS_MONEDA_SUPERVIVENCIA) {
    if (recurso === recursoBuscado || enRiesgo.includes(recurso)) continue;
    const fraccion = fraccionDisponible(asentamiento, recurso);
    if (fraccion > mejorFraccion) {
      mejorFraccion = fraccion;
      mejor = recurso;
    }
  }
  if (mejor) return mejor;
  if (recursoBuscado !== 'oro' && !enRiesgo.includes('oro') && fraccionDisponible(asentamiento, 'oro') > COLCHON_EXCEDENTE_SUPERVIVENCIA) {
    return 'oro';
  }
  return undefined;
}

/**
 * `necesitado` YA tiene ayuda en camino para `recurso`: cualquier acuerdo 'activo' donde lo esté RECIBIENDO
 * (sin importar de qué socio) — a diferencia de comprobar solo el PAR específico, esto evita que un
 * asentamiento acumule pactos redundantes con varios socios sucesivos para la misma necesidad mientras el
 * primero sigue pendiente de entrega (gap real encontrado corriendo el batch — ver
 * Consideraciones/Diarios_Simulaciones_Batch/Diario_Simulaciones_Batch_100_Facciones_TruequeSupervivencia.md,
 * Hallazgo #4: 7723 propuestas, solo 137 cumplidas).
 */
function yaTieneAyudaEnCaminoPara(acuerdos: AcuerdoTrueque[], necesitadoId: string, recurso: RecursoTipo): boolean {
  return acuerdos.some(
    (ac) =>
      ac.estado === 'activo' &&
      ((ac.asentamientoAId === necesitadoId && ac.recursoB === recurso) || (ac.asentamientoBId === necesitadoId && ac.recursoA === recurso))
  );
}

/**
 * Trueque de SUPERVIVENCIA: para cada asentamiento con algún recurso de Mantenimiento en riesgo
 * (`recursosMantenimientoEnRiesgo`), busca CUALQUIER otro asentamiento — de CUALQUIER Facción, no solo la
 * propia — con excedente real de ese recurso (mismo colchón de seguridad que se le exige a quien pide) y le
 * propone un trueque (`proponerTrueque`, motor sin tocar — ya soporta pares de Facciones distintas, ver
 * `tasaComision` en `engine/trade.ts`), pagando con el mejor recurso propio de sobra
 * (`mejorRecursoDePagoSupervivencia`; oro como último recurso). No propone uno nuevo si el necesitado YA
 * tiene ayuda activa en camino para ese recurso, sin importar el socio (`yaTieneAyudaEnCaminoPara`).
 */
function truequeDeSupervivencia(
  asentamientos: Asentamiento[],
  capitalesPorFaccion: Map<string, Asentamiento | undefined>,
  acuerdosExistentes: AcuerdoTrueque[],
  tickActual: number,
  contadorInicial: number,
  esNpc: (faccionId: string) => boolean
): { acuerdosNuevos: AcuerdoTrueque[]; eventos: string[]; contador: number; propuestos: number } {
  const acuerdosNuevos: AcuerdoTrueque[] = [];
  const eventos: string[] = [];
  let contador = contadorInicial;
  let propuestos = 0;

  for (const necesitado of asentamientos) {
    if (!esNpc(necesitado.faccionId)) continue;
    const capital = capitalesPorFaccion.get(necesitado.faccionId);
    const enRiesgo = recursosMantenimientoEnRiesgo(necesitado, capital);
    if (enRiesgo.length === 0) continue;

    for (const recurso of enRiesgo) {
      if (yaTieneAyudaEnCaminoPara([...acuerdosExistentes, ...acuerdosNuevos], necesitado.id, recurso)) continue;

      // El SOCIO también tiene que ser NPC: `proponerTrueque` (motor) pacta sin pedir consentimiento al otro
      // lado, así que sin este filtro un NPC comprometería recursos de un asentamiento del jugador humano sin
      // que este lo aprobara (a petición del usuario). En batch, donde no hay humano, `esNpc` es siempre true
      // y el comportamiento es el de siempre: cualquier socio de cualquier Facción.
      const socio = asentamientos.find(
        (s) => s.id !== necesitado.id && esNpc(s.faccionId) && fraccionDisponible(s, recurso) > COLCHON_EXCEDENTE_SUPERVIVENCIA
      );
      if (!socio) continue;

      const pago = mejorRecursoDePagoSupervivencia(necesitado, recurso, enRiesgo);
      if (!pago) continue;

      try {
        const acuerdo = proponerTrueque(
          asentamientos,
          necesitado.id,
          socio.id,
          pago,
          recurso,
          CANTIDAD_TRUEQUE_SUPERVIVENCIA,
          CANTIDAD_TRUEQUE_SUPERVIVENCIA,
          tickActual,
          contador++
        );
        acuerdosNuevos.push(acuerdo);
        propuestos++;
        eventos.push(
          `${necesitado.id} propone trueque de supervivencia: ${CANTIDAD_TRUEQUE_SUPERVIVENCIA} ${pago} por ${CANTIDAD_TRUEQUE_SUPERVIVENCIA} ${recurso} con ${socio.id} (Facción ${socio.faccionId}).`
        );
      } catch (err) {
        if (!(err instanceof TruequeInvalidoError)) throw err;
      }
    }
  }

  return { acuerdosNuevos, eventos, contador, propuestos };
}

/**
 * Reclutamiento (punto 7a): cada residente intenta reclutar/reponer su propio escuadrón de la tropa
 * indicada — por defecto `milicia_lanceros` (`centroUrbano`, ya activo desde la fundación, sin depender de
 * construir Barracón, ver `TROPAS_RECLUTABLES` en `src/constants.ts`). Silencioso si un residente no puede
 * pagar/no tiene pesants disponibles todavía (`ReclutamientoInvalidoError`) — se reintenta en el próximo tick.
 *
 * Un solo gate de prudencia propio del NPC: no se intenta NINGÚN reclutamiento este tick si el almacén de
 * madera no llega a `RESERVA_MADERA_ANTES_DE_RECLUTAR`, para no competir con lo que Mantenimiento necesita.
 *
 * YA NO limita cuántos residentes reclutan a la vez por nivel (antes: 1 en nivel 1, los 5 desde nivel 2 —
 * retirado a petición del usuario). Ese límite era un parche artificial contra un síntoma; el motor ahora
 * tiene la regla real: `reclutarTropa` exige reserva de trigo proyectada antes de reclutar
 * (`engine/tropas.ts`), así que si los 5 residentes intentan reclutar el mismo tick, los primeros agotan el
 * margen y el resto falla limpio (`ReclutamientoInvalidoError`, atrapado abajo) — sin necesidad de un tope
 * artificial por nivel, y protegiendo igual al reclutamiento manual de un jugador humano, que este throttle
 * nunca alcanzaba. Ver `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto".
 */
function reclutarParaTodos(
  asentamiento: Asentamiento,
  tickActual: number,
  contadorInicial: number,
  tropaId: string,
  origen: 'pesants' | 'artesanos'
): { asentamiento: Asentamiento; reclutamientosExitosos: number; contador: number } {
  if ((asentamiento.almacen['madera']?.cantidad ?? 0) < RESERVA_MADERA_ANTES_DE_RECLUTAR) {
    return { asentamiento, reclutamientosExitosos: 0, contador: contadorInicial };
  }

  const residentes = residentesDe(asentamiento);

  let actual = asentamiento;
  let contador = contadorInicial;
  let exitosos = 0;
  for (const jugadorId of residentes) {
    try {
      actual = reclutarTropa(actual, jugadorId, tropaId, origen, tickActual, contador++);
      exitosos++;
    } catch (err) {
      if (!(err instanceof ReclutamientoInvalidoError)) throw err;
    }
  }
  return { asentamiento: actual, reclutamientosExitosos: exitosos, contador };
}

/**
 * Ataque a campamentos de bandidos (punto 7b): si el asentamiento asignado a un campamento sigue vivo, tiene
 * AL MENOS UN escuadrón (de cualquier residente), su nutrición no está por debajo de
 * `UMBRAL_NUTRICION_ANTES_DE_ATACAR` y sus escuadrones no están ya heridos por debajo de
 * `SALUD_ESCUADRONES_ANTES_DE_ATACAR`, ataca con TODOS sus escuadrones juntos a la vez — a propósito, no uno a
 * la vez: la XP de Facción por combate se multiplica por `jugadoresParticipantes` (jugadores DISTINTOS entre
 * los escuadrones atacantes, ver `engine/combate.ts`), así que juntar los escuadrones de varios residentes en
 * un solo ataque maximiza la XP obtenida por evento en vez de repartirla en varios ataques de 1 jugador cada
 * uno. El spawn/respawn de campamentos ya lo resuelve solo `avanzarSpawnBandidos` dentro de
 * `avanzarSimulacion` — este NPC solo decide atacar los que YA existen.
 *
 * El gate de nutrición es NUEVO (a petición del usuario, tras confirmar en batch que el combate sin ningún
 * freno era la causa dominante de que ninguna Facción llegara nunca a una segunda Granja —
 * `issues/granjas_no_escalan_con_poblacion.md`, y que seguía causando colapso masivo incluso después de
 * frenar el reclutamiento — `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto"): un
 * asentamiento que ya está mal alimentado no arriesga MÁS tropas que no podría reponer (la reserva de trigo
 * de `reclutarTropa` se lo impediría), así que atacar en ese estado solo cava más hondo. Un asentamiento sano
 * sigue atacando exactamente igual que siempre.
 */
function atacarCampamentosCercanos(
  asentamientos: Asentamiento[],
  campamentos: CampamentoBandido[],
  facciones: Faccion[],
  tickActual: number,
  esNpc: (faccionId: string) => boolean,
  rng: RandomFn
): {
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  campamentos: CampamentoBandido[];
  bandidosProximoSpawnTick: number | undefined;
  eventos: string[];
  destruidos: number;
  fallidos: number;
} {
  let asentamientosActuales = asentamientos;
  let faccionesActuales = facciones;
  let campamentosActuales = campamentos;
  const eventos: string[] = [];
  let destruidos = 0;
  let fallidos = 0;
  let bandidosProximoSpawnTick: number | undefined;

  for (const campamento of campamentos) {
    const asentamiento = asentamientosActuales.find((a) => a.id === campamento.asentamientoId);
    if (
      !asentamiento ||
      !esNpc(asentamiento.faccionId) ||
      asentamiento.escuadrones.length === 0 ||
      nutricionPoblacionDe(asentamiento) < UMBRAL_NUTRICION_ANTES_DE_ATACAR ||
      saludEscuadrones(asentamiento) < SALUD_ESCUADRONES_ANTES_DE_ATACAR
    )
      continue;

    try {
      const resultado = atacarCampamentoBandidos(
        asentamiento,
        asentamiento.escuadrones.map((e) => e.id),
        campamento,
        tickActual,
        faccionesActuales,
        rng
      );
      asentamientosActuales = asentamientosActuales.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
      faccionesActuales = resultado.facciones;
      // `engine/combate.ts` ya emite eventos estructurados, pero el NPC lleva su propio flujo en texto plano:
      // migrarlo es una pasada aparte (este módulo NO es un comando, es el NPC jugando como jugaría alguien),
      // así que aquí se toma solo el mensaje. Sus eventos salen como `codigo: 'npc.accion'`, ver
      // `comandos/avanzarFaccionesNpc.ts`.
      eventos.push(...resultado.eventos.map((e) => (typeof e === 'string' ? e : e.mensaje)));
      if (resultado.campamentoDestruido) {
        campamentosActuales = campamentosActuales.filter((c) => c.id !== campamento.id);
        bandidosProximoSpawnTick = tickActual + CAMPAMENTOS_BANDIDOS.ticksRespawn;
        destruidos++;
      } else {
        fallidos++;
      }
    } catch (err) {
      if (!(err instanceof CombateInvalidoError)) throw err;
    }
  }

  return { asentamientos: asentamientosActuales, facciones: faccionesActuales, campamentos: campamentosActuales, bandidosProximoSpawnTick, eventos, destruidos, fallidos };
}

/**
 * Expansión (punto 1): lanza una Caravana de Fundación (`lanzarCaravanaFundacion`, `engine/expansion.ts`)
 * desde cualquier asentamiento que ya cumpla los dos requisitos reales del motor — `nivelActualDe >= 2` y
 * recursos suficientes para `costoCaravanaFundacion()` — hacia el destino que decida `buscarDestino` (el
 * escenario decide CÓMO elegir un punto libre; este NPC solo decide CUÁNDO lanzar, para no acoplar la
 * estrategia de posicionamiento a este archivo reutilizable). Si `buscarDestino` no encuentra sitio, o el
 * motor rechaza el lanzamiento (`ExpansionInvalidaError` — cap de Facción alcanzado, etc.), no hace nada
 * este tick y se reintenta en el siguiente.
 */
function expandirSiPuede(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  caravanas: Caravana[],
  mapa: Mapa,
  tickActual: number,
  contadorInicial: number,
  buscarDestino: (origen: Asentamiento, mapa: Mapa, asentamientos: Asentamiento[]) => Point | undefined,
  jugadoresPorCaravana: number,
  esNpc: (faccionId: string) => boolean
): { asentamientos: Asentamiento[]; caravanas: Caravana[]; lanzadas: number; contador: number } {
  let asentamientosActuales = asentamientos;
  let caravanasActuales = caravanas;
  let contador = contadorInicial;
  let lanzadas = 0;
  const costo = costoCaravanaFundacion();

  for (const asentamiento of asentamientos) {
    if (!esNpc(asentamiento.faccionId)) continue;
    if (nivelActualDe(asentamiento) < 2) continue;
    if (!tieneRecursos(asentamiento.almacen, costo)) continue;
    const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
    if (!faccion) continue;
    const destino = buscarDestino(asentamiento, mapa, asentamientosActuales);
    if (!destino) continue;

    try {
      const resultado = lanzarCaravanaFundacion(
        mapa,
        asentamiento,
        faccion,
        destino,
        asentamientosActuales,
        caravanasActuales,
        jugadoresPorCaravana,
        tickActual,
        contador++
      );
      asentamientosActuales = asentamientosActuales.map((a) => (a.id === resultado.origenActualizado.id ? resultado.origenActualizado : a));
      caravanasActuales = [...caravanasActuales, resultado.caravana];
      lanzadas++;
    } catch (err) {
      if (!(err instanceof ExpansionInvalidaError)) throw err;
    }
  }

  return { asentamientos: asentamientosActuales, caravanas: caravanasActuales, lanzadas, contador };
}

export interface ConfigNpcGobernanza {
  /**
   * Facciones que este NPC gobierna. En la partida real son las que el jugador marcó como "controlada por
   * NPC" en la pestaña Facción (ver `GameState.faccionesNpcIds`) — TODO lo que decide este archivo queda
   * acotado a ellas: cargos, reservas, construcción, trueques (también del lado del socio), reclutamiento,
   * ataques y expansión. Omitirlo = gobernar el mundo entero, que es lo que quieren los scripts de batch.
   */
  faccionesIds?: string[];
  /** Tropa a reclutar por defecto — 'milicia_lanceros' (edificio centroUrbano, sin depender de Barracón). */
  tropaId?: string;
  origenReclutamiento?: 'pesants' | 'artesanos';
  /** Jugadores que viajan en cada Caravana de Fundación lanzada (tope real: FUNDACION.maxJugadoresFundacionGrupal). */
  jugadoresPorCaravanaFundacion?: number;
  /** Jugadores fundadores del PRIMER asentamiento de una Facción NPC sin ninguno todavía (ver
   * `fundarAsentamientosIniciales`). Por defecto 5 — mismo tope que la fundación manual. */
  jugadoresPorFundacionInicial?: number;
  /** Posición del primer asentamiento de una Facción NPC sin ninguno. Por defecto,
   * `buscarPosicionFundacionInicialPorDefecto` (barrido en rejilla). Solo se usa cuando `faccionesIds` está
   * presente — ver `fundarAsentamientosIniciales`. */
  buscarPosicionFundacionInicial?: (mapa: Mapa, asentamientos: Asentamiento[]) => Point | undefined;
  /**
   * Primer número de secuencia para los ids que genera el motor este tick (`caravana-…-<tick>-<contador>`,
   * `trueque-…`, `escuadron-…`). En batch da igual arrancar de 0 cada tick porque nadie más crea entidades;
   * en la partida real el jugador humano también las crea a través de `GameStore` (que lleva su propio
   * `GeneradorIds`, ver `session/idGenerator.ts`), y dos acciones del mismo tick sobre el mismo asentamiento
   * podrían chocar de id. El store pasa aquí su contador y luego lo adelanta con `contadorFinal` del resultado.
   */
  contadorInicial?: number;
  /** Punto 7b: ataca campamentos de bandidos cercanos con todos los escuadrones disponibles. Por defecto
   * `true` (comportamiento de siempre, sin cambios). `false` es una palanca de EXPERIMENTO para aislar cuánto
   * del reclutamiento continuo del NPC lo sostiene reponer bajas de combate frente a deserción por hambre —
   * ver `issues/granjas_no_escalan_con_poblacion.md`. No pensada para uso en partida real. */
  atacarCampamentos?: boolean;
  /** Decide el punto de destino de una Caravana de Fundación para un asentamiento dado. Por defecto,
   * `buscarDestinoFundacionPorDefecto` (barrido radial). Un escenario de batch puede sustituirlo por su
   * propia estrategia de posicionamiento. `undefined` como retorno = no expandir este tick. */
  buscarDestinoFundacion?: (origen: Asentamiento, mapa: Mapa, asentamientos: Asentamiento[]) => Point | undefined;
}

/**
 * Destino por defecto de una Caravana de Fundación: barrido radial alrededor del origen (anillos de 150 en
 * 150 hasta 600, un punto cada 20°), quedándose con el primer punto que el MOTOR considere viable
 * (`evaluarViabilidadFundacion`, `engine/settlement.ts` — solo lectura, decide él, no este archivo). Mismo
 * criterio que usaba `scripts/run-batch-sim.ts`, movido aquí para que la partida real no tenga que traer su
 * propia heurística de posicionamiento.
 */
export function buscarDestinoFundacionPorDefecto(origen: Asentamiento, mapa: Mapa, asentamientos: Asentamiento[]): Point | undefined {
  for (let radio = 150; radio <= 600; radio += 150) {
    for (let angulo = 0; angulo < 360; angulo += 20) {
      const rad = (angulo * Math.PI) / 180;
      const posicion = { x: origen.posicion.x + Math.cos(rad) * radio, y: origen.posicion.y + Math.sin(rad) * radio };
      if (posicion.x < 0 || posicion.y < 0 || posicion.x >= mapa.limites.ancho || posicion.y >= mapa.limites.alto) continue;
      if (evaluarViabilidadFundacion(mapa, posicion, asentamientos).recomendable) return posicion;
    }
  }
  return undefined;
}

/** Minerales que sirven de bonus (no obligatorios) al elegir dónde fundar el primer asentamiento de una
 * Facción NPC (ver `buscarPosicionFundacionInicialPorDefecto`) — cada uno presente en el radio inicial suma
 * un punto al candidato. Mismo conjunto que usa `scripts/run-batch-sim.ts` para su propia heurística de
 * posicionamiento (`OTROS_MINERALES`), menos duplicado que inventar otra lista aquí. */
// Exportada (a diferencia del resto de constantes internas de este archivo) para que `scripts/run-batch-sim.ts`
// reutilice la MISMA lista en su propia selección de posiciones de fundación inicial, en vez de mantener una
// paralela — la divergencia entre ambas (piedra como filtro blando vs. duro, paso de barrido 100 vs. 25) fue
// justo lo que causó que el batch fundara el 76% de sus asentamientos sin piedra alcanzable, bloqueando el
// gate de nivel 2 para casi todos (ver `issues/nivel_3_inalcanzable_sin_jugador_humano.md` y el diagnóstico
// de `granjas_no_escalan_con_poblacion.md`, que arrancó investigando por qué el batch nunca alcanzaba nivel 2).
export const MINERALES_BONUS_FUNDACION: RecursoTipo[] = ['cobre', 'estano', 'oro', 'livestock'];

/**
 * Distancia entre puntos muestreados al buscar dónde fundar (ver `buscarPosicionFundacionInicialPorDefecto`).
 * DELIBERADAMENTE por debajo de `ZONA_INFLUENCIA.radioInicial` (30, el radio real con el que se evalúa cada
 * candidato) — a petición del usuario, tras encontrar que un paso más grueso que el radio se salta clusters
 * de recursos enteros entre dos puntos consecutivos: ninguno de los dos cae lo bastante cerca para verlos,
 * aunque el mapa tenga de sobra un punto intermedio con madera + piedra + otro mineral a la vez.
 */
const PASO_BUSQUEDA_FUNDACION_INICIAL = 25;

/**
 * Posición para el PRIMER asentamiento de una Facción NPC (a diferencia de `buscarDestinoFundacionPorDefecto`,
 * que expande desde un origen ya existente, aquí no hay ninguno todavía): UN barrido en rejilla denso de todo
 * el mapa (ver `PASO_BUSQUEDA_FUNDACION_INICIAL`), evaluando cada punto con el motor
 * (`evaluarViabilidadFundacion` — solo lectura, decide él, no este archivo).
 *
 * Madera y piedra son OBLIGATORIAS: un punto sin madera alcanzable (`recomendable` ya lo exige) o sin ningún
 * nodo de piedra en el radio se descarta sin más — a petición del usuario, porque sin Cantera un asentamiento
 * nunca puede cumplir el gate de nivel 2 (Doc Fase_0_6: 3 de los 6 extractores, y Cantera es el único que da
 * piedra). Entre los que cumplen ambas, gana el que además tenga más minerales de `MINERALES_BONUS_FUNDACION`
 * en el radio — se recorre el mapa entero y se compara CADA candidato válido contra el mejor visto hasta el
 * momento, no solo los del primer grupo que aparezca, así que un candidato con 3 recursos a 400px del origen
 * del barrido gana a uno con 2 encontrado antes. Corta antes de terminar el barrido solo si ya encuentra un
 * candidato con el bonus máximo posible (los 4 minerales) — no hay ninguno mejor que buscar.
 *
 * Si ningún punto del mapa tiene madera Y piedra a la vez, no se funda este tick (se reintenta en el
 * siguiente, igual que el resto de pasos del NPC).
 */
export function buscarPosicionFundacionInicialPorDefecto(mapa: Mapa, asentamientos: Asentamiento[]): Point | undefined {
  let mejor: { posicion: Point; bonusMinerales: number } | undefined;

  for (let x = PASO_BUSQUEDA_FUNDACION_INICIAL; x < mapa.limites.ancho; x += PASO_BUSQUEDA_FUNDACION_INICIAL) {
    for (let y = PASO_BUSQUEDA_FUNDACION_INICIAL; y < mapa.limites.alto; y += PASO_BUSQUEDA_FUNDACION_INICIAL) {
      const posicion = { x, y };
      const viabilidad = evaluarViabilidadFundacion(mapa, posicion, asentamientos);
      if (!viabilidad.recomendable) continue;
      const tienePiedra = viabilidad.recursosEnRadio.some((r) => r.tipo === 'piedra' && r.nodos > 0);
      if (!tienePiedra) continue;

      const bonusMinerales = MINERALES_BONUS_FUNDACION.filter((tipo) =>
        viabilidad.recursosEnRadio.some((r) => r.tipo === tipo && r.nodos > 0)
      ).length;
      if (mejor && bonusMinerales <= mejor.bonusMinerales) continue;

      mejor = { posicion, bonusMinerales };
      if (bonusMinerales === MINERALES_BONUS_FUNDACION.length) return posicion;
    }
  }

  return mejor?.posicion;
}

/**
 * Fundación inicial (a petición del usuario, tras confirmar que una Facción NPC recién cedida sin ningún
 * asentamiento se queda inerte para siempre — `avanzarNpcGobernanza` solo sabe GOBERNAR asentamientos
 * existentes, nunca creaba el primero): para cada Facción de `faccionesIds` que todavía no tenga ninguno,
 * funda uno con `fundarAsentamiento` (motor, sin tocar) en el primer punto viable que encuentre
 * `buscarPosicion`. Silencioso si no encuentra sitio o el motor rechaza el punto — se reintenta el próximo
 * tick, igual que el resto de pasos del NPC.
 *
 * Deliberadamente NO se ejecuta en batch (`faccionesIds` ausente, ver `avanzarNpcGobernanza`): los escenarios
 * de batch fundan sus asentamientos iniciales ellos mismos, ANTES de arrancar el bucle de ticks, y ya deciden
 * su propia estrategia de posicionamiento (piedra/minerales, separación mínima) — auto-fundar aquí también
 * cambiaría el resultado de simulaciones ya corridas y documentadas en los diarios de batch.
 */
function fundarAsentamientosIniciales(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  faccionesIds: string[],
  mapa: Mapa,
  tickActual: number,
  jugadoresPorFundacion: number,
  buscarPosicion: (mapa: Mapa, asentamientos: Asentamiento[]) => Point | undefined
): { asentamientos: Asentamiento[]; facciones: Faccion[]; eventos: string[] } {
  let asentamientosActuales = asentamientos;
  let faccionesActuales = facciones;
  const eventos: string[] = [];

  for (const faccionId of faccionesIds) {
    if (asentamientosActuales.some((a) => a.faccionId === faccionId)) continue;
    const faccion = faccionesActuales.find((f) => f.id === faccionId);
    if (!faccion) continue;

    const posicion = buscarPosicion(mapa, asentamientosActuales);
    if (!posicion) continue;

    const jugadoresIds = Array.from({ length: jugadoresPorFundacion }, (_, i) => `npc-${faccionId}-${i + 1}`);
    try {
      const resultado = fundarAsentamiento(mapa, faccionesActuales, faccionId, posicion, jugadoresIds, asentamientosActuales, tickActual);
      asentamientosActuales = [...asentamientosActuales, resultado.asentamiento];
      faccionesActuales = resultado.facciones;
      eventos.push(`${faccion.nombre} funda su asentamiento inicial ${resultado.asentamiento.id}.`);
    } catch (err) {
      if (!(err instanceof FundacionInvalidaError)) throw err;
    }
  }

  return { asentamientos: asentamientosActuales, facciones: faccionesActuales, eventos };
}

/**
 * Un tick completo de decisiones del NPC de gobernanza: gobernanza+reserva base → **Granjas mínimas
 * (gate: sin esto, no se avanza a los dos pasos siguientes este tick)** → infraestructura comercial
 * (Mercado+caravana, para cualquier asentamiento) → núcleo militar (Barracón/Galería) → trueque de
 * SUPERVIVENCIA (cualquier Facción, prioriza lo que Mantenimiento necesita) → trueque de especialización
 * (delega en `avanzarAutoComercioSimulado`, sin reimplementarlo — solo dentro de la misma Facción) →
 * reclutamiento (con gate de reserva) → ataque a campamentos de bandidos → expansión. Llamar DESPUÉS de
 * `avanzarSimulacion` en el mismo tick.
 */
export function avanzarNpcGobernanza(
  estado: EstadoSimulacion,
  mapa: Mapa,
  contexto: ContextoSimulacion,
  config: ConfigNpcGobernanza
): ResultadoNpcGobernanza {
  // Mismo contexto que consume el motor (`avanzarSimulacion`): el NPC decide con las funciones PÚBLICAS del
  // motor, así que necesita exactamente las mismas entradas externas — tick, momento y aleatoriedad.
  const { tick: tickActual, rng } = contexto;
  const eventos: string[] = [];
  let contador = config.contadorInicial ?? 0;

  // Único punto donde se decide "¿esta Facción la juego yo?". Sin `faccionesIds` (batch) responde siempre que
  // sí y el NPC se comporta exactamente como antes de existir este filtro.
  const esNpc = config.faccionesIds ? (faccionId: string) => config.faccionesIds!.includes(faccionId) : () => true;

  // Paso 0: fundar el primer asentamiento de cualquier Facción cedida que todavía no tenga ninguno — sin esto
  // una Facción recién marcada como NPC se queda inerte para siempre (nada más de este archivo sabe crear un
  // asentamiento, solo gobernar uno existente). Gateado a `faccionesIds` a propósito (ver
  // `fundarAsentamientosIniciales`): el batch sigue fundando sus asentamientos iniciales fuera de este archivo.
  let facciones = estado.facciones;
  let eventosIniciales: string[] = [];
  let asentamientosBase = estado.asentamientos;
  if (config.faccionesIds) {
    const inicial = fundarAsentamientosIniciales(
      asentamientosBase,
      facciones,
      config.faccionesIds,
      mapa,
      tickActual,
      config.jugadoresPorFundacionInicial ?? 5,
      config.buscarPosicionFundacionInicial ?? buscarPosicionFundacionInicialPorDefecto
    );
    asentamientosBase = inicial.asentamientos;
    facciones = inicial.facciones;
    eventosIniciales = inicial.eventos;
  }
  eventos.push(...eventosIniciales);

  let asentamientos = asentamientosBase.map((a) => (esNpc(a.faccionId) ? asegurarGobernanzaBase(a, facciones) : a));
  let caravanas = [...estado.caravanas];

  const zonas = computeTodasLasZonas(asentamientos);
  const reclamos = reclamosDeFuentes(asentamientos);
  const capitalesPorFaccion = new Map(facciones.map((f) => [f.id, encontrarCapital(f.id, asentamientos)]));

  asentamientos = asentamientos.map((asentamiento) => {
    if (!esNpc(asentamiento.faccionId)) return asentamiento;
    const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
    if (!faccion) return asentamiento;
    const zonaPoligono = zonas.find((z) => z.asentamientoId === asentamiento.id)?.poligono ?? [];
    const capital = capitalesPorFaccion.get(faccion.id);

    // Seguridad alimentaria primero: mientras no estén aseguradas las `GRANJAS_MINIMAS_ANTES_DE_COMERCIO`, el
    // asentamiento no llega a infraestructura comercial ni a núcleo militar este tick — las dos son
    // construcción MANUAL discrecional, y comida va antes que comercio o guarnición.
    const trasGranjas = asegurarGranjasMinimas(asentamiento, faccion, zonaPoligono, mapa, capital, reclamos, contador++);
    if (!trasGranjas.aseguradas) return trasGranjas.asentamiento;

    const resultado = asegurarInfraestructuraComercial(
      trasGranjas.asentamiento,
      faccion,
      caravanas,
      zonaPoligono,
      mapa,
      capital,
      reclamos,
      tickActual,
      contador++
    );
    if (resultado.caravanaNueva) caravanas = [...caravanas, resultado.caravanaNueva];
    return asegurarNucleoMilitar(resultado.asentamiento, faccion, zonaPoligono, mapa, capital, reclamos, contador++);
  });

  const trueque = truequeDeSupervivencia(asentamientos, capitalesPorFaccion, estado.acuerdos, tickActual, contador, esNpc);
  contador = trueque.contador;
  eventos.push(...trueque.eventos);

  const estadoConGobernanzaBase: EstadoSimulacion = {
    ...estado,
    asentamientos,
    facciones,
    caravanas,
    acuerdos: [...estado.acuerdos, ...trueque.acuerdosNuevos],
  };
  // Trueque de especialización: se DELEGA en el motor (`avanzarAutoComercioSimulado`) sin tocarlo ni una
  // línea. Para acotarlo a las Facciones NPC se le pasa una VISTA del estado con `facciones` ya filtrado —
  // ese módulo recorre `estado.facciones` y saca de ahí los asentamientos "propios" de cada una, así que
  // filtrar la lista basta para que ignore por completo las Facciones del jugador humano.
  //
  // `asentamientos` va COMPLETO a propósito: el módulo calcula zonas de influencia y reclamos de yacimientos
  // sobre lo que reciba, y recortarlos le haría ver un mundo vacío alrededor — colocaría edificios sobre
  // suelo o fuentes que en realidad ya pertenecen a otro. `facciones` se restaura entera al salir, porque el
  // motor devuelve `{...estado}` con la vista recortada dentro.
  //
  // Este paso solo hace algo con `SIMULACION_AUTO_COMERCIO.activo = 1` (apagado por defecto, encendible en
  // caliente desde la pestaña "Valores de simulación"). Con el flag apagado el NPC conserva los otros 6 pasos.
  const faccionesNpc = facciones.filter((f) => esNpc(f.id));
  const trasComercioParcial = avanzarAutoComercioSimulado({ ...estadoConGobernanzaBase, facciones: faccionesNpc }, mapa, tickActual);
  const trasComercio: EstadoSimulacion = { ...trasComercioParcial, facciones };

  asentamientos = trasComercio.asentamientos;
  let reclutamientosExitosos = 0;
  const tropaId = config.tropaId ?? 'milicia_lanceros';
  const origenReclutamiento = config.origenReclutamiento ?? 'pesants';
  asentamientos = asentamientos.map((a) => {
    if (!esNpc(a.faccionId)) return a;
    const resultado = reclutarParaTodos(a, tickActual, contador, tropaId, origenReclutamiento);
    contador = resultado.contador;
    reclutamientosExitosos += resultado.reclutamientosExitosos;
    return resultado.asentamiento;
  });

  const trasBandidos =
    config.atacarCampamentos === false
      ? {
          asentamientos,
          facciones: trasComercio.facciones,
          campamentos: trasComercio.campamentosBandidos,
          bandidosProximoSpawnTick: undefined,
          eventos: [] as string[],
          destruidos: 0,
          fallidos: 0,
        }
      : atacarCampamentosCercanos(asentamientos, trasComercio.campamentosBandidos, trasComercio.facciones, tickActual, esNpc, rng);
  eventos.push(...trasBandidos.eventos);

  const trasExpansion = expandirSiPuede(
    trasBandidos.asentamientos,
    trasBandidos.facciones,
    trasComercio.caravanas,
    mapa,
    tickActual,
    contador,
    config.buscarDestinoFundacion ?? buscarDestinoFundacionPorDefecto,
    config.jugadoresPorCaravanaFundacion ?? 5,
    esNpc
  );

  return {
    estado: {
      ...trasComercio,
      asentamientos: trasExpansion.asentamientos,
      facciones: trasBandidos.facciones,
      caravanas: trasExpansion.caravanas,
      campamentosBandidos: trasBandidos.campamentos,
      bandidosProximoSpawnTick: trasBandidos.bandidosProximoSpawnTick ?? trasComercio.bandidosProximoSpawnTick,
    },
    eventos,
    stats: {
      reclutamientosExitosos,
      truequesSupervivenciaPropuestos: trueque.propuestos,
      campamentosDestruidos: trasBandidos.destruidos,
      campamentosAtacadosSinExito: trasBandidos.fallidos,
      caravanasFundacionLanzadas: trasExpansion.lanzadas,
    },
    contadorFinal: trasExpansion.contador,
  };
}
