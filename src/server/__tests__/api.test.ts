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
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresPorDefecto } from '../identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnDisco } from '../identidad/repositorioEnDisco';
import { instanteDeTick } from '../../session/estado';

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

describe('GET /admin/partidas (Fase C12: descubrimiento)', () => {
  it('lista las partidas guardadas en disco, con resumen', async () => {
    await partidaCreada('g1');
    const { admin } = await partidaCreada('g2');

    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json().partidas.map((p: { gameId: string }) => p.gameId).sort()).toEqual(['g1', 'g2']);
    expect(res.json().partidas[0]).toHaveProperty('mapaId');
    // D4/D6: el resumen fecha la partida con el instante de mundo (el tick interno no viaja).
    expect(res.json().partidas[0]).toMatchObject({ instante: instanteDeTick(0) });
  });

  it('401 sin sesion, 403 sin ser administrador global', async () => {
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas' })).statusCode).toBe(401);

    // `ana` nunca se declaró en ADMINISTRADORES (solo 'jefa' lo está, ver ADMINS arriba).
    const ana = await sesionDe('ana');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas', headers: ana })).statusCode).toBe(403);
  });
});

describe('POST /admin/partidas', () => {
  it('un administrador global crea la partida', async () => {
    const { res } = await partidaCreada('g1');

    expect(res.statusCode).toBe(201);
    expect(res.json()).toEqual({ gameId: 'g1', instante: instanteDeTick(0), version: 0, mapaId: expect.any(String) });
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
    // La membresia concreta manda sobre ser administrador global (fix C8, 2026-08-26) — `esAdministradorGlobal`
    // sigue en `true` por separado (siguiente aserción), pero el ROL con el que actúa EN esta partida es el
    // de su Membresia.
    expect(res.json().rol).toBe('administrador_partida');
    expect(res.json().esAdministradorGlobal).toBe(true);

    // Sigue siendo jugador de ninguna: administrar no es jugar (doc 5).
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
    expect(res.json()).toEqual({ gameId: 'g1', instante: instanteDeTick(0), version: 0, mapaId: expect.any(String) });
  });
});

describe('POST /admin/partidas/:gameId/tick', () => {
  it('avanza el tick', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.json().instante).toBe(instanteDeTick(1));
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

  it('trae preciosReferencia (doc 9): un administrador ve el mismo precio calculado en servidor', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });

    expect(Object.keys(res.json().preciosReferencia).sort()).toEqual(['cobre', 'estano', 'livestock', 'madera', 'piedra', 'trigo'].sort());
  });
});

describe('GET /admin/partidas/:gameId/exportar (Fase C12)', () => {
  it('descarga el snapshot completo, con Content-Disposition de adjunto', async () => {
    const { admin } = await partidaCreada('g1', 7);
    const auth = await jugadorEn('g1');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/exportar', headers: admin });

    expect(res.statusCode).toBe(200);
    expect(res.headers['content-disposition']).toContain('attachment');
    expect(res.headers['content-disposition']).toContain('g1.json');
    expect(res.json().state.facciones).toHaveLength(1);
    expect(res.json().worldgenVersion).toEqual(expect.any(Number));
    expect(res.json().estadoRng).toEqual(expect.any(Number));
  });

  it('401 sin sesion, 403 para un jugador', async () => {
    await partidaCreada('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/exportar' })).statusCode).toBe(401);

    const jugador = await jugadorEn('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/exportar', headers: jugador })).statusCode).toBe(403);
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

  it('trae preciosReferencia (doc 9): calculado en el servidor con TODO el mundo, nunca por el jugador', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: auth });

    expect(Object.keys(res.json().preciosReferencia).sort()).toEqual(['cobre', 'estano', 'livestock', 'madera', 'piedra', 'trigo'].sort());
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

  it('geometría por frame (Fase C10): el jugador solo ve su propia zona/trazado, nunca los de un rival', async () => {
    const { admin } = await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    const creadaAna = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    const fundadaAna = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: creadaAna.json().resultado.datos.faccionId, posicion: { x: 500, y: 500 } } },
    });
    const asentamientoAna = fundadaAna.json().resultado.datos.asentamientoId;

    const luis = await jugadorEn('g1', 'luis');
    const creadaLuis = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: luis,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Troya' } },
    });
    const fundadaLuis = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: luis,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: creadaLuis.json().resultado.datos.faccionId, posicion: { x: 1200, y: 1200 } } },
    });
    const asentamientoLuis = fundadaLuis.json().resultado.datos.asentamientoId;

    const proyeccionAna = (await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: ana })).json();
    expect(proyeccionAna.zonas.map((z: { asentamientoId: string }) => z.asentamientoId)).toEqual([asentamientoAna]);
    expect(proyeccionAna.zonasFusionadas).toHaveLength(1);
    expect(proyeccionAna.zonasFusionadas[0].faccionId).toBe(creadaAna.json().resultado.datos.faccionId);
    expect(Object.keys(proyeccionAna.trazadoPorAsentamiento)).toEqual([asentamientoAna]);

    // El admin, en cambio, ve la geometría de las DOS Facciones — es quien observa toda la partida (doc 5).
    const estadoAdmin = (await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin })).json();
    expect(estadoAdmin.zonas.map((z: { asentamientoId: string }) => z.asentamientoId).sort()).toEqual([asentamientoAna, asentamientoLuis].sort());
    expect(estadoAdmin.zonasFusionadas).toHaveLength(2);
    expect(Object.keys(estadoAdmin.trazadoPorAsentamiento).sort()).toEqual([asentamientoAna, asentamientoLuis].sort());
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

