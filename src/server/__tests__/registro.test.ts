// Alta de cuenta local (`POST /v1/registro`) + login con el proveedor `clave`. El flujo que un jugador del
// playtest hace desde el front: registrarse con nick + contraseña, y a partir de ahí entrar como con
// cualquier proveedor.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresDeProceso } from '../identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../identidad/repositorioEnMemoria';

let directorio: string;

function servidor(codigoRegistro?: string): FastifyInstance {
  const repositorio = crearRepositorioIdentidadEnMemoria();
  return crearServidor({
    directorio,
    codigoRegistro,
    identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
  });
}

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-registro-'));
});
afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

describe('POST /v1/registro', () => {
  it('alta + login: la cuenta recién creada sirve para entrar', async () => {
    const app = servidor();
    try {
      const alta = await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'Ana', clave: 'secreto123' } });
      expect(alta.statusCode).toBe(201);
      expect(alta.json().nick).toBe('ana');

      const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'clave ana:secreto123' } });
      expect(login.statusCode).toBe(201);
      expect(login.json().sesionId).toBeTruthy();
    } finally {
      await app.close();
    }
  });

  it('nick ya registrado -> 409; contraseña corta -> 400', async () => {
    const app = servidor();
    try {
      await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'ana', clave: 'secreto123' } });
      const repe = await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'ANA', clave: 'otraaa' } });
      expect(repe.statusCode).toBe(409);

      const corta = await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'bruno', clave: 'x' } });
      expect(corta.statusCode).toBe(400);
    } finally {
      await app.close();
    }
  });

  it('login con contraseña incorrecta -> 401', async () => {
    const app = servidor();
    try {
      await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'ana', clave: 'secreto123' } });
      const mala = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'clave ana:incorrecta' } });
      expect(mala.statusCode).toBe(401);
    } finally {
      await app.close();
    }
  });

  it('con CODIGO_REGISTRO, exige el código', async () => {
    const app = servidor('abrete-sesamo');
    try {
      const sinCodigo = await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick: 'ana', clave: 'secreto123' } });
      expect(sinCodigo.statusCode).toBe(403);

      const conCodigo = await app.inject({
        method: 'POST',
        url: '/v1/registro',
        payload: { nick: 'ana', clave: 'secreto123', codigo: 'abrete-sesamo' },
      });
      expect(conCodigo.statusCode).toBe(201);
    } finally {
      await app.close();
    }
  });
});
