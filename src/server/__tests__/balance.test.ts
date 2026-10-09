// Fase C7: el balance servido como datos, sin autenticar (doc 9, patrón Static Data Export).
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';
import { BALANCE_VERSION, EDIFICIO_CATALOGO } from '../../constants';

let directorio: string;
let app: FastifyInstance;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-balance-'));
  app = crearServidor({ directorio });
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

describe('GET /v1/balance', () => {
  it('responde 200 sin sesión: es regla pública, no estado de partida', async () => {
    const res = await app.inject({ method: 'GET', url: '/v1/balance' });
    expect(res.statusCode).toBe(200);
  });

  it('trae la versión y al menos una tabla real, sin transformar', async () => {
    const cuerpo = (await app.inject({ method: 'GET', url: '/v1/balance' })).json();

    expect(cuerpo.version).toBe(BALANCE_VERSION);
    expect(cuerpo.catalogos.EDIFICIO_CATALOGO).toEqual(EDIFICIO_CATALOGO);
  });

  it('publica lo que el cliente copiaba a mano: carros, animales, oro de reclutar, radios, capital y puerta (2026-10-09)', async () => {
    const cuerpo = (await app.inject({ method: 'GET', url: '/v1/balance' })).json();

    expect(cuerpo.caravanas.CARRO_CATALOGO.basico).toMatchObject({ capacidadBase: expect.any(Number), costo: expect.any(Object) });
    expect(cuerpo.caravanas.ANIMAL_CATALOGO.buey).toMatchObject({ factorCarga: expect.any(Number), velocidad: expect.any(Number), costo: expect.any(Object) });
    expect(cuerpo.mundoYMilitar.RECLUTAMIENTO_ORO_POR_ESCALON['1']).toBeGreaterThan(0);
    expect(cuerpo.mundoYMilitar.ORO_POR_CABALLO).toBeGreaterThan(0);
    expect(cuerpo.mundoYMilitar.MOVIMIENTO).toMatchObject({ radioInspeccion: 40, radioPuerta: 10 });
    expect(cuerpo.politica.CAPITAL.cooldownDias).toBeGreaterThan(0);
    expect(cuerpo.politica.PUERTA.cerradaAPorDefecto).toEqual(expect.arrayContaining(['neutrales', 'enemigos']));
  });

  it('aparece en el contrato publicado sin exigir seguridad', async () => {
    const doc = (await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json();

    expect(doc.paths['/balance']).toBeTruthy();
    expect(doc.paths['/balance'].get.security ?? []).toEqual([]);
  });
});
