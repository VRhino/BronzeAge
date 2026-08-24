// El motor debe ser reproducible: mismo seed de mundo + mismo RNG inyectado (los únicos dos puntos de
// aleatoriedad, ver `population.ts`/`combate.ts`/`bandidos.ts`, todos reciben su `rng` de `avanzarSimulacion`)
// deben producir SIEMPRE el mismo resultado. Este test no valida ningún número en particular — protege
// contra la clase de regresión más traicionera: que alguien cuele una fuente de no-determinismo nueva
// (Date.now(), Math.random() suelto sin pasar por el rng inyectado, iterar un Map/Set en un orden no
// garantizado, etc.) que haga que dos partidas "idénticas" diverjan en producción sin que ningún test de
// valores concretos lo note.
import { describe, expect, it } from 'vitest';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const TICKS = 80;

function correrSimulacion(seed: number) {
  const rng = createRng(seed);
  const mapa = crearMapaDeterminista(seed);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

  let estado: EstadoSimulacion = {
    asentamientos: [asentamiento],
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
  const eventosPorTick: string[][] = [];
  for (let tick = 1; tick <= TICKS; tick++) {
    const resultado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
    eventosPorTick.push(resultado.eventos);
    estado = resultado;
  }
  return { estadoFinal: estado, eventosPorTick };
}

describe('determinismo del motor', () => {
  it(`dos corridas independientes de ${TICKS} ticks con el mismo seed producen el mismo estado y los mismos eventos`, () => {
    const corridaA = correrSimulacion(42);
    const corridaB = correrSimulacion(42);

    expect(corridaB.estadoFinal).toEqual(corridaA.estadoFinal);
    expect(corridaB.eventosPorTick).toEqual(corridaA.eventosPorTick);

    // Que las dos corridas coincidan no sirve de mucho si no pasó nada — confirma que hubo dinámica real.
    expect(corridaA.estadoFinal.asentamientos).toHaveLength(1);
    expect(corridaA.estadoFinal.asentamientos[0]!.poblacion.pesants).toBeGreaterThan(20);
  });

  it('un seed distinto produce un resultado distinto (el test anterior no está comparando dos vacíos)', () => {
    const corridaA = correrSimulacion(42);
    const corridaC = correrSimulacion(43);

    expect(corridaC.estadoFinal).not.toEqual(corridaA.estadoFinal);
  });
});
