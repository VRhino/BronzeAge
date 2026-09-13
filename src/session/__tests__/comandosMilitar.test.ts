// Grupo de comandos militares (`session/comandos/militar.ts`).
//
// Los rechazos por ENTIDAD INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí. Lo que queda: la regla de negocio de `reclutarTropa`
// (asentamiento sin abastecer) y que los comandos de combate consuman la aleatoriedad DEL CONTEXTO, que es
// lo que mantiene la partida reproducible.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { atacarCampamentoBandidos, reclutarTropa } from '../comandos/militar';
import { movilizarEjercito } from '../comandos/ejercitos';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';
import { OPC, partidaConAsentamiento } from './fixtures';

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
  return { ...base, sesion: abastecer(base.sesion) };
}

/** Almacén lleno y 200 pesants en el (único) asentamiento — ver `partidaAbastecida`. */
function abastecer(original: GameSession): GameSession {
  const payload = original.exportar();
  const asentamiento = payload.state.asentamientos[0]!;
  const almacen = Object.fromEntries(
    Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: item.capacidad }])
  );
  return GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [{ ...asentamiento, almacen, poblacion: { ...asentamiento.poblacion, pesants: 200 } }],
    },
  });
}

describe('reclutarTropa', () => {
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

  it('con su escuadra dentro de un ejército, el jugador no recluta otra de ese tipo en casa (Doc 2.5)', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaAbastecida();
    const params = (jugadorId: string) => ({ asentamientoId, jugadorId, tropaId: 'milicia_lanceros', origen: 'pesants' as const });
    expect(sesion.ejecutar(reclutarTropa, params(fundador), OPC).ok).toBe(true);
    const escuadronIds = sesion.getState().asentamientos[0]!.escuadrones.map((e) => e.id);
    const objetivo = { tipo: 'punto', punto: { x: 900, y: 900 } } as const;
    expect(sesion.ejecutar(movilizarEjercito, { asentamientoId, jugadorId: fundador, escuadronIds, objetivo }, OPC).ok).toBe(true);
    // El carro del ejército se lleva trigo del almacén hasta la reserva: se rellena para que lo único que
    // pueda rechazar sea la unicidad.
    const enCampana = abastecer(sesion);

    const segunda = enCampana.ejecutar(reclutarTropa, params(fundador), OPC);
    expect(segunda.codigoError).toBe(CODIGOS_ERROR.tropasReclutamientoInvalido);
    // Control: la plaza SÍ puede reclutar esa tropa ahora mismo (el vecino lo consigue), así que el rechazo de
    // arriba es por la unicidad y no por falta de pesants, equipo o trigo.
    expect(enCampana.ejecutar(reclutarTropa, params(vecino), OPC).ok).toBe(true);
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
