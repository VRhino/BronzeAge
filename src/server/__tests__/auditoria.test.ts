// Registro de auditoría de comandos (Fase E2) — el módulo por su cuenta. La integración con las rutas HTTP
// (qué se audita en cada una de las cuatro salidas de `ejecutarComandoHttp`, y el 400 de esquema que no llega
// a ella) se prueba en `api.test.ts`, contra el servidor real.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { leerAuditoria, podarAuditoria, RegistroDeAuditoria, rutaDeAuditoria } from '../auditoria';

let directorio: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-auditoria-'));
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
  vi.restoreAllMocks();
});

/** Reloj de pared controlable: la auditoría fecha con reloj de PARED (no de mundo), así que sin inyectarlo
 * los tests dependerían de la hora del sistema. */
function relojDesde(...momentos: string[]): () => string {
  let i = 0;
  return () => momentos[Math.min(i++, momentos.length - 1)]!;
}

const BASE = { gameId: 'g1', actor: 'jugador-1', comando: 'fundarAsentamiento', resultado: 'aceptado' as const };

describe('RegistroDeAuditoria', () => {
  it('escribe una linea JSON por entrada, con formato y momento', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-05T10:00:00.000Z'));
    registro.registrar({ ...BASE, version: 3, instante: 1_234_000 });
    await registro.drenar();

    const { entradas, corruptas } = await leerAuditoria(directorio, 'g1');
    expect(corruptas).toBe(0);
    expect(entradas).toEqual([
      {
        formatoVersion: 1,
        momento: '2026-09-05T10:00:00.000Z',
        gameId: 'g1',
        actor: 'jugador-1',
        comando: 'fundarAsentamiento',
        resultado: 'aceptado',
        version: 3,
        instante: 1_234_000,
      },
    ]);
  });

  it('cada partida tiene su propio archivo', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-05T10:00:00.000Z'));
    registro.registrar({ ...BASE, gameId: 'g1' });
    registro.registrar({ ...BASE, gameId: 'g2' });
    await registro.drenar();

    expect((await leerAuditoria(directorio, 'g1')).entradas).toHaveLength(1);
    expect((await leerAuditoria(directorio, 'g2')).entradas).toHaveLength(1);
  });

  it('conserva el orden aunque se registren en rafaga sin esperar', async () => {
    // Es lo que hace la ruta: `registrar` no se espera, para no meter latencia de disco en la respuesta del
    // comando. Sin la cola interna, dos `appendFile` concurrentes pueden entrelazarse y romper el JSONL.
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-05T10:00:00.000Z'));
    for (let i = 0; i < 50; i++) registro.registrar({ ...BASE, version: i });
    await registro.drenar();

    const { entradas, corruptas } = await leerAuditoria(directorio, 'g1');
    expect(corruptas).toBe(0);
    expect(entradas.map((e) => e.version)).toEqual([...Array(50).keys()]);
  });

  it('un fallo de escritura no propaga la excepcion, pero se cuenta y se grita', async () => {
    // La decisión con filo de `auditoria.ts`: con el disco lleno se sigue jugando. Lo que NO se acepta es que
    // pase inadvertido — de ahí el contador (que leerá E3) y el `console.error`.
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    // Un ARCHIVO donde el registro espera un directorio: falla igual en Windows y en POSIX (ENOTDIR/EEXIST),
    // a diferencia de inventar una ruta con caracteres que solo un sistema considera invalidos.
    const estorbo = join(directorio, 'estorbo');
    await writeFile(estorbo, 'no soy un directorio', 'utf-8');
    const registro = new RegistroDeAuditoria(join(estorbo, 'dentro'), relojDesde('2026-09-05T10:00:00.000Z'));
    registro.registrar(BASE);
    await expect(registro.drenar()).resolves.toBeUndefined();

    expect(registro.fallos).toBe(1);
    expect(error).toHaveBeenCalled();
  });
});

