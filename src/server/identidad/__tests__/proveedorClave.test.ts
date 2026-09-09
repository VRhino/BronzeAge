import { describe, expect, it } from 'vitest';
import { CredencialInvalidaError } from '../../../acceso/proveedorIdentidad';
import { crearRepositorioIdentidadEnMemoria } from '../repositorioEnMemoria';
import {
  crearProveedorClave,
  DatosDeRegistroInvalidosError,
  ESQUEMA_CLAVE,
  NickYaRegistradoError,
  registrarCredencial,
} from '../proveedorClave';

const AHORA = '2026-09-09T10:00:00.000Z';

function conCuenta(nick: string, clave: string) {
  const repositorio = crearRepositorioIdentidadEnMemoria();
  registrarCredencial(repositorio, nick, clave, AHORA);
  return { repositorio, proveedor: crearProveedorClave(repositorio) };
}

describe('registrarCredencial', () => {
  it('guarda un hash, nunca la contraseña en claro', () => {
    const { repositorio } = conCuenta('Ana', 'secreto123');
    const guardada = repositorio.buscarCredencialLocal('ana')!;
    expect(guardada.hash).not.toContain('secreto123');
    expect(guardada.salt).toHaveLength(32);
  });

  it('normaliza el nick (trim + minúsculas)', () => {
    const { repositorio } = conCuenta('  Bruno  ', 'secreto123');
    expect(repositorio.buscarCredencialLocal('bruno')).toBeDefined();
  });

  it('rechaza un nick ya registrado, sin distinguir mayúsculas', () => {
    const { repositorio } = conCuenta('ana', 'secreto123');
    expect(() => registrarCredencial(repositorio, 'ANA', 'otra-clave', AHORA)).toThrow(NickYaRegistradoError);
  });

  it('rechaza contraseña corta y nick vacío', () => {
    const repositorio = crearRepositorioIdentidadEnMemoria();
    expect(() => registrarCredencial(repositorio, 'ana', 'corta', AHORA)).toThrow(DatosDeRegistroInvalidosError);
    expect(() => registrarCredencial(repositorio, '   ', 'secreto123', AHORA)).toThrow(DatosDeRegistroInvalidosError);
  });
});

describe('crearProveedorClave().autenticar', () => {
  it('expone el esquema clave', () => {
    expect(crearProveedorClave(crearRepositorioIdentidadEnMemoria()).esquema).toBe(ESQUEMA_CLAVE);
  });

  it('resuelve la identidad con la contraseña correcta', async () => {
    const { proveedor } = conCuenta('ana', 'secreto123');
    await expect(proveedor.autenticar('ana:secreto123')).resolves.toEqual({ proveedor: 'clave', sujetoId: 'ana' });
  });

  it('acepta contraseñas que contienen ":"', async () => {
    const { proveedor } = conCuenta('ana', 'a:b:c:123');
    await expect(proveedor.autenticar('ana:a:b:c:123')).resolves.toMatchObject({ sujetoId: 'ana' });
  });

  it('mismo error para nick inexistente y contraseña incorrecta', async () => {
    const { proveedor } = conCuenta('ana', 'secreto123');
    await expect(proveedor.autenticar('ana:mala')).rejects.toThrow(CredencialInvalidaError);
    await expect(proveedor.autenticar('nadie:loquesea')).rejects.toThrow(CredencialInvalidaError);
  });

  it('rechaza una credencial sin ":"', async () => {
    const { proveedor } = conCuenta('ana', 'secreto123');
    await expect(proveedor.autenticar('ana')).rejects.toThrow(CredencialInvalidaError);
  });
});
