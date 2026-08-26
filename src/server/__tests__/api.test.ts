// Contrato de las superficies HTTP (Fase C3): `/sesiones`, `/admin/*` y `/jugador/*`. Usa `app.inject()` de
// Fastify — ejercita las rutas de verdad (routing, validación de esquema, códigos de estado) sin abrir un
// socket real.
//
// Lo que estas pruebas vigilan por encima de todo es la SEPARACIÓN: que administrar y jugar exijan cosas
// distintas, y que ninguna de las dos siga accesible sin sesión como ocurría antes de C3.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';

/** El operador declara administradores por identidad externa; `dev jefa` es la de las pruebas. */
const ADMINS = [{ proveedor: 'dev', sujetoId: 'jefa' }];

let directorio: string;
let app: FastifyInstance;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-api-'));
  app = crearServidor({ directorio, administradoresGlobales: ADMINS });
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

/** Login con el proveedor de desarrollo; devuelve la cabecera lista para el resto de peticiones. */
async function sesionDe(sujetoId: string) {
  const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `dev ${sujetoId}` } });
  return { authorization: `sesion ${login.json().sesionId}` };
}

/** Admin autenticado + partida creada. */
async function partidaCreada(gameId = 'g1', seed = 42) {
  const admin = await sesionDe('jefa');
  const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId, seed } });
  return { admin, res };
}

/** Usuario corriente unido a la partida como jugador. */
async function jugadorEn(gameId: string, sujetoId = 'ana') {
  const auth = await sesionDe(sujetoId);
  await app.inject({ method: 'POST', url: `/v1/jugador/partidas/${gameId}/membresia`, headers: auth });
  return auth;
}

describe('POST /sesiones (login)', () => {
  it('devuelve usuarioId y sesionId', async () => {
    const res = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana' } });

    expect(res.statusCode).toBe(201);
    expect(res.json().usuarioId).toMatch(/^usuario-/);
    expect(res.json().sesionId).toBeTruthy();
  });

  it('rechaza sin cabecera, o con un esquema sin proveedor registrado', async () => {
    expect((await app.inject({ method: 'POST', url: '/v1/sesiones' })).statusCode).toBe(401);
    const otro = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'oauth token-x' } });
    expect(otro.statusCode).toBe(401);
  });

  it('el mismo sujetoId reutiliza el Usuario, pero emite sesion nueva', async () => {
    const primero = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana' } });
    const segundo = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana' } });

    expect(segundo.json().usuarioId).toBe(primero.json().usuarioId);
    expect(segundo.json().sesionId).not.toBe(primero.json().sesionId);
  });
});

describe('GET /sesiones/actual (whoami)', () => {
  it('sin gameId informa solo de la identidad y si es administrador global', async () => {
    const auth = await sesionDe('jefa');
    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual', headers: auth });

    expect(res.statusCode).toBe(200);
    expect(res.json().esAdministradorGlobal).toBe(true);
  });

  it('un usuario corriente no es administrador global', async () => {
    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual', headers: auth });
    expect(res.json().esAdministradorGlobal).toBe(false);
  });

  it('con gameId informa del rol en esa partida, para no tener que descubrirlo a base de 403', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual?gameId=g1', headers: auth });

    expect(res.json()).toMatchObject({ gameId: 'g1', rol: 'jugador' });
    expect(res.json().jugadorId).toBeTruthy();
  });

  it('sin membresia en esa partida, el rol es null', async () => {
    await partidaCreada('g1');
    const auth = await sesionDe('ana');

    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual?gameId=g1', headers: auth });
    expect(res.json().rol).toBeNull();
  });

  it('401 sin sesion', async () => {
    expect((await app.inject({ method: 'GET', url: '/v1/sesiones/actual' })).statusCode).toBe(401);
  });
});

