// La memoria de Facción (niebla de guerra, Paso 2): lo que se ve queda grabado, y lo grabado no se borra.
// Los asentamientos salen del motor real (`fundarAsentamientoDeTest`), no de objetos a mano, para que el
// `radioPotencial` con el que se calcula la vigilancia sea el de verdad.
import { describe, expect, it } from 'vitest';
import type { Ejercito, Escuadron } from '../../domain/types';
import { VISION } from '../../constants';
import { instante } from '../../domain/tiempo';
import { crearMapaDeterminista, crearFacciones, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
import { celdasExploradas, estaExplorado, rejillaDe } from '../exploracion';
import { grabarLoVisto, MEMORIA_VACIA } from '../memoria';

const mapa = crearMapaDeterminista(42);
const REJILLA = rejillaDe(mapa.limites);

/** Dos plazas de Facciones distintas, lo bastante lejos como para no verse: 900 separa de sobra. */
function dosPlazasLejanas() {
  const facciones = crearFacciones();
  const mia = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, { x: 400, y: 400 }).asentamiento;
  const suya = fundarAsentamientoDeTest(mapa, facciones, 'faccion-2', [mia], 0, { x: 1300, y: 1300 }).asentamiento;
  return { facciones, mia, suya };
}

function ejercito(faccionId: string, x: number, y: number): Ejercito {
  const escuadron: Escuadron = {
    id: 's1',
    heroeId: 'j1',
    tropaId: 'milicia_lanceros',
    nombre: 'Lanceros',
    origen: 'pesants',
    cantidad: 10,
    veterania: 0,
    moral: 100,
  };
  return {
    id: `e-${faccionId}`,
    faccionId,
    origenAsentamientoId: 'origen',
    participantes: [{ heroeId: 'j1', unidoEn: instante(0) }],
    tipo: 'ejercito',
    politicaDeUnion: 'rechazar',
    liderId: 'j1',
    escuadrones: [escuadron],
    suministro: { trigo: 500 },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: { x, y } },
    ruta: [{ x: 0, y: 0 }, { x, y }],
    progreso: 1,
    posicionActual: { x, y },
    estado: 'marchando',
  };
}

function contexto(over: Partial<Parameters<typeof grabarLoVisto>[1]> = {}) {
  const { facciones, mia, suya } = dosPlazasLejanas();
  return { asentamientos: [mia, suya], ejercitos: [], facciones, limites: mapa.limites, instante: instanteDeTest(1), ...over };
}

describe('grabarLoVisto: la exploracion', () => {
  it('una plaza explora lo que vigila — su radio MAS el margen', () => {
    const { facciones, mia, suya } = dosPlazasLejanas();
    const memoria = grabarLoVisto({}, { asentamientos: [mia, suya], ejercitos: [], facciones, limites: mapa.limites, instante: instanteDeTest(1) });
    const alcance = mia.radioPotencial + VISION.margenAsentamiento;

    expect(estaExplorado(memoria['faccion-1']!.exploracion, REJILLA, mia.posicion)).toBe(true);
    expect(estaExplorado(memoria['faccion-1']!.exploracion, REJILLA, { x: 400 + alcance - 20, y: 400 })).toBe(true);
    expect(estaExplorado(memoria['faccion-1']!.exploracion, REJILLA, { x: 400 + alcance + 60, y: 400 })).toBe(false);
    // Y no explora lo que vigila la de enfrente: cada Faccion tiene sus propios ojos.
    expect(estaExplorado(memoria['faccion-1']!.exploracion, REJILLA, suya.posicion)).toBe(false);
  });

  it('un ejercito explora por donde pasa, y lo explorado se ACUMULA tick a tick', () => {
    const { facciones, mia, suya } = dosPlazasLejanas();
    const base = { asentamientos: [mia, suya], facciones, limites: mapa.limites };

    const tick1 = grabarLoVisto({}, { ...base, ejercitos: [ejercito('faccion-1', 700, 700)], instante: instanteDeTest(1) });
    const tick2 = grabarLoVisto(tick1, { ...base, ejercitos: [ejercito('faccion-1', 1000, 1000)], instante: instanteDeTest(2) });

    // Donde estuvo en el tick 1 sigue explorado en el 2, aunque la columna ya no este alli.
    expect(estaExplorado(tick2['faccion-1']!.exploracion, REJILLA, { x: 700, y: 700 })).toBe(true);
    expect(estaExplorado(tick2['faccion-1']!.exploracion, REJILLA, { x: 1000, y: 1000 })).toBe(true);
    expect(celdasExploradas(tick2['faccion-1']!.exploracion)).toBeGreaterThan(celdasExploradas(tick1['faccion-1']!.exploracion));
  });

  it('una Faccion sin plazas ni columnas no aparece en la memoria: nadie tiene ojos por ella', () => {
    const memoria = grabarLoVisto({}, contexto());
    expect(memoria['faccion-3']).toBeUndefined();
  });

  it('un tick sin novedad devuelve la MISMA memoria, no una copia', () => {
    const c = contexto();
    const primero = grabarLoVisto({}, c);
    const segundo = grabarLoVisto(primero, { ...c, instante: instanteDeTest(2) });

    expect(segundo['faccion-1']).toBe(primero['faccion-1']);
    expect(segundo['faccion-2']).toBe(primero['faccion-2']);
  });
});

