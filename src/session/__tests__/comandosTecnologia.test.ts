// Comando `adoptarTecnologia` (`session/comandos/tecnologia.ts`, Doc 6.5): el Rey, en la capital, paga la tarifa de
// la Era con el almacén de la capital. Y lo que el jugador ve de la tecnología (Doc 6.4).
import { describe, expect, it } from 'vitest';
import { AEDAS, TARIFA_ADOPCION } from '../../constants';
import { instante } from '../../domain/tiempo';
import { precioDeVenta } from '../../engine/tecnologia';
import { instanteDeTick, type GeometriaAsentamientos } from '../estado';
import { adoptarTecnologia, comprarTecnologiaAeda } from '../comandos/tecnologia';
import { MATRIZ_AUTORIZACION, verificarAutorizacion } from '../comandos/autorizacion';
import { GameSession } from '../gameSession';
import { proyectarParaJugador } from '../proyecciones/jugador';
import { abastecer, ACTOR, OPC, partidaConAsentamiento } from './fixtures';
import type { TecnologiaId } from '../../domain/types';

const SIN_GEOMETRIA: GeometriaAsentamientos = { zonas: [], zonasFusionadas: [], trazadoPorAsentamiento: {} };

/** La partida del fixture, con el almacén lleno y `metalurgia_cobre` aparecida a su Facción. */
function conCobreAparecido(): { sesion: GameSession; faccionId: string; asentamientoId: string } {
  const { sesion: base, faccionId, asentamientoId } = partidaConAsentamiento();
  const payload = abastecer(base).exportar();
  const tecnologia = {
    ...payload.state.tecnologia,
    porFaccion: {
      [faccionId]: { aparecidas: ['leva_comunal', 'hostigamiento_tribal', 'metalurgia_cobre'], adoptadas: ['leva_comunal', 'hostigamiento_tribal'] },
    },
  } as typeof payload.state.tecnologia;
  return { sesion: GameSession.importar({ ...payload, state: { ...payload.state, tecnologia } }), faccionId, asentamientoId };
}

describe('adoptarTecnologia', () => {
  it('el Rey en la capital la adopta y la paga el almacén de la capital', () => {
    const { sesion, faccionId, asentamientoId } = conCobreAparecido();
    const antes = sesion.getState().asentamientos[0]!.almacen;

    const r = sesion.ejecutar(adoptarTecnologia, { faccionId, tecnologiaId: 'metalurgia_cobre' }, OPC);

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    expect(estado.tecnologia.porFaccion[faccionId]!.adoptadas).toContain('metalurgia_cobre');
    const despues = estado.asentamientos[0]!.almacen;
    const tarifa = TARIFA_ADOPCION.reinos_palaciales;
    expect(despues['oro']!.cantidad).toBe(antes['oro']!.cantidad - tarifa.oro!);
    expect(despues['madera']!.cantidad).toBe(antes['madera']!.cantidad - tarifa.madera!);
    expect(r.eventos[0]).toMatchObject({ codigo: 'tecnologia.adoptada', asentamientoId });
  });

  it('rechazo: una tecnología que no le ha aparecido, o ya adoptada, sale `tecnologia.adopcion_invalida` y no muta', () => {
    const { sesion, faccionId } = conCobreAparecido();
    const antes = sesion.getState();

    const oculta = sesion.ejecutar(adoptarTecnologia, { faccionId, tecnologiaId: 'aleacion_bronce' }, OPC);
    const deArranque = sesion.ejecutar(adoptarTecnologia, { faccionId, tecnologiaId: 'leva_comunal' }, OPC);

    expect(oculta.codigoError).toBe('tecnologia.adopcion_invalida');
    expect(deArranque.codigoError).toBe('tecnologia.adopcion_invalida');
    expect(sesion.getState()).toBe(antes);
  });

  it('rechazo: el Rey fuera de la capital no adopta', () => {
    const { sesion: enCasa, faccionId } = conCobreAparecido();
    const payload = enCasa.exportar();
    const heroes = payload.state.heroes.map((h) => (h.id === ACTOR ? { ...h, ubicacion: { tipo: 'desconectado' as const, punto: { x: 900, y: 900 } } } : h));
    const sesion = GameSession.importar({ ...payload, state: { ...payload.state, heroes } });

    const r = sesion.ejecutar(adoptarTecnologia, { faccionId, tecnologiaId: 'metalurgia_cobre' }, OPC);

    expect(r.codigoError).toBe('tecnologia.adopcion_invalida');
  });

  it('el jugador ve la Era y las tecnologías de su Facción', () => {
    const { sesion, faccionId } = conCobreAparecido();
    const vista = proyectarParaJugador(sesion.getState(), ACTOR, SIN_GEOMETRIA).tecnologia;
    expect(vista.era).toBe('reinos_palaciales');
    expect(vista.propias?.aparecidas).toContain('metalurgia_cobre');
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)).not.toHaveProperty('tecnologias');
  });
});

