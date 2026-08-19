import type { Asentamiento, EdificioTipo, RecursoTipo } from '../domain/types';
import { MANTENIMIENTO, NIVEL_ASENTAMIENTO, RESERVA_CONSTRUCCION } from '../constants';
import { edificiosPorTipoYEstado, nivelActualDe, poblacionTotal } from './asentamientoQuery';
import { descontarRecursos } from './almacen';
import { consumoComidaPoblacion } from './population';
import { consumoRacionTropas } from './tropas';

function distancia(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Nivel de asentamiento — rediseño de progreso (Fase 0, Doc 4.5): reemplaza la fórmula de puntos anterior por
 * un modelo de GATES (población + edificios activos requeridos, ver NIVEL_ASENTAMIENTO.requisitos). Sube de
 * forma MONÓTONA: nunca baja aunque la población caiga después, evalúa gate por gate desde el nivel actual.
 *
 * `edificiosMinimo` (Doc Fase_0_6, a petición del usuario): por defecto el gate exige TODOS los tipos de
 * `edificios` (mismo comportamiento que antes) — pero si se especifica, basta con tener construidos AL MENOS
 * esa cantidad de tipos DISTINTOS del conjunto (nunca instancias repetidas del mismo tipo: dos Canteras no
 * cuentan como 2). Lo usa el gate de nivel 2 ("≥3 de 6 edificios de extracción"); el de nivel 3 (transformación
 * + militar) no lo necesita porque ambos subconjuntos exigen "todos los suyos" — concatenar la lista completa
 * ya representa esa condición sin hacer falta un contador aparte.
 */
export function calcularNivelAsentamiento(asentamiento: Asentamiento): number {
  let nivel = asentamiento.nivel;
  while (nivel < NIVEL_ASENTAMIENTO.nivelMaximo) {
    const requisito = NIVEL_ASENTAMIENTO.requisitos[nivel + 1];
    if (!requisito) break;
    const cumplePoblacion = asentamiento.poblacion.pesants >= requisito.pesants && asentamiento.poblacion.artesanos >= requisito.artesanos;
    const tiposConstruidos = requisito.edificios.filter(
      (tipo) => edificiosPorTipoYEstado(asentamiento, tipo as EdificioTipo).length > 0
    ).length;
    const cumpleEdificios = tiposConstruidos >= (requisito.edificiosMinimo ?? requisito.edificios.length);
    if (!cumplePoblacion || !cumpleEdificios) break;
    nivel += 1;
  }
  return nivel;
}

/**
 * Sube `nivel` (nivelAlcanzado) mientras se cumplan los gates de `calcularNivelAsentamiento` Y (Doc
 * Fase_0_5 §5, a petición del usuario) haya CUPO libre en el nivel objetivo — un asentamiento puede cumplir
 * los gates de sobra y quedarse "elegible, esperando cupo" varios ticks, sin caer nunca de nivel por eso.
 * `tieneCupoParaNivel` es opcional (tests que no verifican cupo pueden omitirlo, tratado como "cupo
 * ilimitado") y, si se da, se llama UNA vez por cada escalón que se intenta subir — quien la implementa
 * decide si consume el cupo al devolver `true` (ver `simulation.ts`, que también libera el cupo del nivel
 * que se abandona al subir, p. ej. subir de 2 a 3 libera el cupo de 2).
 */
export function avanzarNivelAsentamiento(
  asentamiento: Asentamiento,
  tieneCupoParaNivel?: (nivelObjetivo: number) => boolean
): { asentamiento: Asentamiento; eventos: string[] } {
  const nivelElegible = calcularNivelAsentamiento(asentamiento);
  let nuevoNivel = asentamiento.nivel;
  while (nuevoNivel < nivelElegible && (!tieneCupoParaNivel || tieneCupoParaNivel(nuevoNivel + 1))) {
    nuevoNivel += 1;
  }
  if (nuevoNivel === asentamiento.nivel) return { asentamiento, eventos: [] };

  // Doc Fase_0_5 §6.2: una promoción de nivelAlcanzado legítima (gates + cupo cumplidos) sube `nivelActual`
  // junto con `nivel` de inmediato, SIN esperar racha — la racha de recuperación solo aplica tras una
  // degradación (`avanzarMantenimiento`). Excepción: si el asentamiento ya estaba degradado (nivelActual <
  // nivel antes de esta subida), la nueva promoción no "cura" esa degradación de golpe.
  const yaEstabaAlDia = nivelActualDe(asentamiento) === asentamiento.nivel;
  const siguiente = { ...asentamiento, nivel: nuevoNivel, ...(yaEstabaAlDia ? { nivelActual: nuevoNivel } : {}) };
  return { asentamiento: siguiente, eventos: [`${asentamiento.id} sube a nivel ${nuevoNivel}.`] };
}

/** "Centro de poder de la Facción" (Doc 4.5): placeholder = su asentamiento vivo más antiguo (proxy de capital). */
export function encontrarCapital(faccionId: string, asentamientos: Asentamiento[]): Asentamiento | undefined {
  return asentamientos
    .filter((a) => a.faccionId === faccionId)
    .sort((a, b) => a.fundadoEnTick - b.fundadoEnTick)[0];
}

/**
 * Coste de mantenimiento (Doc 4.5, rediseño Doc Fase_0_5 §3.2): escala por POBLACIÓN real (los materiales se
 * SUMAN, no se reemplazan) y por distancia a la capital (mecanismo anti-snowball ya existente: más lejos =
 * más caro — sigue siendo el eje de "cohesión", ver Fase_0_5_Definicion...md §5.1). Ya NO escala por nivel
 * directamente — un `nivelActual` degradado no reduce este coste: la misma gente sigue comiendo/gastando lo
 * mismo con o sin nivel (Doc §6.2). El NIVEL (nivelAlcanzado, `asentamiento.nivel`) sigue decidiendo QUÉ
 * recursos se cobran (`nivelParaPiedra`/`nivelParaOro`, abajo) — eso no baja nunca porque `nivel` no baja.
 *
 * El trigo NO forma parte de este coste (fix: antes había una "mecánica repetida" — Mantenimiento cobraba un
 * valor fijo de trigo ADEMÁS del que ya se descontaba por separado en `avanzarNutricionPoblacion`/`avanzarMantenimientoTropas`,
 * duplicando el gasto). El consumo real de trigo (población + tropas) se sigue descontando únicamente en esas
 * dos funciones; para mostrarlo en el panel de Mantenimiento, ver `gameStore.mantenimientoInfo`.
 */
export function calcularCostoMantenimiento(asentamiento: Asentamiento, capital: Asentamiento | undefined): Partial<Record<string, number>> {
  const nivel = asentamiento.nivel;
  const factorPoblacion = 1 + poblacionTotal(asentamiento) / MANTENIMIENTO.poblacionReferencia;
  const dist = capital ? distancia(asentamiento.posicion, capital.posicion) : 0;
  const factorDistancia = 1 + Math.min(1, dist / MANTENIMIENTO.escalaDistancia) * (MANTENIMIENTO.factorDistanciaMax - 1);
  const escala = factorPoblacion * factorDistancia;

  const costo: Partial<Record<string, number>> = {
    madera: MANTENIMIENTO.costoBase.madera * escala,
  };
  if (nivel >= MANTENIMIENTO.nivelParaPiedra) costo.piedra = MANTENIMIENTO.piedraBase * escala;
  if (nivel >= MANTENIMIENTO.nivelParaOro) costo.oro = MANTENIMIENTO.oroBase * escala;
  return costo;
}

/**
 * Qué recursos cobra Mantenimiento a este nivel de asentamiento — igual criterio que `calcularCostoMantenimiento`
 * pero sin necesitar `capital`/distancia (esos solo afectan el MONTO, no qué recursos aparecen). Lo usa
 * `engine/construction.ts` para saber qué recursos debe respetar la reserva mínima de construcción
 * (`RESERVA_CONSTRUCCION`) en un momento dado — "los recursos que consuma el asentamiento en ese momento".
 * Trigo no aparece aquí (Mantenimiento ya no lo cobra directamente, ver `calcularCostoMantenimiento`).
 */
export function recursosProtegidosPorMantenimiento(nivel: number): RecursoTipo[] {
  const recursos: RecursoTipo[] = ['madera'];
  if (nivel >= MANTENIMIENTO.nivelParaPiedra) recursos.push('piedra');
  if (nivel >= MANTENIMIENTO.nivelParaOro) recursos.push('oro');
  return recursos;
}

/**
 * Reserva mínima que la auto-construcción no puede tocar al comprometer (pagar) un proyecto nuevo — overhaul
 * de auto-construcción: reemplaza los umbrales fijos anteriores (`RESERVA_CONSTRUCCION` ya no lleva cifras
 * por recurso) por una proyección real de cuánto va a cobrar Mantenimiento + consumo de comida en los
 * próximos `horizonteTicks*` ticks (ver `RESERVA_CONSTRUCCION` en constants.ts). Así el margen de seguridad
 * escala solo con el mantenimiento/población real del asentamiento en vez de quedarse en un número fijo
 * pensado para el asentamiento inicial (causa real de colapsos tras subir de nivel, ver bitácora de bugs).
 */
export function reservaDinamicaConstruccion(
  asentamiento: Asentamiento,
  capital: Asentamiento | undefined
): Partial<Record<RecursoTipo, number>> {
  const costoMantenimiento = calcularCostoMantenimiento(asentamiento, capital);
  const reserva: Partial<Record<RecursoTipo, number>> = {};
  for (const [recurso, cantidad] of Object.entries(costoMantenimiento)) {
    reserva[recurso as RecursoTipo] = (cantidad ?? 0) * RESERVA_CONSTRUCCION.horizonteTicksMantenimiento;
  }
  reserva.trigo =
    (consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento)) * RESERVA_CONSTRUCCION.horizonteTicksComida;
  return reserva;
}

