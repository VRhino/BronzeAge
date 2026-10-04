// Residencia (Doc 2.5, 2026-10-02): cooldown entre cambios de residencia, `dejarResidencia`, y que abandonar la Facción
// se lleve la casa y los cargos locales. La mudanza en sí ya se prueba en `comandosHeroe.test.ts`.
import { describe, expect, it } from 'vitest';
import { CIUDADANIA, SIMULACION } from '../../constants';
import { GameSession } from '../gameSession';
import { asignarCargoLocal, cambiarResidencia, dejarResidencia } from '../comandos/cargos';
import { dejarFaccion } from '../comandos/dejarFaccion';
import { partidaConAsentamiento } from './fixtures';

const TICKS_POR_DIA = (24 * 60 * 60 * 1000) / SIMULACION.duracionTickMs;

function adelantarDias(sesion: GameSession, dias: number): GameSession {
  const exportada = sesion.exportar();
  return GameSession.importar({ ...exportada, state: { ...exportada.state, tick: exportada.state.tick + dias * TICKS_POR_DIA } });
}

/** La plaza de la fixture más una segunda de la misma Facción a la que mudarse (se construye a mano: el cap de fundación en nivel 1 es uno). */
function conSegundaPlaza() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const a = payload.state.asentamientos[0]!;
  const otra = { ...a, id: 'otra-plaza', heroesFundadoresIds: [], casasCompradas: [], posicion: { x: a.posicion.x + 300, y: a.posicion.y } };
  const sesion = GameSession.importar({ ...payload, state: { ...payload.state, asentamientos: [...payload.state.asentamientos, otra] } });
  return { ...base, sesion, opc: { actor: base.fundador } };
}

const plazaDe = (sesion: GameSession, id: string) => sesion.getState().asentamientos.find((a) => a.id === id)!;

describe('cambiarResidencia — cooldown', () => {
  it('rechaza mudarse otra vez antes de que pasen los días, y deja hacerlo al cumplirse', () => {
    const { sesion, asentamientoId, fundador, opc } = conSegundaPlaza();
    expect(sesion.ejecutar(cambiarResidencia, { destinoId: 'otra-plaza', heroeId: fundador }, opc).ok).toBe(true);

    const pronto = sesion.ejecutar(cambiarResidencia, { destinoId: asentamientoId, heroeId: fundador }, opc);
    expect(pronto.ok).toBe(false);
    expect(pronto.codigoError).toBe('faccion.invalida');

    const despues = adelantarDias(sesion, CIUDADANIA.cooldownCambioResidenciaDias);
    expect(despues.ejecutar(cambiarResidencia, { destinoId: asentamientoId, heroeId: fundador }, opc).ok).toBe(true);
  });

  it('la primera mudanza es libre', () => {
    const { sesion, fundador, opc } = conSegundaPlaza();
    expect(sesion.getState().cambiosResidenciaPorHeroe?.[fundador]).toBeUndefined();
    expect(sesion.ejecutar(cambiarResidencia, { destinoId: 'otra-plaza', heroeId: fundador }, opc).ok).toBe(true);
  });
});

describe('dejarResidencia', () => {
  it('libera la casa y los cargos locales, y el héroe sigue siendo ciudadano', () => {
    const { sesion, faccionId, asentamientoId, fundador, opc } = conSegundaPlaza();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, opc);
    expect(plazaDe(sesion, asentamientoId).cargos.gobernadorId).toBe(fundador);

    expect(sesion.ejecutar(dejarResidencia, { heroeId: fundador }, opc).ok).toBe(true);

    const plaza = plazaDe(sesion, asentamientoId);
    expect(plaza.heroesFundadoresIds).not.toContain(fundador);
    expect(plaza.casasCompradas).not.toContain(fundador);
    expect(plaza.cargos.gobernadorId).toBeNull();
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)!.ciudadanosIds).toContain(fundador);
  });

  it('rechazo: quien no reside en ninguna plaza no tiene qué dejar, y no versiona', () => {
    const { sesion, fundador, opc } = conSegundaPlaza();
    sesion.ejecutar(dejarResidencia, { heroeId: fundador }, opc);
    const antes = sesion.getState();

    expect(sesion.ejecutar(dejarResidencia, { heroeId: fundador }, opc).ok).toBe(false);
    expect(sesion.getState()).toBe(antes);
  });

  it('cuenta para el cooldown: no se muda a otra plaza enseguida, sí pasado el plazo', () => {
    const { sesion, fundador, opc } = conSegundaPlaza();
    sesion.ejecutar(dejarResidencia, { heroeId: fundador }, opc);

    expect(sesion.ejecutar(cambiarResidencia, { destinoId: 'otra-plaza', heroeId: fundador }, opc).ok).toBe(false);
    const despues = adelantarDias(sesion, CIUDADANIA.cooldownCambioResidenciaDias);
    expect(despues.ejecutar(cambiarResidencia, { destinoId: 'otra-plaza', heroeId: fundador }, opc).ok).toBe(true);
  });
});

describe('dejarFaccion — se lleva la casa', () => {
  it('quien abandona la Facción deja su residencia y sus cargos locales', () => {
    const { sesion, asentamientoId, fundador, opc } = conSegundaPlaza();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, opc);

    expect(sesion.ejecutar(dejarFaccion, {}, opc).ok).toBe(true);

    const plaza = plazaDe(sesion, asentamientoId);
    expect(plaza.heroesFundadoresIds).not.toContain(fundador);
    expect(plaza.cargos.gobernadorId).toBeNull();
  });
});
