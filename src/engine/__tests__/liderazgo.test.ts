// LIDERAZGO (Doc 5.11): cuánta tropa puede sacar a campaña un jugador.
//
// Lo que congela este archivo es sobre todo la TABLA de Doc 5.11.1. No está escrita a mano en el catálogo —
// se deriva de `poderBase × unidadesPorDefecto × factorCoste` precisamente porque `poderBase` sigue sin
// calibrar y once números a mano se desincronizarían. El precio de derivarla es que nadie ve los valores al
// leer `constants.ts`, así que el test es el sitio donde la tabla del documento y el código se miran a la
// cara: si alguien recalibra el poder de una tropa, este archivo se pone rojo y obliga a decidir a
// conciencia si el nuevo coste de liderazgo es el que se quería.
import { describe, expect, it } from 'vitest';
import type { Escuadron, Jugador } from '../../domain/types';
import { LIDERAZGO } from '../../constants';
import { costeLiderazgo, liderazgoComprometido, liderazgoDisponible, liderazgoDe, puedeLlevar } from '../liderazgo';

const escuadron = (tropaId: string, jugadorId = 'jugador-1'): Escuadron => ({
  id: `e-${tropaId}`,
  nombre: tropaId,
  jugadorId,
  origen: 'pesants',
  cantidad: 10,
  veterania: 0,
  moral: 100,
  tropaId,
});

describe('costeLiderazgo — la tabla de Doc 5.11.1', () => {
  // Anclada por el usuario: la Milicia de lanceros cuesta 10. Todo lo demás sale de ahí.
  const esperado: Record<string, number> = {
    milicia_lanceros: 10,
    lanceros_mimbre: 12,
    espadachines_cobre: 16,
    honderos: 25,
    hacheros_ligeros: 25.2,
    escaramuzadores_jabalina: 32,
    espadachines_bronce: 32.4,
    hacheros_armados: 36,
    lanceros_pesados: 42,
    arqueros: 45,
    arqueros_compuesto: 60,
  };

  for (const [tropaId, coste] of Object.entries(esperado)) {
    it(`${tropaId} cuesta ${coste}`, () => {
      expect(costeLiderazgo(tropaId)).toBeCloseTo(coste);
    });
  }

  it('una tropa que no existe en el catálogo no cuesta nada', () => {
    expect(costeLiderazgo('__no_existe__')).toBe(0);
  });
});

describe('el gate de élite es real (Doc 5.11.1)', () => {
  it('los Arqueros con arco compuesto NO caben en el liderazgo base', () => {
    expect(costeLiderazgo('arqueros_compuesto')).toBeGreaterThan(LIDERAZGO.base);
    expect(puedeLlevar(undefined, [escuadron('arqueros_compuesto')])).toBe(false);
  });

  it('pero un jugador con liderazgo suficiente sí los saca', () => {
    const veterano: Jugador = { id: 'jugador-1', liderazgoBase: 60 };
    expect(puedeLlevar(veterano, [escuadron('arqueros_compuesto')])).toBe(true);
  });
});

describe('el límite es de SALIDA, no de posesión', () => {
  it('suma los costes de lo que se lleva', () => {
    const carga = [escuadron('milicia_lanceros'), escuadron('lanceros_mimbre'), escuadron('espadachines_cobre')];
    expect(liderazgoComprometido(carga)).toBeCloseTo(38);
  });

  it('acepta una carga que cabe justa y rechaza la que se pasa por poco', () => {
    const cabe = [escuadron('milicia_lanceros'), escuadron('honderos')]; // 35
    const noCabe = [escuadron('milicia_lanceros'), escuadron('honderos'), escuadron('espadachines_cobre')]; // 51

    expect(puedeLlevar(undefined, cabe)).toBe(true);
    expect(puedeLlevar(undefined, noCabe)).toBe(false);
  });

  it('el ejemplo del usuario: con 50 no caben milicia + honderos + lanceros de mimbre a la vez... sí caben', () => {
    // El usuario planteó el caso con números de ejemplo (10/15/30 = 55 > 50). Con la tabla DERIVADA los
    // mismos tres suman 47 y sí entran — la regla que él fijó ("a mayor poder, mayor coste") manda sobre
    // aquellos números, que él mismo confirmó como ilustrativos. Queda escrito aquí para que nadie lo lea
    // como una regresión al comparar con la conversación de diseño.
    const tres = [escuadron('milicia_lanceros'), escuadron('honderos'), escuadron('lanceros_mimbre')];
    expect(liderazgoComprometido(tres)).toBeCloseTo(47);
    expect(puedeLlevar(undefined, tres)).toBe(true);
  });

  it('no hay tope agregado: cada jugador se valida contra el SUYO (Doc 5.11)', () => {
    // Dos jugadores llevando cada uno 45 puntos: 90 en el mismo ejército, y es válido.
    const a = [escuadron('arqueros', 'jugador-a')];
    const b = [escuadron('arqueros', 'jugador-b')];

    expect(puedeLlevar(undefined, a)).toBe(true);
    expect(puedeLlevar(undefined, b)).toBe(true);
    expect(liderazgoComprometido([...a, ...b])).toBeCloseTo(90);
  });
});

describe('liderazgoDe / liderazgoDisponible', () => {
  it('un jugador sin registro en la partida usa el base — por eso no hace falta migrar nada', () => {
    expect(liderazgoDe(undefined)).toBe(LIDERAZGO.base);
  });

  it('lo que queda libre nunca es negativo', () => {
    const pasado = [escuadron('arqueros_compuesto')]; // 60 > 50
    expect(liderazgoDisponible(undefined, pasado)).toBe(0);
  });

  it('descuenta lo ya comprometido', () => {
    expect(liderazgoDisponible(undefined, [escuadron('milicia_lanceros')])).toBeCloseTo(40);
  });
});
