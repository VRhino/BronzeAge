// El servicio de bots administrable (src/bots/control/): el canal de control por WebSocket contra un servidor del juego de
// verdad. Lo que hace el panel «Bots» del cliente admin, sin el panel.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtemp, rm } from 'node:fs/promises';
import { existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import WebSocket from 'ws';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../../server/api';
import { crearRegistroProveedores } from '../../acceso/proveedorIdentidad';
import { proveedoresDeProceso } from '../../server/identidad/proveedoresActivos';
import { crearRepositorioIdentidadEnMemoria } from '../../server/identidad/repositorioEnMemoria';
import { ProcesoDeBots } from '../remoto/procesoDeBots';
import { ServicioDeBots } from '../control/servicio';
import { servidorDeControl, type ServidorDeControl } from '../control/servidorDeControl';
import type { ConfigBots, EstadoServicio, MensajeDeServicio } from '../control/contrato';

const CODIGO = 'codigo-de-bots';
const TOKEN = 'token-de-control';

let directorio: string;
let app: FastifyInstance;
let servidor: string;
let admin: { authorization: string };
let servicio: ServicioDeBots;
let control: ServidorDeControl;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-control-'));
  const repositorio = crearRepositorioIdentidadEnMemoria();
  app = crearServidor({
    directorio,
    administradoresGlobales: [{ proveedor: 'dev', sujetoId: 'jefa' }],
    codigoRegistroBots: CODIGO,
    identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(repositorio)), repositorio },
  });
  servidor = await app.listen({ port: 0, host: '127.0.0.1' });
  const login = await fetch(`${servidor}/v1/sesiones`, { method: 'POST', headers: { authorization: 'dev jefa' } });
  admin = { authorization: `sesion ${((await login.json()) as { sesionId: string }).sesionId}` };
  const partida = await fetch(`${servidor}/v1/admin/partidas`, { method: 'POST', headers: { ...admin, 'content-type': 'application/json' }, body: JSON.stringify({ gameId: 'g1', seed: 7 }) });
  expect(partida.status).toBe(201);

  servicio = new ServicioDeBots({
    directorioRegistros: join(directorio, 'registros'),
    rutaEstado: join(directorio, 'registros', 'servicio.json'),
    codigoRegistroBots: CODIGO,
    porDefecto: { servidor },
  });
  control = await servidorDeControl(servicio, { token: TOKEN, puerto: 0, host: '127.0.0.1' });
});

afterEach(async () => {
  await servicio.cerrar();
  await control.cerrar();
  await app.close();
  await rm(directorio, { recursive: true, force: true });
});

/** Un cliente del canal de control, como la pestaña «Bots». */
function conectar() {
  const ws = new WebSocket(`ws://127.0.0.1:${control.puerto}/control`);
  const mensajes: MensajeDeServicio[] = [];
  const pendientes = new Map<number, (m: Extract<MensajeDeServicio, { tipo: 'respuesta' }>) => void>();
  let siguiente = 1;
  ws.on('message', (d) => {
    const m = JSON.parse(String(d)) as MensajeDeServicio;
    mensajes.push(m);
    if (m.tipo === 'respuesta') pendientes.get(m.id)?.(m);
  });
  const abierto = new Promise<void>((r) => ws.once('open', () => r()));
  return {
    ws,
    mensajes,
    abierto,
    pedir: (orden: Record<string, unknown>) =>
      new Promise<Extract<MensajeDeServicio, { tipo: 'respuesta' }>>((resolver) => {
        const id = siguiente++;
        pendientes.set(id, resolver);
        ws.send(JSON.stringify({ id, ...orden }));
      }),
    estado: () => [...mensajes].reverse().find((m): m is Extract<MensajeDeServicio, { tipo: 'estado' }> => m.tipo === 'estado')?.estado,
  };
}

async function autenticado() {
  const c = conectar();
  await c.abierto;
  expect((await c.pedir({ accion: 'autenticar', token: TOKEN })).ok).toBe(true);
  return c;
}

const CONFIG: ConfigBots = { servidor: '', partida: 'g1', total: 3, diasLlegada: 1 / 1440, semilla: 7, cadaMs: 20, horario: 'siempre' };

const tick = async () => expect((await fetch(`${servidor}/v1/admin/partidas/g1/tick`, { method: 'POST', headers: admin })).status).toBe(200);

async function esperar<T>(leer: () => T | undefined | false, ms = 8000): Promise<T> {
  const limite = Date.now() + ms;
  for (;;) {
    const v = leer();
    if (v) return v;
    if (Date.now() > limite) throw new Error('no ocurrió a tiempo');
    await new Promise((r) => setTimeout(r, 25));
  }
}

