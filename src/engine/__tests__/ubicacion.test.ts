// `grabarExploracionPersonal` (Doc 1.3, paso 9 del jugador situado): lo que ve quien todavía no tiene
// bandera, grabado en su propio registro porque no hay `MemoriaFaccion` en la que anotarlo.
import { describe, expect, it } from 'vitest';
import { instante } from '../../domain/tiempo';
import type { Ejercito, Heroe } from '../../domain/types';
import { estaExplorado, rejillaDe } from '../exploracion';
import { grabarExploracionPersonal } from '../ubicacion';
import { heroeDePrueba } from './fixtures';

const LIMITES = { ancho: 2000, alto: 2000 };

function columnaHuerfana(posicionActual = { x: 400, y: 400 }): Ejercito {
  return {
    id: 'columna-1',
    faccionId: '',
    origenAsentamientoId: '',
    participantes: [{ heroeId: 'jugador-1', unidoEn: instante(0) }],
    tipo: 'personal',
    liderId: 'jugador-1',
    politicaDeUnion: 'rechazar',
    escuadronIds: [],
    suministro: {},
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: posicionActual },
    ruta: [],
    progreso: 0,
    posicionActual,
    estado: 'estacionado',
  };
}

function jugadorEnColumna(id: string, ejercitoId: string, exploracionPersonal?: string): Heroe {
  return heroeDePrueba(id, { tipo: 'columna', ejercitoId }, { exploracionPersonal });
}

describe('grabarExploracionPersonal', () => {
  it('graba lo que ve la columna de quien no tiene bandera', () => {
    const columna = columnaHuerfana();
    const heroes = [jugadorEnColumna('jugador-1', columna.id)];

    const resultado = grabarExploracionPersonal(heroes, [columna], LIMITES);
    const rejilla = rejillaDe(LIMITES);

    expect(resultado[0]!.exploracionPersonal).toBeTruthy();
    expect(estaExplorado(resultado[0]!.exploracionPersonal!, rejilla, columna.posicionActual)).toBe(true);
    expect(estaExplorado(resultado[0]!.exploracionPersonal!, rejilla, { x: 1900, y: 1900 })).toBe(false);
  });

  it('no toca a quien ya tiene bandera: eso lo graba `memoriaPorFaccion`, no aquí', () => {
    const columnaDeFaccion: Ejercito = { ...columnaHuerfana(), faccionId: 'faccion-1' };
    const heroes = [jugadorEnColumna('jugador-1', columnaDeFaccion.id)];

    const resultado = grabarExploracionPersonal(heroes, [columnaDeFaccion], LIMITES);

    expect(resultado).toBe(heroes); // ni copia el array: nada cambió
    expect(resultado[0]!.exploracionPersonal).toBeUndefined();
  });

  it('no toca a quien no está en una columna (dentro de una plaza, o desconectado)', () => {
    const heroes: Heroe[] = [heroeDePrueba('jugador-1', { tipo: 'asentamiento', asentamientoId: 'a1' })];

    expect(grabarExploracionPersonal(heroes, [], LIMITES)).toBe(heroes);
  });
});
