// Fuente de ticks (Fase C12) + reloj de mundo con catch-up (D5, doc 10). Contrato: `intervaloTickMs` es
// opt-in — sin declararlo (como en TODOS los demás tests de este repo, que crean `RegistroDePartidas`/
// `crearServidor` sin este campo), ninguna partida avanza sola. Ver el comentario de `RegistroDePartidas`.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { RegistroDePartidas } from '../registroDePartidas';

let directorio: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-registro-'));
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

function esperar(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('RegistroDePartidas — fuente de ticks (Fase C12)', () => {
  it('sin intervaloTickMs (default), una partida abierta NO avanza sola', async () => {
    const registro = new RegistroDePartidas(directorio);
    const runner = await registro.abrir('g1', { seed: 1 });

    await esperar(100);

    expect(runner.getState().tick).toBe(0);
  });

  it('con intervaloTickMs configurado, una partida recién abierta avanza sola', async () => {
    const registro = new RegistroDePartidas(directorio, 10);
    const runner = await registro.abrir('g1', { seed: 1 });

    await esperar(150);
    runner.detenerRelojDeMundo();
    await runner.esperarColaVacia(); // apagado limpio: no dejar una escritura a disco en vuelo cuando `afterEach` borre el directorio

    expect(runner.getState().tick).toBeGreaterThan(0);
  });

  it('con intervaloTickMs configurado, una partida creada por `descartarYCrear` también avanza sola', async () => {
    const registro = new RegistroDePartidas(directorio, 10);
    const runner = await registro.descartarYCrear('g1', { seed: 1 });

    await esperar(150);
    runner.detenerRelojDeMundo();
    await runner.esperarColaVacia();

    expect(runner.getState().tick).toBeGreaterThan(0);
  });

  it('reabrir una partida guardada hace un rato ejecuta el catch-up de los ticks vencidos (D5)', async () => {
    let ahoraMs = Date.parse('2026-03-01T00:00:00.000Z');
    const reloj = () => new Date(ahoraMs).toISOString();

    // Se crea y persiste (tick 0) con un `RegistroDePartidas` que NO avanza solo.
    const primero = new RegistroDePartidas(directorio, undefined, reloj);
    await primero.abrir('g-catchup', { seed: 5 });
    await primero.cerrar();

    // "Reinicio del proceso" 4 minutos después, esta vez con reloj de mundo de 1 min por tick.
    ahoraMs += 4 * 60_000;
    const segundo = new RegistroDePartidas(directorio, 60_000, reloj);
    const runner = await segundo.abrir('g-catchup', { seed: 5 });
    await runner.esperarColaVacia();
    await segundo.cerrar();

    expect(runner.getState().tick).toBe(4);
  });
});
