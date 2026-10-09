// `perseguir` como comando (Doc 5.12.3): solo se persigue lo que se ve desde la propia columna, y dos columnas de
// clases distintas no se persiguen.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { VISION } from '../../constants';
import { frenteACampamento } from './fixtures';

function conLaOtraA(distancia: number, tipoDeLaOtra: 'personal' | 'ejercito' = 'personal') {
  const { sesion, fundador, vecino, columnaVecino } = frenteACampamento(false);
  const payload = sesion.exportar();
  const propia = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === fundador))!;
  const otra = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      ejercitos: payload.state.ejercitos.map((e) =>
        e.id === columnaVecino ? { ...e, faccionId: 'faccion-rival', tipo: tipoDeLaOtra, posicionActual: { x: propia.posicionActual.x + distancia, y: propia.posicionActual.y } } : e
      ),
    },
  });
  const perseguir = () => otra.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: fundador, objetivo: { tipo: 'ejercito', id: columnaVecino } }, { actor: fundador });
  return { sesion: otra, perseguir, fundador, vecino, propiaId: propia.id, columnaVecino };
}

describe('perseguir', () => {
  it('fija la presa si se ve desde la propia columna', () => {
    const { sesion, perseguir, propiaId, columnaVecino } = conLaOtraA(VISION.ejercito - 10);

    expect(perseguir().ok).toBe(true);
    expect(sesion.getState().ejercitos.find((e) => e.id === propiaId)!.persiguiendo).toMatchObject({ tipo: 'ejercito', id: columnaVecino });
  });

  it('no se persigue lo que está fuera de la vista de la propia columna', () => {
    const { sesion, perseguir, propiaId } = conLaOtraA(VISION.ejercito + 10);

    expect(perseguir().ok).toBe(false);
    expect(sesion.getState().ejercitos.find((e) => e.id === propiaId)!.persiguiendo).toBeUndefined();
  });

  it('un ejército no persigue a una columna personal, ni al revés', () => {
    const delEjercito = conLaOtraA(20, 'ejercito');
    expect(delEjercito.perseguir().ok, 'una personal no persigue a un ejército').toBe(false);

    // La otra clase: la columna del fundador pasa a ser ejército y la presa sigue siendo personal.
    const { sesion, fundador, columnaVecino } = conLaOtraA(20, 'personal');
    const payload = sesion.exportar();
    const alReves = GameSession.importar({
      ...payload,
      state: { ...payload.state, ejercitos: payload.state.ejercitos.map((e) => (e.participantes.some((p) => p.heroeId === fundador) ? { ...e, tipo: 'ejercito' as const } : e)) },
    });
    expect(alReves.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: fundador, objetivo: { tipo: 'ejercito', id: columnaVecino } }, { actor: fundador }).ok).toBe(false);
  });

  it('no se persigue a los tuyos ni a un aliado (Doc 5.12.3)', () => {
    const { sesion, fundador, columnaVecino } = conLaOtraA(20);
    const payload = sesion.exportar();
    const propia = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === fundador))!;
    const mismaFaccion = GameSession.importar({
      ...payload,
      state: { ...payload.state, ejercitos: payload.state.ejercitos.map((e) => (e.id === columnaVecino ? { ...e, faccionId: propia.faccionId } : e)) },
    });
    const propio = mismaFaccion.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: fundador, objetivo: { tipo: 'ejercito', id: columnaVecino } }, { actor: fundador });
    expect(propio.ok === false && propio.detalleError).toMatch(/los tuyos ni a un aliado/);

    const aliada = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        relaciones: [{ id: 'al-1', tipo: 'alianza', faccionAId: propia.faccionId, faccionBId: 'faccion-rival', creadoEn: payload.state.relaciones[0]?.creadoEn ?? (0 as never), estado: 'activa' }],
      },
    });
    const r = aliada.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: fundador, objetivo: { tipo: 'ejercito', id: columnaVecino } }, { actor: fundador });
    expect(r.ok === false && r.detalleError).toMatch(/los tuyos ni a un aliado/);
  });

  it('un ejército no persigue caravanas, y solo el Líder decide a quién persigue la columna', () => {
    const { sesion, fundador, vecino, propiaId, columnaVecino } = conLaOtraA(20);
    const payload = sesion.exportar();
    const comoEjercito = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: payload.state.ejercitos.map((e) => (e.id === propiaId ? { ...e, tipo: 'ejercito' as const } : e)),
        caravanas: [{ id: 'car-x', tipo: 'comercial', origenAsentamientoId: 'otra', contenido: {}, posicionActual: payload.state.ejercitos.find((e) => e.id === propiaId)!.posicionActual, progreso: 0, estado: 'en_transito', carros: [] }],
      },
    });
    const alCamino = comoEjercito.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: fundador, objetivo: { tipo: 'caravana', id: 'car-x' } }, { actor: fundador });
    expect(alCamino.ok === false && alCamino.detalleError).toMatch(/solo persigue a otro ejército/);

    // El vecino va en la columna del fundador, pero no la manda.
    const acompanante = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: payload.state.ejercitos.filter((e) => e.id !== columnaVecino).map((e) => (e.id === propiaId ? { ...e, participantes: [...e.participantes, { heroeId: vecino, unidoEn: e.participantes[0]!.unidoEn }] } : e)),
      },
    });
    const noLider = acompanante.ejecutar(REGISTRO_COMANDOS.perseguir, { heroeId: vecino, objetivo: { tipo: 'ejercito', id: columnaVecino } }, { actor: vecino });
    expect(noLider.ok === false && noLider.detalleError).toMatch(/Solo el Líder/);
  });
});
