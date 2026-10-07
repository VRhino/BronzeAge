// El informe del ataque a un campamento de bandidos cuenta también lo que se gana o se pierde (2026-10-07): el oro de botín de cada héroe si
// cae, y lo que se le quita del carro a la columna si aguanta.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { frenteACampamento } from './fixtures';

/** El mismo frente, con el campamento a un poder que fija el resultado (el combate con números lleva un jitter). */
function conPoder(sesion: GameSession, poder: number): GameSession {
  const payload = sesion.exportar();
  return GameSession.importar({ ...payload, state: { ...payload.state, campamentosBandidos: payload.state.campamentosBandidos.map((c) => ({ ...c, poder })) } }, { batallasEnUnity: false });
}

const atacar = (sesion: GameSession, heroeId: string) =>
  sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: heroeId });

describe('informe del ataque a bandidos', () => {
  it('si cae, dice cuánto oro de botín gana cada héroe', () => {
    const base = frenteACampamento(false);
    const r = atacar(conPoder(base.sesion, 0.01), base.fundador);
    const informe = r.eventos.find((e) => e.codigo === 'combate.campamento_destruido');
    expect((informe?.payload as { oroPorHeroe?: Record<string, number> }).oroPorHeroe?.[base.fundador]).toBeGreaterThan(0);
  });

  it('si aguanta, dice qué pierde la columna de su carro', () => {
    const base = frenteACampamento(false);
    const r = atacar(conPoder(base.sesion, 1e9), base.fundador);
    const informe = r.eventos.find((e) => e.codigo === 'combate.ataque_campamento_fallido');
    expect((informe?.payload as { carroPerdido?: Record<string, number> }).carroPerdido?.['trigo']).toBeGreaterThan(0);
  });
});
