// Borrar una partida se lleva todo menos los respaldos, y restaurar un respaldo la devuelve con sus jugadores
// (`DELETE /admin/partidas/:gameId` + `/admin/respaldos`).
import { mkdtemp, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresDeProceso } from '../identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../identidad/repositorioEnMemoria';
import type { RepositorioIdentidad } from '../../acceso/repositorio';

const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };

let directorio: string;
let repositorio: RepositorioIdentidad;
let app: FastifyInstance;
let admin: { authorization: string };

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-borrado-'));
  repositorio = crearRepositorioIdentidadEnMemoria();
  app = crearServidor({
    directorio,
    codigoRegistroBots: 'bots',
    administradoresGlobales: [{ proveedor: 'dev', sujetoId: 'jefa' }],
    identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
  });
  admin = { authorization: `sesion ${(await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } })).json().sesionId}` };
  await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g', seed: 42 } });
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

/** Cuenta (de bot con `codigo: 'bots'`), membresía y héroe en `g`; devuelve su usuarioId. */
async function jugador(nick: string, codigo?: string): Promise<string> {
  await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick, clave: 'secreto123', ...(codigo ? { codigo } : {}) } });
  const login = (await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `clave ${nick}:secreto123` } })).json();
  const auth = { authorization: `sesion ${login.sesionId}` };
  await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g/membresia', headers: auth });
  const r = await app.inject({
    method: 'POST',
    url: '/v1/jugador/partidas/g/comandos',
    headers: auth,
    payload: { tipo: 'crearHeroe', params: { displayName: nick, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR } },
  });
  expect(r.statusCode).toBe(200);
  return login.usuarioId;
}

describe('borrar una partida', () => {
  it('se lleva guardado, auditoría y cuentas de bot; las membresías humanas quedan revocadas; los respaldos se quedan', async () => {
    const ana = await jugador('ana');
    const robot = await jugador('robot', 'bots');
    expect((await app.inject({ method: 'POST', url: '/v1/admin/partidas/g/respaldos', headers: admin })).statusCode).toBe(201);

    const res = await app.inject({ method: 'DELETE', url: '/v1/admin/partidas/g', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toMatchObject({ gameId: 'g', borrada: true, membresiasRevocadas: 2, cuentasDeBotBorradas: 1 });
    expect((await readdir(directorio)).filter((f) => f.startsWith('g.'))).toEqual([]);
    expect(repositorio.obtenerUsuario(robot)).toBeUndefined();
    expect(repositorio.obtenerMembresia(ana, 'g')).toMatchObject({ motivoFin: 'partida_borrada' });
    expect((await readdir(join(directorio, 'respaldos'))).some((f) => f.startsWith('g--'))).toBe(true);
  });
});

describe('respaldos', () => {
  it('lista los de una partida borrada y restaurarla la devuelve con sus jugadores humanos', async () => {
    const ana = await jugador('ana');
    const hecho = (await app.inject({ method: 'POST', url: '/v1/admin/partidas/g/respaldos', headers: admin })).json();
    await app.inject({ method: 'DELETE', url: '/v1/admin/partidas/g', headers: admin });

    const lista = (await app.inject({ method: 'GET', url: '/v1/admin/respaldos', headers: admin })).json().respaldos;
    expect(lista).toEqual([expect.objectContaining({ gameId: 'g', archivo: hecho.archivo, partidaExiste: false })]);

    const restaurada = await app.inject({ method: 'POST', url: '/v1/admin/respaldos/restaurar', headers: admin, payload: { archivo: hecho.archivo } });

    expect(restaurada.statusCode).toBe(200);
    expect(restaurada.json()).toMatchObject({ gameId: 'g', membresiasReactivadas: 2 });
    const estado = (await app.inject({ method: 'GET', url: '/v1/admin/partidas/g', headers: admin })).json();
    expect(estado.heroes.map((h: { displayName: string }) => h.displayName)).toEqual(['ana']);
    expect(repositorio.obtenerMembresia(ana, 'g')?.hasta).toBeUndefined();
  });

  it('borra un respaldo; un nombre que no es de respaldo (o con ruta) no toca nada', async () => {
    const hecho = (await app.inject({ method: 'POST', url: '/v1/admin/partidas/g/respaldos', headers: admin })).json();

    expect((await app.inject({ method: 'DELETE', url: '/v1/admin/respaldos/..%2Fg.json', headers: admin })).statusCode).toBe(404);
    expect((await app.inject({ method: 'POST', url: '/v1/admin/respaldos/restaurar', headers: admin, payload: { archivo: '../g.json' } })).statusCode).toBe(400);
    expect((await app.inject({ method: 'DELETE', url: `/v1/admin/respaldos/${hecho.archivo}`, headers: admin })).statusCode).toBe(200);
    expect((await app.inject({ method: 'GET', url: '/v1/admin/respaldos', headers: admin })).json().respaldos).toEqual([]);
    expect((await readdir(directorio)).includes('g.json')).toBe(true);
  });

  it('sin ser administrador global, 403', async () => {
    const ana = { authorization: `sesion ${(await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana' } })).json().sesionId}` };
    expect((await app.inject({ method: 'GET', url: '/v1/admin/respaldos', headers: ana })).statusCode).toBe(403);
  });
});
