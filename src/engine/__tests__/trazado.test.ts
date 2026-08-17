// Invariantes del TRAZADO URBANO DINÁMICO de la Vista de Asentamiento.
//
// Especificación: `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`. Estos tests cubren las tres cosas
// que el enfoque anterior (rejilla de calles pre-generada al fundar) rompía y que motivaron el rediseño:
// edificios encima de calles, calles encima de calles y granjas pegadas al Centro Urbano. Se comprueban sobre
// una simulación real de 300 ticks, no sobre casos armados a mano, porque los tres bugs solo aparecían con la
// ciudad ya crecida.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { TRAZADO } from '../../constants';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import {
  aristasDePerimetro,
  celdasDeEdificio,
  edificiosInternos,
  esDeAfueras,
  redDeCalles,
  segmentosDeRed,
} from '../trazado';
import {
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  mockMathRandomDeterminista,
  posicionRecomendable,
} from './fixtures';

const SEED = 99;
const TICKS = 300;

function simular(): Asentamiento[] {
  const mapa = crearMapaDeterminista(SEED);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);

  let estado: EstadoSimulacion = {
    asentamientos: [asentamiento],
    facciones: faccionesTrasFundar,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnTick: 0,
  };
  for (let tick = 1; tick <= TICKS; tick++) estado = avanzarSimulacion(estado, mapa, tick);
  return estado.asentamientos;
}

describe('trazado urbano dinámico', () => {
  let restaurarMathRandom: () => void;
  let asentamientos: Asentamiento[];

  beforeEach(() => {
    restaurarMathRandom = mockMathRandomDeterminista(SEED);
    asentamientos = simular();
  });

  afterEach(() => {
    restaurarMathRandom();
  });

  it('la simulación produce ciudades con material suficiente para juzgar el trazado', () => {
    expect(asentamientos.length).toBeGreaterThan(0);
    const internos = asentamientos.flatMap((a) => edificiosInternos(a.edificios));
    expect(internos.length).toBeGreaterThan(5);
  });

  // El bug histórico "edificios encima de la calle" se resolvió por construcción (las calles corren sobre
  // aristas, no sobre celdas), pero el solapamiento ENTRE EDIFICIOS dejó de ser gratis al pasar de una celda
  // por edificio a huellas de hasta 6x6: esto ya sí hay que comprobarlo.
  it('ningún par de edificios comparte una celda', () => {
    for (const asentamiento of asentamientos) {
      const ocupadas = new Map<string, Edificio>();
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        for (const celda of celdasDeEdificio(edificio)) {
          const clave = `${celda.col},${celda.row}`;
          const previo = ocupadas.get(clave);
          expect(
            previo,
            `${edificio.tipo} (${edificio.id}) pisa la celda ${clave} de ${previo?.tipo} (${previo?.id})`
          ).toBeUndefined();
          ocupadas.set(clave, edificio);
        }
      }
    }
  });

  it('todo edificio interno toca una calle o un camino por al menos una arista de su perímetro', () => {
    for (const asentamiento of asentamientos) {
      const red = redDeCalles(asentamiento.id, asentamiento.edificios);
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        const conectado = aristasDePerimetro(edificio).some((a) => red.calles.has(a) || red.caminos.has(a));
        expect(conectado, `${edificio.tipo} (${edificio.id}) quedó sin salida a la calle`).toBe(true);
      }
    }
  });

  it('ningún tramo es diagonal y ninguno se repite', () => {
    for (const asentamiento of asentamientos) {
      const { calles, caminos } = segmentosDeRed(redDeCalles(asentamiento.id, asentamiento.edificios));
      const vistos = new Set<string>();
      for (const tramo of [...calles, ...caminos]) {
        const rectilineo = tramo.desde.x === tramo.hasta.x || tramo.desde.y === tramo.hasta.y;
        expect(rectilineo, `tramo diagonal: ${JSON.stringify(tramo)}`).toBe(true);
        const clave = `${tramo.desde.x},${tramo.desde.y}->${tramo.hasta.x},${tramo.hasta.y}`;
        expect(vistos.has(clave), `tramo duplicado: ${clave}`).toBe(false);
        vistos.add(clave);
      }
    }
  });

  it('Granja y Corral se quedan a las afueras, nunca pegados al Centro Urbano', () => {
    for (const asentamiento of asentamientos) {
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        if (!esDeAfueras(edificio.tipo)) continue;
        const distancia = Math.hypot(edificio.posicion.x, edificio.posicion.y);
        expect(distancia, `${edificio.tipo} (${edificio.id}) quedó a ${distancia.toFixed(1)} del centro`).toBeGreaterThanOrEqual(
          TRAZADO.radioAfuerasMin
        );
      }
    }
  });

  // La red no se guarda en ningún sitio: se deriva recorriendo los edificios en orden de construcción. Si esa
  // derivación no fuera estable, la ciudad se redibujaría distinta en cada frame y las partidas guardadas no
  // reproducirían su propio trazado al cargarlas.
  it('la red es determinista: mismo asentamiento, misma red', () => {
    for (const asentamiento of asentamientos) {
      const primera = redDeCalles(asentamiento.id, asentamiento.edificios);
      const segunda = redDeCalles(asentamiento.id, asentamiento.edificios);
      expect([...segunda.calles]).toEqual([...primera.calles]);
      expect([...segunda.caminos]).toEqual([...primera.caminos]);
    }
  });
});
