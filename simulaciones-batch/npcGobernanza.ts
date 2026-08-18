// NPC-solo-para-simulación: comportamiento reutilizable entre escenarios de batch (Consideraciones/
// Diario_Simulaciones_Batch_*.md). Juega el rol de Gobernador/Tesorero/Rey que un jugador humano jugaría
// manualmente en el juego real (Doc 3.2/2.2) — el motor (`src/engine/*`) NUNCA lo llama solo, ni siquiera
// con `SIMULACION_AUTO_COMERCIO.activo = 1` (ese flag solo cubre la parte de comercio, ver
// `src/engine/simulacionAutoComercio.ts`). Este archivo NO se importa desde `src/` — vive fuera del motor a
// propósito, solo para scripts de prueba externos (ver `src/engine/__tests__/fixtures.ts` para el patrón
// equivalente usado en tests unitarios: siempre las funciones REALES del motor, nunca estado inventado a mano).
//
// Un escenario (un test file temporal, normalmente en `src/engine/__tests__/`, borrado tras generar su
// diario) construye el mundo/Facciones/asentamientos iniciales por su cuenta y, cada tick, llama:
//
//   estado = avanzarSimulacion(estado, mapa, tick);           // motor real, sin tocar
//   estado = avanzarNpcGobernanza(estado, mapa, tick, cfg).estado;
//
// `avanzarNpcGobernanza` no genera mundo, no funda nada inicial y no escribe archivos — solo decide, con las
// funciones PÚBLICAS del motor, qué haría un Gobernador/Tesorero/Rey NPC en ese tick.