describe('POST /admin/partidas', () => {
  it('un administrador global crea la partida', async () => {
    const { res } = await partidaCreada('g1');

    expect(res.statusCode).toBe(201);
    expect(res.json()).toEqual({ gameId: 'g1', tick: 0, version: 0, mapaId: expect.any(String) });
  });

  it('401 sin sesion — antes de C3 este endpoint era abierto', async () => {
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', payload: { gameId: 'g1', seed: 42 } });
    expect(res.statusCode).toBe(401);
  });

  it('403 para un usuario autenticado que no es administrador global', async () => {
    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: auth, payload: { gameId: 'g1', seed: 42 } });

    expect(res.statusCode).toBe(403);
    expect(res.json().error).toMatch(/administrador_global/);
  });

  it('sin administradores configurados NADIE puede crear: el default no concede nada', async () => {
    const cerrado = crearServidor({ directorio });
    try {
      const login = await cerrado.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } });
      const res = await cerrado.inject({
        method: 'POST',
        url: '/v1/admin/partidas',
        headers: { authorization: `sesion ${login.json().sesionId}` },
        payload: { gameId: 'g1', seed: 42 },
      });
      expect(res.statusCode).toBe(403);
    } finally {
      await cerrado.close();
    }
  });

  it('quien crea la partida queda como administrador_partida de ella', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual?gameId=g1', headers: admin });
    expect(res.json().rol).toBe('administrador_global'); // global manda sobre la membresia concreta

    // La membresía existe igualmente: sin ella no podría ejecutar comandos de administración.
    const comando = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'crearFaccion', params: { nombre: 'X' } },
    });
    expect(comando.statusCode).toBe(403); // rol_insuficiente: administrar no es jugar
  });

  it('rechaza un cuerpo sin seed (validacion de esquema)', async () => {
    const admin = await sesionDe('jefa');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g1' } });
    expect(res.statusCode).toBe(400);
  });

  it('rechaza region invalida y acepta la valida', async () => {
    const admin = await sesionDe('jefa');
    const ok = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g1', seed: 42, region: 'egeo' } });
    expect(ok.statusCode).toBe(201);

    const mal = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g2', seed: 42, region: 'atlantida' } });
    expect(mal.statusCode).toBe(400);
  });

  it('409 al crear dos veces el mismo gameId mientras siga abierto', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g1', seed: 999 } });
    expect(res.statusCode).toBe(409);
  });

  it('`forzar` descarta la partida en curso y empieza de cero', async () => {
    const { admin } = await partidaCreada('g1');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });

    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g1', seed: 999, forzar: true } });

    expect(res.statusCode).toBe(201);
    expect(res.json()).toEqual({ gameId: 'g1', tick: 0, version: 0, mapaId: expect.any(String) });
  });
});

describe('POST /admin/partidas/:gameId/tick', () => {
  it('avanza el tick', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json().tick).toBe(1);
  });

  it('401 sin sesion; 403 para un jugador de la partida', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick' })).statusCode).toBe(401);

    const jugador = await jugadorEn('g1');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: jugador });
    expect(res.statusCode).toBe(403);
  });

  it('404 si la partida no esta abierta en este proceso', async () => {
    const admin = await sesionDe('jefa');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/no-existe/tick', headers: admin });
    expect(res.statusCode).toBe(404);
  });
});

describe('GET /admin/partidas/:gameId (estado completo)', () => {
  it('devuelve el estado con sus colecciones', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toMatchObject({ tick: 0, asentamientos: [], facciones: [] });
  });

  it('un jugador NO puede leerlo: es el estado sin proyectar (fuga pendiente de C4)', async () => {
    await partidaCreada('g1');
    const jugador = await jugadorEn('g1');

    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: jugador });
    expect(res.statusCode).toBe(403);
  });

  it('401 sin sesion', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1' })).statusCode).toBe(401);
  });

  it('sin `mapa` (Fase C11): trae `mapaId` en su lugar, el mapa no viaja en cada lectura de estado', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });

    expect(res.json()).not.toHaveProperty('mapa');
    expect(res.json().mapaId).toEqual(expect.any(String));
  });
});

