// `canales.ts` sobre estado de partida genuino, misma fixture que el resto de tests de `session/`.
import { describe, expect, it } from 'vitest';
import { partidaConAsentamiento, OPC } from './fixtures';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { CANAL_GENERAL, canalDeAsentamiento, canalDeEvento, puedeSuscribirseA } from '../canales';

describe('canalDeEvento', () => {
  it('sin asentamientoId va al canal general', () => {
    const evento = { codigo: 'faccion.creada', mensaje: '', momento: '2026-01-01T00:00:00.000Z' };
    expect(canalDeEvento(evento)).toBe(CANAL_GENERAL);
  });

  it('con asentamientoId va a su propio canal', () => {
    const evento = { codigo: 'x', mensaje: '', momento: '2026-01-01T00:00:00.000Z', asentamientoId: 'a1' };
    expect(canalDeEvento(evento)).toBe(canalDeAsentamiento('a1'));
    expect(canalDeAsentamiento('a1')).toBe('asentamiento/a1');
  });
});

describe('puedeSuscribirseA', () => {
  it('el canal general está abierto a cualquiera', () => {
    const { sesion } = partidaConAsentamiento();
    expect(puedeSuscribirseA(sesion.getState(), 'quien-sea', CANAL_GENERAL)).toBe(true);
  });

  it('el ciudadano de la Facción dueña puede suscribirse a su asentamiento', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    expect(puedeSuscribirseA(sesion.getState(), fundador, canalDeAsentamiento(asentamientoId))).toBe(true);
  });

  it('un forastero no puede suscribirse a un asentamiento ajeno', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    expect(puedeSuscribirseA(sesion.getState(), 'forastero', canalDeAsentamiento(asentamientoId))).toBe(false);
  });

  it('un ciudadano de la Facción rival tampoco puede', () => {
    const base = partidaConAsentamiento();
    const opcRival = { ...OPC, actor: 'rival' };
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
    base.sesion.ejecutar(fundarAsentamiento, { faccionId: rf.datos!.faccionId, posicion: { x: 900, y: 900 } }, opcRival);

    expect(puedeSuscribirseA(base.sesion.getState(), 'rival', canalDeAsentamiento(base.asentamientoId))).toBe(false);
  });

  it('un asentamiento inexistente no autoriza (no se distingue de "ajeno")', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    expect(puedeSuscribirseA(sesion.getState(), fundador, canalDeAsentamiento('no-existe'))).toBe(false);
  });

  it('un canal con forma desconocida no autoriza', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    expect(puedeSuscribirseA(sesion.getState(), fundador, 'faccion/x')).toBe(false);
  });
});
