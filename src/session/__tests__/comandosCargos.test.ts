// Grupo de comandos de cargos y ciudadanía (`session/comandos/cargos.ts`). Verifica el contrato de la capa de
// partida, no las reglas del motor.
//
// Los rechazos por ENTIDAD INEXISTENTE (Facción/asentamiento que no existe -> `codigoError` limpio, en vez
// del `TypeError` de `.find(...)!` que daba `GameStore`) están unificados en `comandosContratoIds.test.ts`,
// no repetidos aquí. Lo que queda son las reglas de NEGOCIO propias de cada comando.
import { describe, expect, it } from 'vitest';
import { activarPolitica, asignarCargoLocal, asignarEmbajador, asignarRey } from '../comandos/cargos';
import { OPC, partidaConAsentamiento } from './fixtures';


describe('asignarRey / asignarEmbajador', () => {
  it('la Facción nace con Rey: su creador (a petición del usuario, 2026-09-10)', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    expect(sesion.getState().facciones[0]!.reyId).toBe(fundador);
  });

  it('traspaso: el Rey vigente puede pasar el trono a otro ciudadano, y queda en el historial', () => {
    const { sesion, faccionId, vecino } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarRey, { faccionId, heroeId: vecino }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().facciones[0]!.reyId).toBe(vecino);
    expect(sesion.getState().historialHeroes[vecino]!.some((e) => e.mensaje.includes('Rey'))).toBe(true);
  });

  it('rechazo: un no-ciudadano no puede ser Rey; el trono no cambia', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarRey, { faccionId, heroeId: 'forastero' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('cargo.invalido');
    expect(sesion.getState().facciones[0]!.reyId).toBe(fundador);
  });

  it('Embajador: el Rey lo designa (la Facción siempre tiene Rey desde su creación)', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const r = sesion.ejecutar(asignarEmbajador, { faccionId, heroeId: fundador }, OPC);
    expect(r.ok).toBe(true);
    expect(sesion.getState().facciones[0]!.embajadorId).toBe(fundador);
  });
});

describe('asignarCargoLocal', () => {
  it('éxito: asigna Gobernador y lo registra en el historial del jugador', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBe(fundador);
    expect(resultado.eventos[0]!.asentamientoId).toBe(asentamientoId);
  });
});

describe('activarPolitica', () => {
  it('rechazo: sin el cargo correspondiente, el motor la rechaza', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(activarPolitica, { asentamientoId, cargo: 'gobernador', politicaId: 'lineas_produccion' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('politica.invalida');
  });
});
