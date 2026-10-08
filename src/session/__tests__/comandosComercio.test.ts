// Grupo de comandos de comercio (`session/comandos/comercio.ts`).
//
// Los rechazos por ASENTAMIENTO INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están
// unificados en `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from './fundarDePrueba';
import { aceptarTrueque, asignarEscolta, colocarOrdenMercado, crearCaravana, proponerTrueque, quitarEscolta, rechazarTrueque } from '../comandos/comercio';
import { verificarAutorizacion } from '../comandos/autorizacion';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';
import type { Caravana, Escuadron } from '../../domain/types';
import { conHeroe, enPie } from './fixtures';

const OPC = { actor: 'jugador-test' };

/** Dos asentamientos de Facciones distintas, lo bastante separados para que ambos sean fundables. */
function partidaConDosAsentamientos() {
  let sesion = conHeroe(conHeroe(GameSession.crear('comercio-test', { seed: 42 }), 'jugador-a'), 'jugador-b');
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
      { asentamientoAId: aId, lineasA: [{ recurso: 'madera', cantidad }], asentamientoBId: bId, lineasB: [{ recurso: 'piedra', cantidad }] },
      OPC
    );
    if (!r.ok) throw new Error(`setup del test: la propuesta falló (${r.codigoError})`);
    return r.datos!.acuerdoId;
  }

  it('proponer solo OFRECE: el acuerdo nace propuesto', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);

    expect(sesion.getState().acuerdos).toHaveLength(1);
    expect(sesion.getState().acuerdos[0]!.id).toBe(acuerdoId);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('propuesto');
  });

  it('aceptar activa el acuerdo pero no traza camino: el camino nace con las caravanas (red de caminos, Doc 1.6)', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);

    const resultado = sesion.ejecutar(aceptarTrueque, { acuerdoId }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('activo');
    expect(sesion.getState().red?.aristas ?? []).toEqual([]);
  });

  it('rechazar deja constancia', () => {
    const { sesion, aId, bId } = partidaConDosAsentamientos();
    const acuerdoId = proponer(sesion, aId, bId);

    const resultado = sesion.ejecutar(rechazarTrueque, { acuerdoId }, OPC);

    expect(resultado.ok).toBe(true);
    expect(sesion.getState().acuerdos[0]!.estado).toBe('rechazado');
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

describe('escolta de una caravana comercial (Doc 3.13.4): ceder y retirar', () => {
  const HEROE = 'jugador-a';
  const escuadra = (id: string, extra: Partial<Escuadron> = {}) => escuadronDePrueba(id, HEROE, 'milicia_lanceros', 15, extra);

  /** La plaza de `jugador-a` con Mercado del nivel dado, una caravana parada y escuadras suyas en su campamento. */
  function conCaravana(nivelMercado: number, escuadrones: Escuadron[], caravana: Partial<Caravana> = {}) {
    const { sesion, aId } = partidaConDosAsentamientos();
    const p = sesion.exportar();
    const parada: Caravana = {
      id: 'c1', tipo: 'comercial', origenAsentamientoId: aId, contenido: {}, posicionActual: { x: 400, y: 400 }, progreso: 0, estado: 'disponible',
      carros: [{ tipoCarro: 'basico', animal: 'buey' }], ...caravana,
    };
    return {
      aId,
      sesion: GameSession.importar({
        ...p,
        state: {
          ...p.state,
          caravanas: [...p.state.caravanas, parada],
          asentamientos: p.state.asentamientos.map((a) =>
            a.id === aId
              ? { ...a, edificios: [...a.edificios, { id: 'mercado-t', tipo: 'mercado', posicion: { x: 1, y: 1 }, estado: 'activo', ambito: 'asentamiento', nivelInterno: nivelMercado } as never] }
              : a
          ),
          heroes: p.state.heroes.map((h) => (h.id === HEROE ? { ...h, escuadrones } : h)),
        },
      }),
    };
  }
  const caravanaDe = (sesion: GameSession) => sesion.getState().caravanas.find((c) => c.id === 'c1')!;
  const escuadraDe = (sesion: GameSession, id: string) => sesion.getState().heroes.find((h) => h.id === HEROE)!.escuadrones.find((e) => e.id === id)!;

  it('cede una escuadra: sale del campamento a la escolta de la caravana', () => {
    const { sesion } = conCaravana(1, [escuadra('e1')]);
    const r = sesion.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ['e1'] }, { actor: HEROE });

    expect(r.ok).toBe(true);
    expect(r.datos).toEqual({ caravanaId: 'c1', escolta: 1 });
    expect(caravanaDe(sesion).escoltaIds).toEqual(['e1']);
    expect(escuadraDe(sesion, 'e1').contenedor).toEqual({ tipo: 'escolta', caravanaId: 'c1' });
    expect(r.eventos.map((e) => e.codigo)).toEqual(['comercio.caravana_escolta_cedida']);
  });

  it('el cupo es de la caravana, en puntos de Liderazgo: nivel 1 = 100 → 7 lanceros (14 pts) y la 8ª no cabe, ceda quien ceda', () => {
    const lanceros = Array.from({ length: 8 }, (_, i) => escuadra(`e${i}`));
    const { sesion } = conCaravana(1, lanceros);
    const ceder = (ids: string[]) => sesion.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ids }, { actor: HEROE });
    expect(ceder(lanceros.slice(0, 7).map((e) => e.id)).ok).toBe(true);
    const mas = ceder(['e7']);
    expect(mas.ok).toBe(false);
    expect(mas.codigoError).toBe('comercio.caravana_invalida');
    expect(escuadraDe(sesion, 'e7').contenedor, 'la rechazada no se mueve').toEqual({ tipo: 'campamento' });
  });

  it('prestar no gasta el Liderazgo de quien presta: con 1 de Liderazgo sigue cediendo hasta llenar el cupo de la caravana', () => {
    const lanceros = Array.from({ length: 14 }, (_, i) => escuadra(`e${i}`));
    const { sesion } = conCaravana(2, lanceros); // 200 pts = 14 lanceros
    const p = sesion.exportar();
    const flaco = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === HEROE ? { ...h, liderazgoBase: 1 } : h)) } });
    const todas = lanceros.map((e) => e.id);
    expect(flaco.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: todas }, { actor: HEROE }).ok).toBe(true);
    expect(flaco.getState().caravanas.find((c) => c.id === 'c1')!.escoltaIds).toHaveLength(14);
  });

  it('no se cede lo que está en la guarnición, ni a una caravana que ya viaja', () => {
    const { sesion } = conCaravana(3, [escuadra('e1', { enGuarnicion: true }), escuadra('e2')]);
    expect(sesion.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ['e1'] }, { actor: HEROE }).ok).toBe(false);

    const viaja = conCaravana(3, [escuadra('e1')], { estado: 'en_transito' });
    expect(viaja.sesion.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ['e1'] }, { actor: HEROE }).ok).toBe(false);
  });

  it('quitarEscolta devuelve las escuadras a su campamento, solo las propias', () => {
    const { sesion } = conCaravana(2, [escuadra('e1'), escuadra('e2')]);
    sesion.ejecutar(asignarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ['e1', 'e2'] }, { actor: HEROE });

    const una = sesion.ejecutar(quitarEscolta, { caravanaId: 'c1', heroeId: HEROE, escuadronIds: ['e1'] }, { actor: HEROE });
    expect(una.ok).toBe(true);
    expect(caravanaDe(sesion).escoltaIds).toEqual(['e2']);
    expect(escuadraDe(sesion, 'e1').contenedor).toEqual({ tipo: 'campamento' });

    // Sin ids, retira todas las suyas.
    expect(sesion.ejecutar(quitarEscolta, { caravanaId: 'c1', heroeId: HEROE }, { actor: HEROE }).datos).toEqual({ caravanaId: 'c1', retirados: 1 });
    expect(caravanaDe(sesion).escoltaIds).toBeUndefined();
    expect(sesion.ejecutar(quitarEscolta, { caravanaId: 'c1', heroeId: HEROE }, { actor: HEROE }).ok, 'ya no queda nada que retirar').toBe(false);
  });

  it('nadie cede ni retira a nombre de otro, y hay que residir en el origen', () => {
    const { sesion } = conCaravana(2, [escuadra('e1')]);
    const estado = sesion.getState();
    const autorizado = (tipo: 'asignarEscolta' | 'quitarEscolta', actor: string, heroeId: string) =>
      verificarAutorizacion(tipo, { caravanaId: 'c1', heroeId, escuadronIds: ['e1'] } as never, estado, { rol: 'jugador', heroeId: actor }).autorizado;

    expect(autorizado('asignarEscolta', HEROE, HEROE)).toBe(true);
    expect(autorizado('quitarEscolta', HEROE, HEROE)).toBe(true);
    expect(autorizado('asignarEscolta', 'jugador-b', HEROE), 'otro héroe en nombre de jugador-a').toBe(false);
    expect(autorizado('asignarEscolta', 'jugador-b', 'jugador-b'), 'jugador-b no reside en el origen').toBe(false);
  });
});
