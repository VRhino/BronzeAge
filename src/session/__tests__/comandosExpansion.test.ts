// Comandos de expansión (`session/comandos/expansion.ts`).
//
// Los rechazos por ENTIDAD INEXISTENTE (en `GameStore` eran `.find(...)!` y reventaban) están unificados en
// `comandosContratoIds.test.ts`, no repetidos aquí.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { desarmarCaravanaFundacion, lanzarCaravanaFundacion } from '../comandos/expansion';
import { OPC, partidaConAsentamiento } from './fixtures';

describe('lanzarCaravanaFundacion', () => {
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