describe('leerAuditoria', () => {
  it('una partida sin auditoria devuelve vacio, no un error', async () => {
    await expect(leerAuditoria(directorio, 'inexistente')).resolves.toEqual({ entradas: [], corruptas: 0 });
  });

  it('una linea truncada se descarta y las demas siguen leyendose', async () => {
    // El escenario que justifica JSONL sobre un array JSON: un corte de luz a mitad de escritura. Con `[...]`
    // el archivo entero sería ilegible; aquí se pierde solo la línea a medias.
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-05T10:00:00.000Z'));
    registro.registrar({ ...BASE, version: 1 });
    registro.registrar({ ...BASE, version: 2 });
    await registro.drenar();
    const ruta = rutaDeAuditoria(directorio, 'g1');
    await writeFile(ruta, `${await readFile(ruta, 'utf-8')}{"formatoVersion":1,"gameId":"g1","act`, 'utf-8');

    const { entradas, corruptas } = await leerAuditoria(directorio, 'g1');
    expect(entradas.map((e) => e.version)).toEqual([1, 2]);
    expect(corruptas).toBe(1);
  });

  it('filtra por actor, por rechazo y por fecha', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-05T10:00:00.000Z', '2026-09-05T11:00:00.000Z', '2026-09-06T10:00:00.000Z'));
    registro.registrar({ ...BASE, actor: 'ana' });
    registro.registrar({ ...BASE, actor: 'bruno', resultado: 'rechazado', causa: 'autorizacion', detalle: 'no comanda esos escuadrones' });
    registro.registrar({ ...BASE, actor: 'ana', resultado: 'rechazado', causa: 'dominio', detalle: 'recursos.insuficientes' });
    await registro.drenar();

    expect((await leerAuditoria(directorio, 'g1', { actor: 'ana' })).entradas).toHaveLength(2);
    expect((await leerAuditoria(directorio, 'g1', { soloRechazos: true })).entradas).toHaveLength(2);
    expect((await leerAuditoria(directorio, 'g1', { desde: '2026-09-06T00:00:00.000Z' })).entradas).toHaveLength(1);
    // El cruce de filtros es el que usará la moderación de E3: "los rechazos de este actor".
    const abuso = await leerAuditoria(directorio, 'g1', { actor: 'bruno', soloRechazos: true });
    expect(abuso.entradas.map((e) => e.causa)).toEqual(['autorizacion']);
  });
});

describe('podarAuditoria', () => {
  it('conserva lo posterior al limite y descarta lo anterior', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-01T10:00:00.000Z', '2026-09-05T10:00:00.000Z', '2026-09-09T10:00:00.000Z'));
    registro.registrar({ ...BASE, version: 1 });
    registro.registrar({ ...BASE, version: 2 });
    registro.registrar({ ...BASE, version: 3 });
    await registro.drenar();

    expect(await podarAuditoria(directorio, 'g1', '2026-09-05T00:00:00.000Z')).toBe(1);
    const { entradas } = await leerAuditoria(directorio, 'g1');
    expect(entradas.map((e) => e.version)).toEqual([2, 3]);
  });

  it('no reescribe el archivo si no hay nada que descartar', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-09T10:00:00.000Z'));
    registro.registrar(BASE);
    await registro.drenar();
    const antes = await readFile(rutaDeAuditoria(directorio, 'g1'), 'utf-8');

    expect(await podarAuditoria(directorio, 'g1', '2026-09-01T00:00:00.000Z')).toBe(0);
    expect(await readFile(rutaDeAuditoria(directorio, 'g1'), 'utf-8')).toBe(antes);
  });

  it('el archivo sigue siendo legible despues de podar', async () => {
    const registro = new RegistroDeAuditoria(directorio, relojDesde('2026-09-01T10:00:00.000Z', '2026-09-09T10:00:00.000Z'));
    registro.registrar({ ...BASE, version: 1 });
    registro.registrar({ ...BASE, version: 2 });
    await registro.drenar();
    await podarAuditoria(directorio, 'g1', '2026-09-05T00:00:00.000Z');

    // Y se puede seguir añadiendo encima: podar no deja el archivo en un estado al que no se pueda apendar.
    registro.registrar({ ...BASE, version: 3 });
    await registro.drenar();
    const { entradas, corruptas } = await leerAuditoria(directorio, 'g1');
    expect(corruptas).toBe(0);
    expect(entradas.map((e) => e.version)).toEqual([2, 3]);
  });
});
