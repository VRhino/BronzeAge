// Casos puntuales tomados de Consideraciones/Correcciones_Durante_Desarrollo.md: bugs reales que ya se
// arreglaron una vez. Cada test aquí reproduce la condición que los disparaba — si alguien reintroduce el
// bug (a propósito o sin querer, ej. al refactorizar), el test correspondiente debe fallar.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { EdificioTipo } from '../../domain/types';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import {
  crearFacciones,
  crearMundoDeterminista,
  fundarAsentamientoDeTest,
  mockMathRandomDeterminista,
  posicionRecomendable,
} from './fixtures';

const SEED = 11;

function estadoInicialConUnAsentamiento(posicion?: { x: number; y: number }) {
  const world = crearMundoDeterminista(SEED);
  const facciones = crearFacciones();
  const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(world, facciones, 'faccion-1', [], 0, posicion);
  const estado: EstadoSimulacion = {
    asentamientos: [asentamiento],
    facciones: faccionesTrasFundar,
    caravanas: [],
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
  };
  return { world, estado };
}

function contarPorTipo(edificios: { tipo: EdificioTipo; estado: string }[], tipo: EdificioTipo): number {
  return edificios.filter((e) => e.tipo === tipo).length;
}

describe('regresiones históricas (Correcciones_Durante_Desarrollo.md)', () => {
  let restaurarMathRandom: () => void;

  beforeEach(() => {
    restaurarMathRandom = mockMathRandomDeterminista(SEED);
  });

  afterEach(() => {
    restaurarMathRandom();
  });

  // Bug #7: "muerte instantánea de todo asentamiento nuevo" — el coste de Mantenimiento exigía trigo desde
  // el tick 1, antes de que la Granja llegara a completarse, destruyendo el asentamiento en ~9 ticks siempre.
  // La corrección es MANTENIMIENTO.graciaTicks: ningún asentamiento puede caer en ruinas antes de esos ticks.
  it('un asentamiento recién fundado no puede caer en ruinas durante la gracia de mantenimiento, sin importar su emplazamiento', () => {
    // Posición deliberadamente sin garantía de bosque cercano (a diferencia de `posicionRecomendable`):
    // si la gracia no protegiera, este sería justo el caso que colapsaría en ~9 ticks.
    const { world, estado: estadoInicial } = estadoInicialConUnAsentamiento({ x: 500, y: 500 });
    let estado = estadoInicial;
    const graciaTicks = 60;

    for (let tick = 1; tick < graciaTicks; tick++) {
      estado = avanzarSimulacion(estado, world, tick);
      expect(estado.asentamientos, `tick ${tick}: el asentamiento sigue en pie durante la gracia`).toHaveLength(1);
      expect(estado.asentamientos[0]!.medidorMantenimiento, `tick ${tick}: medidor intacto durante la gracia`).toBe(100);
    }
  });

  // Bug #1/#11: "ningún edificio podía construirse nunca" / "la Leñera nunca llegaba a construirse" — un
  // asentamiento recién fundado quedaba congelado en el tick 0 para siempre porque el único edificio que
  // produce madera (Leñera) también cuesta madera, y el asentamiento no arrancaba con reserva suficiente ni
  // prioridad correcta. La corrección (reserva inicial de materiales + prioridad de supervivencia) debe
  // permitir que la auto-construcción avance más allá del set inicial de edificios (centroUrbano/granja/vivienda).
  it('la auto-construcción no se congela: una Leñera llega a activarse en un emplazamiento con bosque alcanzable', () => {
    const { world, estado: estadoInicial } = estadoInicialConUnAsentamiento(posicionRecomendable(crearMundoDeterminista(SEED)));
    let estado = estadoInicial;

    let leneraActiva = false;
    for (let tick = 1; tick <= 40 && !leneraActiva; tick++) {
      estado = avanzarSimulacion(estado, world, tick);
      const asentamiento = estado.asentamientos[0];
      leneraActiva = !!asentamiento && asentamiento.edificios.some((e) => e.tipo === 'lenera' && e.estado === 'activo');
    }

    expect(leneraActiva, 'una Leñera se activó dentro de los primeros 40 ticks').toBe(true);
  });

  // Bug #2: "hambruna silenciosa" — solo se construía una Granja en toda la vida del asentamiento aunque la
  // población (y por tanto el consumo de trigo) siguiera creciendo sin límite; el trigo caía a 0 sin que se
  // disparara ninguna respuesta automática. La corrección hace que la auto-construcción encole Granjas
  // adicionales mientras la producción de trigo esté por debajo del consumo (ver `enDeficitTrigo`, construction.ts).
  it('la Granja escala con la demanda: aparece más de una según crece la población', () => {
    const posicion = posicionRecomendable(crearMundoDeterminista(SEED));
    const { world, estado: estadoInicial } = estadoInicialConUnAsentamiento(posicion);
    let estado = estadoInicial;

    let maxGranjas = 0;
    for (let tick = 1; tick <= 300; tick++) {
      estado = avanzarSimulacion(estado, world, tick);
      if (estado.asentamientos.length === 0) break; // se arruinó — no es lo que este test evalúa.
      maxGranjas = Math.max(maxGranjas, contarPorTipo(estado.asentamientos[0]!.edificios, 'granja'));
    }

    expect(maxGranjas, 'en algún momento de la simulación hay más de 1 Granja (activa o en camino)').toBeGreaterThan(1);
  });
});
