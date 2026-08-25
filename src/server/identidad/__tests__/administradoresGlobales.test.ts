import { describe, expect, it } from 'vitest';
import { crearDirectorioDeAdministradores, parsearAdministradores } from '../administradoresGlobales';
import { crearRepositorioIdentidadEnMemoria } from '../repositorioEnMemoria';

function repoConUsuario(proveedor: string, sujetoId: string) {
  const repo = crearRepositorioIdentidadEnMemoria();
  const usuario = repo.crearUsuario({ creadoEn: '2026-01-01T00:00:00.000Z' });
  repo.vincularIdentidad({ usuarioId: usuario.id, proveedor, sujetoId, vinculadaEn: '2026-01-01T00:00:00.000Z' });
  return { repo, usuario };
}

describe('crearDirectorioDeAdministradores', () => {
  it('reconoce al usuario cuya identidad externa esta configurada', () => {
    const { repo, usuario } = repoConUsuario('dev', 'jefa');
    const directorio = crearDirectorioDeAdministradores([{ proveedor: 'dev', sujetoId: 'jefa' }], repo);

    expect(directorio.esAdministradorGlobal(usuario.id)).toBe(true);
  });

  it('no reconoce a otro sujeto, ni al mismo sujeto de otro proveedor', () => {
    const { repo, usuario } = repoConUsuario('dev', 'ana');
    expect(crearDirectorioDeAdministradores([{ proveedor: 'dev', sujetoId: 'jefa' }], repo).esAdministradorGlobal(usuario.id)).toBe(false);
    expect(crearDirectorioDeAdministradores([{ proveedor: 'oauth', sujetoId: 'ana' }], repo).esAdministradorGlobal(usuario.id)).toBe(false);
  });

  it('sin configuracion no hay administradores: el default no concede nada', () => {
    const { repo, usuario } = repoConUsuario('dev', 'jefa');
    expect(crearDirectorioDeAdministradores([], repo).esAdministradorGlobal(usuario.id)).toBe(false);
  });

  it('un usuarioId desconocido no es administrador', () => {
    const { repo } = repoConUsuario('dev', 'jefa');
    expect(crearDirectorioDeAdministradores([{ proveedor: 'dev', sujetoId: 'jefa' }], repo).esAdministradorGlobal('usuario-inventado')).toBe(false);
  });
});

describe('parsearAdministradores', () => {
  it('parsea una lista separada por comas', () => {
    expect(parsearAdministradores('dev:jefa, oauth:1234')).toEqual([
      { proveedor: 'dev', sujetoId: 'jefa' },
      { proveedor: 'oauth', sujetoId: '1234' },
    ]);
  });

  it('vacio o ausente = sin administradores', () => {
    expect(parsearAdministradores(undefined)).toEqual([]);
    expect(parsearAdministradores('   ')).toEqual([]);
  });

  it('conserva los `:` internos del sujetoId', () => {
    expect(parsearAdministradores('oauth:https://idp/u/1')).toEqual([{ proveedor: 'oauth', sujetoId: 'https://idp/u/1' }]);
  });

  it('una configuracion escrita pero sin ninguna entrada valida es un fallo de arranque, no un 403 tardio', () => {
    expect(() => parsearAdministradores('jefa')).toThrow(/ADMINISTRADORES/);
    expect(() => parsearAdministradores(':sin-proveedor')).toThrow(/ADMINISTRADORES/);
    expect(() => parsearAdministradores('dev:')).toThrow(/ADMINISTRADORES/);
  });
});
