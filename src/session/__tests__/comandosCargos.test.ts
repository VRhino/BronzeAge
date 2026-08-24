// Grupo de comandos de cargos, ciudadanía y gobernanza NPC (`session/comandos/cargos.ts` y
// `alternarFaccionNpc.ts`). Verifica el contrato de la capa de partida, no las reglas del motor.
//
// Incluye a propósito los casos de "entidad inexistente": en `GameStore` esos comandos hacían `.find(...)!`,
// así que una Facción o un asentamiento que no existiera producía un TypeError en vez de un rechazo. Aquí
// deben devolver un `codigoError` limpio.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { alternarFaccionNpc } from '../comandos/alternarFaccionNpc';
import { activarPolitica, asignarCargoLocal, asignarEmbajador, asignarRey, comprarCasa } from '../comandos/cargos';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const ACTOR = 'jugador-test';
const OPC = { momento: MOMENTO, actor: ACTOR };

/** Partida con Facción y un asentamiento fundado; devuelve también el id del primer jugador fundador, que es
 * ciudadano de la Facción (requisito del motor para ser Rey). */
function partidaConAsentamiento() {
  const sesion = GameSession.crear('cargos-test', { seed: 42 });
  const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
  const faccionId = rf.datos!.faccionId;
  const ra = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, OPC);
  const asentamientoId = ra.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  return { sesion, faccionId, asentamientoId, fundador };
}

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

  it('rechazo: Facción inexistente devuelve codigoError, NO revienta (en GameStore era un `.find(...)!`)', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarRey, { faccionId: 'no-existe', jugadorId: fundador }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.no_existe');
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

  it('rechazo: asentamiento inexistente devuelve codigoError, NO revienta', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(asignarCargoLocal, { asentamientoId: 'no-existe', cargo: 'gobernador', jugadorId: fundador }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('asentamiento.no_existe');
  });
});

describe('comprarCasa', () => {
  it('rechazo: asentamiento inexistente se traduce a error de dominio del motor', () => {
    const { sesion } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(comprarCasa, { asentamientoId: 'no-existe', jugadorId: 'nuevo' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.invalida');
  });

  it('un rechazo no altera el estado ni la versión', () => {
    const { sesion } = partidaConAsentamiento();
    const antes = sesion.getState();
    sesion.ejecutar(comprarCasa, { asentamientoId: 'no-existe', jugadorId: 'nuevo' }, OPC);
    expect(sesion.getState()).toBe(antes);
  });
});

describe('activarPolitica', () => {
  it('rechazo: asentamiento inexistente devuelve codigoError, NO revienta', () => {
    const { sesion } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(activarPolitica, { asentamientoId: 'no-existe', cargo: 'gobernador', politicaId: 'lineas_produccion' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('asentamiento.no_existe');
  });

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

  it('rechazo: Facción inexistente', () => {
    const { sesion } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(alternarFaccionNpc, { faccionId: 'no-existe', activo: true }, OPC);
    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('faccion.no_existe');
  });

  it('tras cederla, `avanzarFaccionesNpc` ya actúa sobre ella (cierra el hueco que había)', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.ejecutar(alternarFaccionNpc, { faccionId, activo: true }, OPC);

    const resultado = sesion.avanzarFaccionesNpc(MOMENTO);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBeTruthy();
  });
});