describe('grabarLoVisto: la ficha de lo ajeno', () => {
  it('ver una plaza rival guarda su ficha, con el instante en que se vio', () => {
    const { facciones, mia, suya } = dosPlazasLejanas();
    // Una columna propia plantada encima de la plaza rival: la ve de sobra.
    const memoria = grabarLoVisto({}, {
      asentamientos: [mia, suya],
      ejercitos: [ejercito('faccion-1', suya.posicion.x, suya.posicion.y)],
      facciones,
      limites: mapa.limites,
      instante: instanteDeTest(7),
    });

    expect(memoria['faccion-1']!.asentamientos[suya.id]).toEqual({
      asentamientoId: suya.id,
      nombre: suya.nombre,
      faccionId: 'faccion-2',
      posicion: suya.posicion,
      nivel: suya.nivel,
      conocidoEn: instanteDeTest(7),
      // Hasta donde llegaba su tierra: el RADIO, no la silueta. La silueta real esta recortada contra
      // vecinos que quiza no conozcas, asi que congelarla seria congelar informacion de terceros.
      radioPotencial: suya.radioPotencial,
    });
  });

  it('la plaza PROPIA no se guarda: viaja completa en la proyeccion, duplicarla seria el mismo hecho dos veces', () => {
    const memoria = grabarLoVisto({}, contexto());
    expect(memoria['faccion-1']!.asentamientos).toEqual({});
  });

  it('una plaza rival que no se ve no deja ficha', () => {
    const memoria = grabarLoVisto({}, contexto());
    expect(memoria['faccion-1']!.asentamientos).toEqual({});
    expect(memoria['faccion-2']!.asentamientos).toEqual({});
  });

  it('la ficha se queda CONGELADA cuando el ejercito se va, y se refresca si vuelve', () => {
    const { facciones, mia, suya } = dosPlazasLejanas();
    const base = { asentamientos: [mia, suya], facciones, limites: mapa.limites };
    const encima = ejercito('faccion-1', suya.posicion.x, suya.posicion.y);

    const visto = grabarLoVisto({}, { ...base, ejercitos: [encima], instante: instanteDeTest(1) });
    // La plaza sube de nivel y el ejercito se retira: la memoria NO se entera.
    const crecida = { ...suya, nivel: 3, radioPotencial: suya.radioPotencial + 40 };
    const lejos = grabarLoVisto(visto, {
      ...base,
      asentamientos: [mia, crecida],
      ejercitos: [ejercito('faccion-1', 600, 600)],
      instante: instanteDeTest(2),
    });
    expect(lejos['faccion-1']!.asentamientos[suya.id]!.nivel).toBe(suya.nivel);
    expect(lejos['faccion-1']!.asentamientos[suya.id]!.conocidoEn).toBe(instanteDeTest(1));
    expect(lejos['faccion-1']!.asentamientos[suya.id]!.radioPotencial).toBe(suya.radioPotencial);

    // Y al volver, la foto se actualiza — no caduca, se refresca.
    const devuelta = grabarLoVisto(lejos, { ...base, asentamientos: [mia, crecida], ejercitos: [encima], instante: instanteDeTest(9) });
    expect(devuelta['faccion-1']!.asentamientos[suya.id]!.nivel).toBe(3);
    expect(devuelta['faccion-1']!.asentamientos[suya.id]!.conocidoEn).toBe(instanteDeTest(9));
    expect(devuelta['faccion-1']!.asentamientos[suya.id]!.radioPotencial).toBe(suya.radioPotencial + 40);
  });
});

describe('MEMORIA_VACIA', () => {
  it('es lo que vale una Faccion sin registro: nada explorado y nada conocido', () => {
    expect(celdasExploradas(MEMORIA_VACIA.exploracion)).toBe(0);
    expect(MEMORIA_VACIA.asentamientos).toEqual({});
  });
});
