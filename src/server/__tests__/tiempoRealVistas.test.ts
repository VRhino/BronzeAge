// Canales de vista de administración por WebSocket (contrato 03, fase 1): handshake solo para administradores, foto al suscribirse, delta tras
// un tick, y que un jugador no pueda ni nombrar un canal `admin/*`. Usa `app.injectWS()` como `tiempoReal.test.ts`.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import type WebSocket from 'ws';
import { crearServidor } from '../api';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresPorDefecto } from '../identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../identidad/repositorioEnMemoria';

const ADMINS = [{ proveedor: 'dev', sujetoId: 'jefa' }];

let directorio: string;
let app: FastifyInstance;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-vistas-'));
  app = crearServidor({
    directorio,
    administradoresGlobales: ADMINS,
    identidad: { proveedores: crearRegistroProveedores(proveedoresPorDefecto()), repositorio: crearRepositorioIdentidadEnMemoria() },
    vistas: { coalescerMs: 0 },
  });
  await app.ready();
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

async function sesionDe(sujetoId: string) {
  const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `dev ${sujetoId}` } });
  return { authorization: `sesion ${login.json().sesionId}` };
}

/** Los mensajes que vayan llegando, para esperar al n-ésimo sin carreras. */
function recoger(ws: WebSocket): Record<string, any>[] {
  const recibidos: Record<string, any>[] = [];
  ws.on('message', (data: Buffer) => recibidos.push(JSON.parse(data.toString())));
  return recibidos;
}
const hasta = async (cond: () => boolean) => {
  for (let i = 0; i < 100 && !cond(); i++) await new Promise((r) => setTimeout(r, 10));
  if (!cond()) throw new Error('no llegó lo esperado');
};

async function partida(gameId = 'g1') {
  const admin = await sesionDe('jefa');
  await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin, payload: { gameId, seed: 42 } });
  return admin;
}

describe('handshake de /admin/partidas/:gameId/tiempo-real', () => {
  it('sin sesión 401; sin rol de administración 403', async () => {
    await partida();
    await expect(app.injectWS('/v1/admin/partidas/g1/tiempo-real')).rejects.toThrow(/401/);
    const ana = await sesionDe('ana');
    await expect(app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: ana })).rejects.toThrow(/403/);
  });

  it('una partida que no está abierta es 404', async () => {
    const admin = await sesionDe('jefa');
    await expect(app.injectWS('/v1/admin/partidas/nada/tiempo-real', { headers: admin })).rejects.toThrow(/404/);
  });
});

describe('admin/partida', () => {
  it('suscribir da suscrito + foto, y un tick de administración llega como delta encadenado', async () => {
    const admin = await partida();
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);

    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida' }));
    await hasta(() => recibidos.length >= 2);
    const [suscrito, foto] = recibidos;
    expect(suscrito).toMatchObject({ tipo: 'suscrito', canal: 'admin/partida' });
    expect(foto).toMatchObject({ tipo: 'foto', gameId: 'g1', canal: 'admin/partida' });

    const tick = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    expect(tick.statusCode).toBe(200);
    await hasta(() => recibidos.some((m) => m.tipo === 'delta'));
    const delta = recibidos.find((m) => m.tipo === 'delta')!;
    expect(delta).toMatchObject({ canal: 'admin/partida', desde: foto!.version });
    expect(delta.hasta).toBeGreaterThan(delta.desde);
    ws.terminate();
  });

  it('resincronizar devuelve la foto; un canal inexistente, canal_desconocido', async () => {
    const admin = await partida();
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida' }));
    await hasta(() => recibidos.length >= 2);

    ws.send(JSON.stringify({ accion: 'resincronizar', canal: 'admin/partida' }));
    await hasta(() => recibidos.length >= 3);
    expect(recibidos[2]).toMatchObject({ tipo: 'error', codigo: 'demasiado_frecuente' }); // acaba de recibir la primera

    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'vista/mapa' }));
    await hasta(() => recibidos.length >= 4);
    expect(recibidos[3]).toMatchObject({ tipo: 'error', canal: 'vista/mapa', codigo: 'canal_desconocido' });
    ws.terminate();
  });
});

describe('un jugador no ve canales de administración', () => {
  it('suscribir a admin/* es canal_desconocido; vista/* sin héroe (o de una plaza que no es suya), no_autorizado', async () => {
    await partida();
    const ana = await sesionDe('ana');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: ana });
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana });
    const recibidos = recoger(ws);
    for (const canal of ['admin/partida', 'vista/asentamiento/a1', 'vista/mapa']) ws.send(JSON.stringify({ accion: 'suscribir', canal }));
    await hasta(() => recibidos.length >= 3);
    expect(recibidos.map((m) => m.codigo)).toEqual(['canal_desconocido', 'no_autorizado', 'no_autorizado']);
    ws.terminate();
  });
});

