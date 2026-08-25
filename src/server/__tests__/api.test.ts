// Contrato de la API HTTP administrativa mínima (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3):
// crear partida, avanzar tick, consultar estado. Usa `app.inject()` de Fastify — ejercita las rutas de
// verdad (routing, validación de esquema, códigos de estado) sin abrir un socket real.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';

let directorio: string;
let app: FastifyInstance;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-api-'));
  app = crearServidor({ directorio });
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

describe('POST /partidas', () => {
  it('crea una partida nueva y devuelve su resumen', async () => {
    const res = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });

    expect(res.statusCode).toBe(201);
    expect(res.json()).toEqual({ gameId: 'g1', tick: 0, version: 0 });
  });

  it('rechaza un cuerpo sin `seed` (validación de esquema)', async () => {
    const res = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1' } });
    expect(res.statusCode).toBe(400);
  });

  it('rechaza crear dos veces el mismo gameId mientras siga abierto en este proceso', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const res = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 999 } });

    expect(res.statusCode).toBe(409);
  });

  it('acepta una región válida y rechaza una que no lo es', async () => {
    const ok = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42, region: 'egeo' } });
    expect(ok.statusCode).toBe(201);

    const mal = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g2', seed: 42, region: 'atlantida' } });
    expect(mal.statusCode).toBe(400);
  });
});

describe('POST /partidas/:gameId/tick', () => {
  it('avanza un tick y lo refleja en el resumen devuelto', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });

    const res = await app.inject({ method: 'POST', url: '/partidas/g1/tick' });

    expect(res.statusCode).toBe(200);
    const cuerpo = res.json();
    expect(cuerpo.tick).toBe(1);
    expect(cuerpo.resultado.ok).toBe(true);
  });

  it('404 si la partida no está abierta en este proceso', async () => {
    const res = await app.inject({ method: 'POST', url: '/partidas/no-existe/tick' });
    expect(res.statusCode).toBe(404);
  });
});

describe('GET /partidas/:gameId', () => {
  it('devuelve el estado completo de la partida', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    await app.inject({ method: 'POST', url: '/partidas/g1/tick' });

    const res = await app.inject({ method: 'GET', url: '/partidas/g1' });

    expect(res.statusCode).toBe(200);
    const estado = res.json();
    expect(estado.gameId).toBe('g1');
    expect(estado.tick).toBe(1);
    expect(Array.isArray(estado.asentamientos)).toBe(true);
  });

  it('404 si la partida no está abierta en este proceso', async () => {
    const res = await app.inject({ method: 'GET', url: '/partidas/no-existe' });
    expect(res.statusCode).toBe(404);
  });
});

describe('reanudación tras "reinicio del proceso"', () => {
  it('crear con un gameId que ya tiene snapshot en disco retoma la partida, no la resetea', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    await app.inject({ method: 'POST', url: '/partidas/g1/tick' });
    await app.inject({ method: 'POST', url: '/partidas/g1/tick' });
    await app.close();

    // Proceso "nuevo": mismo directorio de persistencia, ningún runner en memoria.
    const appReiniciada = crearServidor({ directorio });
    try {
      const res = await appReiniciada.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 999 } });

      expect(res.statusCode).toBe(201);
      expect(res.json().tick).toBe(2); // retomó los 2 ticks ya guardados, no volvió a 0
    } finally {
      await appReiniciada.close();
    }
  });
});
