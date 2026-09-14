// Contrato del WebSocket de tiempo real (Fase C5): autenticación en el handshake, protocolo de
// suscripción/desuscripción, y difusión de eventos de dominio a quien está suscrito al canal que le
// corresponde a cada uno. Usa `app.injectWS()` — el equivalente de `.inject()` para WebSocket, sin abrir un
// socket real (ver README de `@fastify/websocket`).
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import type WebSocket from 'ws';
import { crearServidor } from '../api';
import { HubDeDifusion } from '../difusion/hub';

const ADMINS = [{ proveedor: 'dev', sujetoId: 'jefa' }];

let directorio: string;
let app: FastifyInstance;
let hub: HubDeDifusion;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-ws-'));
  hub = new HubDeDifusion();
  app = crearServidor({ directorio, administradoresGlobales: ADMINS, hub });
  await app.ready(); // injectWS lo exige (README: "fastify.ready() needs to be awaited")
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

async function sesionDe(sujetoId: string) {
  const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `dev ${sujetoId}` } });
  return { authorization: `sesion ${login.json().sesionId}` };
}

async function partidaCreada(gameId = 'g1') {
  const admin = await sesionDe('jefa');
  await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId, seed: 42 } });
}

/** Unido a la partida y ya con su héroe, que es quien actúa y quien se suscribe. */
async function jugadorEn(gameId: string, sujetoId: string) {
  const auth = await sesionDe(sujetoId);
  const res = await app.inject({ method: 'POST', url: `/v1/jugador/partidas/${gameId}/membresia`, headers: auth });
  await ejecutar(gameId, auth, 'crearHeroe', {
    displayName: sujetoId,
    classDefinitionId: 'Spear',
    genero: 'femenino',
    avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
  });
  return { auth, jugadorId: res.json().jugadorId as string };
}

async function ejecutar(gameId: string, auth: { authorization: string }, tipo: string, params: unknown) {
  return app.inject({ method: 'POST', url: `/v1/jugador/partidas/${gameId}/comandos`, headers: auth, payload: { tipo, params } });
}

/** Próximo mensaje JSON que llegue por el socket. */
function esperarMensaje(ws: WebSocket): Promise<Record<string, unknown>> {
  return new Promise((resolve) => ws.once('message', (data: Buffer) => resolve(JSON.parse(data.toString()))));
}

/** Suscribe y espera la confirmación — abreviatura usada en casi todos los tests de este archivo. */
async function suscribir(ws: WebSocket, canal: string) {
  ws.send(JSON.stringify({ accion: 'suscribir', canal }));
  return esperarMensaje(ws);
}

describe('handshake: autenticación antes de completar la conexión', () => {
  it('rechaza sin sesion (401), como respuesta HTTP a la peticion de upgrade, no como un socket abierto y cerrado', async () => {
    await partidaCreada('g1');
    await expect(app.injectWS('/v1/jugador/partidas/g1/tiempo-real')).rejects.toThrow(/401/);
  });

  it('rechaza con sesion pero sin membresia de jugador (403)', async () => {
    await partidaCreada('g1');
    const auth = await sesionDe('ana');
    await expect(app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth })).rejects.toThrow(/403/);
  });

  it('un administrador de la partida tampoco puede conectar: para jugar hace falta ser jugador', async () => {
    await partidaCreada('g1');
    const admin = await sesionDe('jefa');
    await expect(app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: admin })).rejects.toThrow(/403/);
  });

  it('acepta con sesion + membresia de jugador: la conexion queda abierta', async () => {
    await partidaCreada('g1');
    const { auth } = await jugadorEn('g1', 'ana');

    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });
    expect(ws.readyState).toBe(ws.OPEN);
    expect(hub.conexionesAbiertas('g1')).toBe(1);
    ws.terminate();
  });

  it('acepta la sesion tambien por query string (?sesion=), la via que un navegador SI puede usar', async () => {
    await partidaCreada('g1');
    const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev ana' } });
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: { authorization: `sesion ${login.json().sesionId}` } });

    const ws = await app.injectWS(`/v1/jugador/partidas/g1/tiempo-real?sesion=${login.json().sesionId}`);
    expect(ws.readyState).toBe(ws.OPEN);
    ws.terminate();
  });
});

describe('protocolo de suscripcion', () => {
  it('suscribirse al canal general siempre se acepta', async () => {
    await partidaCreada('g1');
    const { auth } = await jugadorEn('g1', 'ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });

    expect(await suscribir(ws, 'mapa/general')).toEqual({ tipo: 'suscrito', canal: 'mapa/general' });
    ws.terminate();
  });

  it('suscribirse al propio asentamiento se acepta; a uno ajeno se rechaza', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    const faccion = await ejecutar('g1', ana.auth, 'crearFaccion', { nombre: 'Micenas' });
    const fundacion = await ejecutar('g1', ana.auth, 'fundarAsentamiento', {
      faccionId: faccion.json().resultado.datos.faccionId,
      posicion: { x: 500, y: 500 },
    });
    const asentamientoId = fundacion.json().resultado.datos.asentamientoId as string;

    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana.auth });

    expect(await suscribir(ws, `asentamiento/${asentamientoId}`)).toEqual({ tipo: 'suscrito', canal: `asentamiento/${asentamientoId}` });
    expect(await suscribir(ws, 'asentamiento/no-existe')).toEqual({ tipo: 'error', canal: 'asentamiento/no-existe', error: 'no autorizado' });
    ws.terminate();
  });

  it('un mensaje que no es {accion, canal} responde error, sin cerrar la conexion', async () => {
    await partidaCreada('g1');
    const { auth } = await jugadorEn('g1', 'ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });

    ws.send('no es json');
    expect((await esperarMensaje(ws)).tipo).toBe('error');

    ws.send(JSON.stringify({ hola: 'mundo' }));
    expect((await esperarMensaje(ws)).tipo).toBe('error');

    // La conexión sigue viva tras los dos mensajes malformados.
    expect(await suscribir(ws, 'mapa/general')).toEqual({ tipo: 'suscrito', canal: 'mapa/general' });
    ws.terminate();
  });

  it('desuscribirse no exige autorizacion: siempre se acepta, incluso de un canal nunca suscrito', async () => {
    await partidaCreada('g1');
    const { auth } = await jugadorEn('g1', 'ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });

    ws.send(JSON.stringify({ accion: 'desuscribir', canal: 'asentamiento/nunca-suscrito' }));
    expect(await esperarMensaje(ws)).toEqual({ tipo: 'desuscrito', canal: 'asentamiento/nunca-suscrito' });
    ws.terminate();
  });
});