/** Unido a la partida y con su héroe creado, que es lo que hace falta para tener vistas. */
async function jugadorConHeroe(sujetoId: string) {
  const auth = await sesionDe(sujetoId);
  await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: auth });
  const creado = await app.inject({
    method: 'POST',
    url: '/v1/jugador/partidas/g1/comandos',
    headers: auth,
    payload: {
      tipo: 'crearHeroe',
      params: { displayName: sujetoId, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' } },
    },
  });
  return { auth, heroeId: creado.json().resultado.datos.heroeId as string };
}

describe('vista/mapa y vista/heroe', () => {
  it('juntas reproducen la proyección HTTP del jugador: nada de más y nada de menos en los campos que reparten', async () => {
    await partida();
    const { auth } = await jugadorConHeroe('ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });
    const recibidos = recoger(ws);
    for (const canal of ['vista/mapa', 'vista/heroe']) ws.send(JSON.stringify({ accion: 'suscribir', canal }));
    await hasta(() => recibidos.filter((m) => m.tipo === 'foto').length === 2);

    const http = (await app.inject({ method: 'GET', url: '/v1/jugador/partidas/g1', headers: auth })).json() as Record<string, unknown>;
    const fotos = Object.fromEntries(recibidos.filter((m) => m.tipo === 'foto').map((m) => [m.canal, m.datos as Record<string, unknown>]));
    const norm = (x: unknown) => JSON.parse(JSON.stringify(x));
    for (const [campo, valor] of Object.entries(fotos['vista/mapa']!)) expect(norm(valor), `vista/mapa.${campo}`).toEqual(norm(http[campo]));
    // vista/heroe: igual salvo los dos campos con id sintético (informes y historial) que el canal reparte con `id`.
    for (const [campo, valor] of Object.entries(fotos['vista/heroe']!)) {
      if (campo === 'historial' || campo === 'informesDeInspeccion') continue;
      expect(norm(valor), `vista/heroe.${campo}`).toEqual(norm(http[campo]));
    }
    // Ningún campo viaja por los dos canales.
    const enMapa = new Set(Object.keys(fotos['vista/mapa']!));
    expect(Object.keys(fotos['vista/heroe']!).filter((k) => enMapa.has(k))).toEqual([]);
    ws.terminate();
  });

  it('un tick de administración llega a vista/heroe y a vista/mapa como deltas encadenados', async () => {
    const admin = await partida();
    const { auth } = await jugadorConHeroe('ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });
    const recibidos = recoger(ws);
    for (const canal of ['vista/mapa', 'vista/heroe']) ws.send(JSON.stringify({ accion: 'suscribir', canal }));
    await hasta(() => recibidos.filter((m) => m.tipo === 'foto').length === 2);
    const versiones = Object.fromEntries(recibidos.filter((m) => m.tipo === 'foto').map((m) => [m.canal, m.version]));

    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await hasta(() => recibidos.filter((m) => m.tipo === 'delta').length === 2);
    for (const d of recibidos.filter((m) => m.tipo === 'delta')) expect(d.desde).toBe(versiones[d.canal]);
    ws.terminate();
  });

  it('perder la membresía revoca los canales en el siguiente flush', async () => {
    const admin = await partida();
    const { auth } = await jugadorConHeroe('ana');
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: auth });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'vista/mapa' }));
    await hasta(() => recibidos.some((m) => m.tipo === 'foto'));

    const usuarioId = (await app.inject({ method: 'GET', url: '/v1/sesiones/actual', headers: auth })).json().usuarioId as string;
    const revocada = await app.inject({ method: 'DELETE', url: `/v1/admin/partidas/g1/membresias/${usuarioId}`, headers: admin });
    expect(revocada.statusCode).toBe(200);
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await hasta(() => recibidos.some((m) => m.tipo === 'revocado'));
    expect(recibidos.find((m) => m.tipo === 'revocado')).toMatchObject({ canal: 'vista/mapa' });
    ws.terminate();
  });
});

describe('métricas', () => {
  it('/admin/metricas incluye los canales de vista con sus suscriptores', async () => {
    const admin = await partida();
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida' }));
    await hasta(() => recibidos.length >= 2);

    const m = await app.inject({ method: 'GET', url: '/v1/admin/metricas', headers: admin });
    expect(m.json().vistas.suscriptores).toBe(1);
    expect(m.json().vistas.porCanal['admin/partida']).toMatchObject({ suscriptores: 1, claves: 1 });
    ws.terminate();
  });
});