describe('GET .../eventos?desde= (Fase C13: cursor incremental)', () => {
  it('admin: desde=0 trae todo; desde=<version actual> no trae nada nuevo', async () => {
    const { admin } = await partidaCreada('g1');
    const auth = await jugadorEn('g1');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    const desdeCero = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/eventos?desde=0', headers: admin });
    expect(desdeCero.statusCode).toBe(200);
    expect(desdeCero.json().eventos).toHaveLength(1);
    expect(desdeCero.json().eventos[0].version).toBe(1);

    const desdeActual = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/eventos?desde=1', headers: admin });
    expect(desdeActual.json().eventos).toEqual([]);
  });

  it('sin `desde` equivale a 0', async () => {
    const { admin } = await partidaCreada('g1');
    const auth = await jugadorEn('g1');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/eventos', headers: admin });
    expect(res.json().eventos).toHaveLength(1);
  });

  it('400 si `desde` no es un entero no negativo', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/eventos?desde=-1', headers: admin });
    expect(res.statusCode).toBe(400);
  });

  it('jugador: filtrado por Facción propia, igual que la proyección', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    const fAna = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    const fundadaAna = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: fAna.json().resultado.datos.faccionId, posicion: { x: 500, y: 500 } } },
    });
    const asentamientoAna = fundadaAna.json().resultado.datos.asentamientoId;

    const luis = await jugadorEn('g1', 'luis');
    const fLuis = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: luis,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Troya' } },
    });
    const fundadaLuis = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: luis,
      payload: { tipo: 'fundarAsentamiento', params: { faccionId: fLuis.json().resultado.datos.faccionId, posicion: { x: 1200, y: 1200 } } },
    });
    const asentamientoLuis = fundadaLuis.json().resultado.datos.asentamientoId;

    const eventosAna = (await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1/eventos?desde=0', headers: ana })).json().eventos;
    // Los eventos de Facción (`crearFaccion`, sin `asentamientoId`) son públicos para cualquiera — mismo
    // criterio que `facciones` en la proyección ("sin redactar"). Lo que SÍ se filtra es lo atribuido a un
    // asentamiento: Ana ve el suyo, nunca el de Luis.
    expect(eventosAna.some((e: { asentamientoId?: string }) => e.asentamientoId === asentamientoAna)).toBe(true);
    expect(eventosAna.some((e: { asentamientoId?: string }) => e.asentamientoId === asentamientoLuis)).toBe(false);
  });

  it('401 sin sesion', async () => {
    await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/eventos' });
    expect(res.statusCode).toBe(401);
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

  it('la proyeccion trae preciosReferencia (doc 9): calculado en el servidor, nunca por el jugador', async () => {
    await partidaCreada('g1');
    const auth = await jugadorEn('g1');

    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });

    expect(res.json().proyeccion.preciosReferencia.madera).toEqual(expect.any(Number));
  });

  it('la respuesta de administrador NO lleva proyeccion: su GET de estado completo ya esta ahi', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'alternarFaccionNpc', params: { faccionId: 'no-existe', activo: true } },
    });

    // 200, no 403: quien crea la partida recibe Membresia `administrador_partida` (`otorgarAdministracion`),
    // así que SÍ pasa la matriz de autorización — lo que rechaza esta petición es que la Facción no existe,
    // no el rol. Antes del fix de C8 (`rolEnPartida`, 2026-08-26) esto daba 403 en todos los casos, y esta
    // aserción no lo distinguía: un cuerpo `{error}` de un 403 también carece de `proyeccion`.
    expect(res.statusCode).toBe(200);
    expect(res.json().resultado.ok).toBe(false);
    expect(res.json().proyeccion).toBeUndefined();
  });

  it('alternarFaccionNpc: el administrador que crea la partida SÍ puede (doc 5, fix C8 2026-08-26)', async () => {
    const { admin } = await partidaCreada('g1');
    const auth = await jugadorEn('g1');
    const crear = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: auth,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
    });
    const faccionId = crear.json().resultado.datos.faccionId;

    const res = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'alternarFaccionNpc', params: { faccionId, activo: true } },
    });

    expect(res.statusCode).toBe(200);
    expect(res.json().resultado.ok).toBe(true);

    const estado = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: admin });
    expect(estado.json().faccionesNpcIds).toContain(faccionId);
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

  describe('esquema de params por comando (Fase C9)', () => {
    it('400, no 409: falta un campo requerido', async () => {
      await partidaCreada('g1');
      const auth = await jugadorEn('g1');

      const res = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        // `fundarAsentamiento` exige `faccionId` y `posicion` — este cuerpo no trae ninguno.
        payload: { tipo: 'fundarAsentamiento', params: {} },
      });

      expect(res.statusCode).toBe(400);
    });

    it('400, no 409: tipo JS equivocado en un campo', async () => {
      await partidaCreada('g1');
      const auth = await jugadorEn('g1');

      const res = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'crearFaccion', params: { nombre: 123 } },
      });

      expect(res.statusCode).toBe(400);
    });

    it('400, no un crash del motor: EdificioTipo fuera del catálogo (antes reventaba en `EDIFICIO_CATALOGO[tipo].costo`)', async () => {
      await partidaCreada('g1');
      const auth = await jugadorEn('g1');
      const creada = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
      });
      const faccionId = creada.json().resultado.datos.faccionId;
      const fundada = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'fundarAsentamiento', params: { faccionId, posicion: { x: 500, y: 500 } } },
      });
      const asentamientoId = fundada.json().resultado.datos.asentamientoId;

      // `tipo: 'noExiste'` no pasa el esquema (400) antes de que importe si `cargo`/autorización son
      // correctos — por eso este `cargo` no necesita ser el del Gobernador real de este asentamiento.
      const res = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'anadirEdificioManualmente', params: { asentamientoId, cargo: 'gobernador', tipo: 'noExiste' } },
      });

      expect(res.statusCode).toBe(400);
    });

    it('sigue aceptando un comando bien formado (no es una restricción de más)', async () => {
      await partidaCreada('g1');
      const auth = await jugadorEn('g1');

      const res = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'crearFaccion', params: { nombre: 'Micenas' } },
      });

      expect(res.statusCode).toBe(200);
      expect(res.json().resultado.ok).toBe(true);
    });

    it('un `nombre` vacío SIGUE siendo un rechazo de dominio (200, ok:false), no un 400: el esquema no debe adelantarse a esa regla', async () => {
      await partidaCreada('g1');
      const auth = await jugadorEn('g1');

      const res = await app.inject({
        method: 'POST',
        url: '/v1/jugador/partidas/g1/comandos',
        headers: auth,
        payload: { tipo: 'crearFaccion', params: { nombre: '' } },
      });

      expect(res.statusCode).toBe(200);
      expect(res.json().resultado).toMatchObject({ ok: false, codigoError: 'faccion.nombre_vacio' });
    });

    it('el contrato publicado describe params por comando, ya no `params: {}`', async () => {
      const doc = (await app.inject({ method: 'GET', url: '/v1/openapi.json' })).json();
      const cuerpo = doc.paths['/jugador/partidas/{gameId}/comandos'].post.requestBody.content['application/json'].schema;

      // `@fastify/swagger` convierte `const` a `enum: [valorUnico]` al publicar: OpenAPI 3.0 no tiene `const`
      // (llegó en JSON Schema draft 6, y 3.0 se basa en un dialecto anterior) — ajv en runtime sí lo entiende
      // tal cual (la validación de verdad usa el `schema.body` de Fastify, no este documento).
      // 35 + los 4 de ejércitos (Paso 3 del movimiento de ejércitos, 2026-09-02: movilizar, unirse, replegar
      // y estacionar) + `alternarReabastecerAliados` (Paso 8: abrir el almacén a los ejércitos aliados) +
      // `adjuntarCaravana`/`soltarCaravana` (Paso 9a: el tren de suministros) + `cargarCaravana`/
      // `entregarDeCaravana` (Paso 9b: la escolta) — MENOS `combateCampoAbierto` e `interceptarCaravana`,
      // retirados en el Paso 11 (2026-09-04) al pasar a ser resoluciones del motor por geometría. La
      // superficie de jugador encogió, que es parte del diseño: se manda un ejército en vez de declarar un
      // ataque desde el sofá.
      //
      // +3 con la presencia del jugador (paso 3 del jugador situado, 2026-09-06): `salirAlMundo`,
      // `entrarEnAsentamiento` y `salirDeAsentamiento`. Aquí la superficie CRECE, y también es parte del
      // diseño: entrar y salir dejan de ser efectos colaterales de otra cosa y pasan a ser actos del jugador.
      // +1 con `marcharA` (paso 4): la columna que el paso 3 crea, en marcha.
      // +4 con la composición de la columna (paso 4b): unirse en campo, contestar la petición, separarse y
      // ceder el liderazgo.
      // +2 con la puerta (paso 5): fijar la política de acceso y vetar.
      expect(cuerpo.oneOf.length).toBe(52);
      const ramaCrearFaccion = cuerpo.oneOf.find((r: { properties: { tipo: { enum: string[] } } }) => r.properties.tipo.enum[0] === 'crearFaccion');
      expect(ramaCrearFaccion.properties.params.required).toEqual(['nombre']);
    });
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
    expect(res.json().instante).toBe(instanteDeTick(2)); // retomó los 2 ticks ya guardados, no volvió a 0
  });
});

