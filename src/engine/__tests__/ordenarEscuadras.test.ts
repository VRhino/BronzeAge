import { describe, expect, it } from 'vitest';
import { HeroeInvalidoError, ordenarEscuadras } from '../heroe';
import { escuadronDePrueba, heroeDePrueba } from './fixtures';

const heroe = heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'e1' }, {
  escuadrones: [escuadronDePrueba('a', 'h1'), escuadronDePrueba('b', 'h1'), escuadronDePrueba('c', 'h1')],
});

describe('ordenarEscuadras', () => {
  it('pone primero las pedidas, en ese orden, y el resto detrás', () => {
    const r = ordenarEscuadras(heroe, undefined, ['c', 'a']);
    expect(r.heroe.escuadrones.map((e) => e.id)).toEqual(['c', 'a', 'b']);
  });

  it('en la columna mueve solo las suyas, dentro de sus huecos', () => {
    const columna = { escuadronIds: ['a', 'x-de-otro', 'b', 'c'] };
    const r = ordenarEscuadras(heroe, columna, ['c', 'b', 'a']);
    expect(r.columna!.escuadronIds).toEqual(['c', 'x-de-otro', 'b', 'a']);
  });

  it('rechaza una escuadra ajena o repetida, diciendo cuál', () => {
    expect(() => ordenarEscuadras(heroe, undefined, ['a', 'zz'])).toThrow(new HeroeInvalidoError('La escuadra zz no es tuya.'));
    expect(() => ordenarEscuadras(heroe, undefined, ['a', 'a'])).toThrow(HeroeInvalidoError);
  });
});
