// Comando `solicitarAscenso` (`session/comandos/ascenso.ts`). La regla (gates, coste, solvencia, cupo) se prueba en
// el motor (`engine/__tests__/ascenso.test.ts`) y quién puede pedirla en `autorizacionComandos.test.ts`; aquí solo
// el enganche con la partida: el código de error estable, que un rechazo no toca nada, y que el éxito se guarda.
import { describe, expect, it } from 'vitest';
import { prepararParaSubirANivel2 } from '../../engine/__tests__/fixtures';
import { solicitarAscenso } from '../comandos/ascenso';
import { GameSession } from '../gameSession';
import { OPC, partidaConAsentamiento } from './fixtures';

/** La misma partida con su asentamiento listo para subir a nivel 2, sobre el mapa de la propia partida. */
function listaParaSubir(original: GameSession): GameSession {
  const payload = original.exportar();
  const listo = prepararParaSubirANivel2(payload.state.asentamientos[0]!, original.getMapa());
  return GameSession.importar({ ...payload, state: { ...payload.state, asentamientos: [listo] } });
}

describe('solicitarAscenso', () => {
  it('rechazo: un asentamiento recién fundado no cumple nada, sale `ascenso.invalido` y no muta', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const antes = sesion.getState();

    const r = sesion.ejecutar(solicitarAscenso, { asentamientoId }, OPC);

    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('ascenso.invalido');
    expect(sesion.getState()).toBe(antes);
  });

  it('éxito: arranca la obra hacia el nivel 2 sin subir todavía, y el evento lleva el asentamiento', () => {
    const { sesion: base, asentamientoId } = partidaConAsentamiento();
    const sesion = listaParaSubir(base);

    const r = sesion.ejecutar(solicitarAscenso, { asentamientoId }, OPC);

    expect(r.ok).toBe(true);
    const a = sesion.getState().asentamientos[0]!;
    expect(a.nivel).toBe(1);
    expect(a.ascenso?.nivelObjetivo).toBe(2);
    expect(r.eventos[0]).toMatchObject({ codigo: 'asentamiento.ascenso_iniciado', asentamientoId });
  });
});
