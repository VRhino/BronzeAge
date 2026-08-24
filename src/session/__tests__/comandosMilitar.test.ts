// Grupo de comandos militares (`session/comandos/militar.ts`).
//
// Interesan sobre todo dos cosas de la capa de partida: que los ids inexistentes se traduzcan a códigos de
// rechazo (en `GameStore` eran `.find(...)!` y reventaban), y que los comandos de combate consuman la
// aleatoriedad DEL CONTEXTO, que es lo que mantiene la partida reproducible.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { atacarCampamentoBandidos, combateCampoAbierto, interceptarCaravana, iniciarAsedio, reclutarTropa } from '../comandos/militar';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-test' };

function partidaConAsentamiento() {
  const sesion = GameSession.crear('militar-test', { seed: 42 });
  const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
  const r = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, OPC);
  const asentamientoId = r.datos!.asentamientoId;
  const fundador = sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[0]!;
  return { sesion, faccionId, asentamientoId, fundador };
}

/**
 * Asentamiento en condiciones de reclutar. Un recién fundado NO puede, y por dos reglas REALES del motor —no
 * son obstáculos del test, así que en vez de sortearlas se construye el estado que tendría una partida ya en
 * marcha (vía `importar()`, porque no hay comandos para rellenar almacén ni población):
 *
 *  1. `milicia_lanceros` recluta 25 unidades de golpe (`unidadesPorDefecto`) y un asentamiento nuevo arranca
 *     con 20 pesants (`POBLACION.pesants.inicial`) — de los que además hay que descontar la mano de obra que
 *     sostiene la producción (`poblacionDisponibleParaReclutar`).
 *  2. Reclutar exige reserva de trigo proyectada y equipo (25 × 2 = 50 madera), que el almacén inicial no
 *     cubre.
 */
function partidaAbastecida() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const asentamiento = payload.state.asentamientos[0]!;
  const almacen = Object.fromEntries(
    Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: item.capacidad }])
  );
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [{ ...asentamiento, almacen, poblacion: { ...asentamiento.poblacion, pesants: 200 } }],
    },
  });
  return { ...base, sesion };
}

describe('reclutarTropa', () => {
  it('rechazo: asentamiento inexistente devuelve codigoError, NO revienta', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(
      reclutarTropa,
      { asentamientoId: 'no-existe', jugadorId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' },
      OPC
    );

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('asentamiento.no_existe');
  });

  it('éxito: recluta, informa cuántos y lo anota en el historial del jugador', () => {
    const { sesion, asentamientoId, fundador } = partidaAbastecida();
    const resultado = sesion.ejecutar(
      reclutarTropa,
      { asentamientoId, jugadorId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' },
      OPC
    );

    expect(resultado.ok).toBe(true);
    expect(resultado.datos!.reclutados).toBeGreaterThan(0);
    expect(sesion.getState().asentamientos[0]!.escuadrones).toHaveLength(1);
    expect(sesion.getState().historialJugadores[fundador]!.some((e) => e.mensaje.includes('Recluta'))).toBe(true);
  });

  it('rechazo: una tropa que no existe en el catálogo', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const antes = sesion.getState();
    const resultado = sesion.ejecutar(
      reclutarTropa,
      { asentamientoId, jugadorId: fundador, tropaId: 'no-existe', origen: 'pesants' },
      OPC
    );

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('tropas.reclutamiento_invalido');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('comandos de combate — ids inexistentes', () => {
  it('iniciarAsedio: asentamiento inexistente', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(iniciarAsedio, { atacanteId: asentamientoId, defensorId: 'no-existe', escuadronIds: [] }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('asentamiento.no_existe');
  });

  it('combateCampoAbierto: asentamiento inexistente', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(
      combateCampoAbierto,
      { asentamientoAId: asentamientoId, escuadronIdsA: [], asentamientoBId: 'no-existe', escuadronIdsB: [] },
      OPC
    );

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('asentamiento.no_existe');
  });

  it('interceptarCaravana: caravana inexistente tiene su propio código', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(interceptarCaravana, { atacanteId: asentamientoId, escuadronIds: [], caravanaId: 'no-existe' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('caravana.no_existe');
  });

  it('atacarCampamentoBandidos: campamento inexistente tiene su propio código', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = sesion.ejecutar(atacarCampamentoBandidos, { atacanteId: asentamientoId, escuadronIds: [], campamentoId: 'no-existe' }, OPC);

    expect(resultado.ok).toBe(false);
    expect(resultado.codigoError).toBe('campamento.no_existe');
  });

  it('un rechazo por id inexistente no muta el estado', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();
    sesion.ejecutar(atacarCampamentoBandidos, { atacanteId: asentamientoId, escuadronIds: [], campamentoId: 'no-existe' }, OPC);
    expect(sesion.getState()).toBe(antes);
  });
});

describe('combate y reproducibilidad', () => {
  it('atacar un campamento consume el rng del contexto: dos partidas iguales dan el mismo resultado', () => {
    function correr() {
      const { sesion, asentamientoId, fundador } = partidaAbastecida();
      const reclutado = sesion.ejecutar(reclutarTropa, { asentamientoId, jugadorId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' }, OPC);
      if (!reclutado.ok) throw new Error('setup del test: no se pudo reclutar');

      // Se inyecta un campamento a mano: el spawn natural depende del tick y aquí solo interesa el combate.
      const payload = sesion.exportar();
      const conCampamento = GameSession.importar({
        ...payload,
        state: {
          ...payload.state,
          campamentosBandidos: [{ id: 'camp-1', posicion: { x: 520, y: 520 }, bosqueId: 'b1', asentamientoId, poder: 50 }],
        },
      });

      const escuadronIds = conCampamento.getState().asentamientos[0]!.escuadrones.map((e) => e.id);
      const resultado = conCampamento.ejecutar(atacarCampamentoBandidos, { atacanteId: asentamientoId, escuadronIds, campamentoId: 'camp-1' }, OPC);
      return { ok: resultado.ok, destruido: resultado.datos?.destruido, estado: conCampamento.getState() };
    }

    const a = correr();
    const b = correr();

    expect(a.ok).toBe(true);
    expect(b).toEqual(a);
  });
});
