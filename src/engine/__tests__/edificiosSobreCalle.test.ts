// GUARDIÁN de §E6.16 (doc trazado): ningún edificio interno puede quedar construido encima de una celda de
// calle. El invariante ya lo comprobaba `perfilesTrazado.test.ts`, pero solo sobre SU semilla — y §E6.16 dejó
// escrito que el arreglo necesitaba antes un caso que lo reprodujera, porque el fixture existente no lo
// tocaba: "la ciudad del laboratorio (seed 1, 200 ticks) sí — conviene añadirla como segundo caso del test".
// Este archivo es ese caso.
//
// Por qué varias semillas y no solo la 1: el bug era dependiente del orden en que los candidatos de un tick
// se proponen frente a cómo se comprometen (por score), así que aparece o no según qué se construya en qué
// tick. Una sola ciudad no da confianza; estas tres cubren trazados distintos y son baratas.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { ZONA_INFLUENCIA } from '../../constants';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { celdasDeEdificio, edificiosInternos, sueloOcupado } from '../trazado';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

/** Corre una ciudad real `ticks` ticks. Devuelve `undefined` si el asentamiento colapsó: este arnés fuerza
 * nivel 2 sin gobernanza NPC, y en algunas semillas la ciudad no se sostiene — ahí no hay nada que verificar
 * (y darlo por fallo escondería el resultado real detrás de un `TypeError`). */
function ciudad(seed: number, ticks: number): Asentamiento | undefined {
  const mapa = crearMapaDeterminista(seed);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel: 2,
    nivelActual: 2,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[2] ?? asentamiento.radioPotencial,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
  };
  let estado = crearEstadoDeTest([preparado], facciones);
  const rng = createRng(seed);
  for (let t = 1; t <= ticks; t++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(t, rng));
  return estado.asentamientos[0];
}

describe('§E6.16 — ningún edificio se construye sobre una calle', () => {
  // seed 1 es el repro original del laboratorio que reportó el usuario; 60 y 200 son ciudades distintas.
  for (const seed of [1, 60, 200]) {
    it(`seed ${seed}: ninguna celda de edificio interno cae sobre la red de calles`, () => {
      const a = ciudad(seed, 200);
      if (!a) return;
      const { red } = sueloOcupado(a.id, a.edificios);
      const pisadas: string[] = [];
      for (const e of edificiosInternos(a.edificios)) {
        for (const c of celdasDeEdificio(e)) {
          if (red.calles.has(`${c.col},${c.row}`)) pisadas.push(`${e.tipo}@${c.col},${c.row}`);
        }
      }
      expect(pisadas, `seed ${seed}: ${pisadas.length} celdas pisadas → ${pisadas.join(' · ')}`).toEqual([]);
    });
  }
});
