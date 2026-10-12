// Compresión `permessage-deflate` del WebSocket (contrato 03 §13.2): se negocia, es opcional, y no cambia lo que llega. Con un servidor real
// escuchando, porque `injectWS` no pasa por la negociación del handshake.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import WebSocket from 'ws';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { crearServidor, type OpcionesServidor } from '../api';
import { compresionDeEntorno, perMessageDeflateDe, UMBRAL_POR_DEFECTO } from '../difusion/compresion';
import { proveedoresPorDefecto } from '../identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../identidad/repositorioEnMemoria';

let directorio: string;
let app: FastifyInstance | undefined;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-deflate-'));
});
afterEach(async () => {
  await app?.close();
  await rm(directorio, { recursive: true, force: true });
});

async function servidor(compresionWs?: OpcionesServidor['compresionWs']) {
  app = crearServidor({
    directorio,
    administradoresGlobales: [{ proveedor: 'dev', sujetoId: 'jefa' }],
    identidad: { proveedores: crearRegistroProveedores(proveedoresPorDefecto()), repositorio: crearRepositorioIdentidadEnMemoria() },
    vistas: { coalescerMs: 0 },
    ...(compresionWs === undefined ? {} : { compresionWs }),
  });
  await app.listen({ port: 0, host: '127.0.0.1' });
  const puerto = (app.server.address() as { port: number }).port;
  const login = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } });
  const sesion = login.json().sesionId as string;
  await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: { authorization: `sesion ${sesion}` }, payload: { gameId: 'g1', seed: 42 } });
  return { url: `ws://127.0.0.1:${puerto}/v1/admin/partidas/g1/tiempo-real?sesion=${sesion}` };
}

/** Conecta, suscribe a `admin/partida` y devuelve la extensión negociada y la foto recibida. */
async function foto(url: string, cliente: WebSocket.ClientOptions) {
  const ws = new WebSocket(url, cliente);
  const mensajes: Record<string, any>[] = [];
  ws.on('message', (d: Buffer) => mensajes.push(JSON.parse(d.toString())));
  await new Promise<void>((ok, mal) => {
    ws.once('open', ok);
    ws.once('error', mal);
  });
  const extensiones = ws.extensions;
  ws.send(JSON.stringify({ accion: 'suscribir', canal: 'admin/partida' }));
  for (let i = 0; i < 100 && !mensajes.some((m) => m.tipo === 'foto'); i++) await new Promise((r) => setTimeout(r, 10));
  ws.close();
  return { extensiones, foto: mensajes.find((m) => m.tipo === 'foto')! };
}

describe('permessage-deflate', () => {
  it('por defecto se negocia con un cliente que la ofrece, y la foto llega entera', async () => {
    const { url } = await servidor();
    const r = await foto(url, { perMessageDeflate: true });
    expect(r.extensiones).toContain('permessage-deflate');
    expect(r.foto).toMatchObject({ tipo: 'foto', canal: 'admin/partida' });
  });

  it('un cliente sin soporte (no la ofrece) recibe lo mismo, sin comprimir', async () => {
    const { url } = await servidor();
    const r = await foto(url, { perMessageDeflate: false });
    expect(r.extensiones).toBe('');
    expect(r.foto).toMatchObject({ tipo: 'foto', canal: 'admin/partida' });
  });

  it('apagada por configuración no se negocia aunque el cliente la ofrezca', async () => {
    const { url } = await servidor(false);
    const r = await foto(url, { perMessageDeflate: true });
    expect(r.extensiones).toBe('');
    expect(r.foto.tipo).toBe('foto');
  });
});

describe('configuración', () => {
  it('umbral y nivel por defecto, y el entorno los ajusta o la apaga', () => {
    expect(perMessageDeflateDe(undefined)).toMatchObject({ threshold: UMBRAL_POR_DEFECTO, zlibDeflateOptions: { level: 3 }, serverNoContextTakeover: true });
    expect(perMessageDeflateDe({ umbralBytes: 4096, nivel: 6 })).toMatchObject({ threshold: 4096, zlibDeflateOptions: { level: 6 } });
    expect(perMessageDeflateDe(false)).toBe(false);
    expect(compresionDeEntorno({})).toBeUndefined();
    expect(compresionDeEntorno({ WS_COMPRESION: 'off' })).toBe(false);
    expect(compresionDeEntorno({ WS_COMPRESION_UMBRAL: '2048', WS_COMPRESION_NIVEL: '1' })).toEqual({ umbralBytes: 2048, nivel: 1 });
    expect(compresionDeEntorno({ WS_COMPRESION_NIVEL: '99' })).toEqual({});
  });
});
