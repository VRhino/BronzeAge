// Persistencia del dominio de acceso (cierre de Fase C): usuarios, sesiones y membresías sobreviven a un
// reinicio del proceso. Se prueba a través del disco de verdad (`mkdtemp`), no un fake en memoria — un fake
// no ejercitaría el `rename` atómico ni la recarga desde archivo, que es justo lo que aquí importa.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearRepositorioIdentidadEnDisco } from '../identidad/repositorioEnDisco';
import { FormatoIdentidadNoSoportadoError, leerIdentidad } from '../persistenciaIdentidad';

let directorio: string;
let ruta: string;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-identidad-'));
  ruta = join(directorio, 'identidad.json');
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

const AHORA = '2026-01-01T00:00:00.000Z';

describe('crearRepositorioIdentidadEnDisco', () => {
  it('arranca vacío cuando el archivo no existe todavía', async () => {
    const { repositorio } = await crearRepositorioIdentidadEnDisco(ruta);
    expect(repositorio.obtenerUsuario('usuario-1')).toBeUndefined();
  });

  it('un usuario, su identidad, su sesión y su membresía sobreviven a un "reinicio del proceso"', async () => {
    const primero = await crearRepositorioIdentidadEnDisco(ruta);
    const usuario = primero.repositorio.crearUsuario({ creadoEn: AHORA });
    primero.repositorio.vincularIdentidad({ usuarioId: usuario.id, proveedor: 'dev', sujetoId: 'ana', vinculadaEn: AHORA });
    primero.repositorio.crearSesion({ id: 's1', usuarioId: usuario.id, emitidaEn: AHORA, expiraEn: '2026-01-02T00:00:00.000Z' });
    primero.repositorio.otorgarMembresia({ usuarioId: usuario.id, gameId: 'g1', jugadorId: null, rol: 'moderador', desde: AHORA });
    await primero.esperarEscrituras();

    // Un segundo repositorio sobre el MISMO archivo = el proceso reiniciado.
    const segundo = await crearRepositorioIdentidadEnDisco(ruta);
    expect(segundo.repositorio.obtenerUsuario(usuario.id)).toEqual(usuario);
    expect(segundo.repositorio.buscarUsuarioPorIdentidadExterna('dev', 'ana')?.id).toBe(usuario.id);
    expect(segundo.repositorio.buscarIdentidadDeUsuario(usuario.id)?.sujetoId).toBe('ana');
    expect(segundo.repositorio.buscarSesion('s1')?.usuarioId).toBe(usuario.id);
    expect(segundo.repositorio.obtenerMembresia(usuario.id, 'g1')?.rol).toBe('moderador');
  });

  it('no reasigna ids de usuario tras recargar: el contador retoma desde el mayor existente', async () => {
    const primero = await crearRepositorioIdentidadEnDisco(ruta);
    const u1 = primero.repositorio.crearUsuario({ creadoEn: AHORA });
    const u2 = primero.repositorio.crearUsuario({ creadoEn: AHORA });
    await primero.esperarEscrituras();

    const segundo = await crearRepositorioIdentidadEnDisco(ruta);
    const u3 = segundo.repositorio.crearUsuario({ creadoEn: AHORA });
    expect([u1.id, u2.id, u3.id]).toEqual(['usuario-1', 'usuario-2', 'usuario-3']);
    // `crearUsuario` encola una escritura en segundo plano: hay que dejarla terminar antes de que
    // `afterEach` borre el directorio, o el `rename` en vuelo choca con el `rmdir` (ENOTEMPTY en Windows).
    await segundo.esperarEscrituras();
  });

  it('revocar una membresía persiste el `hasta`', async () => {
    const primero = await crearRepositorioIdentidadEnDisco(ruta);
    primero.repositorio.otorgarMembresia({ usuarioId: 'usuario-1', gameId: 'g1', jugadorId: null, rol: 'observador', desde: AHORA });
    expect(primero.repositorio.revocarMembresia('usuario-1', 'g1', '2026-06-01T00:00:00.000Z')).toBe(true);
    expect(primero.repositorio.revocarMembresia('usuario-1', 'ausente', AHORA)).toBe(false);
    await primero.esperarEscrituras();

    const segundo = await crearRepositorioIdentidadEnDisco(ruta);
    expect(segundo.repositorio.obtenerMembresia('usuario-1', 'g1')?.hasta).toBe('2026-06-01T00:00:00.000Z');
  });

  it('escritura atómica: el archivo final siempre es JSON válido y completo', async () => {
    const { repositorio, esperarEscrituras } = await crearRepositorioIdentidadEnDisco(ruta);
    for (let i = 0; i < 5; i++) repositorio.crearUsuario({ creadoEn: AHORA });
    await esperarEscrituras();

    const contenido = JSON.parse(await readFile(ruta, 'utf-8'));
    expect(contenido.formatoVersion).toBe(1);
    expect(contenido.datos.usuarios).toHaveLength(5);
  });

  it('rechaza un archivo de un formato de envoltorio desconocido', async () => {
    await writeFile(ruta, JSON.stringify({ formatoVersion: 99, datos: {} }), 'utf-8');
    await expect(leerIdentidad(ruta)).rejects.toBeInstanceOf(FormatoIdentidadNoSoportadoError);
  });
});
