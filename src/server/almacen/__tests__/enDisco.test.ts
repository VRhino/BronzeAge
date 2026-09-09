// Adaptador de disco de `AlmacenDeObjetos`: el contrato compartido + lo que es propio del disco (`.tmp`,
// crear la raíz, propagar un fallo de escritura). Un directorio temporal real por test — un fake no
// ejercitaría el `.tmp`+`rename` ni el `ENOENT -> null`.
import { mkdtemp, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearAlmacenEnDisco } from '../enDisco';
import { pruebasDeContrato } from './contrato';

const creados: string[] = [];
async function almacenEnDiscoNuevo() {
  const dir = await mkdtemp(join(tmpdir(), 'bronzeage-almacen-'));
  creados.push(dir);
  return crearAlmacenEnDisco(dir);
}
afterEach(async () => {
  await Promise.all(creados.splice(0).map((d) => rm(d, { recursive: true, force: true })));
});

pruebasDeContrato('enDisco', almacenEnDiscoNuevo);

describe('crearAlmacenEnDisco — propio del disco', () => {
  let directorio: string;
  let almacen: ReturnType<typeof crearAlmacenEnDisco>;

  beforeEach(async () => {
    directorio = await mkdtemp(join(tmpdir(), 'bronzeage-almacen-d-'));
    creados.push(directorio);
    almacen = crearAlmacenEnDisco(directorio);
  });

  it('escribir no deja ningún .tmp tras un guardado exitoso', async () => {
    await almacen.escribir('g1.json', 'viejo');
    await almacen.escribir('g1.json', 'nuevo');
    expect((await readdir(directorio)).some((f) => f.endsWith('.tmp'))).toBe(false);
  });

  it('escribir crea la raíz si no existe', async () => {
    const nuevo = crearAlmacenEnDisco(join(directorio, 'sub', 'dir'));
    await nuevo.escribir('x.json', 'ok');
    expect(await nuevo.leer('x.json')).toBe('ok');
  });

  it('un fallo de escritura se propaga (la raíz es un archivo, no un directorio)', async () => {
    await writeFile(join(directorio, 'archivo'), 'soy un archivo', 'utf-8');
    const roto = crearAlmacenEnDisco(join(directorio, 'archivo', 'dentro'));
    await expect(roto.escribir('x.json', 'y')).rejects.toThrow();
  });
});