describe('GET /admin|jugador/partidas/:gameId/mapa/:mapaId (Fase C11)', () => {
  it('sirve el mapa real, con cache eterna, a un administrador', async () => {
    const { admin } = await partidaCreada('g1', 42);
    const estado = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });
    const { mapaId } = estado.json();

    const res = await app.inject({ method: 'GET', url: `/v1/admin/partidas/g1/mapa/${mapaId}`, headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json().config.seed).toBe(42);
    expect(res.headers['cache-control']).toMatch(/immutable/);
  });

  it('sirve el mapa real a un jugador con membresia, no solo a administracion', async () => {
    await partidaCreada('g1', 42);
    const jugador = await jugadorEn('g1');
    const proyeccion = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: jugador });
    const { mapaId } = proyeccion.json();

    const res = await app.inject({ method: 'GET', url: `/v1/jugador/partidas/g1/mapa/${mapaId}`, headers: jugador });

    expect(res.statusCode).toBe(200);
    expect(res.json().config.seed).toBe(42);
  });

  it('el segmento :mapaId es solo cache-buster: cualquier valor sirve el mapa vigente', async () => {
    const { admin } = await partidaCreada('g1', 42);
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/mapa/cualquier-cosa', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json().config.seed).toBe(42);
  });

  it('el mapaId de la proyeccion de jugador coincide con el de la vista de administracion', async () => {
    const { admin } = await partidaCreada('g1');
    const jugador = await jugadorEn('g1');

    const estadoAdmin = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });
    const proyeccion = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: jugador });

    expect(proyeccion.json().mapaId).toBe(estadoAdmin.json().mapaId);
  });

  it('401 sin sesion en ambas superficies', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/mapa/x' })).statusCode).toBe(401);
    expect((await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1/mapa/x' })).statusCode).toBe(401);
  });

  it('403 a un jugador sin membresia en la superficie de jugador', async () => {
    await partidaCreada('g1');
    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1/mapa/x', headers: auth });
    expect(res.statusCode).toBe(403);
  });
});

describe('POST /jugador/partidas/:gameId/membresia (unirse)', () => {
  it('crea la membresia y devuelve el jugadorId', async () => {
    await partidaCreada('g1');
    const auth = await sesionDe('ana');

    const res = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: auth });

    expect(res.statusCode).toBe(201);
    expect(res.json().jugadorId).toBeTruthy();
  });

  it('409 al unirse dos veces', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: auth });
    expect(res.statusCode).toBe(409);
  });

  it('401 sin sesion; 404 si la partida no esta abierta', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia' })).statusCode).toBe(401);

    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/no-existe/membresia', headers: auth });
    expect(res.statusCode).toBe(404);
  });
});

describe('GET /jugador/partidas/:gameId (proyeccion, Fase C4 Slice 1)', () => {
  it('devuelve la proyeccion del jugador, no el estado completo', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: auth });

    expect(res.statusCode).toBe(200);
    const cuerpo = res.json();
    expect(cuerpo.jugadorId).toBeTruthy();
    expect(cuerpo.asentamientos).toEqual([]); // sin Facción todavía
    expect(cuerpo.facciones).toEqual([]);
  });

  it('no incluye asentamientos de una Faccion rival, aunque el admin sí los vea', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    // El admin creador de la partida no es ciudadano de ninguna Facción del juego, así que "fundar" con su
    // usuario no pertenece a este escenario — se usa un segundo jugador para tener una Facción rival real.
    const luis = await jugadorEn('g1', 'luis');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: luis,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Troya' } },
    });

    const proyeccionAna = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: ana });
    // Ana no fundó ningún asentamiento (solo creó la Facción): su proyección no tiene ninguno, ni el suyo ni
    // el de nadie — es justo la ausencia de niebla de guerra la que impide que Slice 1 le muestre algo ajeno.
    expect(proyeccionAna.json().asentamientos).toEqual([]);
    // Pero sí ve los METADATOS de ambas Facciones (necesarios para la pantalla de diplomacia).
    expect(proyeccionAna.json().facciones.map((f: { nombre: string }) => f.nombre).sort()).toEqual(['Micenas', 'Troya']);
  });

  it('un admin no puede leer la proyeccion de jugador: para jugar hace falta ser jugador', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: admin });
    expect(res.statusCode).toBe(403);
  });

  it('401 sin sesion', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1' })).statusCode).toBe(401);
  });

  it('403 (no 404) en una partida inexistente sin membresia: no delata si existe antes de comprobar permiso', async () => {
    // Mismo orden que POST .../comandos: se comprueba la membresía antes que la existencia de la partida, así
    // que sin unirse nunca se llega a saber si 'no-existe' está abierta o no.
    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/no-existe', headers: auth });
    expect(res.statusCode).toBe(403);
  });
});