/** Da ticks al mundo hasta que el estado cumpla. */
async function hasta(c: ReturnType<typeof conectar>, cumple: (e: EstadoServicio) => boolean): Promise<EstadoServicio> {
  return esperar(
    () => {
      const e = c.estado();
      return e && cumple(e) ? e : undefined;
    },
    20000
  ).catch(async () => {
    throw new Error(`el estado no llegó: ${JSON.stringify(c.estado()?.llegadas)} ${JSON.stringify(c.estado()?.salud)}`);
  });
}

describe('canal de control', () => {
  it('no cuenta nada sin autenticar y cierra con un token incorrecto', async () => {
    const sin = conectar();
    await sin.abierto;
    expect((await sin.pedir({ accion: 'pausar' })).ok).toBe(false);
    expect(sin.mensajes.some((m) => m.tipo === 'estado')).toBe(false);
    sin.ws.close();

    const mal = conectar();
    await mal.abierto;
    const cerrado = new Promise<number>((r) => mal.ws.once('close', (codigo) => r(codigo)));
    expect((await mal.pedir({ accion: 'autenticar', token: 'otro' })).ok).toBe(false);
    expect(await cerrado).toBe(1008);
  });

  it('autenticado, recibe el estado inerte con lo que propone el entorno', async () => {
    const c = await autenticado();
    const e = await esperar(() => c.estado());
    expect(e.fase).toBe('inactivo');
    expect(e.codigoEnEntorno).toBe(true);
    expect(e.porDefecto.servidor).toBe(servidor);
    expect(e.bots).toEqual([]);
  });

  it('inicia, llegan los bots, se pausa y se reanuda, se fuerza una llegada, se maneja un bot y se para', async () => {
    const c = await autenticado();
    const r = await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor } });
    expect(r.error).toBeUndefined();
    expect(c.estado()?.fase).toBe('corriendo');

    for (let i = 0; i < 6; i++) await tick();
    const e = await hasta(c, (s) => s.bots.length === 3);
    expect(e.llegadas.hechas).toBeGreaterThan(0);
    expect(e.llegadas.plan.every((l) => l.hecha)).toBe(true);
    expect(e.bots.every((b) => b.conectado === true)).toBe(true);
    expect(e.salud.peticionesMin).toBeGreaterThan(0);
    expect(e.salud.socketsAbiertos).toBe(3);
    // La actividad llega en vivo, con el resultado de cada comando.
    await esperar(() => c.mensajes.some((m) => m.tipo === 'accion'));

    expect((await c.pedir({ accion: 'pausar' })).ok).toBe(true);
    expect(c.estado()?.fase).toBe('pausado');
    expect((await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor } })).ok).toBe(false);
    expect((await c.pedir({ accion: 'reanudar' })).ok).toBe(true);

    expect((await c.pedir({ accion: 'forzarLlegada', perfil: 'solitario' })).datos).toEqual({ creados: 1 });
    const cuatro = await hasta(c, (s) => s.bots.length === 4);
    expect(cuatro.llegadas.extras).toBe(1);
    expect(cuatro.bots[3]?.nombre).toBe('Bot 4');

    const id = cuatro.bots[0]!.heroeId;
    expect((await c.pedir({ accion: 'modoBot', heroeId: id, modo: 'congelado' })).ok).toBe(true);
    expect(c.estado()?.bots.find((b) => b.heroeId === id)?.modo).toBe('congelado');
    const memoria = await c.pedir({ accion: 'volcarMemoria', heroeId: id });
    expect(memoria.ok).toBe(true);
    expect(memoria.datos).toHaveProperty('memoria');
    // Las pizarras llevan el detalle además de los recuentos.
    for (const p of c.estado()!.pizarras) {
      expect(p.detalle.bandidosVistos).toHaveLength(p.bandidos);
      expect(p.detalle.salidasAbiertas).toHaveLength(p.salidas);
      expect(p.detalle.exploradosEn).toHaveLength(p.explorados);
    }

    expect((await c.pedir({ accion: 'retirarBot', heroeId: id })).ok).toBe(true);
    expect(c.estado()?.bots.find((b) => b.heroeId === id)).toMatchObject({ retirado: true, conectado: false });
    expect((await c.pedir({ accion: 'pensarYa', heroeId: id })).ok).toBe(false);

    expect((await c.pedir({ accion: 'parar' })).ok).toBe(true);
    expect(c.estado()).toMatchObject({ fase: 'inactivo', bots: [] });
    expect(c.estado()?.config?.partida).toBe('g1');
  });

  it('rechaza una configuración inválida sin arrancar, y el registro se puede borrar parado', async () => {
    const c = await autenticado();
    expect((await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor, total: 0 } })).error).toMatch(/total/);
    expect((await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor: 'http://otro.example' } })).error).toMatch(/solo da su código/);
    expect(c.estado()?.fase).toBe('inactivo');

    await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor } });
    const registro = join(directorio, 'registros', 'bots-g1.json');
    expect(existsSync(registro)).toBe(true);
    expect((await c.pedir({ accion: 'reiniciarRegistro' })).ok).toBe(false);
    // El de otra partida sí, aunque esté jugando; y un id con ruta, no.
    expect((await c.pedir({ accion: 'reiniciarRegistro', partida: 'otra' })).ok).toBe(true);
    expect((await c.pedir({ accion: 'reiniciarRegistro', partida: '../g1' })).ok).toBe(false);
    await c.pedir({ accion: 'parar' });
    expect((await c.pedir({ accion: 'reiniciarRegistro' })).ok).toBe(true);
    expect(existsSync(registro)).toBe(false);
  });

  it('si el proceso se cae en marcha, al volver retoma solo', async () => {
    const c = await autenticado();
    await c.pedir({ accion: 'iniciar', config: { ...CONFIG, servidor } });
    await servicio.cerrar();

    const otro = new ServicioDeBots({
      directorioRegistros: join(directorio, 'registros'),
      rutaEstado: join(directorio, 'registros', 'servicio.json'),
      codigoRegistroBots: CODIGO,
    });
    expect(await otro.reanudarSiToca()).toBe(true);
    expect(otro.estado()).toMatchObject({ fase: 'corriendo', config: { partida: 'g1' } });
    await otro.parar();
    // Parado a propósito, no retoma.
    expect(await otro.reanudarSiToca()).toBe(false);
  });
});

