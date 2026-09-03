// Comandos de ejércitos (`session/comandos/ejercitos.ts`, Paso 3 del movimiento de ejércitos).
//
// Este es el primer punto de la mecánica que se puede VER funcionar: hasta aquí solo había tipos, constantes
// y un módulo puro. Lo que congela este archivo es lo que el consejo pidió comprobar en vivo antes de dar los
// pasos anteriores por cerrados — que el tope de Liderazgo rechaza una movilización de verdad — más los tres
// invariantes de composición: los escuadrones se van DE VERDAD del asentamiento, replegar no teletransporta,
// y unirse exige proximidad.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { estacionarEjercito, movilizarEjercito, replegarEjercito, unirseAEjercito } from '../comandos/ejercitos';
import { OPC, partidaConAsentamiento } from './fixtures';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';

/**
 * Partida con escuadrones YA puestos en el asentamiento, inyectados vía `importar` en vez de reclutados.
 *
 * Reclutar de verdad exigiría Barracón y Galería de tiro construidos y con nivel interno, más el equipo en
 * almacén — obstáculos reales del motor (`reclutarTropa`) pero ajenos a lo que se prueba aquí, que es la
 * COMPOSICIÓN de un ejército. Mismo atajo que usa `autorizacionComandos.test.ts` por la misma razón.
 *
 * Las tropas elegidas cubren los tres escalones de coste de Liderazgo: milicia 10, lanceros de mimbre 12,
 * honderos 25.
 */
function partidaConTropas(liderazgoBase?: number) {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const asentamiento = payload.state.asentamientos[0]!;
  const escuadron = (id: string, tropaId: string, jugadorId: string) => ({
    id,
    nombre: tropaId,
    jugadorId,
    origen: 'pesants' as const,
    cantidad: 10,
    veterania: 0,
    moral: 100,
    tropaId,
  });
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [
        {
          ...asentamiento,
          escuadrones: [
            escuadron('esc-milicia', 'milicia_lanceros', base.fundador),
            escuadron('esc-mimbre', 'lanceros_mimbre', base.fundador),
            escuadron('esc-honderos', 'honderos', base.fundador),
            escuadron('esc-vecino', 'milicia_lanceros', base.vecino),
          ],
        },
        ...payload.state.asentamientos.slice(1),
      ],
      jugadores: liderazgoBase === undefined ? [] : [{ id: base.fundador, liderazgoBase }],
    },
  });
  return { ...base, sesion };
}

const PUNTO_LEJOS = { tipo: 'punto', punto: { x: 900, y: 900 } } as const;

describe('movilizarEjercito', () => {
  it('saca los escuadrones DE VERDAD del asentamiento y crea el ejército', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    // La guarnición se quedó sin ese escuadrón: es lo que hace que "tu ciudad queda desnuda" se cumpla por
    // construcción, sin ningún predicado extra (Doc 5.12.4).
    expect(estado.asentamientos[0]!.escuadrones.map((e) => e.id)).not.toContain(escuadronId);
    expect(estado.ejercitos).toHaveLength(1);
    const ejercito = estado.ejercitos[0]!;
    expect(ejercito.escuadrones.map((e) => e.id)).toEqual([escuadronId]);
    expect(ejercito.estado).toBe('marchando');
    expect(ejercito.origenAsentamientoId).toBe(asentamientoId);
    // La ruta se calcula al salir, como una caravana al despacharse.
    expect(ejercito.ruta.length).toBeGreaterThanOrEqual(2);
    expect(ejercito.progreso).toBe(0);
    // El carro nace vacío a propósito: cargarlo del almacén es el Paso 6.
    expect(ejercito.suministro).toEqual({});
  });

  it('RECHAZA si los escuadrones exceden el Liderazgo del jugador', () => {
    // Liderazgo 10: justo la Milicia de lanceros y nada más.
    const { sesion, asentamientoId, fundador } = partidaConTropas(10);
    const milicia = 'esc-milicia';
    const honderos = 'esc-honderos';

    const cabe = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [milicia], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(cabe.ok).toBe(true);

    const noCabe = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [honderos], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(noCabe.ok).toBe(false);
    expect(noCabe.ok === false && noCabe.codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
    // Y el rechazo no deja rastro: el escuadrón sigue en casa y no hay un segundo ejército.
    expect(sesion.getState().asentamientos[0]!.escuadrones.map((e) => e.id)).toContain(honderos);
    expect(sesion.getState().ejercitos).toHaveLength(1);
  });

  it('rechaza llevarse el escuadrón de otro jugador', () => {
    const { sesion, asentamientoId } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: 'jugador-intruso', escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(r.ok).toBe(false);
  });

  it('rechaza salir hacia el propio asentamiento de origen', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';

    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: { tipo: 'asentamiento', id: asentamientoId } },
      OPC
    );
    expect(r.ok).toBe(false);
  });

  it('rechaza salir sin escuadrones', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const r = sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [], objetivo: PUNTO_LEJOS },
      OPC
    );
    expect(r.ok).toBe(false);
  });
});

