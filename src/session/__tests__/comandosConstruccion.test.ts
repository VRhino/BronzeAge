// Grupo de comandos de construcción y gestión del asentamiento (`session/comandos/construccion.ts`).
//
// Los cuatro comandos "simples" (alternar auto-construcción, calibrar reserva, renombrar) son los que peor
// estaban en `GameStore`: sin try/catch y con `.find(...)!`, así que un id inexistente reventaba al construir
// el mensaje de log. Aquí se comprueba que devuelven rechazo.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { asignarCargoLocal } from '../comandos/cargos';
import {
  alternarAutoConstruccion,
  anadirEdificioManualmente,
  calibrarReservaManual,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
} from '../comandos/construccion';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-test' };

function partidaConAsentamiento() {
  const sesion = GameSession.crear('construccion-test', { seed: 42 });
  const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
  const r = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, OPC);
  const asentamientoId = r.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  return { sesion, faccionId, asentamientoId, fundador };
}

describe('comandos de cola — ids inexistentes', () => {
  it('anadirEdificioManualmente rechaza asentamiento inexistente', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(anadirEdificioManualmente, { asentamientoId: 'no-existe', cargo: 'gobernador', tipo: 'vivienda' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });

  it('quitarDeCola rechaza asentamiento inexistente', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(quitarDeCola, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });

  it('moverEnCola rechaza asentamiento inexistente', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(moverEnCola, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x', direccion: 'arriba' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });

  it('mejorarEdificioAhora rechaza asentamiento inexistente', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(mejorarEdificioAhora, { asentamientoId: 'no-existe', cargo: 'gobernador', edificioId: 'x' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });
});

describe('anadirEdificioManualmente', () => {
  it('rechazo: sin el cargo asignado, el motor lo rechaza y no muta nada', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();
    const r = sesion.ejecutar(anadirEdificioManualmente, { asentamientoId, cargo: 'gobernador', tipo: 'vivienda' }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('construccion.invalida');
    expect(sesion.getState()).toBe(antes);
  });

  it('éxito: con Gobernador asignado, el edificio entra en la cola', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    const edificiosAntes = sesion.getState().asentamientos[0]!.edificios.length;

    const r = sesion.ejecutar(anadirEdificioManualmente, { asentamientoId, cargo: 'gobernador', tipo: 'vivienda' }, OPC);

    expect(r.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.edificios.length).toBe(edificiosAntes + 1);
    expect(r.eventos[0]!.asentamientoId).toBe(asentamientoId);
  });
});

describe('alternarAutoConstruccion', () => {
  it('pausa y reanuda', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();

    const pausar = sesion.ejecutar(alternarAutoConstruccion, { asentamientoId, pausada: true }, OPC);
    expect(pausar.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.autoConstruccionPausada).toBe(true);

    const reanudar = sesion.ejecutar(alternarAutoConstruccion, { asentamientoId, pausada: false }, OPC);
    expect(reanudar.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.autoConstruccionPausada).toBe(false);
  });

  it('es idempotente: reanudar algo que ya está activo no muta ni versiona', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();
    const r = sesion.ejecutar(alternarAutoConstruccion, { asentamientoId, pausada: false }, OPC);

    expect(r.ok).toBe(true);
    expect(r.version).toBe(antes.version);
    expect(sesion.getState()).toBe(antes);
  });

  it('rechaza asentamiento inexistente en vez de reventar (no tenía try/catch en GameStore)', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(alternarAutoConstruccion, { asentamientoId: 'no-existe', pausada: true }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });
});

describe('calibrarReservaManual', () => {
  it('rechazo: sin Tesorero asignado', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const r = sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: 100 }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('reserva.sin_tesorero');
  });

  it('éxito con Tesorero: guarda el valor y acota el rango a 0..999', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    // El motor exige Gobernador antes de cualquier otro cargo local (`engine/cargos.ts`).
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', jugadorId: fundador }, OPC);

    sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: 150 }, OPC);
    expect(sesion.getState().asentamientos[0]!.reservaManual?.madera).toBe(150);

    sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: 5000 }, OPC);
    expect(sesion.getState().asentamientos[0]!.reservaManual?.madera).toBe(999);

    sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: -20 }, OPC);
    expect(sesion.getState().asentamientos[0]!.reservaManual?.madera).toBe(0);
  });

  it('no ensucia el log administrativo: es un ajuste de slider, no un hecho narrable', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    // El motor exige Gobernador antes de cualquier otro cargo local (`engine/cargos.ts`).
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', jugadorId: fundador }, OPC);
    const logAntes = sesion.getState().log.length;

    const r = sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: 50 }, OPC);

    expect(r.ok).toBe(true);
    expect(r.eventos).toEqual([]);
    expect(sesion.getState().log.length).toBe(logAntes);
  });
});

describe('renombrarAsentamiento', () => {
  it('renombra, y un nombre vacío devuelve el asentamiento a mostrarse por su id', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();

    sesion.ejecutar(renombrarAsentamiento, { asentamientoId, nombre: '  Micenas Alta  ' }, OPC);
    expect(sesion.getState().asentamientos[0]!.nombre).toBe('Micenas Alta');

    sesion.ejecutar(renombrarAsentamiento, { asentamientoId, nombre: '   ' }, OPC);
    expect(sesion.getState().asentamientos[0]!.nombre).toBeUndefined();
  });

  it('rechaza asentamiento inexistente en vez de reventar', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(renombrarAsentamiento, { asentamientoId: 'no-existe', nombre: 'X' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });
});
