// Grupo de comandos de construcción y gestión del asentamiento (`session/comandos/construccion.ts`).
//
// Los rechazos por ASENTAMIENTO INEXISTENTE (en `GameStore` estos cuatro comandos no tenían try/catch y
// usaban `.find(...)!`, así que reventaban al construir el mensaje de log) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { asignarCargoLocal } from '../comandos/cargos';
import { alternarAutoConstruccion, anadirEdificioManualmente, calibrarReservaManual, renombrarAsentamiento } from '../comandos/construccion';
import { OPC, partidaConAsentamiento } from './fixtures';

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
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
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
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', heroeId: fundador }, OPC);

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
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', heroeId: fundador }, OPC);
    const eventosAntes = sesion.getState().eventosDominio.length;

    const r = sesion.ejecutar(calibrarReservaManual, { asentamientoId, recurso: 'madera', valor: 50 }, OPC);

    expect(r.ok).toBe(true);
    expect(r.eventos).toEqual([]);
    expect(sesion.getState().eventosDominio.length).toBe(eventosAntes);
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
});