describe('gestión de membresías técnicas (/admin/partidas/:gameId/membresias, cierre de Fase C)', () => {
  /** `usuarioId` de un sujeto tras iniciar sesión — lo que un admin necesita para otorgarle un rol. */
  async function usuarioIdDe(sujetoId: string): Promise<string> {
    const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `dev ${sujetoId}` } });
    return login.json().usuarioId;
  }

  it('el admin otorga un rol, aparece en la lista, y se puede revocar', async () => {
    const { admin } = await partidaCreada('g1');
    const beto = await usuarioIdDe('beto');

    const otorgar = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/membresias',
      headers: admin,
      payload: { usuarioId: beto, rol: 'moderador' },
    });
    expect(otorgar.statusCode).toBe(201);

    const lista = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/membresias', headers: admin });
    expect(lista.json().membresias).toContainEqual(expect.objectContaining({ usuarioId: beto, rol: 'moderador', vigente: true }));

    // Beto ya puede administrar la partida.
    const betoAuth = { authorization: `sesion ${(await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev beto' } })).json().sesionId}` };
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: betoAuth })).statusCode).toBe(200);

    const revocar = await app.inject({ method: 'DELETE', url: `/v1/admin/partidas/g1/membresias/${beto}`, headers: admin });
    expect(revocar.statusCode).toBe(200);

    // Revocada: ya no administra, y la lista lo marca no vigente.
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1', headers: betoAuth })).statusCode).toBe(403);
    const listaTras = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/membresias', headers: admin });
    expect(listaTras.json().membresias).toContainEqual(expect.objectContaining({ usuarioId: beto, vigente: false }));
  });

  it('404 si el usuario nunca inició sesión; 409 si ya tiene membresía', async () => {
    const { admin } = await partidaCreada('g1');
    expect(
      (await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId: 'usuario-999', rol: 'moderador' } })).statusCode
    ).toBe(404);

    const beto = await usuarioIdDe('beto');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId: beto, rol: 'observador' } });
    const repetido = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId: beto, rol: 'moderador' } });
    expect(repetido.statusCode).toBe(409);
  });

  it('el esquema rechaza un rol no otorgable (jugador, administrador_global)', async () => {
    const { admin } = await partidaCreada('g1');
    const beto = await usuarioIdDe('beto');
    for (const rol of ['jugador', 'administrador_global', 'servicio_npc']) {
      const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId: beto, rol } });
      expect(res.statusCode).toBe(400);
    }
  });

  it('un moderador puede administrar la partida pero NO repartir roles', async () => {
    const { admin } = await partidaCreada('g1');
    const beto = await usuarioIdDe('beto');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId: beto, rol: 'moderador' } });
    const betoAuth = { authorization: `sesion ${(await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev beto' } })).json().sesionId}` };
    const carlos = await usuarioIdDe('carlos');

    const res = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: betoAuth, payload: { usuarioId: carlos, rol: 'observador' } });
    expect(res.statusCode).toBe(403);
  });

  it('404 al revocar una membresía inexistente; 403 sin sesión de administración', async () => {
    const { admin } = await partidaCreada('g1');
    expect((await app.inject({ method: 'DELETE', url: '/v1/admin/partidas/g1/membresias/usuario-999', headers: admin })).statusCode).toBe(404);

    const ana = await sesionDe('ana');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/membresias', headers: ana })).statusCode).toBe(403);
  });
});

