// Grupo de comandos de cargos, ciudadanía y gobernanza NPC (`session/comandos/cargos.ts` y
// `alternarFaccionNpc.ts`). Verifica el contrato de la capa de partida, no las reglas del motor.
//
// Los rechazos por ENTIDAD INEXISTENTE (Facción/asentamiento que no existe -> `codigoError` limpio, en vez
// del `TypeError` de `.find(...)!` que daba `GameStore`) están unificados en `comandosContratoIds.test.ts`,
// no repetidos aquí. Lo que queda son las reglas de NEGOCIO propias de cada comando.
import { describe, expect, it } from 'vitest';
import { alternarFaccionNpc } from '../comandos/alternarFaccionNpc';
import { activarPolitica, asignarCargoLocal, asignarEmbajador, asignarRey } from '../comandos/cargos';
import { OPC, partidaConAsentamiento } from './fixtures';

const MOMENTO = OPC.momento;

describe('asignarRey / asignarEmbajador', () => {
  it('éxito: un ciudadano fundador puede ser Rey, y queda registrado en su historial', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().facciones[0]!.reyId).toBe(fundador);
    expect(sesion.getState().historialJugadores[fundador]!.some((e) => e.mensaje.includes('Rey'))).toBe(true);
  });

  it('rechazo: un no-ciudadano no puede ser Rey', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarRey, { faccionId, jugadorId: 'forastero' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('cargo.invalido');
    expect(sesion.getState().facciones[0]!.reyId).toBeNull();
  });

  it('Embajador exige que la Facción ya tenga Rey', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const sinRey = sesion.ejecutar(asignarEmbajador, { faccionId, jugadorId: fundador }, OPC);
    expect(sinRey.ok).toBe(false);
    expect(sinRey.codigoError).toBe('cargo.invalido');

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    const conRey = sesion.ejecutar(asignarEmbajador, { faccionId, jugadorId: fundador }, OPC);
    expect(conRey.ok).toBe(true);
    expect(sesion.getState().facciones[0]!.embajadorId).toBe(fundador);
  });
});

describe('asignarCargoLocal', () => {
  it('éxito: asigna Gobernador y lo registra en el historial del jugador', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);

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

describe('alternarFaccionNpc', () => {
  it('cede la Facción al NPC y la devuelve a control manual', () => {
    const { sesion, faccionId } = partidaConAsentamiento();

    const ceder = sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, OPC);
    expect(ceder.ok).toBe(true);
    expect(sesion.getState().faccionesNpcIds).toEqual([faccionId]);

    const recuperar = sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: false }, OPC);
    expect(recuperar.ok).toBe(true);
    expect(sesion.getState().faccionesNpcIds).toEqual([]);
  });

  it('es idempotente: pedir el estado en el que ya está no muta ni versiona', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, OPC);
    const antes = sesion.getState();

    const repetido = sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, OPC);

    expect(repetido.ok).toBe(true);
    expect(repetido.version).toBe(antes.version);
    expect(sesion.getState()).toBe(antes);
  });

  it('tras cederla, `avanzarFaccionesNpc` ya actúa sobre ella (cierra el hueco que había)', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, OPC);

    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBeTruthy();
  });
});
