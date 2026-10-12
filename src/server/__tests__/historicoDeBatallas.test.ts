// Histórico de batallas (`historicoDeBatallas.ts`): una batalla cerrada se conserva en el estado el tiempo que dice
// `BATALLA.conservarCerradasMinutos` y después pasa, con el id de la partida, al JSONL hermano del snapshot.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { RunnerDePartida } from '../runnerDePartida';
import { guardarPartida } from '../persistenciaPartida';
import { anexarHistoricoDeBatallas, claveDeHistoricoDeBatallas, leerHistoricoDeBatallas } from '../historicoDeBatallas';
import { frenteACampamento } from '../../session/__tests__/fixtures';
import { BATALLA } from '../../constants';

const MOMENTO = '2026-01-01T00:00:00.000Z';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-historico-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

async function conBatallaFallida() {
  const { sesion, fundador } = frenteACampamento();
  await guardarPartida(almacen, sesion, MOMENTO);
  const runner = await RunnerDePartida.cargarOCrear(sesion.gameId, { seed: 1 }, { almacen, ahora: () => MOMENTO, sesion: { batallasEnUnity: true } });
  const abierta = await runner.ejecutar('atacar', { heroeId: fundador, objetivo: { tipo: 'campamento', id: 'camp-1' } }, fundador);
  expect(abierta.ok).toBe(true);
  for (let i = 0; i < BATALLA.plazoAsignacionMinutos; i++) await runner.avanzarTick(); // nadie la asigna: fallida
  return { runner, gameId: sesion.gameId, battleId: abierta.datos!.battleId as string };
}

describe('histórico de batallas', () => {
  it('la cerrada se queda en el estado durante la retención y después pasa al histórico con el id de la partida', async () => {
    const { runner, gameId, battleId } = await conBatallaFallida();
    expect(runner.getState().batallas.map((b) => [b.id, b.estado])).toEqual([[battleId, 'fallida']]);

    for (let i = 0; i < BATALLA.conservarCerradasMinutos - 1; i++) await runner.avanzarTick();
    expect(runner.getState().batallas, 'aún dentro de la retención').toHaveLength(1);
    expect(await leerHistoricoDeBatallas(almacen, gameId)).toEqual([]);

    await runner.avanzarTick();
    expect(runner.getState().batallas).toEqual([]);
    const historico = await leerHistoricoDeBatallas(almacen, gameId);
    expect(historico).toHaveLength(1);
    expect(historico[0]).toMatchObject({ gameId, archivadaEn: MOMENTO, batalla: { id: battleId, estado: 'fallida' } });
  });

  it('el snapshot guardado ya no la lleva, y una caída justo después no la pierde ni la duplica', async () => {
    const { runner, gameId } = await conBatallaFallida();
    for (let i = 0; i < BATALLA.conservarCerradasMinutos; i++) await runner.avanzarTick();

    const recuperada = await RunnerDePartida.cargarOCrear(gameId, { seed: 1 }, { almacen, ahora: () => MOMENTO, sesion: { batallasEnUnity: true } });
    expect(recuperada.getState().batallas).toEqual([]);
    expect(await leerHistoricoDeBatallas(almacen, gameId)).toHaveLength(1);
  });

  it('una batalla repetida en el fichero (corte entre anexar y anotar) se lee una sola vez', async () => {
    const { runner, gameId } = await conBatallaFallida();
    const [batalla] = runner.getState().batallas;
    await anexarHistoricoDeBatallas(almacen, gameId, [batalla!, batalla!], MOMENTO);
    await almacen.anexar(claveDeHistoricoDeBatallas(gameId), '{"gameId":"a medias');

    expect(await leerHistoricoDeBatallas(almacen, gameId)).toHaveLength(1);
  });
});
