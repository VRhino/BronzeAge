import { describe, expect, it } from 'vitest';
import { crearRepositorioIdentidadEnMemoria } from '../repositorio';

describe('crearRepositorioIdentidadEnMemoria', () => {
  it('crea usuarios con id incremental y sin deshabilitar', () => {
    const repo = crearRepositorioIdentidadEnMemoria();
    const u1 = repo.crearUsuario({ creadoEn: '2026-01-01T00:00:00.000Z' });
    const u2 = repo.crearUsuario({ creadoEn: '2026-01-01T00:00:01.000Z' });
    expect(u1.id).not.toBe(u2.id);
    expect(u1.deshabilitado).toBe(false);
    expect(repo.obtenerUsuario(u1.id)).toEqual(u1);
  });

  it('vincula identidad externa y permite buscar por ella', () => {
    const repo = crearRepositorioIdentidadEnMemoria();
    const usuario = repo.crearUsuario({ creadoEn: '2026-01-01T00:00:00.000Z' });
    repo.vincularIdentidad({ usuarioId: usuario.id, proveedor: 'dev', sujetoId: 'ana', vinculadaEn: '2026-01-01T00:00:00.000Z' });

    expect(repo.buscarUsuarioPorIdentidadExterna('dev', 'ana')).toEqual(usuario);
    expect(repo.buscarUsuarioPorIdentidadExterna('dev', 'otra')).toBeUndefined();
    expect(repo.buscarUsuarioPorIdentidadExterna('otro-proveedor', 'ana')).toBeUndefined();
  });

  it('guarda y recupera sesiones por id', () => {
    const repo = crearRepositorioIdentidadEnMemoria();
    const sesion = { id: 's1', usuarioId: 'usuario-1', emitidaEn: '2026-01-01T00:00:00.000Z', expiraEn: '2026-01-01T12:00:00.000Z' };
    repo.crearSesion(sesion);
    expect(repo.buscarSesion('s1')).toEqual(sesion);
    expect(repo.buscarSesion('inexistente')).toBeUndefined();
  });

  it('otorga y consulta membresias por usuario+partida', () => {
    const repo = crearRepositorioIdentidadEnMemoria();
    const membresia = { usuarioId: 'usuario-1', gameId: 'partida-1', jugadorId: 'jugador-1', faccionId: null, rol: 'jugador' as const, desde: '2026-01-01T00:00:00.000Z' };
    repo.otorgarMembresia(membresia);
    expect(repo.obtenerMembresia('usuario-1', 'partida-1')).toEqual(membresia);
    expect(repo.obtenerMembresia('usuario-1', 'otra-partida')).toBeUndefined();
  });
});
