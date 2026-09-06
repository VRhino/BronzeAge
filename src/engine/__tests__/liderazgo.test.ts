// LIDERAZGO (Doc 5.11): cuánta tropa puede sacar a campaña un jugador.
//
// Lo que congela este archivo es la TABLA DE ESCALONES de Doc 5.11.1 y, sobre todo, el ESPACIO DE
// COMPOSICIÓN que produce: qué mezclas caben en el presupuesto y cuáles se pasan por poco. Esa es la
// mecánica — no los costes en sí, sino las decisiones que obligan a tomar.
//
// Rediseño 2026-09-04: el coste dejó de derivarse de `poderBase × unidades × factor` y pasa a ser el de su
// ESCALÓN. La razón está en `LIDERAZGO` (constants.ts) y es de fondo: con el coste proporcional al poder, el
// poder por punto salía idéntico para las once tropas y elegir composición no era una decisión sino
// aritmética. Este archivo cambió entero con ese rediseño.
import { describe, expect, it } from 'vitest';
import type { Escuadron, Jugador } from '../../domain/types';
import { LIDERAZGO, TROPAS_RECLUTABLES } from '../../constants';
import { costeLiderazgo, liderazgoComprometido, liderazgoDisponible, liderazgoDe, puedeLlevar } from '../liderazgo';

const escuadron = (tropaId: string, jugadorId = 'jugador-1'): Escuadron => ({
  id: `e-${tropaId}-${jugadorId}`,
  nombre: tropaId,
  jugadorId,
  origen: 'pesants',
  cantidad: 10,
  veterania: 0,
  moral: 100,
  tropaId,
});

/** Una tropa cualquiera de ese escalón — los tests hablan de escalones, no de nombres propios. */
const deEscalon = (escalon: number): string => TROPAS_RECLUTABLES.find((t) => t.escalon === escalon)!.id;

describe('costeLiderazgo — la tabla de escalones (Doc 5.11.1)', () => {
  it('cada tropa cuesta lo de su escalón, y nada más', () => {
    for (const tropa of TROPAS_RECLUTABLES) {
      expect(costeLiderazgo(tropa.id), tropa.id).toBe(LIDERAZGO.costePorEscalon[tropa.escalon]);
    }
  });

  it('el coste crece con el escalón — sin empates ni inversiones', () => {
    const costes = [1, 2, 3, 4, 5].map((e) => LIDERAZGO.costePorEscalon[e]!);
    for (let i = 1; i < costes.length; i++) {
      expect(costes[i]!, `escalón ${i + 1} debe costar más que el ${i}`).toBeGreaterThan(costes[i - 1]!);
    }
  });

  it('el coste NO sigue al poder: dos tropas del mismo escalón cuestan igual aunque una rinda más', () => {
    // Es el punto del rediseño. Si el coste siguiera al poder, el poder por punto sería constante y no habría
    // decisión que tomar; que dos veteranas de poder distinto cuesten lo mismo es lo que crea la tier list.
    const veteranas = TROPAS_RECLUTABLES.filter((t) => t.escalon === 3);
    expect(veteranas.length, 'hace falta más de una para que la prueba diga algo').toBeGreaterThan(1);
    expect(new Set(veteranas.map((t) => t.poderBase)).size, 'y con poderes distintos').toBeGreaterThan(1);
    expect(new Set(veteranas.map((t) => costeLiderazgo(t.id))).size).toBe(1);
  });

  it('una tropa que no existe en el catálogo no cuesta nada', () => {
    expect(costeLiderazgo('__no_existe__')).toBe(0);
  });
});