describe('persistencia de identidad tras "reinicio del proceso" (cierre de Fase C)', () => {
  let enDisco: Awaited<ReturnType<typeof crearRepositorioIdentidadEnDisco>>;

  async function servidorConIdentidadEnDisco(): Promise<FastifyInstance> {
    enDisco = await crearRepositorioIdentidadEnDisco(join(directorio, 'identidad.json'));
    return crearServidor({
      directorio,
      administradoresGlobales: ADMINS,
      identidad: { proveedores: crearRegistroProveedores(proveedoresPorDefecto()), repositorio: enDisco.repositorio },
      // Sin esto, `afterEach` borraba el directorio con una escritura de identidad en vuelo y el test salía
      // intermitente con ENOTEMPTY en Windows.
      alCerrar: () => enDisco.esperarEscrituras(),
    });
  }

  it('una membresía de jugador sobrevive a recrear el servidor sobre el mismo directorio', async () => {
    await app.close();
    app = await servidorConIdentidadEnDisco();

    const admin = await sesionDe('jefa');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId: 'g1', seed: 42 } });
    const ana = await sesionDe('ana');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: ana });
    await enDisco.esperarEscrituras();
    await app.close();

    // Proceso "nuevo": relee el archivo de identidad.
    app = await servidorConIdentidadEnDisco();
    const anaOtraVez = await sesionDe('ana');
    const res = await app.inject({ method: 'GET', url: '/v1/sesiones/actual?gameId=g1', headers: anaOtraVez });
    expect(res.json().rol).toBe('jugador');
  });
});