import type { AcuerdoTrueque, Asentamiento, Caravana, CampamentoBandido, Faccion, Point, RecursoTipo } from '../src/domain/types';
import type { Mapa } from '../src/world/mapa';
import type { EstadoSimulacion } from '../src/engine/simulation';
import { avanzarAutoComercioSimulado } from '../src/engine/simulacionAutoComercio';
import { reclutarTropa, ReclutamientoInvalidoError } from '../src/engine/tropas';
import { atacarCampamentoBandidos, CombateInvalidoError } from '../src/engine/combate';
import { lanzarCaravanaFundacion, costoCaravanaFundacion, ExpansionInvalidaError } from '../src/engine/expansion';
import { nivelActualDe, tieneMercadoActivo, cupoCaravanas } from '../src/engine/asentamientoQuery';
import { tieneRecursos } from '../src/engine/almacen';
import { asignarCargoLocal, CargoInvalidoError } from '../src/engine/cargos';
import { anadirEdificioManualmente, reclamosDeFuentes, ConstruccionManualInvalidaError } from '../src/engine/construction';
import { construirCaravanaComercial, proponerTrueque, CaravanaInvalidaError, TruequeInvalidoError } from '../src/engine/trade';
import { computeTodasLasZonas } from '../src/engine/zones';
import { calcularCostoMantenimiento, encontrarCapital } from '../src/engine/mantenimiento';
import { CAMPAMENTOS_BANDIDOS } from '../src/constants';

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
  contadorInicial: number
): { acuerdosNuevos: AcuerdoTrueque[]; eventos: string[]; contador: number; propuestos: number } {
  const acuerdosNuevos: AcuerdoTrueque[] = [];
  const eventos: string[] = [];
  let contador = contadorInicial;
  let propuestos = 0;

  for (const necesitado of asentamientos) {
    const capital = capitalesPorFaccion.get(necesitado.faccionId);
    const enRiesgo = recursosMantenimientoEnRiesgo(necesitado, capital);
    if (enRiesgo.length === 0) continue;

    for (const recurso of enRiesgo) {
      if (yaTieneAyudaEnCaminoPara([...acuerdosExistentes, ...acuerdosNuevos], necesitado.id, recurso)) continue;

      const socio = asentamientos.find((s) => s.id !== necesitado.id && fraccionDisponible(s, recurso) > COLCHON_EXCEDENTE_SUPERVIVENCIA);
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
 * Dos gates de prudencia propios del NPC (no del motor — `reclutarTropa` no respeta ninguna reserva ni
 * limita cuántos residentes reclutan a la vez):
 * 1. No se intenta NINGÚN reclutamiento este tick si el almacén de madera no llega a
 *    `RESERVA_MADERA_ANTES_DE_RECLUTAR`, para no competir con lo que Mantenimiento necesita.
 * 2. Mientras el asentamiento sigue en nivel 1 (`nivelActualDe`), solo UN residente recluta por tick (no los
 *    5) — a petición del usuario, para no vaciar de golpe la madera recién protegida apenas se cruza el
 *    umbral: 5 residentes reclutando el mismo tick (250 madera de golpe, 25×2×5) drenarían la reserva casi
 *    tan rápido como el problema original diagnosticado. Desde nivel 2 en adelante, sin este límite.
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
  const candidatos = nivelActualDe(asentamiento) <= 1 ? residentes.slice(0, 1) : residentes;

  let actual = asentamiento;
  let contador = contadorInicial;
  let exitosos = 0;
  for (const jugadorId of candidatos) {
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
 * Ataque a campamentos de bandidos (punto 7b): si el asentamiento asignado a un campamento sigue vivo y
 * tiene AL MENOS UN escuadrón (de cualquier residente), ataca con TODOS sus escuadrones juntos a la vez —
 * a propósito, no uno a la vez: la XP de Facción por combate se multiplica por `jugadoresParticipantes`
 * (jugadores DISTINTOS entre los escuadrones atacantes, ver `engine/combate.ts`), así que juntar los
 * escuadrones de varios residentes en un solo ataque maximiza la XP obtenida por evento en vez de repartirla
 * en varios ataques de 1 jugador cada uno. El spawn/respawn de campamentos ya lo resuelve solo
 * `avanzarSpawnBandidos` dentro de `avanzarSimulacion` — este NPC solo decide atacar los que YA existen.
 */
function atacarCampamentosCercanos(
  asentamientos: Asentamiento[],
  campamentos: CampamentoBandido[],
  facciones: Faccion[],
  tickActual: number
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
    if (!asentamiento || asentamiento.escuadrones.length === 0) continue;

    try {
      const resultado = atacarCampamentoBandidos(
        asentamiento,
        asentamiento.escuadrones.map((e) => e.id),
        campamento,
        tickActual,
        faccionesActuales
      );
      asentamientosActuales = asentamientosActuales.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
      faccionesActuales = resultado.facciones;
      eventos.push(...resultado.eventos);
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
  jugadoresPorCaravana: number
): { asentamientos: Asentamiento[]; caravanas: Caravana[]; lanzadas: number; contador: number } {
  let asentamientosActuales = asentamientos;
  let caravanasActuales = caravanas;
  let contador = contadorInicial;
  let lanzadas = 0;
  const costo = costoCaravanaFundacion();

  for (const asentamiento of asentamientos) {
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
  /** Tropa a reclutar por defecto — 'milicia_lanceros' (edificio centroUrbano, sin depender de Barracón). */
  tropaId?: string;
  origenReclutamiento?: 'pesants' | 'artesanos';
  /** Jugadores que viajan en cada Caravana de Fundación lanzada (tope real: FUNDACION.maxJugadoresFundacionGrupal). */
  jugadoresPorCaravanaFundacion?: number;
  /** Decide el punto de destino de una Caravana de Fundación para un asentamiento dado — responsabilidad del
   * escenario (conoce el mapa/las posiciones ya ocupadas), no de este NPC. `undefined` = no expandir este tick. */
  buscarDestinoFundacion: (origen: Asentamiento, mapa: Mapa, asentamientos: Asentamiento[]) => Point | undefined;
}

/**
 * Un tick completo de decisiones del NPC de gobernanza: gobernanza+reserva base → infraestructura comercial
 * (Mercado+caravana, para cualquier asentamiento) → trueque de SUPERVIVENCIA (cualquier Facción, prioriza lo
 * que Mantenimiento necesita) → trueque de especialización (delega en `avanzarAutoComercioSimulado`, sin
 * reimplementarlo — solo dentro de la misma Facción) → reclutamiento (con gate de reserva) → ataque a
 * campamentos de bandidos → expansión. Llamar DESPUÉS de `avanzarSimulacion` en el mismo tick.
 */
export function avanzarNpcGobernanza(estado: EstadoSimulacion, mapa: Mapa, tickActual: number, config: ConfigNpcGobernanza): ResultadoNpcGobernanza {
  const eventos: string[] = [];
  let contador = 0;

  let asentamientos = estado.asentamientos.map((a) => asegurarGobernanzaBase(a, estado.facciones));
  let caravanas = [...estado.caravanas];

  const zonas = computeTodasLasZonas(asentamientos);
  const reclamos = reclamosDeFuentes(asentamientos);
  const capitalesPorFaccion = new Map(estado.facciones.map((f) => [f.id, encontrarCapital(f.id, asentamientos)]));

  asentamientos = asentamientos.map((asentamiento) => {
    const faccion = estado.facciones.find((f) => f.id === asentamiento.faccionId);
    if (!faccion) return asentamiento;
    const zonaPoligono = zonas.find((z) => z.asentamientoId === asentamiento.id)?.poligono ?? [];
    const resultado = asegurarInfraestructuraComercial(
      asentamiento,
      faccion,
      caravanas,
      zonaPoligono,
      mapa,
      capitalesPorFaccion.get(faccion.id),
      reclamos,
      tickActual,
      contador++
    );
    if (resultado.caravanaNueva) caravanas = [...caravanas, resultado.caravanaNueva];
    return resultado.asentamiento;
  });

  const trueque = truequeDeSupervivencia(asentamientos, capitalesPorFaccion, estado.acuerdos, tickActual, contador);
  contador = trueque.contador;
  eventos.push(...trueque.eventos);

  const estadoConGobernanzaBase: EstadoSimulacion = {
    ...estado,
    asentamientos,
    caravanas,
    acuerdos: [...estado.acuerdos, ...trueque.acuerdosNuevos],
  };
  const trasComercio = avanzarAutoComercioSimulado(estadoConGobernanzaBase, mapa, tickActual);

  asentamientos = trasComercio.asentamientos;
  let reclutamientosExitosos = 0;
  const tropaId = config.tropaId ?? 'milicia_lanceros';
  const origenReclutamiento = config.origenReclutamiento ?? 'pesants';
  asentamientos = asentamientos.map((a) => {
    const resultado = reclutarParaTodos(a, tickActual, contador, tropaId, origenReclutamiento);
    contador = resultado.contador;
    reclutamientosExitosos += resultado.reclutamientosExitosos;
    return resultado.asentamiento;
  });

  const trasBandidos = atacarCampamentosCercanos(asentamientos, trasComercio.campamentosBandidos, trasComercio.facciones, tickActual);
  eventos.push(...trasBandidos.eventos);

  const trasExpansion = expandirSiPuede(
    trasBandidos.asentamientos,
    trasBandidos.facciones,
    trasComercio.caravanas,
    mapa,
    tickActual,
    contador,
    config.buscarDestinoFundacion,
    config.jugadoresPorCaravanaFundacion ?? 5
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
  };
}
