// Comando `adoptarTecnologia` (`session/comandos/tecnologia.ts`, Doc 6.5): el Rey, en la capital, paga la tarifa de
// la Era con el almacén de la capital. Y lo que el jugador ve de la tecnología (Doc 6.4).
import { describe, expect, it } from 'vitest';
import { TARIFA_ADOPCION } from '../../constants';
import type { GeometriaAsentamientos } from '../estado';
import { adoptarTecnologia } from '../comandos/tecnologia';
import { GameSession } from '../gameSession';
import { proyectarParaJugador } from '../proyecciones/jugador';
import { abastecer, ACTOR, OPC, partidaConAsentamiento } from './fixtures';

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
