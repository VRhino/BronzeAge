// Contrato publicado (Fase C6, doc 4: "para que los repos de cliente generen su cliente tipado"). No valida
// el documento contra el meta-esquema de OpenAPI (fuera de alcance aquí) — comprueba que el endpoint
// responde, que describe las rutas reales bajo /v1, y que las de seguridad quedan marcadas como tales.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';

let directorio: string;
let app: FastifyInstance;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-openapi-'));
  app = crearServidor({ directorio });
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

describe('GET /v1/openapi.json', () => {
  it('responde 200 con un documento OpenAPI 3.0', async () => {
    const res = await app.inject({ method: 'GET', url: '/v1/openapi.json' });

    expect(res.statusCode).toBe(200);
    const doc = res.json();
    expect(doc.openapi).toBe('3.0.0');
    expect(doc.info.title).toBeTruthy();
  });

  it('no exige sesion: es lo primero que un cliente nuevo necesita leer', async () => {
    const res = await app.inject({ method: 'GET', url: '/v1/openapi.json' });
    expect(res.statusCode).toBe(200);
  });

  it('describe las rutas reales (relativas a servers[0].url = /v1, ya lleva el prefijo)', async () => {
    const doc = (await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json();

    expect(doc.servers).toEqual([{ url: '/v1', description: 'Prefijo de versión de este proceso' }]);
    const rutas = Object.keys(doc.paths);
    expect(rutas).toContain('/sesiones');
    expect(rutas).toContain('/admin/partidas');
    expect(rutas).toContain('/jugador/partidas/{gameId}/comandos');
  });

  it('marca las rutas autenticadas con seguridad, y el login con la suya propia', async () => {
    const doc = (await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json();

    expect(doc.paths['/sesiones'].post.security).toEqual([{ credencialProveedor: [] }]);
    expect(doc.paths['/admin/partidas'].post.security).toEqual([{ sesionAuth: [] }]);
    expect(doc.paths['/jugador/partidas/{gameId}/comandos'].post.security).toEqual([{ sesionAuth: [] }]);
  });

  it('las rutas simples (login, membresia) tienen su cuerpo de respuesta modelado', async () => {
    const doc = (await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json();

    const login = doc.paths['/sesiones'].post.responses['201'].content['application/json'].schema;
    expect(login.required).toEqual(expect.arrayContaining(['usuarioId', 'sesionId', 'expiraEn']));
  });

  it('el WebSocket de tiempo real NO aparece: OpenAPI 3.0 no describe WebSocket', async () => {
    const rutas = Object.keys((await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json().paths);
    expect(rutas.some((r) => r.includes('tiempo-real'))).toBe(false);
  });
});
