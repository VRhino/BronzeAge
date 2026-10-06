// Grupo de comandos de cargos y ciudadanía (`session/comandos/cargos.ts`). Verifica el contrato de la capa de
// partida, no las reglas del motor.
//
// Los rechazos por ENTIDAD INEXISTENTE (Facción/asentamiento que no existe -> `codigoError` limpio, en vez
// del `TypeError` de `.find(...)!` que daba `GameStore`) están unificados en `comandosContratoIds.test.ts`,
// no repetidos aquí. Lo que queda son las reglas de NEGOCIO propias de cada comando.
import { describe, expect, it } from 'vitest';
import { activarPolitica, admitirOtrasFacciones, asignarCargoLocal, asignarEmbajador, asignarRey, designarCapital } from '../comandos/cargos';
import { GameSession } from '../gameSession';
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
    const resultado = sesion.ejecutar(activarPolitica, { asentamientoId, cargo: 'gobernador', politicaId: 'postura_defensiva' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('politica.invalida');
  });
});

describe('admitirOtrasFacciones (Doc 2.2, 5.15.1b)', () => {
  it('es un ajuste permanente de la Facción: desactivado de origen, se activa y se desactiva', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    expect(sesion.getState().facciones[0]!.admiteOtrasEnAtaques, 'desactivado por defecto').toBeUndefined();

    expect(sesion.ejecutar(admitirOtrasFacciones, { faccionId, admitir: true }, OPC).ok).toBe(true);
    expect(sesion.getState().facciones[0]!.admiteOtrasEnAtaques).toBe(true);

    expect(sesion.ejecutar(admitirOtrasFacciones, { faccionId, admitir: false }, OPC).ok).toBe(true);
    expect(sesion.getState().facciones[0]!.admiteOtrasEnAtaques).toBe(false);
  });
});

describe('designarCapital (Doc 2.2)', () => {
  it('rechazo: un asentamiento sin Palacio no puede ser la capital, y no versiona', () => {
    const { sesion, faccionId, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();

    const r = sesion.ejecutar(designarCapital, { faccionId, asentamientoId }, OPC);

    expect(r.ok).toBe(false);
    expect(sesion.getState()).toBe(antes);
  });

  it('rechazo: la capital actual no se puede designar otra vez (el traslado feliz se prueba en el motor)', () => {
    const { sesion, faccionId, asentamientoId } = partidaConAsentamiento();
    const payload = sesion.exportar();
    const a = payload.state.asentamientos[0]!;
    const conPalacio = GameSession.importar({
      ...payload,
      state: { ...payload.state, asentamientos: [{ ...a, edificios: [...a.edificios, { ...a.edificios[0]!, id: 'palacio-1', tipo: 'palacio', estado: 'activo' as const }] }] },
    });

    // Es la única plaza, y por tanto ya es la capital por antigüedad.
    expect(conPalacio.ejecutar(designarCapital, { faccionId, asentamientoId }, OPC).ok).toBe(false);
  });
});
