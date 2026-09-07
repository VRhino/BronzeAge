// Grupo de comandos de comercio (`session/comandos/comercio.ts`).
//
// Los rechazos por ASENTAMIENTO INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están
// unificados en `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { aceptarTrueque, colocarOrdenMercado, crearCaravana, proponerTrueque, rechazarTrueque } from '../comandos/comercio';
import { enPie } from './fixtures';

const OPC = { actor: 'jugador-test' };

/** Dos asentamientos de Facciones distintas, lo bastante separados para que ambos sean fundables. */
function partidaConDosAsentamientos() {
  let sesion = GameSession.crear('comercio-test', { seed: 42 });
  // Dos actores distintos al CREAR: un jugador solo puede crear una Facción (Doc 2 "Entidades"). Cada uno
  // funda la suya: ya tiene columna en el mundo desde que la creó (Doc 1.3, se funda donde se está), así que
  // ninguno de los dos necesita un alta aparte — se le lleva al punto elegido antes de fundar.
  const fa = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { ...OPC, actor: 'jugador-a' }).datos!.faccionId;
  const fb = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;
  sesion = enPie(sesion, 'jugador-a', { x: 400, y: 400 });
  const a = sesion.ejecutar(fundarAsentamiento, { faccionId: fa }, { ...OPC, actor: 'jugador-a' });
  sesion = enPie(sesion, 'jugador-b', { x: 900, y: 900 });
  const b = sesion.ejecutar(fundarAsentamiento, { faccionId: fb }, { ...OPC, actor: 'jugador-b' });
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

describe('proponerTrueque y su respuesta', () => {
  /** Propone un trueque del par y devuelve su id. */
  function proponer(sesion: GameSession, aId: string, bId: string, cantidad = 5): string {
    const r = sesion.ejecutar(
      proponerTrueque,
      { asentamientoAId: aId, recursoA: 'madera', cantidadA: cantidad, asentamientoBId: bId, recursoB: 'piedra', cantidadB: cantidad },
      OPC
    );
    if (!r.ok) throw new Error(`setup del test: la propuesta falló (${r.codigoError})`);
    return r.datos!.acuerdoId;
  }

  it('proponer solo OFRECE: el acuerdo nace propuesto y todavía no traza camino', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    expect(sesion.getState().caminos).toEqual([]);

    const acuerdoId = proponer(sesion, aId, bId);

    expect(sesion.getState().acuerdos).toHaveLength(1);
    expect(sesion.getState().acuerdos[0]!.id).toBe(acuerdoId);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('propuesto');
    // Un camino es infraestructura permanente (Doc 1.6): una propuesta que el otro lado no ha contestado no
    // basta para plantársela.
    expect(sesion.getState().caminos).toEqual([]);
  });

  it('aceptar activa el acuerdo Y abre el camino comercial del par', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);

    const resultado = sesion.ejecutar(aceptarTrueque, { acuerdoId }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('activo');
    expect(sesion.getState().caminos).toHaveLength(1);
    expect(resultado.eventos.some((e) => e.mensaje.includes('camino comercial'))).toBe(true);
  });

  it('el camino comercial NO se duplica al aceptar un segundo trueque del mismo par', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    sesion.ejecutar(aceptarTrueque, { acuerdoId: proponer(sesion, aId, bId) }, OPC);
    const caminosTrasPrimero = sesion.getState().caminos.length;

    const segundo = sesion.ejecutar(aceptarTrueque, { acuerdoId: proponer(sesion, aId, bId, 3) }, OPC);

    expect(segundo.ok).toBe(true);
    expect(sesion.getState().caminos).toHaveLength(caminosTrasPrimero);
    expect(segundo.eventos.some((e) => e.mensaje.includes('camino comercial'))).toBe(false);
  });

  it('rechazar deja constancia y no traza ningún camino', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);

    const resultado = sesion.ejecutar(rechazarTrueque, { acuerdoId }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('rechazado');
    expect(sesion.getState().caminos).toEqual([]);
  });

  it('un acuerdo ya contestado no se vuelve a contestar', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);
    sesion.ejecutar(aceptarTrueque, { acuerdoId }, OPC);
    const antes = sesion.getState();

    const segunda = sesion.ejecutar(rechazarTrueque, { acuerdoId }, OPC);

    expect(segunda.ok).toBe(false);
    expect(segunda.codigoError).toBe('comercio.trueque_invalido');
    expect(sesion.getState()).toBe(antes);
  });

  it('rechazo: contestar a un acuerdo que no existe', () => {
    const { sesion } = partidaConDosAsentamientos();
    const resultado = sesion.ejecutar(aceptarTrueque, { acuerdoId: 'acuerdo-fantasma' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('acuerdo.no_existe');
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
