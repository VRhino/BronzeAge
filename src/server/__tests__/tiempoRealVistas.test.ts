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
  it('suscribir a admin/* o vista/* por el WS de jugador es canal_desconocido, sin revelar nada', async () => {
    const admin = await partida();
    const ana = await sesionDe('ana');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: ana });
    const ws = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana });
    const recibidos = recoger(ws);
    for (const canal of ['admin/partida', 'vista/mapa']) ws.send(JSON.stringify({ accion: 'suscribir', canal }));
    await hasta(() => recibidos.length >= 2);
    expect(recibidos.map((m) => m.codigo)).toEqual(['canal_desconocido', 'canal_desconocido']);
    expect(admin.authorization).toBeTruthy();
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
