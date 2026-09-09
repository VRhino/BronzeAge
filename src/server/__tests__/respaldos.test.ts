// Respaldos y restauración (Fase E2). La prueba que da sentido al módulo entero es
// "restaurar de verdad devuelve la partida al punto respaldado" — un respaldo que nunca se ha restaurado no
// se sabe si es un respaldo. Todo lo demás de este archivo existe para sostener esa.
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import { crearFaccion } from '../../session/comandos/crearFaccion';
import { cargarPartida, guardarPartida } from '../persistenciaPartida';
import { RegistroDeAuditoria } from '../auditoria';
import {
  DIRECTORIO_RESPALDOS,
  listarRespaldos,
  podarRespaldos,
  respaldarPartida,
  restaurarPartida,
  RespaldoInservibleError,
} from '../respaldos';

let directorio: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-respaldos-'));
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

/** Una partida guardada en disco, con `n` Facciones creadas — cada una sube `version`, que es lo que después
 * distingue el estado respaldado del estado posterior. */
async function partidaConFacciones(gameId: string, n: number): Promise<GameSession> {
  const sesion = GameSession.crear(gameId, { seed: 42 });
  for (let i = 0; i < n; i++) sesion.ejecutar(crearFaccion, { nombre: `Faccion ${i}` }, { actor: `jugador-${i}` });
  await guardarPartida(directorio, sesion, '2026-09-05T10:00:00.000Z');
  return sesion;
}

describe('respaldarPartida', () => {
  it('copia el snapshot sin tocar el vigente', async () => {
    await partidaConFacciones('g1', 2);
    const antes = await readFile(join(directorio, 'g1.json'), 'utf-8');

    const respaldo = await respaldarPartida(directorio, 'g1', '2026-09-05T11:22:33.444Z');

    expect(respaldo).not.toBeNull();
    expect(respaldo!.bytes).toBeGreaterThan(0);
    // El snapshot vigente es intocable: un respaldo no puede ser una operación con riesgo sobre la partida.
    expect(await readFile(join(directorio, 'g1.json'), 'utf-8')).toBe(antes);
    expect(await readFile(join(directorio, DIRECTORIO_RESPALDOS, respaldo!.archivo), 'utf-8')).toBe(antes);
  });

  it('una partida sin snapshot todavia devuelve null, no un error', async () => {
    await expect(respaldarPartida(directorio, 'inexistente', '2026-09-05T11:00:00.000Z')).resolves.toBeNull();
  });

  it('se lleva la auditoria consigo', async () => {
    // Restaurar sin su auditoría dejaría el registro narrando una partida que ya no es la que hay.
    await partidaConFacciones('g1', 1);
    const registro = new RegistroDeAuditoria(directorio, () => '2026-09-05T10:00:00.000Z');
    registro.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado', version: 1 });
    await registro.drenar();

    await respaldarPartida(directorio, 'g1', '2026-09-05T11:22:33.444Z');

    const copiados = await readdir(join(directorio, DIRECTORIO_RESPALDOS));
    expect(copiados.some((a) => a.endsWith('.auditoria.jsonl'))).toBe(true);
  });

  it('el nombre del archivo no lleva ":" — inservible en Windows — y conserva el momento exacto', async () => {
    await partidaConFacciones('g1', 1);
    const respaldo = await respaldarPartida(directorio, 'g1', '2026-09-05T11:22:33.444Z');

    expect(respaldo!.archivo).not.toContain(':');
    // Y la conversión es reversible: `listarRespaldos` recupera el ISO original del propio nombre.
    expect((await listarRespaldos(directorio, 'g1'))[0]!.momento).toBe('2026-09-05T11:22:33.444Z');
  });
});

describe('listarRespaldos', () => {
  it('devuelve del mas reciente al mas antiguo, y solo los de esa partida', async () => {
    await partidaConFacciones('g1', 1);
    await partidaConFacciones('g2', 1);
    await respaldarPartida(directorio, 'g1', '2026-09-01T10:00:00.000Z');
    await respaldarPartida(directorio, 'g1', '2026-09-09T10:00:00.000Z');
    await respaldarPartida(directorio, 'g1', '2026-09-05T10:00:00.000Z');
    await respaldarPartida(directorio, 'g2', '2026-09-07T10:00:00.000Z');

    const respaldos = await listarRespaldos(directorio, 'g1');
    expect(respaldos.map((r) => r.momento)).toEqual([
      '2026-09-09T10:00:00.000Z',
      '2026-09-05T10:00:00.000Z',
      '2026-09-01T10:00:00.000Z',
    ]);
  });

  it('sin ningun respaldo devuelve vacio, no un error', async () => {
    await expect(listarRespaldos(directorio, 'g1')).resolves.toEqual([]);
  });
});