describe('comprarTecnologiaAeda (Doc 6.7)', () => {
  /** La partida abastecida, con el cobre desbloqueado por otra Facción hace más que el retraso de los Aedas, tras un tick que los crea. */
  function conAedaYCobreConocido() {
    const { sesion: base, faccionId, asentamientoId } = partidaConAsentamiento();
    const payload = abastecer(base).exportar();
    const hace = instante(instanteDeTick(0) - (AEDAS.retrasoConocimientoMinutos + 1) * 60_000);
    const tecnologia = {
      ...payload.state.tecnologia,
      logros: { metalurgia_cobre: hace },
      primeros: { metalurgia_cobre: { faccionId: 'otra-faccion', en: hace } },
    } as typeof payload.state.tecnologia;
    const sesion = GameSession.importar({ ...payload, state: { ...payload.state, tecnologia } });
    sesion.avanzarTick();
    return { sesion, faccionId, asentamientoId };
  }
  const compra = { tecnologiaId: 'metalurgia_cobre' as TecnologiaId };

  it('un Aeda en la plaza se la revela a la Facción y el jugador la ve revelada y al Aeda a la vista', () => {
    const { sesion, faccionId } = conAedaYCobreConocido();
    const vista = proyectarParaJugador(sesion.getState(), ACTOR, SIN_GEOMETRIA);
    expect(vista.tecnologia.reveladas).toEqual([expect.objectContaining({ tecnologiaId: 'metalurgia_cobre', descubridorFaccionId: 'otra-faccion' })]);
    expect(vista.tecnologia.propias?.aparecidas).not.toContain('metalurgia_cobre');
    expect(vista.aedasAvistados.length).toBeGreaterThan(0);
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)).not.toHaveProperty('reveladas');
  });

  it('comprarla la hace aparecer sin hito, cobra el doble del oro de la tarifa al almacén de la plaza y deja el evento en la crónica de la Facción', () => {
    const { sesion, faccionId, asentamientoId } = conAedaYCobreConocido();
    const oroAntes = sesion.getState().asentamientos[0]!.almacen['oro']!.cantidad;

    const r = sesion.ejecutar(comprarTecnologiaAeda, { asentamientoId, ...compra }, OPC);

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    const t = estado.tecnologia.porFaccion[faccionId]!;
    expect(t.aparecidas).toContain('metalurgia_cobre');
    expect(t.adoptadas).not.toContain('metalurgia_cobre');
    expect(t.reveladas).toEqual([]);
    expect(estado.asentamientos[0]!.almacen['oro']!.cantidad).toBe(oroAntes - precioDeVenta('metalurgia_cobre'));
    expect(r.eventos[0]).toMatchObject({ codigo: 'aedas.venta', asentamientoId });
    // Y desde ahí se adopta como cualquier tecnología aparecida, pagando la tarifa.
    expect(sesion.ejecutar(adoptarTecnologia, { faccionId, tecnologiaId: 'metalurgia_cobre' }, OPC).ok).toBe(true);
  });

  it('rechazo: sin Aeda en la plaza sale `aedas.venta_invalida` y no muta', () => {
    const { sesion: sinTick, asentamientoId } = partidaConAsentamiento();
    const antes = sinTick.getState();
    expect(sinTick.ejecutar(comprarTecnologiaAeda, { asentamientoId, ...compra }, OPC).codigoError).toBe('aedas.venta_invalida');
    expect(sinTick.getState()).toBe(antes);
  });

  it('solo la compra el Rey o el Gobernador de la plaza, estando en ella', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const quien = (heroeId: string) => verificarAutorizacion('comprarTecnologiaAeda', { asentamientoId, ...compra }, sesion.getState(), { rol: 'jugador', heroeId });
    expect(quien(fundador)).toEqual({ autorizado: true });
    expect(quien(vecino)).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
    expect(MATRIZ_AUTORIZACION.comprarTecnologiaAeda.rolesPermitidos).toEqual(['jugador']);
  });
});