describe('replegarEjercito — cancelar la marcha (Doc 5.12.6)', () => {
  it('marchando: da media vuelta SIN moverse del sitio', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );

    const antes = sesion.getState().ejercitos[0]!;
    const r = sesion.ejecutar(replegarEjercito, { ejercitoId: antes.id }, OPC);
    expect(r.ok).toBe(true);

    const despues = sesion.getState().ejercitos[0]!;
    expect(despues.estado).toBe('regresando');
    // La posición no se mueve ni un punto: volver cuesta el mismo camino que costó ir.
    expect(despues.posicionActual).toEqual(antes.posicionActual);
    // Ruta invertida y progreso espejado, que es lo que preserva la posición sobre la polilínea.
    expect(despues.ruta[0]).toEqual(antes.ruta[antes.ruta.length - 1]);
    expect(despues.progreso).toBeCloseTo(1 - antes.progreso);
  });

  it('estacionado: calcula una ruta nueva a casa en vez de desandar', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;
    expect(sesion.ejecutar(estacionarEjercito, { ejercitoId }, OPC).ok).toBe(true);
    expect(sesion.getState().ejercitos[0]!.estado).toBe('estacionado');

    const r = sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC);
    expect(r.ok).toBe(true);
    const despues = sesion.getState().ejercitos[0]!;
    expect(despues.estado).toBe('regresando');
    expect(despues.progreso).toBe(0);
  });

  it('no se puede replegar dos veces', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    expect(sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC).ok).toBe(true);
    expect(sesion.ejecutar(replegarEjercito, { ejercitoId }, OPC).ok).toBe(false);
  });
});

describe('unirseAEjercito', () => {
  it('recoge tropa del asentamiento por el que pasa', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const primero = 'esc-milicia';
    const segundo = 'esc-mimbre';

    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [primero], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    // Todavía no se ha movido: sigue en la posición del asentamiento, así que está dentro del radio.
    const r = sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: [segundo] }, OPC);
    expect(r.ok).toBe(true);

    const estado = sesion.getState();
    expect(estado.ejercitos[0]!.escuadrones.map((e) => e.id).sort()).toEqual([primero, segundo].sort());
    expect(estado.asentamientos[0]!.escuadrones.map((e) => e.id)).not.toContain(segundo);
  });

  it('el refuerzo también cuenta contra el Liderazgo, sumando lo que ese jugador YA lleva dentro', () => {
    // Liderazgo 15: cabe la milicia (10) sola, pero no milicia + lanceros de mimbre (10 + 12 = 22).
    const { sesion, asentamientoId, fundador } = partidaConTropas(15);
    const primero = 'esc-milicia';
    const segundo = 'esc-mimbre';

    sesion.ejecutar(
      movilizarEjercito,
      { asentamientoId, jugadorId: fundador, escuadronIds: [primero], objetivo: PUNTO_LEJOS },
      OPC
    );
    const ejercitoId = sesion.getState().ejercitos[0]!.id;

    const r = sesion.ejecutar(unirseAEjercito, { ejercitoId, asentamientoId, jugadorId: fundador, escuadronIds: [segundo] }, OPC);
    expect(r.ok).toBe(false);
    expect(r.ok === false && r.codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
  });

  it('rechaza unirse a un ejército que no existe', () => {
    const { sesion, asentamientoId, fundador } = partidaConTropas();
    const escuadronId = 'esc-milicia';
    const r = sesion.ejecutar(
      unirseAEjercito,
      { ejercitoId: 'ejercito-fantasma', asentamientoId, jugadorId: fundador, escuadronIds: [escuadronId] },
      OPC
    );
    expect(r.ok).toBe(false);
    expect(r.ok === false && r.codigoError).toBe(CODIGOS_ERROR.ejercitoNoExiste);
  });
});
