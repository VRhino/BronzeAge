// CORS (Fase C6): sin `origenesPermitidos`, ningún origen cruzado debe recibir la cabecera que un navegador
// necesita para permitir la respuesta. Con la lista configurada, solo los orígenes exactos de esa lista.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';

let directorio: string;
let app: FastifyInstance | undefined;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-cors-'));
});

afterEach(async () => {
  await app?.close();
  await rm(directorio, { recursive: true, force: true });
});

describe('sin origenesPermitidos configurado', () => {
  it('una peticion con Origin no recibe access-control-allow-origin: el default no concede nada', async () => {
    app = crearServidor({ directorio });
    const res = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana', origin: 'https://cualquiera.example' } });

    expect(res.headers['access-control-allow-origin']).toBeUndefined();
  });
});

describe('con origenesPermitidos configurado', () => {
  it('un origen de la lista recibe access-control-allow-origin con ESE origen', async () => {
    app = crearServidor({ directorio, origenesPermitidos: ['https://jugador.example'] });
    const res = await app.inject({
      method: 'POST',
      url: '/v1/sesiones',
      headers: { authorization: 'dev ana', origin: 'https://jugador.example' },
    });

    expect(res.headers['access-control-allow-origin']).toBe('https://jugador.example');
  });

  it('un origen que NO está en la lista no recibe la cabecera', async () => {
    app = crearServidor({ directorio, origenesPermitidos: ['https://jugador.example'] });
    const res = await app.inject({
      method: 'POST',
      url: '/v1/sesiones',
      headers: { authorization: 'dev ana', origin: 'https://otro.example' },
    });

    expect(res.headers['access-control-allow-origin']).toBeUndefined();
  });
});
