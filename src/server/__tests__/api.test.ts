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

/** Login con el proveedor de desarrollo + unirse como jugador a `gameId` (asume la partida ya creada y
 * abierta). Devuelve la cabecera `Authorization: sesion <id>` lista para pasar a `/comandos`. */
async function unirseComoJugador(gameId: string, sujetoId = 'ana') {
  const login = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: `dev ${sujetoId}` } });
  const { sesionId } = login.json();
  const auth = { authorization: `sesion ${sesionId}` };
  await app.inject({ method: 'POST', url: `/partidas/${gameId}/jugadores`, headers: auth });
  return auth;
}

describe('POST /partidas/:gameId/jugadores', () => {
  it('crea la membresia de jugador y devuelve su jugadorId', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const login = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });
    const { sesionId } = login.json();

    const res = await app.inject({ method: 'POST', url: '/partidas/g1/jugadores', headers: { authorization: `sesion ${sesionId}` } });

    expect(res.statusCode).toBe(201);
    expect(res.json().jugadorId).toBeTruthy();
  });

  it('rechaza unirse dos veces a la misma partida', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const auth = await unirseComoJugador('g1');

    const res = await app.inject({ method: 'POST', url: '/partidas/g1/jugadores', headers: auth });
    expect(res.statusCode).toBe(409);
  });

  it('401 sin sesion valida', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const res = await app.inject({ method: 'POST', url: '/partidas/g1/jugadores' });
    expect(res.statusCode).toBe(401);
  });
});

describe('POST /partidas/:gameId/comandos', () => {
  it('ejecuta un comando conocido por su nombre y lo refleja en el resumen devuelto', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const auth = await unirseComoJugador('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.statusCode).toBe(200);
    const cuerpo = res.json();
    expect(cuerpo.version).toBe(1);
    expect(cuerpo.resultado.ok).toBe(true);
    expect(cuerpo.resultado.datos.faccionId).toBeTruthy();
  });

  it('400 si el tipo de comando no existe en el registro', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const auth = await unirseComoJugador('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'noExiste', params: {} },
    });

    expect(res.statusCode).toBe(400);
  });

  it('404 si la partida no está abierta en este proceso', async () => {
    const res = await app.inject({
      method: 'POST',
      url: '/partidas/no-existe/comandos',
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    expect(res.statusCode).toBe(404);
  });

  it('401 sin sesion valida', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.statusCode).toBe(401);
  });

  it('403 con sesion valida pero sin membresia en esa partida', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const login = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });
    const { sesionId } = login.json();

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      headers: { authorization: `sesion ${sesionId}` },
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.statusCode).toBe(403);
  });

  it('403 cuando la matriz de autorización rechaza la condición de dominio (Facción ajena)', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const auth = await unirseComoJugador('g1');
    await app.inject({ method: 'POST', url: '/partidas/g1/comandos', headers: auth, payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } } });

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: 'faccion-de-otro', posicion: { x: 0, y: 0 }, numJugadores: 1 } },
    });

    expect(res.statusCode).toBe(403);
    expect(res.json().error).toMatch(/condicion_dominio/);
  });

  it('un comando rechazado por el dominio responde 200 con `ok: false`, no un error HTTP', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const auth = await unirseComoJugador('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: '' } },
    });

    expect(res.statusCode).toBe(200);
    expect(res.json().resultado.ok).toBe(false);
  });
});

describe('POST /partidas con `forzar`', () => {
  it('sin `forzar`, sigue rechazando con 409 si el gameId ya está abierto', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    const res = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 999 } });
    expect(res.statusCode).toBe(409);
  });

  it('con `forzar: true`, descarta la partida en curso y crea una limpia (no reanuda el snapshot)', async () => {
    await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 42 } });
    await app.inject({ method: 'POST', url: '/partidas/g1/tick' });
    await app.inject({ method: 'POST', url: '/partidas/g1/tick' });

    const res = await app.inject({ method: 'POST', url: '/partidas', payload: { gameId: 'g1', seed: 999, forzar: true } });

    expect(res.statusCode).toBe(201);
    expect(res.json()).toEqual({ gameId: 'g1', tick: 0, version: 0 });
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

describe('POST /sesiones y GET /sesiones/actual', () => {
  it('login con el proveedor de desarrollo devuelve usuarioId + sesionId', async () => {
    const res = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });

    expect(res.statusCode).toBe(201);
    const body = res.json();
    expect(body.usuarioId).toMatch(/^usuario-/);
    expect(body.sesionId).toBeTruthy();
    expect(body.expiraEn).toBeTruthy();
  });

  it('rechaza login sin cabecera Authorization', async () => {
    const res = await app.inject({ method: 'POST', url: '/sesiones' });
    expect(res.statusCode).toBe(401);
  });

  it('rechaza login con un esquema sin proveedor registrado', async () => {
    const res = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'oauth token-x' } });
    expect(res.statusCode).toBe(401);
  });

  it('la sesion emitida por el login resuelve en /sesiones/actual', async () => {
    const login = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });
    const { usuarioId, sesionId } = login.json();

    const res = await app.inject({ method: 'GET', url: '/sesiones/actual', headers: { authorization: `sesion ${sesionId}` } });

    expect(res.statusCode).toBe(200);
    expect(res.json().usuarioId).toBe(usuarioId);
  });

  it('/sesiones/actual rechaza sin sesion, con esquema equivocado, o con un id inexistente', async () => {
    const sinCabecera = await app.inject({ method: 'GET', url: '/sesiones/actual' });
    expect(sinCabecera.statusCode).toBe(401);

    const esquemaEquivocado = await app.inject({ method: 'GET', url: '/sesiones/actual', headers: { authorization: 'dev ana' } });
    expect(esquemaEquivocado.statusCode).toBe(401);

    const inexistente = await app.inject({ method: 'GET', url: '/sesiones/actual', headers: { authorization: 'sesion no-existe' } });
    expect(inexistente.statusCode).toBe(401);
  });

  it('el mismo sujetoId reutiliza el Usuario entre logins distintos', async () => {
    const primero = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });
    const segundo = await app.inject({ method: 'POST', url: '/sesiones', headers: { authorization: 'dev ana' } });

    expect(segundo.json().usuarioId).toBe(primero.json().usuarioId);
    expect(segundo.json().sesionId).not.toBe(primero.json().sesionId);
  });
});
