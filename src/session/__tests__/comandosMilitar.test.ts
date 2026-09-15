// Grupo de comandos militares (`session/comandos/militar.ts`).
//
// Los rechazos por ENTIDAD INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí. Lo que queda: la regla de negocio de `reclutarTropa`
// (asentamiento sin abastecer) y que los comandos de combate consuman la aleatoriedad DEL CONTEXTO, que es
// lo que mantiene la partida reproducible.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { reclutarTropa } from '../comandos/militar';
import { movilizarEjercito } from '../comandos/ejercitos';
import { salirAlMundo } from '../comandos/presencia';
import { atacar } from '../comandos/interaccion';
import { CODIGOS_ERROR } from '../comandos/codigosDeError';
import { abastecer, OPC, partidaConAsentamiento } from './fixtures';
import { campamentoDe } from '../../engine/tropa';

const campamento = (sesion: GameSession) => campamentoDe(sesion.getState().asentamientos[0]!, sesion.getState().heroes);

/** Asentamiento en condiciones de reclutar (ver `abastecer`): un recién fundado no puede, por reglas reales del motor. */
function partidaAbastecida() {
  const base = partidaConAsentamiento();
  return { ...base, sesion: abastecer(base.sesion) };
}

describe('reclutarTropa', () => {
  it('éxito: recluta, informa cuántos y lo anota en el historial del jugador', () => {
    const { sesion, asentamientoId, fundador } = partidaAbastecida();
    const resultado = sesion.ejecutar(
      reclutarTropa,
      { asentamientoId, heroeId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' },
      OPC
    );

    expect(resultado.ok).toBe(true);
    expect(resultado.datos!.reclutados).toBeGreaterThan(0);
    expect(campamento(sesion)).toHaveLength(1);
    expect(sesion.getState().historialHeroes[fundador]!.some((e) => e.mensaje.includes('Recluta'))).toBe(true);
  });

  it('con su escuadra dentro de un ejército, el jugador no recluta otra de ese tipo en casa (Doc 2.5)', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaAbastecida();
    const params = (heroeId: string) => ({ asentamientoId, heroeId, tropaId: 'milicia_lanceros', origen: 'pesants' as const });
    expect(sesion.ejecutar(reclutarTropa, params(fundador), OPC).ok).toBe(true);
    const escuadronIds = campamento(sesion).map((e) => e.id);
    const objetivo = { tipo: 'punto', punto: { x: 900, y: 900 } } as const;
    expect(sesion.ejecutar(movilizarEjercito, { asentamientoId, heroeId: fundador, escuadronIds, objetivo }, OPC).ok).toBe(true);
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

/**
 * El fundador sale al mundo con su milicia y 60 de trigo, y se le planta delante (a `distancia`) un campamento de
 * bandidos de ese `poder`. Se inyecta a mano: el spawn natural depende del tick y aquí solo interesa el combate.
 */
function columnaFrenteACampamento(poder: number, distancia = 0) {
  const { sesion, asentamientoId, fundador } = partidaAbastecida();
  const reclutado = sesion.ejecutar(reclutarTropa, { asentamientoId, heroeId: fundador, tropaId: 'milicia_lanceros', origen: 'pesants' }, OPC);
  if (!reclutado.ok) throw new Error('setup del test: no se pudo reclutar');
  const salida = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: campamento(sesion).map((e) => e.id), carga: { trigo: 60 } }, OPC);
  if (!salida.ok) throw new Error(`setup del test: no se pudo salir (${salida.codigoError})`);

  const payload = sesion.exportar();
  const columna = payload.state.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === fundador))!;
  const posicion = { x: columna.posicionActual.x + distancia, y: columna.posicionActual.y };
  const conCampamento = GameSession.importar({
    ...payload,
    state: { ...payload.state, campamentosBandidos: [{ id: 'camp-1', posicion, bosqueId: 'b1', asentamientoId, poder }] },
  });
  return { sesion: conCampamento, fundador, columnaId: columna.id };
}

const atacarElCampamento = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(atacar, { heroeId, objetivo: { tipo: 'campamento', id: 'camp-1' } }, OPC);

describe('atacar un campamento de bandidos con la columna (Doc 1.9)', () => {
  it('consume el rng del contexto: dos partidas iguales dan el mismo resultado', () => {
    const correr = () => {
      const { sesion, fundador } = columnaFrenteACampamento(50);
      return { ok: atacarElCampamento(sesion, fundador).ok, estado: sesion.getState() };
    };

    const a = correr();
    expect(a.ok).toBe(true);
    expect(correr()).toEqual(a);
  });

  it('si cae, la recompensa va al carro y se agenda su reaparición', () => {
    const { sesion, fundador, columnaId } = columnaFrenteACampamento(1);

    expect(atacarElCampamento(sesion, fundador).ok).toBe(true);

    const estado = sesion.getState();
    expect(estado.campamentosBandidos).toEqual([]);
    expect(estado.bandidosProximoSpawnEn).toBeDefined();
    expect(estado.ejercitos.find((e) => e.id === columnaId)!.suministro['madera'], 'la madera del botín, en el carro').toBeGreaterThan(0);
  });

  it('si aguanta, sus héroes quedan heridos y la columna pierde la mitad del carro (Doc 5.16.4, 5.16.6)', () => {
    const { sesion, fundador, columnaId } = columnaFrenteACampamento(1_000_000);
    const trigoAntes = sesion.getState().ejercitos.find((e) => e.id === columnaId)!.suministro['trigo']!;

    expect(atacarElCampamento(sesion, fundador).ok).toBe(true);

    const estado = sesion.getState();
    expect(estado.campamentosBandidos).toHaveLength(1);
    expect(estado.heroes.find((h) => h.id === fundador)!.heridoHasta).toBeDefined();
    expect(estado.ejercitos.find((e) => e.id === columnaId)!.suministro['trigo']).toBe(trigoAntes / 2);
    // Y herido ya no puede volver a intentarlo.
    expect(atacarElCampamento(sesion, fundador).codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
  });

  it('hay que llegar hasta él: a distancia no se ataca', () => {
    const { sesion, fundador } = columnaFrenteACampamento(1, 200);
    expect(atacarElCampamento(sesion, fundador).codigoError).toBe(CODIGOS_ERROR.movilizacionInvalida);
  });
});
