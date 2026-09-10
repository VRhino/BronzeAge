// Guardas del motor de cargos (`engine/cargos.ts`). Antes solo se probaban indirectamente por el comando
// `asignarCargoLocal`/`asignarRey`/`asignarEmbajador` (`session/comandos/cargos.ts`); desde que una Facción
// SIEMPRE nace con Rey (2026-09-10), el camino "Embajador sin Rey" ya no es alcanzable por comando y su
// guarda hay que probarla aquí, sobre la función pura.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { asignarCargoLocal, asignarEmbajador, asignarRey, CargoInvalidoError } from '../cargos';
import { crearFaccion, otorgarCiudadania } from '../faccion';

function faccionCon(...ciudadanos: string[]): Faccion {
  return ciudadanos.reduce((f, id) => otorgarCiudadania(f, id), crearFaccion('faccion-1', 'Micenas'));
}

function asentamientoDe(faccion: Faccion): Asentamiento {
  return {
    id: 'asent-1',
    faccionId: faccion.id,
    cargos: { gobernadorId: null, maestroObrasId: null, tesoreroId: null, generalId: null, sacerdoteId: null },
  } as unknown as Asentamiento;
}

describe('asignarRey', () => {
  it('un no-ciudadano no puede ser Rey', () => {
    expect(() => asignarRey(faccionCon('rey'), 'forastero')).toThrow(CargoInvalidoError);
  });

  it('un ciudadano sí', () => {
    expect(asignarRey(faccionCon('rey'), 'rey').reyId).toBe('rey');
  });
});

describe('asignarEmbajador', () => {
  it('exige que la Facción ya tenga Rey', () => {
    const sinRey = faccionCon('a', 'b');
    expect(() => asignarEmbajador(sinRey, 'b')).toThrow(CargoInvalidoError);
  });

  it('con Rey, un ciudadano puede ser Embajador', () => {
    const conRey = asignarRey(faccionCon('rey', 'emb'), 'rey');
    expect(asignarEmbajador(conRey, 'emb').embajadorId).toBe('emb');
  });

  it('un no-ciudadano no puede ser Embajador', () => {
    const conRey = asignarRey(faccionCon('rey'), 'rey');
    expect(() => asignarEmbajador(conRey, 'forastero')).toThrow(CargoInvalidoError);
  });
});

describe('asignarCargoLocal', () => {
  it('el designado debe ser ciudadano de la Facción', () => {
    const faccion = faccionCon('gob');
    expect(() => asignarCargoLocal(asentamientoDe(faccion), faccion, 'gobernador', 'forastero')).toThrow(CargoInvalidoError);
  });

  it('el resto de cargos exige que ya haya Gobernador', () => {
    const faccion = faccionCon('gob', 'tes');
    expect(() => asignarCargoLocal(asentamientoDe(faccion), faccion, 'tesorero', 'tes')).toThrow(CargoInvalidoError);
  });

  it('con Gobernador puesto, el resto de cargos sí se pueden asignar', () => {
    const faccion = faccionCon('gob', 'tes');
    const conGobernador = asignarCargoLocal(asentamientoDe(faccion), faccion, 'gobernador', 'gob');
    expect(asignarCargoLocal(conGobernador, faccion, 'tesorero', 'tes').cargos.tesoreroId).toBe('tes');
  });
});
