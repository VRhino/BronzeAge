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

describe('cuentas de bot (CODIGO_REGISTRO_BOTS, doc 12 §8.3)', () => {
  const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };

  function servidorConBots() {
    const repositorio = crearRepositorioIdentidadEnMemoria();
    return crearServidor({
      directorio,
      codigoRegistro: 'humanos',
      codigoRegistroBots: 'bots',
      administradoresGlobales: [{ proveedor: 'dev', sujetoId: 'jefa' }],
      identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
    });
  }

  /** Alta con `codigo`, login, membresía y `crearHeroe`; devuelve la respuesta de crear el héroe y el controlador que quedó. */
  async function heroeDeCuenta(app: FastifyInstance, nick: string, codigo: string, extra: Record<string, unknown> = {}) {
    expect((await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick, clave: 'secreto123', codigo } })).statusCode).toBe(201);
    const sesion = (await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `clave ${nick}:secreto123` } })).json().sesionId;
    const auth = { authorization: `sesion ${sesion}` };
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g/membresia', headers: auth });
    const r = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g/comandos',
      headers: auth,
      payload: { tipo: 'crearHeroe', params: { displayName: nick, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR, ...extra } },
    });
    return r;
  }

  it('el código de bots da una cuenta de bot, cuyo héroe nace bot; el humano, uno humano', async () => {
    const app = servidorConBots();
    try {
      const admin = (await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } })).json().sesionId;
      await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: { authorization: `sesion ${admin}` }, payload: { gameId: 'g', seed: 42 } });

      expect((await heroeDeCuenta(app, 'robot', 'bots')).statusCode).toBe(200);
      expect((await heroeDeCuenta(app, 'ana', 'humanos')).statusCode).toBe(200);

      const estado = (await app.inject({ method: 'GET', url: '/v1/admin/partidas/g', headers: { authorization: `sesion ${admin}` } })).json();
      const controlador = (nombre: string) => (estado.heroes as { displayName: string; controlador: string }[]).find((h) => h.displayName === nombre)!.controlador;
      expect(controlador('robot')).toBe('bot');
      expect(controlador('ana')).toBe('humano');
    } finally {
      await app.close();
    }
  });

  it('un cliente no puede declararse bot: el `controlador` que mande se descarta', async () => {
    const app = servidorConBots();
    try {
      const admin = (await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } })).json().sesionId;
      await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: { authorization: `sesion ${admin}` }, payload: { gameId: 'g', seed: 42 } });

      await heroeDeCuenta(app, 'listo', 'humanos', { controlador: 'bot' });

      const estado = (await app.inject({ method: 'GET', url: '/v1/admin/partidas/g', headers: { authorization: `sesion ${admin}` } })).json();
      expect((estado.heroes as { displayName: string; controlador: string }[]).find((h) => h.displayName === 'listo')!.controlador).toBe('humano');
    } finally {
      await app.close();
    }
  });
});

describe('el código de invitación se cambia sin reiniciar (admin)', () => {
  const ADMIN = { proveedor: 'dev', sujetoId: 'jefa' };

  async function conAdmin(codigoRegistro?: string) {
    const repositorio = crearRepositorioIdentidadEnMemoria();
    const app = crearServidor({
      directorio,
      codigoRegistro,
      administradoresGlobales: [ADMIN],
      identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
    });
    const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } });
    return { app, admin: { authorization: `sesion ${login.json().sesionId}` } };
  }
  const alta = (app: FastifyInstance, nick: string, codigo?: string) => app.inject({ method: 'POST', url: '/v1/registro', payload: { nick, clave: 'secreto123', ...(codigo ? { codigo } : {}) } });

  it('lo lee, lo cambia y el registro lo exige desde ese momento; null lo deja abierto', async () => {
    const { app, admin } = await conAdmin('uno');
    try {
      expect((await app.inject({ method: 'GET', url: '/v1/admin/registro/codigo', headers: admin })).json()).toEqual({ codigo: 'uno' });

      const cambio = await app.inject({ method: 'PUT', url: '/v1/admin/registro/codigo', headers: admin, payload: { codigo: 'dos' } });
      expect(cambio.json()).toEqual({ codigo: 'dos' });
      expect((await alta(app, 'ana', 'uno')).statusCode).toBe(403);
      expect((await alta(app, 'ana', 'dos')).statusCode).toBe(201);

      const abierto = await app.inject({ method: 'PUT', url: '/v1/admin/registro/codigo', headers: admin, payload: { codigo: null } });
      expect(abierto.json()).toEqual({ codigo: null });
      expect((await alta(app, 'bruno')).statusCode).toBe(201);
    } finally {
      await app.close();
    }
  });

  it('solo un administrador global; sin sesión, 401', async () => {
    const { app } = await conAdmin('uno');
    try {
      expect((await app.inject({ method: 'GET', url: '/v1/admin/registro/codigo' })).statusCode).toBe(401);
      await alta(app, 'ana', 'uno');
      const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'clave ana:secreto123' } });
      const jugador = { authorization: `sesion ${login.json().sesionId}` };
      expect((await app.inject({ method: 'PUT', url: '/v1/admin/registro/codigo', headers: jugador, payload: { codigo: 'x' } })).statusCode).toBe(403);
    } finally {
      await app.close();
    }
  });
});
