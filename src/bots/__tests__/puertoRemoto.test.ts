// El proceso de bots contra un servidor de verdad (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): HTTP y WebSocket por un
// puerto de red, cuentas de bot con `CODIGO_REGISTRO_BOTS`, como jugaría BronzeAgeClient. Los ticks los da el admin.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../../server/api';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresDeProceso } from '../../server/identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../../server/identidad/repositorioEnMemoria';
import { ProcesoDeBots } from '../remoto/procesoDeBots';

const ADMINS = [{ proveedor: 'dev', sujetoId: 'jefa' }];
const CODIGO = 'codigo-de-bots';

let directorio: string;
let app: FastifyInstance;
let servidor: string;
let admin: { authorization: string };

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-bots-'));
  // Los proveedores del servidor de verdad: `dev` para la admin de las pruebas y `clave`, que es por donde entran los bots.
  const repositorio = crearRepositorioIdentidadEnMemoria();
  app = crearServidor({
    directorio,
    administradoresGlobales: ADMINS,
    codigoRegistroBots: CODIGO,
    identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
  });
  servidor = await app.listen({ port: 0, host: '127.0.0.1' });
  const login = await fetch(`${servidor}/v1/sesiones`, { method: 'POST', headers: { authorization: 'dev jefa' } });
  admin = { authorization: `sesion ${((await login.json()) as { sesionId: string }).sesionId}` };
  const partida = await fetch(`${servidor}/v1/admin/partidas`, {
    method: 'POST',
    headers: { ...admin, 'content-type': 'application/json' },
    body: JSON.stringify({ gameId: 'g1', seed: 7 }),
  });
  expect(partida.status).toBe(201);
});

afterEach(async () => {
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

const proceso = () =>
  new ProcesoDeBots({
    servidor,
    gameId: 'g1',
    codigoRegistroBots: CODIGO,
    registro: join(directorio, 'bots-g1.json'),
    total: 3,
    // Todos en el primer minuto.
    diasLlegada: 1 / 1440,
    semilla: 7,
    cadaMs: 10,
    horario: 'siempre',
  });

async function ticks(p: ProcesoDeBots, n: number): Promise<void> {
  for (let i = 0; i < n; i++) {
    expect((await fetch(`${servidor}/v1/admin/partidas/g1/tick`, { method: 'POST', headers: admin })).status).toBe(200);
    await p.vuelta();
  }
}

async function estado() {
  const r = await fetch(`${servidor}/v1/admin/partidas/g1`, { headers: admin });
  return (await r.json()) as { heroes: { id: string; controlador: string; desconectaEn?: number }[]; facciones: { ciudadanosIds: string[] }[] };
}

describe('proceso de bots por HTTP y tiempo real', () => {
  it('llegan con cuentas de bot, crean su héroe en un campamento, se buscan Facción y piden su tropa', async () => {
    const p = proceso();
    await p.arrancar();
    await ticks(p, 15);

    const e = await estado();
    const bots = e.heroes.filter((h) => h.controlador === 'bot');
    expect(bots).toHaveLength(3);
    expect(e.facciones.flatMap((f) => f.ciudadanosIds).sort()).toEqual(bots.map((b) => b.id).sort());

    // Apagar el proceso cierra sus conexiones de tiempo real: el servidor los da por desconectados, como a quien cierra el cliente.
    await p.parar();
    await expect.poll(async () => (await estado()).heroes.filter((h) => h.desconectaEn !== undefined).length).toBe(3);
  });

  it('tras un reinicio vuelve a entrar con las mismas cuentas: no crea bots nuevos', async () => {
    const primero = proceso();
    await primero.arrancar();
    await ticks(primero, 3);
    await primero.parar();

    const segundo = proceso();
    await segundo.arrancar();
    await ticks(segundo, 5);

    expect((await estado()).heroes.filter((h) => h.controlador === 'bot')).toHaveLength(3);
    await segundo.parar();
  });
});
