// Comandos de expansión (`session/comandos/expansion.ts`).
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { desarmarCaravanaFundacion, lanzarCaravanaFundacion } from '../comandos/expansion';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-test' };

function partidaConAsentamiento() {
  const sesion = GameSession.crear('expansion-test', { seed: 42 });
  const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
  const r = sesion.ejecutar(fundarAsentamiento, { faccionId, posicion: { x: 500, y: 500 }, numJugadores: 2 }, OPC);
  return { sesion, faccionId, asentamientoId: r.datos!.asentamientoId };
}

describe('lanzarCaravanaFundacion', () => {
  it('rechazo: asentamiento inexistente devuelve codigoError, NO revienta', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: 'no-existe', destino: { x: 700, y: 700 }, numJugadores: 1 }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('asentamiento.no_existe');
  });

  it('rechazo: un asentamiento de nivel 1 todavía no puede expandir, y no muta nada', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();
    const r = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId, destino: { x: 700, y: 700 }, numJugadores: 1 }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('expansion.invalida');
    expect(sesion.getState()).toBe(antes);
  });
});

describe('desarmarCaravanaFundacion', () => {
  it('rechazo: caravana inexistente tiene su propio código', () => {
    const { sesion } = partidaConAsentamiento();
    const r = sesion.ejecutar(desarmarCaravanaFundacion, { caravanaId: 'no-existe' }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('caravana.no_existe');
  });

  it('rechazo: una caravana que no es de Fundación se rechaza como error de dominio', () => {
    // Se inyecta una caravana comercial a mano: `desarmarCaravanaFundacion` solo acepta las de tipo
    // 'construccion' con destino, y aquí interesa comprobar que ese rechazo del motor llega traducido.
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const payload = sesion.exportar();
    const conCaravana = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        caravanas: [
          {
            id: 'car-1',
            tipo: 'comercial',
            origenAsentamientoId: asentamientoId,
            posicionActual: { x: 500, y: 500 },
            estado: 'disponible',
            contenido: {},
            progreso: 0,
          },
        ],
      },
    });

    const r = conCaravana.ejecutar(desarmarCaravanaFundacion, { caravanaId: 'car-1' }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('expansion.invalida');
    expect(conCaravana.getState().caravanas).toHaveLength(1);
  });
});
