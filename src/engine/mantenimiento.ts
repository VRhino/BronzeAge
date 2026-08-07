import type { Asentamiento, EdificioTipo, RecursoTipo } from '../domain/types';
import { MANTENIMIENTO, NIVEL_ASENTAMIENTO } from '../constants';
import { edificiosPorTipoYEstado } from './asentamientoQuery';
import { descontarRecursos } from './almacen';

function distancia(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Nivel de asentamiento — rediseño de progreso (Fase 0, Doc 4.5): reemplaza la fórmula de puntos anterior por
 * un modelo de GATES (población + edificios activos requeridos, ver NIVEL_ASENTAMIENTO.requisitos). Sube de
 * forma MONÓTONA: nunca baja aunque la población caiga después, evalúa gate por gate desde el nivel actual.
 */
export function calcularNivelAsentamiento(asentamiento: Asentamiento): number {
  let nivel = asentamiento.nivel;
  while (nivel < NIVEL_ASENTAMIENTO.nivelMaximo) {
    const requisito = NIVEL_ASENTAMIENTO.requisitos[nivel + 1];
    if (!requisito) break;
    const cumplePoblacion = asentamiento.poblacion.pesants >= requisito.pesants && asentamiento.poblacion.artesanos >= requisito.artesanos;
    const cumpleEdificios = requisito.edificios.every((tipo) => edificiosPorTipoYEstado(asentamiento, tipo as EdificioTipo).length > 0);
    if (!cumplePoblacion || !cumpleEdificios) break;
    nivel += 1;
  }
  return nivel;
}

export function avanzarNivelAsentamiento(asentamiento: Asentamiento): { asentamiento: Asentamiento; eventos: string[] } {
  const nuevoNivel = calcularNivelAsentamiento(asentamiento);
  if (nuevoNivel === asentamiento.nivel) return { asentamiento, eventos: [] };
  return { asentamiento: { ...asentamiento, nivel: nuevoNivel }, eventos: [`${asentamiento.id} sube a nivel ${nuevoNivel}.`] };
}

/** "Centro de poder de la Facción" (Doc 4.5): placeholder = su asentamiento vivo más antiguo (proxy de capital). */
export function encontrarCapital(faccionId: string, asentamientos: Asentamiento[]): Asentamiento | undefined {
  return asentamientos
    .filter((a) => a.faccionId === faccionId)
    .sort((a, b) => a.fundadoEnTick - b.fundadoEnTick)[0];
}

/**
 * Coste de mantenimiento (Doc 4.5): escala por nivel (los materiales se SUMAN, no se reemplazan) y por
 * distancia a la capital (mecanismo anti-snowball: más lejos = más caro).
 */
export function calcularCostoMantenimiento(asentamiento: Asentamiento, capital: Asentamiento | undefined): Partial<Record<string, number>> {
  const nivel = asentamiento.nivel;
  const factorNivel = 1 + (nivel - 1) * MANTENIMIENTO.factorCrecimientoPorNivel;
  const dist = capital ? distancia(asentamiento.posicion, capital.posicion) : 0;
  const factorDistancia = 1 + Math.min(1, dist / MANTENIMIENTO.escalaDistancia) * (MANTENIMIENTO.factorDistanciaMax - 1);
  const escala = factorNivel * factorDistancia;

  const costo: Partial<Record<string, number>> = {
    madera: MANTENIMIENTO.costoBase.madera * escala,
    trigo: MANTENIMIENTO.costoBase.trigo * escala,
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
 */
export function recursosProtegidosPorMantenimiento(nivel: number): RecursoTipo[] {
  const recursos: RecursoTipo[] = ['madera', 'trigo'];
  if (nivel >= MANTENIMIENTO.nivelParaPiedra) recursos.push('piedra');
  if (nivel >= MANTENIMIENTO.nivelParaOro) recursos.push('oro');
  return recursos;
}

function fraccionCubierta(almacen: Asentamiento['almacen'], costo: Partial<Record<string, number>>): number {
  const fracciones = Object.entries(costo).map(([recurso, cantidad]) =>
    cantidad && cantidad > 0 ? Math.min(1, (almacen[recurso]?.cantidad ?? 0) / cantidad) : 1
  );
  return fracciones.length === 0 ? 1 : fracciones.reduce((a, b) => a + b, 0) / fracciones.length;
}

/**
 * Avanza un tick de mantenimiento (Doc 4.5): paga lo que puede, degrada el medidor proporcionalmente al
 * déficit si no llega a cubrir el coste completo, y regenera lentamente si el pago fue íntegro. A 0 el
 * asentamiento "cae en ruinas" — Fase 0 lo representa eliminándolo (libera su zona de influencia).
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

  const medidor =
    cubierta >= 1
      ? Math.min(100, asentamiento.medidorMantenimiento + MANTENIMIENTO.regeneracionSiPagoCompleto)
      : Math.max(0, asentamiento.medidorMantenimiento - MANTENIMIENTO.degradacionPorDeficitTotal * (1 - cubierta));

  if (medidor <= 0) {
    eventos.push(`${asentamiento.id} cae en ruinas por abandono/mal mantenimiento — la zona queda libre.`);
    return { asentamiento: { ...asentamiento, almacen, medidorMantenimiento: 0 }, eventos, destruido: true };
  }
  if (medidor < asentamiento.medidorMantenimiento) {
    eventos.push(`${asentamiento.id}: mantenimiento en déficit, medidor baja a ${medidor.toFixed(0)}.`);
  }

  return { asentamiento: { ...asentamiento, almacen, medidorMantenimiento: medidor }, eventos, destruido: false };
}