describe('restaurarPartida — la prueba de restauracion', () => {
  it('devuelve la partida al estado exacto del respaldo, deshaciendo lo posterior', async () => {
    // El escenario completo, de punta a punta: se juega, se respalda, se sigue jugando, se restaura.
    const sesion = await partidaConFacciones('g1', 2);
    const versionRespaldada = sesion.getState().version;
    const faccionesRespaldadas = sesion.getState().facciones.map((f) => f.nombre);
    const respaldo = await respaldarPartida(directorio, 'g1', '2026-09-05T11:00:00.000Z');

    // La partida sigue: dos Facciones más, guardadas encima del snapshot anterior.
    sesion.ejecutar(crearFaccion, { nombre: 'Posterior A' }, { actor: 'jugador-a' });
    sesion.ejecutar(crearFaccion, { nombre: 'Posterior B' }, { actor: 'jugador-b' });
    await guardarPartida(directorio, sesion, '2026-09-05T12:00:00.000Z');
    expect((await cargarPartida(directorio, 'g1'))!.sesion.getState().facciones).toHaveLength(4);

    const versionRestaurada = await restaurarPartida(directorio, 'g1', respaldo!.archivo);

    expect(versionRestaurada).toBe(versionRespaldada);
    const recuperada = (await cargarPartida(directorio, 'g1'))!.sesion;
    expect(recuperada.getState().version).toBe(versionRespaldada);
    expect(recuperada.getState().facciones.map((f) => f.nombre)).toEqual(faccionesRespaldadas);
    // Y lo posterior al respaldo desapareció de verdad, no quedó mezclado.
    expect(recuperada.getState().facciones.map((f) => f.nombre)).not.toContain('Posterior A');
  });

  it('un respaldo que no existe falla sin tocar el snapshot vigente', async () => {
    await partidaConFacciones('g1', 2);
    const antes = await readFile(join(directorio, 'g1.json'), 'utf-8');

    await expect(restaurarPartida(directorio, 'g1', 'g1--2020-01-01T00-00-00.000Z.json')).rejects.toThrow(RespaldoInservibleError);
    expect(await readFile(join(directorio, 'g1.json'), 'utf-8')).toBe(antes);
  });

  it('un respaldo CORRUPTO falla sin tocar el snapshot vigente — la garantia del modulo', async () => {
    // Es el caso que justifica verificar antes de sustituir: si `restaurarPartida` fuera un `copyFile`, esto
    // dejaría la partida buena machacada por basura y sin forma de volver.
    await partidaConFacciones('g1', 2);
    const antes = await readFile(join(directorio, 'g1.json'), 'utf-8');
    const respaldo = await respaldarPartida(directorio, 'g1', '2026-09-05T11:00:00.000Z');
    await writeFile(join(directorio, DIRECTORIO_RESPALDOS, respaldo!.archivo), '{"formatoVersion":7,"parti', 'utf-8');

    await expect(restaurarPartida(directorio, 'g1', respaldo!.archivo)).rejects.toThrow(RespaldoInservibleError);
    expect(await readFile(join(directorio, 'g1.json'), 'utf-8')).toBe(antes);
  });

  it('no deja el directorio de verificacion tirado, ni cuando falla', async () => {
    await partidaConFacciones('g1', 1);
    const respaldo = await respaldarPartida(directorio, 'g1', '2026-09-05T11:00:00.000Z');
    await restaurarPartida(directorio, 'g1', respaldo!.archivo);
    await expect(restaurarPartida(directorio, 'g1', 'no-existe.json')).rejects.toThrow();

    const dentro = await readdir(join(directorio, DIRECTORIO_RESPALDOS));
    expect(dentro.filter((a) => a.startsWith('.verificacion'))).toEqual([]);
  });
});

describe('podarRespaldos', () => {
  it('conserva los N mas recientes y borra el resto, con su auditoria adjunta', async () => {
    await partidaConFacciones('g1', 1);
    const registro = new RegistroDeAuditoria(directorio, () => '2026-09-01T10:00:00.000Z');
    registro.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado' });
    await registro.drenar();
    for (const dia of ['01', '03', '05', '07', '09']) await respaldarPartida(directorio, 'g1', `2026-09-${dia}T10:00:00.000Z`);

    expect(await podarRespaldos(directorio, 'g1', 2)).toBe(3);

    const quedan = await listarRespaldos(directorio, 'g1');
    expect(quedan.map((r) => r.momento)).toEqual(['2026-09-09T10:00:00.000Z', '2026-09-07T10:00:00.000Z']);
    // Las auditorías adjuntas de los borrados no quedan huérfanas ocupando sitio.
    const archivos = await readdir(join(directorio, DIRECTORIO_RESPALDOS));
    expect(archivos.filter((a) => a.endsWith('.auditoria.jsonl'))).toHaveLength(2);
  });

  it('nunca borra por debajo del minimo pedido, aunque haya menos respaldos que ese minimo', async () => {
    await partidaConFacciones('g1', 1);
    await respaldarPartida(directorio, 'g1', '2026-09-01T10:00:00.000Z');

    expect(await podarRespaldos(directorio, 'g1', 5)).toBe(0);
    expect(await listarRespaldos(directorio, 'g1')).toHaveLength(1);
  });

  it('conservar 0 es un error: dejaria la partida sin ningun punto de retorno', async () => {
    await expect(podarRespaldos(directorio, 'g1', 0)).rejects.toThrow(RangeError);
  });
});