// Auditoría de comandos (Fase E2). Lo que se vigila aquí es que las CUATRO salidas de `ejecutarComandoHttp`
// dejen línea, más el 400 de esquema que ni siquiera llega a esa función (lo rechaza ajv antes, y lo captura
// el gancho `onError` de la ruta). El módulo por su cuenta se prueba en `auditoria.test.ts`.
// Metricas de operacion (Fase E3). Lo que se vigila por HTTP es la FRONTERA: quien puede pedirlas y que
// llegan con la forma declarada. Que cada numero mida lo que dice, en `metricas.test.ts`.
describe('metricas de operacion (E3)', () => {
  it('GET /admin/metricas devuelve proceso, comandos y partidas abiertas', async () => {
    const { admin } = await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: ana, payload: { tipo: 'crearFaccion', params: { nombre: 'A' } } });
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });

    const res = await app.inject({ method: 'GET', url: '/v1/admin/metricas', headers: admin });

    expect(res.statusCode).toBe(200);
    const m = res.json();
    expect(m.proceso.partidasAbiertas).toBe(1);
    expect(m.comandos.aceptados).toBeGreaterThanOrEqual(1);
    expect(m.auditoriaFallida).toBe(0);
    expect(m.partidas[0].gameId).toBe('g1');
    expect(m.partidas[0].ticksEjecutados).toBe(1);
    expect(m.partidas[0].colaPendiente).toBe(0);
    // Campo a campo y no `toMatchObject`: Fastify filtra la respuesta por el esquema, asi que un campo que
    // exista en el tipo pero falte en `ESQUEMA_METRICAS` se descarta EN CALIENTE y ningun test que llame a
    // `recogerMetricas` directo lo detectaria. Paso justo con `ticksOmitidos`, encontrado verificando en vivo.
    for (const campo of ['tick', 'version', 'tickMsUltimo', 'tickMsMedio', 'tickMsMaximo', 'ultimaRafagaTicks', 'mayorRafagaTicks', 'ticksOmitidos', 'conexiones', 'relojDeMundoActivo']) {
      expect(m.partidas[0]).toHaveProperty(campo);
    }
  });

  it('exige administrador GLOBAL, no basta con administrar una partida', async () => {
    // Describe el PROCESO —memoria, uptime, TODAS las partidas abiertas—, asi que concederlo por membresia de
    // una partida filtraria la actividad de las demas.
    await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    expect((await app.inject({ method: 'GET', url: '/v1/admin/metricas', headers: ana })).statusCode).toBe(403);
    expect((await app.inject({ method: 'GET', url: '/v1/admin/metricas' })).statusCode).toBe(401);
  });

  it('los rechazos quedan contados por causa, no agregados', async () => {
    const { admin } = await partidaCreada('g1');
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/comandos', headers: admin, payload: { tipo: 'crearFaccion', params: { nombre: 'X' } } }); // 403

    const m = (await app.inject({ method: 'GET', url: '/v1/admin/metricas', headers: admin })).json();
    expect(m.comandos.autorizacion).toBe(1);
    expect(m.comandos.dominio).toBe(0);
  });
});