describe('POST /jugador/partidas/:gameId/comandos', () => {
  it('ejecuta un comando y lo refleja en el resumen', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.statusCode).toBe(200);
    expect(res.json().version).toBe(1);
    expect(res.json().resultado.ok).toBe(true);
  });

  it('respuesta autosuficiente (Fase C6): incluye la proyeccion propia ya actualizada, sin GET aparte', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.json().proyeccion.facciones).toHaveLength(1);
    expect(res.json().proyeccion.facciones[0].nombre).toBe('Micenas');
  });

  it('la respuesta de administrador NO lleva proyeccion: su GET de estado completo ya esta ahi', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'alternarFaccionNpc', params: { faccionId: 'no-existe', activo: true } },
    });

    expect(res.json().proyeccion).toBeUndefined();
  });

  it('idempotencyKey (Fase C5): repetir la misma peticion no vuelve a aplicar el comando', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');
    const payload = { tipo: 'crearFaccion', params: { nombre: 'Micenas' }, idempotencyKey: 'clave-http-1' };

    const primero = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: auth, payload });
    const segundo = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: auth, payload });

    expect(primero.json().resultado).toEqual(segundo.json().resultado);
    expect(segundo.json().version).toBe(1); // no subió a 2: el segundo POST no se aplicó de verdad
  });

  it('401 sin sesion, 403 con sesion pero sin membresia de jugador', async () => {
    await partidaCreada('g1');
    const payload = { tipo: 'crearFaccion', params: { nombre: 'Micenas' } };

    expect((await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', payload })).statusCode).toBe(401);

    const auth = await sesionDe('ana');
    const res = await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: auth, payload });
    expect(res.statusCode).toBe(403);
  });

  it('un administrador de la partida tampoco puede jugar en ella sin ser jugador', async () => {
    const { admin } = await partidaCreada('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    expect(res.statusCode).toBe(403);
  });

  it('400 si el tipo de comando no existe en el registro', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'noExiste', params: {} },
    });
    expect(res.statusCode).toBe(400);
  });

  it('403 cuando la matriz rechaza la condicion de dominio (no es ciudadano de esa Faccion)', async () => {
    await partidaCreada('g1');
    const primero = await jugadorEn('g1', 'ana');
    // `ana` crea la Facción Y funda en ella: es el caso de arranque (Facción recién creada, sin ciudadanos
    // aún) y queda como su primera ciudadana. Con eso la Facción deja de estar "vacía" para cualquier otro.
    const creada = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: primero,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    const faccionId = creada.json().resultado.datos.faccionId;
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: primero,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId, posicion: { x: 500, y: 500 } } },
    });

    // `luis`, otro jugador sin Facción, intenta fundar en la Facción de `ana` — ya no está vacía, así que la
    // excepción de arranque no aplica y se rechaza igual que a cualquier forastero.
    const segundo = await jugadorEn('g1', 'luis');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: segundo,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId, posicion: { x: 900, y: 900 } } },
    });

    expect(res.statusCode).toBe(403);
    expect(res.json().error).toMatch(/condicion_dominio/);
  });

  it('una entidad inexistente NO es 403: la existencia la juzga el comando, con su codigo de error', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: 'no-existe', posicion: { x: 500, y: 500 } } },
    });

    expect(res.statusCode).toBe(200);
    expect(res.json().resultado).toMatchObject({ ok: false, codigoError: 'fundacion.invalida' });
  });

  it('un comando rechazado por el dominio responde 200 con `ok: false`, no un error HTTP', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: '' } },
    });

    expect(res.statusCode).toBe(200);
    expect(res.json().resultado.ok).toBe(false);
    expect(res.json().version).toBe(0); // un rechazo no versiona
  });
});

describe('las rutas sin prefijo de antes de C3 ya no existen', () => {
  it('404 en /partidas y en /partidas/:gameId', async () => {
    const admin = await sesionDe('jefa');
    expect((await app.inject({ method: 'POST', url: '/partidas', headers: admin, payload: { gameId: 'g1', seed: 42 } })).statusCode).toBe(404);
    expect((await app.inject({ method: 'GET', url: '/partidas/g1', headers: admin })).statusCode).toBe(404);
  });
});

describe('reanudacion tras "reinicio del proceso"', () => {
  it('crear con un gameId que ya tiene snapshot en disco retoma la partida, no la resetea', async () => {
    const { admin } = await partidaCreada('g1');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await app.close();

    // Proceso "nuevo": mismo directorio de persistencia, ningún runner en memoria.
    app = crearServidor({ directorio, administradoresGlobales: ADMINS });
    const nuevaSesion = await sesionDe('jefa');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: nuevaSesion, payload: { gameId: 'g1', seed: 999 } });

    expect(res.statusCode).toBe(201);
    expect(res.json().tick).toBe(2); // retomó los 2 ticks ya guardados, no volvió a 0
  });
});