function fraccionCubierta(almacen: Asentamiento['almacen'], costo: Partial<Record<string, number>>): number {
  const fracciones = Object.entries(costo).map(([recurso, cantidad]) =>
    cantidad && cantidad > 0 ? Math.min(1, (almacen[recurso]?.cantidad ?? 0) / cantidad) : 1
  );
  return fracciones.length === 0 ? 1 : fracciones.reduce((a, b) => a + b, 0) / fracciones.length;
}

/**
 * Avanza un tick de mantenimiento (Doc 4.5, degradación escalonada Doc Fase_0_5 §6.2): paga lo que puede,
 * degrada el medidor proporcionalmente al déficit si no llega a cubrir el coste completo, y regenera
 * lentamente si el pago fue íntegro. Al tocar 0, `nivelActual` baja un escalón (nunca `nivel`/nivelAlcanzado,
 * nunca la población) y el medidor se reinicia — solo cae en RUINAS si ya estaba en `nivelActual` 1. Subir
 * `nivelActual` de vuelta exige una racha de `MANTENIMIENTO.ticksSanosParaRecuperarNivel` ticks seguidos con
 * pago íntegro (`rachaMantenimientoSano`), no un solo tick sano — evita el yo-yo de nivel.
 */
export function avanzarMantenimiento(
  asentamiento: Asentamiento,
  capital: Asentamiento | undefined,
  tickActual: number
): { asentamiento: Asentamiento; eventos: string[]; destruido: boolean } {
  if (tickActual - asentamiento.fundadoEnTick < MANTENIMIENTO.graciaTicks) {
    return { asentamiento, eventos: [], destruido: false };
  }

  const costo = calcularCostoMantenimiento(asentamiento, capital);
  const cubierta = fraccionCubierta(asentamiento.almacen, costo);
  const eventos: string[] = [];
  const nivelActualHoy = nivelActualDe(asentamiento);

  let almacen = asentamiento.almacen;
  if (cubierta >= 1) {
    almacen = descontarRecursos(almacen, costo);
  } else {
    // Paga lo que sí puede cubrir de cada recurso; el resto queda en déficit (degrada el medidor).
    for (const [recurso, cantidad] of Object.entries(costo)) {
      const disponible = almacen[recurso]?.cantidad ?? 0;
      almacen = descontarRecursos(almacen, { [recurso]: Math.min(disponible, cantidad ?? 0) });
    }
  }

  if (cubierta >= 1) {
    const racha = (asentamiento.rachaMantenimientoSano ?? 0) + 1;
    const medidor = Math.min(100, asentamiento.medidorMantenimiento + MANTENIMIENTO.regeneracionSiPagoCompleto);

    if (racha >= MANTENIMIENTO.ticksSanosParaRecuperarNivel && nivelActualHoy < asentamiento.nivel) {
      eventos.push(`${asentamiento.id}: mantenimiento sano y sostenido, recupera nivel actual ${nivelActualHoy + 1}.`);
      return {
        asentamiento: { ...asentamiento, almacen, medidorMantenimiento: medidor, nivelActual: nivelActualHoy + 1, rachaMantenimientoSano: 0 },
        eventos,
        destruido: false,
      };
    }
    return { asentamiento: { ...asentamiento, almacen, medidorMantenimiento: medidor, rachaMantenimientoSano: racha }, eventos, destruido: false };
  }

  // Déficit: la racha de salud se rompe, el medidor degrada proporcional al déficit.
  const medidor = Math.max(0, asentamiento.medidorMantenimiento - MANTENIMIENTO.degradacionPorDeficitTotal * (1 - cubierta));

  if (medidor <= 0) {
    if (nivelActualHoy > 1) {
      eventos.push(`${asentamiento.id}: mantenimiento colapsa, baja de nivel actual ${nivelActualHoy} a ${nivelActualHoy - 1}.`);
      return {
        asentamiento: {
          ...asentamiento,
          almacen,
          medidorMantenimiento: MANTENIMIENTO.medidorInicial,
          nivelActual: nivelActualHoy - 1,
          rachaMantenimientoSano: 0,
        },
        eventos,
        destruido: false,
      };
    }
    // Razón auditable (a petición del usuario): qué recurso(s) faltaron en el tick del colapso y cuánto duró
    // el asentamiento, para poder diagnosticar después SIN tener que reconstruir el estado tick a tick.
    const faltantes = Object.entries(costo)
      .map(([recurso, cantidad]) => [recurso, cantidad ?? 0, asentamiento.almacen[recurso]?.cantidad ?? 0] as const)
      .filter(([, cantidad, disponible]) => disponible < cantidad)
      .map(([recurso, cantidad, disponible]) => `${recurso} (tenía ${disponible.toFixed(1)}/${cantidad.toFixed(1)})`);
    const razon = faltantes.length > 0 ? `no pudo cubrir: ${faltantes.join(', ')}` : 'déficit sostenido';
    const duracion = tickActual - asentamiento.fundadoEnTick;
    eventos.push(
      `${asentamiento.id} cae en ruinas por abandono/mal mantenimiento (${razon}; fundado en tick ${asentamiento.fundadoEnTick}, duró ${duracion} ticks) — la zona queda libre.`
    );
    return { asentamiento: { ...asentamiento, almacen, medidorMantenimiento: 0 }, eventos, destruido: true };
  }

  if (medidor < asentamiento.medidorMantenimiento) {
    eventos.push(`${asentamiento.id}: mantenimiento en déficit, medidor baja a ${medidor.toFixed(0)}.`);
  }

  return { asentamiento: { ...asentamiento, almacen, medidorMantenimiento: medidor, rachaMantenimientoSano: 0 }, eventos, destruido: false };
}