describe('auditoría de comandos (E2)', () => {
  /** La auditoría escribe sin esperar (ver `RegistroDeAuditoria.registrar`), así que hay que drenar antes de
   * leer. Cerrar la app lo hace por el hook `onClose`; aquí basta con un ciclo de la cola. */
  async function auditoriaDe(gameId: string) {
    await app.close();
    const { leerAuditoria } = await import('../auditoria');
    return (await leerAuditoria(directorio, gameId)).entradas;
  }

  it('un comando ACEPTADO deja linea con actor, version e instante de mundo', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Los Alfareros' } },
    });
    expect(res.statusCode).toBe(200);

    const entradas = await auditoriaDe('g1');
    expect(entradas).toHaveLength(1);
    expect(entradas[0]).toMatchObject({ gameId: 'g1', comando: 'crearFaccion', resultado: 'aceptado' });
    // El actor lo resuelve el servidor de la sesión, nunca del cuerpo (doc 2, principio 3). Es el
    // `jugadorId` de la Membresia, que por diseño REUTILIZA el id del Usuario (ver `jugador.ts`: "mismo
    // valor que hoy usa el motor como jugadorId", sin generador de ids aparte).
    expect(entradas[0]!.actor).toMatch(/^usuario-/);
    // `version` e `instante` describen la partida DESPUÉS del comando: son lo que permite cruzar esta línea
    // con el evento de dominio que produjo (`EventoDominioConVersion.version`, C13).
    expect(entradas[0]!.version).toBeGreaterThan(0);
    expect(entradas[0]!.instante).toBe(instanteDeTick(0));
  });

  it('un 403 de autorizacion deja linea — antes de E2 no dejaba ningun rastro', async () => {
    // Es la línea de moderación por excelencia: el intento de actuar sobre lo que no es tuyo.
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/admin/partidas/g1/comandos',
      headers: admin,
      payload: { tipo: 'crearFaccion', params: { nombre: 'X' } },
    });
    expect(res.statusCode).toBe(403);

    const entradas = await auditoriaDe('g1');
    expect(entradas).toHaveLength(1);
    expect(entradas[0]).toMatchObject({ comando: 'crearFaccion', resultado: 'rechazado', causa: 'autorizacion' });
    expect(entradas[0]!.detalle).toBeTruthy();
    // No llegó a ejecutarse, así que no hay instante de mundo que atribuirle.
    expect(entradas[0]!.instante).toBeUndefined();
  });

  it('un rechazo de DOMINIO deja linea con su codigo de error, y no se confunde con un rechazo de permiso', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Primera' } },
    });
    // Segunda Facción con el mismo jugador: el dominio dice que no (un jugador solo crea una).
    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 'Segunda' } },
    });
    expect(res.statusCode).toBe(200); // 200 con `resultado.ok: false`: el dominio rechaza, no la ruta
    expect(res.json().resultado.ok).toBe(false);

    const entradas = await auditoriaDe('g1');
    expect(entradas.map((e) => e.resultado)).toEqual(['aceptado', 'rechazado']);
    expect(entradas[1]).toMatchObject({ causa: 'dominio' });
    expect(entradas[1]!.detalle).toBe(res.json().resultado.codigoError);
  });

  it('un 400 de esquema deja linea aunque nunca llegue al manejador', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    const res = await app.inject({
      method: 'POST',
      url: '/v1/jugador/partidas/g1/comandos',
      headers: ana,
      payload: { tipo: 'crearFaccion', params: { nombre: 123 } }, // `nombre` no es string
    });
    expect(res.statusCode).toBe(400);

    const entradas = await auditoriaDe('g1');
    expect(entradas).toHaveLength(1);
    expect(entradas[0]).toMatchObject({ comando: 'crearFaccion', resultado: 'rechazado', causa: 'esquema' });
    // Se identifica al actor pese a que la petición no pasó validación: la sesión se resuelve de la cabecera,
    // que es independiente del cuerpo malformado.
    expect(entradas[0]!.actor).toMatch(/^usuario-/);
  });

  it('GET /admin/partidas/:gameId/auditoria devuelve el registro, filtrable', async () => {
    // Sin esta ruta la auditoria seria un archivo que nadie puede consultar.
    const { admin } = await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: ana, payload: { tipo: 'crearFaccion', params: { nombre: 'A' } } });
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/comandos', headers: admin, payload: { tipo: 'crearFaccion', params: { nombre: 'X' } } }); // 403

    const todo = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/auditoria', headers: admin });
    expect(todo.statusCode).toBe(200);
    expect(todo.json().entradas).toHaveLength(2);
    // `corruptas` viaja siempre, no solo cuando es > 0: quien lee tiene que poder distinguir un registro
    // completo de uno con agujeros.
    expect(todo.json().corruptas).toBe(0);

    // La vista de moderacion: solo lo rechazado.
    const rechazos = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/auditoria?soloRechazos=true', headers: admin });
    expect(rechazos.json().entradas.map((e: { causa: string }) => e.causa)).toEqual(['autorizacion']);
  });

  it('la auditoria es SOLO de administracion: un jugador no audita a los demas', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/auditoria', headers: ana });
    expect(res.statusCode).toBe(403);
  });

  it('una partida sin auditoria todavia responde vacio, no 404', async () => {
    const { admin } = await partidaCreada('g1');
    const res = await app.inject({ method: 'GET', url: '/v1/admin/partidas/g1/auditoria', headers: admin });
    expect(res.statusCode).toBe(200);
    expect(res.json()).toEqual({ entradas: [], corruptas: 0 });
  });

  it('cada partida audita en su propio archivo', async () => {
    await partidaCreada('g1');
    await partidaCreada('g2');
    const ana = await jugadorEn('g1');
    const bruno = await jugadorEn('g2', 'bruno');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/comandos', headers: ana, payload: { tipo: 'crearFaccion', params: { nombre: 'A' } } });
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g2/comandos', headers: bruno, payload: { tipo: 'crearFaccion', params: { nombre: 'B' } } });

    await app.close();
    const { leerAuditoria } = await import('../auditoria');
    expect((await leerAuditoria(directorio, 'g1')).entradas.map((e) => e.gameId)).toEqual(['g1']);
    expect((await leerAuditoria(directorio, 'g2')).entradas.map((e) => e.gameId)).toEqual(['g2']);
  });
});
