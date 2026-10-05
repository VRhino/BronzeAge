// Grupo de comandos de construcción y gestión del asentamiento (`session/comandos/construccion.ts`).
//
// Los rechazos por ASENTAMIENTO INEXISTENTE (en `GameStore` estos cuatro comandos no tenían try/catch y
// usaban `.find(...)!`, así que reventaban al construir el mensaje de log) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { asignarCargoLocal } from '../comandos/cargos';
import { alternarAutoConstruccion, anadirEdificioManualmente, calibrarReservaManual, mejorarEdificioAhora, renombrarAsentamiento } from '../comandos/construccion';
import { cupoMiradas } from '../../engine/asentamientoQuery';
import { GameSession } from '../gameSession';
import { abastecer, OPC, partidaConAsentamiento } from './fixtures';

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

/** La plaza abastecida y con el Gobernador asignado, del nivel indicado. */
function plazaConGobernador(nivel: number) {
  const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
  const payload = abastecer(sesion).exportar();
  const listo = GameSession.importar({
    ...payload,
    state: { ...payload.state, asentamientos: payload.state.asentamientos.map((a) => ({ ...a, nivel, nivelActual: nivel })) },
  });
  listo.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
  return { sesion: listo, asentamientoId };
}

describe('Taberna (Doc 4.2.1, 5.12.10)', () => {
  it('pide asentamiento de nivel 2', () => {
    const { sesion, asentamientoId } = plazaConGobernador(1);
    const r = sesion.ejecutar(anadirEdificioManualmente, { asentamientoId, cargo: 'gobernador', tipo: 'taberna' }, OPC);
    expect(r.codigoError).toBe('construccion.invalida');
  });

  it('con nivel 2 entra en la cola, y solo puede haber una por asentamiento', () => {
    const { sesion, asentamientoId } = plazaConGobernador(2);
    const r = sesion.ejecutar(anadirEdificioManualmente, { asentamientoId, cargo: 'gobernador', tipo: 'taberna' }, OPC);
    expect(r.ok).toBe(true);
    expect(sesion.getState().asentamientos[0]!.edificios.filter((e) => e.tipo === 'taberna')).toHaveLength(1);
    expect(sesion.ejecutar(anadirEdificioManualmente, { asentamientoId, cargo: 'gobernador', tipo: 'taberna' }, OPC).codigoError).toBe('construccion.invalida');
  });
});

describe('Taberna: niveles internos', () => {
  it('mejorarla a nivel 2 empieza la obra y, al acabar, sube el cupo de Miradas de 1 a 2', () => {
    const { sesion, asentamientoId } = plazaConGobernador(2);
    const payload = sesion.exportar();
    const taberna = { id: 'taberna-1', tipo: 'taberna' as const, posicion: { x: 6, y: 6 }, estado: 'activo' as const, nivelInterno: 1 };
    const conTaberna = GameSession.importar({
      ...payload,
      state: { ...payload.state, asentamientos: payload.state.asentamientos.map((a) => ({ ...a, edificios: [...a.edificios, taberna] })) },
    });
    expect(cupoMiradas(conTaberna.getState().asentamientos[0]!)).toBe(1);

    const r = conTaberna.ejecutar(mejorarEdificioAhora, { asentamientoId, cargo: 'gobernador', edificioId: 'taberna-1' }, OPC);

    expect(r.ok).toBe(true);
    expect(conTaberna.getState().asentamientos[0]!.edificios.find((e) => e.id === 'taberna-1')!.mejora).toMatchObject({ nivelObjetivo: 2 });
    for (let i = 0; i < 800; i++) conTaberna.avanzarTick();
    expect(cupoMiradas(conTaberna.getState().asentamientos[0]!)).toBe(2);
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