describe('el espacio de composición que abre el presupuesto', () => {
  // Las mezclas que el diseño quiere que sean posibles y AJUSTADAS: caben, pero no sobra para nada más. Si
  // alguien toca un coste o la base, aquí se ve si el juego de decisiones sigue existiendo o se ha vuelto
  // trivial en una dirección u otra.
  const mezcla = (...escalones: number[]) => escalones.map((e) => escuadron(deEscalon(e), `j-${e}-${Math.random()}`));

  it('élite + pesada + veterana cabe, y no deja sitio para una más', () => {
    const tres = [5, 4, 3];
    expect(puedeLlevar(undefined, mezcla(...tres))).toBe(true);
    expect(puedeLlevar(undefined, mezcla(...tres, 1)), 'ni siquiera para una leva').toBe(false);
  });

  it('dos pesadas + veterana + tropa de línea cabe justo', () => {
    expect(puedeLlevar(undefined, mezcla(4, 4, 3, 2))).toBe(true);
    expect(puedeLlevar(undefined, mezcla(4, 4, 3, 2, 1))).toBe(false);
  });

  it('la élite ES fiedable, pero cuesta casi media campaña', () => {
    // Antes del rediseño la élite costaba MÁS que el liderazgo base y no salía nunca al mapa. Ya no: entra,
    // y el precio es que se lleva casi la mitad del presupuesto.
    const coste = costeLiderazgo(deEscalon(5));
    expect(coste).toBeLessThan(LIDERAZGO.base);
    expect(coste).toBeGreaterThan(LIDERAZGO.base * 0.4);
    expect(puedeLlevar(undefined, mezcla(5))).toBe(true);
    expect(puedeLlevar(undefined, mezcla(5, 5)), 'dos entran').toBe(true);
    expect(puedeLlevar(undefined, mezcla(5, 5, 5)), 'tres no').toBe(false);
  });

  it('una hueste de leva es numerosa: la cantidad sigue siendo una estrategia', () => {
    const caben = Math.floor(LIDERAZGO.base / costeLiderazgo(deEscalon(1)));
    expect(caben).toBeGreaterThanOrEqual(10);
    expect(puedeLlevar(undefined, mezcla(...Array(caben).fill(1)))).toBe(true);
  });
});

describe('el límite es de SALIDA, no de posesión', () => {
  it('suma los costes de lo que se lleva', () => {
    const carga = [escuadron(deEscalon(1)), escuadron(deEscalon(3)), escuadron(deEscalon(5))];
    expect(liderazgoComprometido(carga)).toBe(
      LIDERAZGO.costePorEscalon[1]! + LIDERAZGO.costePorEscalon[3]! + LIDERAZGO.costePorEscalon[5]!
    );
  });

  it('no hay tope agregado: cada jugador se valida contra el SUYO (Doc 5.11)', () => {
    // Dos jugadores llevando cada uno una élite: en el mismo ejército suman el doble del tope, y es válido.
    const a = [escuadron(deEscalon(5), 'jugador-a')];
    const b = [escuadron(deEscalon(5), 'jugador-b')];

    expect(puedeLlevar(undefined, a)).toBe(true);
    expect(puedeLlevar(undefined, b)).toBe(true);
    expect(liderazgoComprometido([...a, ...b])).toBe(2 * LIDERAZGO.costePorEscalon[5]!);
  });
});

describe('liderazgoDe / liderazgoDisponible', () => {
  it('un jugador sin registro en la partida usa el base — por eso no hace falta migrar nada', () => {
    expect(liderazgoDe(undefined)).toBe(LIDERAZGO.base);
  });

  it('un jugador con más liderazgo saca más', () => {
    // El techo por progresión todavía no existe como mecánica, pero el motor ya lo admite por jugador.
    const veterano: Jugador = { id: 'jugador-1', liderazgoBase: LIDERAZGO.base * 1.5, ubicacion: { tipo: 'asentamiento', asentamientoId: 'a' } };
    const tres = [escuadron(deEscalon(5), 'a'), escuadron(deEscalon(4), 'a'), escuadron(deEscalon(3), 'a')];
    expect(puedeLlevar(veterano, [...tres, escuadron(deEscalon(2), 'a')])).toBe(true);
    expect(puedeLlevar(undefined, [...tres, escuadron(deEscalon(2), 'a')])).toBe(false);
  });

  it('lo que queda libre nunca es negativo', () => {
    const pasado = Array.from({ length: 20 }, (_, i) => escuadron(deEscalon(5), `j${i}`));
    expect(liderazgoDisponible(undefined, pasado)).toBe(0);
  });

  it('descuenta lo ya comprometido', () => {
    expect(liderazgoDisponible(undefined, [escuadron(deEscalon(1))])).toBe(LIDERAZGO.base - LIDERAZGO.costePorEscalon[1]!);
  });
});
