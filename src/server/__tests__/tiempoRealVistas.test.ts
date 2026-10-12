// Canales de vista de administración por WebSocket (contrato 03, fase 1): handshake solo para administradores, foto al suscribirse, delta tras
// un tick, y que un jugador no pueda ni nombrar un canal `admin/*`. Usa `app.injectWS()` como `tiempoReal.test.ts`.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import type WebSocket from 'ws';
import { crearServidor } from '../api';
import { componerProyeccion } from '../../session/proyecciones/jugador';
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
    // vista/mapa se compone con los ajustes de vista/heroe (la unión de los dos canales, 03 §13.1) y da la proyección HTTP.
    const heroe = fotos['vista/heroe']!;
    const ajustes = {
      plazaPisadaId: heroe['plazaPisadaId'],
      detalleDeAvistados: heroe['detalleDeAvistados'] ?? {},
      campamentosPropios: heroe['campamentosPropios'] ?? [],
      heroesVisiblesExtra: heroe['heroesVisiblesExtra'] ?? [],
      ladoPropioPorBatalla: heroe['ladoPropioPorBatalla'] ?? {},
      batallasPropias: heroe['batallasPropias'] ?? [],
    };
    const compuesta = componerProyeccion(fotos['vista/mapa'] as never, (heroe['heroe'] as { id: string }).id, ajustes as never) as Record<string, unknown>;
    for (const campo of Object.keys(fotos['vista/mapa']!)) expect(norm(compuesta[campo]), `vista/mapa.${campo}`).toEqual(norm(http[campo]));
    // vista/heroe: igual salvo los ajustes y los dos campos con id sintético (informes y historial).
    for (const [campo, valor] of Object.entries(heroe)) {
      if (campo === 'historial' || campo === 'informesDeInspeccion' || campo in ajustes) continue;
      expect(norm(valor), `vista/heroe.${campo}`).toEqual(norm(http[campo]));
    }
    // Ningún campo viaja por los dos canales.
    const enMapa = new Set(Object.keys(fotos['vista/mapa']!));
    expect(Object.keys(heroe).filter((k) => enMapa.has(k))).toEqual([]);
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

describe('ritmo', () => {
  it('suscribir con ritmo lo recorta y lo devuelve en suscrito; un ritmo que no es número es error de formato', async () => {
    const admin = await partida();
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida', ritmo: 100000 }));
    await hasta(() => recibidos.length >= 2);
    expect(recibidos[0]).toMatchObject({ tipo: 'suscrito', canal: 'admin/partida', ritmo: 60 });

    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida', ritmo: 'rapido' }));
    await hasta(() => recibidos.length >= 3);
    expect(recibidos[2]).toMatchObject({ tipo: 'error', codigo: 'formato' });
    ws.terminate();
  });
});

describe('admin/eventos', () => {
  it('empuja TODOS los eventos de la partida con su version, sin filtro, y suscrito no lleva foto', async () => {
    const admin = await partida();
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/eventos' }));
    await hasta(() => recibidos.length >= 1);
    expect(recibidos[0]).toEqual({ tipo: 'suscrito', canal: 'admin/eventos' });

    await jugadorConHeroe('ana'); // un comando de jugador: sus eventos llegan a la administración aunque no sean de ninguna plaza
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await hasta(() => recibidos.some((m) => m.tipo === 'evento'));
    const eventos = recibidos.filter((m) => m.tipo === 'evento');
    for (const m of eventos) {
      expect(m).toMatchObject({ tipo: 'evento', gameId: 'g1', canal: 'admin/eventos' });
      expect(m.evento).toMatchObject({ codigo: expect.any(String), mensaje: expect.any(String), momento: expect.any(String) });
    }
    expect(eventos.some((m) => typeof m.evento.version === 'number')).toBe(true);
    ws.terminate();
  });

  it('un jugador no puede suscribirse (canal_desconocido en su endpoint) y quien se desuscribe deja de recibir', async () => {
    const admin = await partida();
    const ana = await sesionDe('ana');
    await app.inject({ method: 'POST', url: '/v1/jugador/partidas/g1/membresia', headers: ana });
    const wsJ = await app.injectWS('/v1/jugador/partidas/g1/tiempo-real', { headers: ana });
    const delJugador = recoger(wsJ);
    wsJ.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/eventos' }));
    await hasta(() => delJugador.length >= 1);
    expect(delJugador[0]).toMatchObject({ tipo: 'error', canal: 'admin/eventos', codigo: 'canal_desconocido' });
    wsJ.terminate();

    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: admin });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/eventos' }));
    await hasta(() => recibidos.length >= 1);
    ws.send(JSON.stringify({ accion: 'desuscribir', canal: 'admin/eventos' }));
    await hasta(() => recibidos.some((m) => m.tipo === 'desuscrito'));
    const n = recibidos.length;
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await new Promise((r) => setTimeout(r, 50));
    expect(recibidos).toHaveLength(n);
    ws.terminate();
  });

  it('perder el permiso de administración revoca el canal en la siguiente difusión', async () => {
    const admin = await partida();
    // Un administrador de partida (no global), al que luego se le quita el rol.
    const bea = await sesionDe('bea');
    const usuarioId = (await app.inject({ method: 'GET', url: '/v1/sesiones/actual', headers: bea })).json().usuarioId as string;
    const alta = await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/membresias', headers: admin, payload: { usuarioId, rol: 'administrador_partida' } });
    expect(alta.statusCode).toBeLessThan(300);
    const ws = await app.injectWS('/v1/admin/partidas/g1/tiempo-real', { headers: bea });
    const recibidos = recoger(ws);
    ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/eventos' }));
    await hasta(() => recibidos.length >= 1);

    await app.inject({ method: 'DELETE', url: `/v1/admin/partidas/g1/membresias/${usuarioId}`, headers: admin });
    await app.inject({ method: 'POST', url: '/v1/admin/partidas/g1/tick', headers: admin });
    await hasta(() => recibidos.some((m) => m.tipo === 'revocado'));
    expect(recibidos.find((m) => m.tipo === 'revocado')).toMatchObject({ canal: 'admin/eventos' });
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