describe('difusion de eventos tras un comando', () => {
  it('un evento GLOBAL (sin asentamientoId) llega a quien esta suscrito a mapa/general', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');

    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana.auth });
    await suscribir(ws, 'mapa/general');

    // El listener se registra ANTES de disparar el comando: la difusión llega dentro de la propia petición
    // HTTP (`ejecutarComandoHttp` -> `hub.difundir`), así que para cuando `ejecutar` resuelve, el mensaje ya
    // pudo haberse mandado — engancharse después lo perdería.
    const difundidoPromesa = esperarMensaje(ws);
    await ejecutar('g1', ana.auth, 'crearFaccion', { nombre: 'Micenas' });

    const difundido = await difundidoPromesa;
    expect(difundido.tipo).toBe('evento');
    expect(difundido.canal).toBe('mapa/general');
    expect((difundido.evento as { codigo: string }).codigo).toBe('faccion.creada');
    ws.terminate();
  });

  it('un evento de UN asentamiento solo llega a quien esta suscrito a ESE asentamiento, no a mapa/general', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    const faccion = await ejecutar('g1', ana.auth, 'crearFaccion', { nombre: 'Micenas' });
    const faccionId = faccion.json().resultado.datos.faccionId;
    const fundacion = await ejecutar('g1', ana.auth, 'fundarAsentamiento', { faccionId, posicion: { x: 500, y: 500 } });
    const asentamientoId = fundacion.json().resultado.datos.asentamientoId as string;

    const wsGeneral = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana.auth });
    await suscribir(wsGeneral, 'mapa/general');

    const wsAsentamiento = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana.auth });
    await suscribir(wsAsentamiento, `asentamiento/${asentamientoId}`);

    let generalRecibioAlgo = false;
    wsGeneral.on('message', () => {
      generalRecibioAlgo = true;
    });
    const difundidoPromesa = esperarMensaje(wsAsentamiento); // registrado ANTES del comando, ver test anterior

    await ejecutar('g1', ana.auth, 'renombrarAsentamiento', { asentamientoId, nombre: 'Nueva Micenas' });

    const difundido = await difundidoPromesa;
    expect(difundido.canal).toBe(`asentamiento/${asentamientoId}`);
    expect(generalRecibioAlgo).toBe(false); // el evento de un asentamiento NO se filtra a mapa/general

    wsGeneral.terminate();
    wsAsentamiento.terminate();
  });

  it('tras desuscribirse, ya no llegan eventos de ese canal', async () => {
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');

    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana.auth });
    await suscribir(ws, 'mapa/general');
    ws.send(JSON.stringify({ accion: 'desuscribir', canal: 'mapa/general' }));
    await esperarMensaje(ws);

    let recibioAlgo = false;
    ws.on('message', () => {
      recibioAlgo = true;
    });
    await ejecutar('g1', ana.auth, 'crearFaccion', { nombre: 'Micenas' });
    await new Promise((r) => setTimeout(r, 50)); // deja tiempo a que llegara, si fuera a llegar

    expect(recibioAlgo).toBe(false);
    ws.terminate();
  });

  it('otro jugador de OTRA Faccion, tambien suscrito a mapa/general, recibe el mismo evento global', async () => {
    // mapa/general es del canal general de la PARTIDA, no de una Faccion: lo global es publico entre
    // jugadores (ver session/proyecciones/jugador.ts, `facciones` sin filtrar).
    await partidaCreada('g1');
    const ana = await jugadorEn('g1', 'ana');
    const luis = await jugadorEn('g1', 'luis');

    const wsLuis = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: luis.auth });
    await suscribir(wsLuis, 'mapa/general');

    const difundidoPromesa = esperarMensaje(wsLuis); // registrado antes del comando — ver test de arriba
    await ejecutar('g1', ana.auth, 'crearFaccion', { nombre: 'Micenas' });

    const difundido = await difundidoPromesa;
    expect((difundido.evento as { codigo: string }).codigo).toBe('faccion.creada');
    wsLuis.terminate();
  });

  it('la conexion se limpia al cerrarse: conexionesAbiertas baja a 0', async () => {
    await partidaCreada('g1');
    const { auth } = await jugadorEn('g1', 'ana');

    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });
    expect(hub.conexionesAbiertas('g1')).toBe(1);

    const cerrado = new Promise((resolve) => ws.once('close', resolve));
    ws.terminate();
    await cerrado;

    expect(hub.conexionesAbiertas('g1')).toBe(0);
  });
});
