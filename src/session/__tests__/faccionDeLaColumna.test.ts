// La columna de un héroe que va solo lleva la Facción que tiene ahora, aunque la haya creado, la haya dejado o haya entrado en ella ya
// fuera de un campamento (Ejercito.faccionId se escribe al salir al mundo y no se actualizaba).
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { dejarFaccion } from '../comandos/dejarFaccion';
import { responderSolicitud, solicitarIngreso } from '../comandos/ingresoEnFaccion';
import { unirseEnCampo } from '../comandos/columna';
import { organizarEjercito } from '../comandos/formacion';
import { conHeroe, enPie } from './fixtures';

/** Dos héroes recién aparecidos con su columna, sin Facción. */
function dosSinFaccion() {
  let sesion = GameSession.crear('faccion-columna', { seed: 42 });
  sesion = conHeroe(conHeroe(sesion, 'ana'), 'beto');
  const faccionDeSuColumna = (heroeId: string) => sesion.getState().ejercitos.find((e) => e.participantes.some((p) => p.heroeId === heroeId))?.faccionId;
  return { sesion, faccionDeSuColumna };
}

describe('la Facción de la columna sigue a la del héroe que va solo', () => {
  it('al crear una Facción, al entrar en una y al dejarla', () => {
    const { sesion, faccionDeSuColumna } = dosSinFaccion();
    expect(faccionDeSuColumna('ana')).toBe('');

    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Casa Ana' }, { actor: 'ana' }).datos!.faccionId;
    expect(faccionDeSuColumna('ana'), 'al crearla').toBe(faccionId);

    sesion.ejecutar(solicitarIngreso, { faccionId }, { actor: 'beto' });
    expect(faccionDeSuColumna('beto'), 'pedir entrar no basta').toBe('');
    sesion.ejecutar(responderSolicitud, { faccionId, heroeId: 'beto', aceptar: true }, { actor: 'ana' });
    expect(faccionDeSuColumna('beto'), 'al ser aceptado').toBe(faccionId);

    sesion.ejecutar(dejarFaccion, {}, { actor: 'beto' });
    expect(faccionDeSuColumna('beto'), 'al dejarla').toBe('');
    expect(faccionDeSuColumna('ana'), 'la de ana no cambia').toBe(faccionId);
  });

  it('quien entra en una Facción ya en el mundo se une a la formación de sus compañeros, y dos sin Facción no se unen entre sí', () => {
    const base = dosSinFaccion().sesion;
    base.ejecutar(organizarEjercito, { heroeId: 'ana', politicaDeUnion: 'aceptar' }, { actor: 'ana' });
    const formacion = base.getState().ejercitos.find((e) => e.participantes.some((p) => p.heroeId === 'ana'))!;
    // Junto a ella: hay que estar a 15 para unirse.
    const sesion = enPie(base, 'beto', formacion.posicionActual);

    expect(sesion.ejecutar(unirseEnCampo, { ejercitoId: formacion.id, heroeId: 'beto' }, { actor: 'beto' }).ok, 'sin Facción ninguno de los dos').toBe(false);

    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Casa Ana' }, { actor: 'ana' }).datos!.faccionId;
    sesion.ejecutar(solicitarIngreso, { faccionId }, { actor: 'beto' });
    sesion.ejecutar(responderSolicitud, { faccionId, heroeId: 'beto', aceptar: true }, { actor: 'ana' });

    expect(sesion.ejecutar(unirseEnCampo, { ejercitoId: formacion.id, heroeId: 'beto' }, { actor: 'beto' }).ok).toBe(true);
  });
});
