// Grupo de comandos de comercio (`session/comandos/comercio.ts`).
//
// Los rechazos por ASENTAMIENTO INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están
// unificados en `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { colocarOrdenMercado, crearCaravana, proponerTrueque } from '../comandos/comercio';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-test' };

/** Dos asentamientos de Facciones distintas, lo bastante separados para que ambos sean fundables. */
function partidaConDosAsentamientos() {
  const sesion = GameSession.crear('comercio-test', { seed: 42 });
  const fa = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
  const fb = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC).datos!.faccionId;
  const a = sesion.ejecutar(fundarAsentamiento, { faccionId: fa, posicion: { x: 400, y: 400 }, numJugadores: 1 }, OPC);
  const b = sesion.ejecutar(fundarAsentamiento, { faccionId: fb, posicion: { x: 900, y: 900 }, numJugadores: 1 }, OPC);
  if (!a.ok || !b.ok) throw new Error('setup del test: no se pudieron fundar los dos asentamientos');
  return { sesion, aId: a.datos!.asentamientoId, bId: b.datos!.asentamientoId };
}

describe('crearCaravana', () => {
  it('rechazo: sin Mercado el motor no deja construir caravana, y no muta nada', () => {
    const { sesion, aId } = partidaConDosAsentamientos();
    const antes = sesion.getState();
    const resultado = sesion.ejecutar(crearCaravana, { asentamientoId: aId }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('comercio.caravana_invalida');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('proponerTrueque', () => {
  it('éxito: crea el acuerdo, devuelve su id y abre el camino comercial entre el par', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    expect(sesion.getState().caminos).toEqual([]);

    const resultado = sesion.ejecutar(
      proponerTrueque,
      { asentamientoAId: aId, recursoA: 'madera', cantidadA: 5, asentamientoBId: bId, recursoB: 'piedra', cantidadB: 5 },
      OPC
    );

    expect(resultado.ok).toBe(true);
    expect(resultado.datos?.acuerdoId).toBeTruthy();
    expect(sesion.getState().acuerdos).toHaveLength(1);
    // El camino se crea junto al primer trueque del par (Doc 1.6), y se anuncia con su propio evento.
    expect(sesion.getState().caminos).toHaveLength(1);
    expect(resultado.eventos.some((e) => e.mensaje.includes('camino comercial'))).toBe(true);
  });

  it('el camino comercial NO se duplica en un segundo trueque del mismo par', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    sesion.ejecutar(proponerTrueque, { asentamientoAId: aId, recursoA: 'madera', cantidadA: 5, asentamientoBId: bId, recursoB: 'piedra', cantidadB: 5 }, OPC);
    const caminosTrasPrimero = sesion.getState().caminos.length;

    const segundo = sesion.ejecutar(
      proponerTrueque,
      { asentamientoAId: aId, recursoA: 'madera', cantidadA: 3, asentamientoBId: bId, recursoB: 'piedra', cantidadB: 3 },
      OPC
    );

    expect(segundo.ok).toBe(true);
    expect(sesion.getState().caminos).toHaveLength(caminosTrasPrimero);
    expect(segundo.eventos.some((e) => e.mensaje.includes('camino comercial'))).toBe(false);
  });
});

describe('colocarOrdenMercado', () => {
  it('rechazo: sin Mercado activo el motor rechaza la orden', () => {
    const { sesion, aId } = partidaConDosAsentamientos();
    const resultado = sesion.ejecutar(colocarOrdenMercado, { asentamientoId: aId, tipo: 'venta', recurso: 'madera', cantidad: 10 }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('mercado.orden_invalida');
    expect(sesion.getState().ordenes).toEqual([]);
  });
});
