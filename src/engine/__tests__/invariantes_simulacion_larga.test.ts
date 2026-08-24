// Corre una simulación larga con varios asentamientos y verifica, tick a tick, las invariantes que el
// motor debe mantener SIEMPRE — sin importar el balance vigente. Si un cambio futuro rompe alguna de estas
// garantías (un recurso queda negativo, aparece un NaN, el medidor de mantenimiento se sale de [0,100]...),
// esto debe fallar aunque el test específico de esa regresión histórica (ver `regresiones_historicas.test.ts`)
// no exista todavía para el caso concreto.
import { beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { createRng, type RandomFn } from '../../worldgen';
import {
  contextoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from './fixtures';

const TICKS = 300;
const SEED = 7;
// Cap de fundación a nivel 1 es 1 asentamiento por Facción (CAP_FUNDACION_POR_NIVEL[0]) — con 3 facciones
// fijas (`crearFacciones`), el máximo fundable de una sola vez sin subir de nivel es 3.
const NUM_ASENTAMIENTOS = 3;

function fundarVarios(mapa: Mapa, facciones: Faccion[]): Asentamiento[] {
  const asentamientos: Asentamiento[] = [];
  for (let i = 0; i < NUM_ASENTAMIENTOS; i++) {
    const faccion = facciones[i % facciones.length]!;
    const posicion = posicionRecomendable(mapa, asentamientos);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, faccion.id, asentamientos, 0, posicion);
    asentamientos.push(asentamiento);
  }
  return asentamientos;
}

function esFinito(n: number): boolean {
  return Number.isFinite(n);
}

describe('invariantes del motor en una simulación larga', () => {
  let rng: RandomFn;

  beforeEach(() => {
    rng = createRng(SEED);
  });

  it(`se mantienen tras ${TICKS} ticks con ${NUM_ASENTAMIENTOS} asentamientos (recursos, población, mantenimiento, nivel)`, () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const asentamientosIniciales = fundarVarios(mapa, facciones);

    let estado: EstadoSimulacion = {
      asentamientos: asentamientosIniciales,
      facciones,
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
      campamentosBandidos: [],
      bandidosProximoSpawnTick: 0,
    };

    const ultimoNivelVisto = new Map<string, number>();

    for (let tick = 1; tick <= TICKS; tick++) {
      const resultado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      estado = resultado;

      for (const asentamiento of resultado.asentamientos) {
        // --- Almacén: nunca negativo, nunca NaN/Infinity, nunca por encima de su propia capacidad ---
        for (const [recurso, item] of Object.entries(asentamiento.almacen)) {
          expect(esFinito(item.cantidad), `tick ${tick} ${asentamiento.id}.almacen.${recurso}.cantidad es finito`).toBe(true);
          expect(item.cantidad, `tick ${tick} ${asentamiento.id}.almacen.${recurso}.cantidad >= 0`).toBeGreaterThanOrEqual(0);
          expect(item.cantidad, `tick ${tick} ${asentamiento.id}.almacen.${recurso}.cantidad <= capacidad`).toBeLessThanOrEqual(item.capacidad);
        }

        // --- Población: nunca negativa, nunca NaN ---
        for (const clase of ['pesants', 'artesanos', 'nobleza'] as const) {
          const valor = asentamiento.poblacion[clase];
          expect(esFinito(valor), `tick ${tick} ${asentamiento.id}.poblacion.${clase} es finito`).toBe(true);
          expect(valor, `tick ${tick} ${asentamiento.id}.poblacion.${clase} >= 0`).toBeGreaterThanOrEqual(0);
        }

        // --- Mantenimiento: medidor siempre en [0, 100] ---
        expect(asentamiento.medidorMantenimiento, `tick ${tick} ${asentamiento.id}.medidorMantenimiento >= 0`).toBeGreaterThanOrEqual(0);
        expect(asentamiento.medidorMantenimiento, `tick ${tick} ${asentamiento.id}.medidorMantenimiento <= 100`).toBeLessThanOrEqual(100);

        // --- Nivel de asentamiento: monótono, nunca baja (Doc 4.5) ---
        const anterior = ultimoNivelVisto.get(asentamiento.id) ?? asentamiento.nivel;
        expect(asentamiento.nivel, `tick ${tick} ${asentamiento.id}.nivel nunca baja (antes ${anterior})`).toBeGreaterThanOrEqual(anterior);
        ultimoNivelVisto.set(asentamiento.id, asentamiento.nivel);
      }

      // --- Reputación de Facción: siempre en [-100, 100] ---
      for (const faccion of resultado.facciones) {
        expect(faccion.reputacion, `tick ${tick} ${faccion.id}.reputacion en rango`).toBeGreaterThanOrEqual(-100);
        expect(faccion.reputacion, `tick ${tick} ${faccion.id}.reputacion en rango`).toBeLessThanOrEqual(100);
      }
    }

    // Los 5 asentamientos se fundaron en posiciones "recomendables" (bosque alcanzable) — con la reserva
    // dinámica + gracia de mantenimiento ya implementadas, el colapso total del grupo no debería ser la norma.
    expect(estado.asentamientos.length, 'al menos un asentamiento sobrevive 300 ticks').toBeGreaterThan(0);
  });
});
