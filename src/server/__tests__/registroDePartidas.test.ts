// Fuente de ticks (Fase C12) + reloj de mundo con catch-up (D5, doc 10). Contrato: `intervaloTickMs` es
// opt-in — sin declararlo (como en TODOS los demás tests de este repo, que crean `RegistroDePartidas`/
// `crearServidor` sin este campo), ninguna partida avanza sola. Ver el comentario de `RegistroDePartidas`.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { RegistroDePartidas } from '../registroDePartidas';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-registro-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

function esperar(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('RegistroDePartidas — fuente de ticks (Fase C12)', () => {
  it('sin intervaloTickMs (default), una partida abierta NO avanza sola', async () => {
    const registro = new RegistroDePartidas(almacen);
    const runner = await registro.abrir('g1', { seed: 1 });

    await esperar(100);

    expect(runner.getState().tick).toBe(0);
  });

  it('con intervaloTickMs configurado, una partida recién abierta avanza sola', async () => {
    const registro = new RegistroDePartidas(almacen, 10);
    const runner = await registro.abrir('g1', { seed: 1 });

    await esperar(150);
    runner.detenerRelojDeMundo();
    await runner.esperarColaVacia(); // apagado limpio: no dejar una escritura a disco en vuelo cuando `afterEach` borre el directorio

    expect(runner.getState().tick).toBeGreaterThan(0);
  });

  it('con intervaloTickMs configurado, una partida creada por `descartarYCrear` también avanza sola', async () => {
    const registro = new RegistroDePartidas(almacen, 10);
    const runner = await registro.descartarYCrear('g1', { seed: 1 });

    await esperar(150);
    runner.detenerRelojDeMundo();
    await runner.esperarColaVacia();

    expect(runner.getState().tick).toBeGreaterThan(0);
  });

  it('descartarYCrear con intervaloTickMs propio arranca el reloj aunque el proceso no lo tenga', async () => {
    const registro = new RegistroDePartidas(almacen); // sin default de proceso: ninguna partida avanzaría sola
    const runner = await registro.descartarYCrear('g1', { seed: 1 }, 10);

    await esperar(120);
    expect(runner.intervaloRelojDeMundoMs()).toBe(10);
    expect(runner.getState().tick).toBeGreaterThan(0);

    runner.detenerRelojDeMundo();
    await runner.esperarColaVacia();
  });

  it('descartarYCrear detiene el runner anterior — no sigue persistiendo el snapshot por detrás', async () => {
    // Regresión: si el reloj del runner viejo sigue vivo tras `descartarYCrear`, sigue escribiendo el
    // snapshot en cada tick y el runner nuevo (versión 0) choca al primer comando con "alguien más escribió
    // este snapshot primero".
    const registro = new RegistroDePartidas(almacen, 10);
    const anterior = await registro.abrir('g1', { seed: 1 });

    await esperar(200);
    const tickAlDescartar = anterior.getState().tick;
    expect(tickAlDescartar).toBeGreaterThan(4);

    const nuevo = await registro.descartarYCrear('g1', { seed: 2 });
    await esperar(50);
    nuevo.detenerRelojDeMundo();
    await nuevo.esperarColaVacia();

    // El viejo quedó congelado en el tick que tenía: su reloj se paró antes de crear el nuevo.
    expect(anterior.getState().tick).toBe(tickAlDescartar);
    // El nuevo arrancó de cero, ajeno a la versión alta que el viejo tenía en disco.
    expect(nuevo.getState().tick).toBeLessThan(tickAlDescartar);
  });

  it('reabrir una partida guardada hace un rato la reanuda donde estaba, sin catch-up', async () => {
    // De extremo a extremo, la decisión del 2026-09-05: el mundo no avanza mientras el servidor está caído.
    // Este test probaba justo lo contrario hasta esa fecha (era la verificación de D5).
    let ahoraMs = Date.parse('2026-03-01T00:00:00.000Z');
    const reloj = () => new Date(ahoraMs).toISOString();

    // Se crea y persiste (tick 0) con un `RegistroDePartidas` que NO avanza solo.
    const primero = new RegistroDePartidas(almacen, undefined, reloj);
    await primero.abrir('g-catchup', { seed: 5 });
    await primero.cerrar();

    // "Reinicio del proceso" 4 minutos después, esta vez con reloj de mundo de 1 min por tick.
    ahoraMs += 4 * 60_000;
    const segundo = new RegistroDePartidas(almacen, 60_000, reloj);
    const runner = await segundo.abrir('g-catchup', { seed: 5 });
    await runner.esperarColaVacia();
    await segundo.cerrar();

    // Los 4 minutos de proceso caído no cuentan: la partida sigue en el tick en que se guardó.
    expect(runner.getState().tick).toBe(0);
  });
});