describe('registro de otro mundo', () => {
  const configuracion = () => ({ servidor, gameId: 'g1', codigoRegistroBots: CODIGO, registro: join(directorio, 'bots-g1.json'), total: 3, diasLlegada: 1 / 1440, semilla: 7, cadaMs: 10, horario: 'siempre' as const });

  it('si la partida se regeneró con el mismo ID, no arranca con un registro cuyos héroes ya no existen', async () => {
    const primero = new ProcesoDeBots(configuracion());
    await primero.arrancar();
    expect(await primero.forzarLlegada('solitario')).toBe(1);
    await primero.parar();

    const regenerada = await fetch(`${servidor}/v1/admin/partidas`, { method: 'POST', headers: { ...admin, 'content-type': 'application/json' }, body: JSON.stringify({ gameId: 'g1', seed: 8, forzar: true }) });
    expect(regenerada.status).toBe(201);

    await expect(new ProcesoDeBots(configuracion()).arrancar()).rejects.toThrow(/no es de este mundo.*heroe-0.*ya no existe/);
  });

  it('el servicio lo dice, se queda en error y deja borrar el registro para empezar de cero', async () => {
    const config: ConfigBots = { ...CONFIG, servidor, total: 1 };
    await servicio.iniciar(config);
    await servicio.manejar({ accion: 'forzarLlegada', perfil: 'solitario' });
    await servicio.parar();
    await fetch(`${servidor}/v1/admin/partidas`, { method: 'POST', headers: { ...admin, 'content-type': 'application/json' }, body: JSON.stringify({ gameId: 'g1', seed: 8, forzar: true }) });

    await expect(servicio.iniciar(config)).rejects.toThrow(/no es de este mundo/);
    expect(servicio.estado()).toMatchObject({ fase: 'error', error: expect.stringContaining('no es de este mundo') });

    await servicio.manejar({ accion: 'reiniciarRegistro' });
    await servicio.iniciar(config);
    expect(servicio.estado().fase).toBe('corriendo');
  });

  it('el mismo mundo de siempre sigue arrancando con su registro', async () => {
    const primero = new ProcesoDeBots(configuracion());
    await primero.arrancar();
    await primero.forzarLlegada('solitario');
    await primero.parar();

    const segundo = new ProcesoDeBots(configuracion());
    await segundo.arrancar();
    expect(segundo.runner.info()).toHaveLength(1);
    await segundo.parar();
  });
});

describe('alta de una llegada que falla a medias', () => {
  it('los bots que ya nacieron quedan en el registro y el error no los pierde', async () => {
    const registro = join(directorio, 'bots-g1.json');
    const proceso = new ProcesoDeBots({ servidor, gameId: 'g1', codigoRegistroBots: CODIGO, registro, total: 3, diasLlegada: 1 / 1440, semilla: 7, cadaMs: 10, horario: 'siempre' });
    await proceso.arrancar();
    // Unos amigos llegan juntos: el tercero falla.
    const llegar = proceso.puerto.llegar.bind(proceso.puerto);
    let llamadas = 0;
    proceso.puerto.llegar = async (...args) => {
      if (++llamadas === 3) throw new Error('el servidor se cayó');
      return llegar(...args);
    };
    expect(await proceso.forzarLlegada('amigos')).toBe(2);

    const guardado = JSON.parse(readFileSync(registro, 'utf-8')) as { bots: { nombre: string }[]; llegadasExtra: number };
    expect(guardado.bots.map((b) => b.nombre)).toEqual(['Bot 1', 'Bot 2']);
    expect(guardado.llegadasExtra).toBe(1);
    await proceso.parar();
  });
});
