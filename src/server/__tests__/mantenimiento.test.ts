// Pasada de mantenimiento (Fase E2): respaldar, podar respaldos, podar auditoría. Se prueba
// `ejecutarPasada()` directamente en vez de esperar al temporizador — lo que importa es QUÉ hace, no cuándo;
// el "cuándo" es un `setInterval` de tres líneas.
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GameSession } from '../../session/gameSession';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { guardarPartida } from '../persistenciaPartida';
import { leerAuditoria, RegistroDeAuditoria } from '../auditoria';
import { listarRespaldos } from '../respaldos';
import { TareaDeMantenimiento } from '../mantenimiento';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-mantenimiento-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
  vi.restoreAllMocks();
});

async function partidaEnDisco(gameId: string): Promise<void> {
  const sesion = GameSession.crear(gameId, { seed: 42 });
  sesion.ejecutar(crearFaccion, { nombre: 'Una' }, { actor: 'jugador-1' });
  await guardarPartida(almacen, sesion, '2026-09-05T10:00:00.000Z');
}

/** Reloj de pared controlable: la pasada fecha respaldos y calcula el límite de retención con él. */
function relojDesde(...momentos: string[]): () => string {
  let i = 0;
  return () => momentos[Math.min(i++, momentos.length - 1)]!;
}

describe('TareaDeMantenimiento', () => {
  it('respalda TODAS las partidas del directorio, no solo las abiertas', async () => {
    // Trabaja sobre el directorio y no sobre el registro: una partida que este proceso nunca abrió también
    // necesita respaldo (mismo criterio que `listarPartidas`, C12).
    await partidaEnDisco('g1');
    await partidaEnDisco('g2');
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));

    const resumen = await tarea.ejecutarPasada();

    expect(resumen).toMatchObject({ partidas: 2, respaldadas: 2, fallos: [] });
    expect(await listarRespaldos(directorio, 'g1')).toHaveLength(1);
    expect(await listarRespaldos(directorio, 'g2')).toHaveLength(1);
  });

  it('pasadas sucesivas acumulan respaldos hasta el limite, y ahi se estabiliza', async () => {
    await partidaEnDisco('g1');
    const reloj = relojDesde(
      '2026-09-01T10:00:00.000Z',
      '2026-09-02T10:00:00.000Z',
      '2026-09-03T10:00:00.000Z',
      '2026-09-04T10:00:00.000Z'
    );
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000, respaldosAConservar: 2 }, reloj);

    await tarea.ejecutarPasada();
    await tarea.ejecutarPasada();
    const tercera = await tarea.ejecutarPasada();

    // A partir del límite, cada pasada añade uno y borra el más viejo: el directorio no crece sin techo.
    expect(tercera.respaldosBorrados).toBe(1);
    const respaldos = await listarRespaldos(directorio, 'g1');
    expect(respaldos.map((r) => r.momento)).toEqual(['2026-09-03T10:00:00.000Z', '2026-09-02T10:00:00.000Z']);
  });

  it('poda la auditoria vencida por EDAD y conserva la reciente', async () => {
    await partidaEnDisco('g1');
    const registro = new RegistroDeAuditoria(almacen, relojDesde('2026-07-01T10:00:00.000Z', '2026-09-04T10:00:00.000Z'));
    registro.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado', version: 1 });
    registro.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado', version: 2 });
    await registro.drenar();
    // Retención de 30 días desde el 5 de septiembre: la línea de julio vence, la del 4 de septiembre no.
    const tarea = new TareaDeMantenimiento(
      almacen,
      directorio,
      { intervaloMs: 60_000, retencionAuditoriaDias: 30 },
      relojDesde('2026-09-05T10:00:00.000Z')
    );

    const resumen = await tarea.ejecutarPasada();

    expect(resumen.auditoriaBorrada).toBe(1);
    const { entradas } = await leerAuditoria(almacen, 'g1');
    expect(entradas.map((e) => e.version)).toEqual([2]);
  });

  it('un fallo en una partida no impide respaldar las demas', async () => {
    // El mantenimiento es trabajo de fondo: que una partida esté rota no es razón para dejar al resto del
    // servidor sin respaldo.
    vi.spyOn(console, 'error').mockImplementation(() => {});
    await partidaEnDisco('g1');
    await partidaEnDisco('g2');
    // Un DIRECTORIO donde el respaldo de `g1` espera escribir un archivo: la copia falla, la de `g2` no.
    const { mkdir } = await import('node:fs/promises');
    await mkdir(join(directorio, 'respaldos', 'g1--2026-09-05T11-00-00.000Z.json'), { recursive: true });

    const resumen = await tarea().ejecutarPasada();

    expect(resumen.fallos.map((f) => f.gameId)).toEqual(['g1']);
    expect(resumen.respaldadas).toBe(1);
    expect(await listarRespaldos(directorio, 'g2')).toHaveLength(1);

    function tarea() {
      return new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));
    }
  });

  it('una pasada no arranca si otra sigue en curso', async () => {
    await partidaEnDisco('g1');
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));

    // Sin la guarda de reentrada, dos pasadas simultáneas competirían por los mismos archivos: una podando lo
    // que la otra acaba de escribir.
    const [primera, segunda] = await Promise.all([tarea.ejecutarPasada(), tarea.ejecutarPasada()]);

    expect(primera.respaldadas + segunda.respaldadas).toBe(1);
    expect(await listarRespaldos(directorio, 'g1')).toHaveLength(1);
  });

  it('iniciar dos veces no duplica el temporizador', async () => {
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));
    const intervalos = vi.spyOn(globalThis, 'setInterval');

    tarea.iniciar();
    tarea.iniciar();
    tarea.detener();

    expect(intervalos).toHaveBeenCalledTimes(1);
  });

  it('un directorio sin ninguna partida no falla ni escribe nada', async () => {
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));
    await expect(tarea.ejecutarPasada()).resolves.toMatchObject({ partidas: 0, respaldadas: 0, fallos: [] });
  });

  it('un snapshot ilegible en el directorio no impide respaldar las demas partidas', async () => {
    await partidaEnDisco('g1');
    await writeFile(join(directorio, 'g2.json'), 'esto no es json', 'utf-8');
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const tarea = new TareaDeMantenimiento(almacen, directorio, { intervaloMs: 60_000 }, relojDesde('2026-09-05T11:00:00.000Z'));

    // Encontrado escribiendo este test: `listarPartidas` lanzaba con un JSON ilegible y abortaba la pasada
    // ANTES del bucle, así que ninguna partida llegaba a respaldarse. El mismo fallo tumbaba
    // `GET /admin/partidas` con un 500 — corregido en `persistenciaPartida.ts`, que ahora excluye el archivo
    // roto del listado y lo grita por consola.
    const resumen = await tarea.ejecutarPasada();
    expect(resumen.partidas).toBe(1);
    expect(await listarRespaldos(directorio, 'g1')).toHaveLength(1);
  });
});
