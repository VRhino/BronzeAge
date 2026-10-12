// Telemetría de carga (03 §14): ventanas, formato de línea, playtest y poda — el módulo por su cuenta.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import type { VentanaDeVistas } from '../difusion/difusorDeVistas';
import { medicion } from '../medicionDeCarga';
import { FORMATO_CARGA_VERSION, leerCarga, podarCarga, RegistroDeCarga, resumir, claveDeCarga } from '../telemetria';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;
beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-carga-'));
  almacen = crearAlmacenEnDisco(directorio);
});
const creados: RegistroDeCarga[] = [];
afterEach(async () => {
  creados.forEach((r) => r.detener());
  creados.length = 0;
  await rm(directorio, { recursive: true, force: true });
});

const ventanaVacia = (): VentanaDeVistas => ({ porCanal: {} });
const ventanaConMapa = (): VentanaDeVistas => ({
  porCanal: {
    'vista/mapa': {
      flushes: 2, bytesEnviados: 900, bytesFlush: [100, 300], bytesFoto: [5000], msProyeccion: [0.5, 1.5], msDiff: [1, 2], msSerializacion: [0.1, 0.2],
      fotos: { alta: 1, resync: 0, lento: 0, visibilidad: 0 }, cambios: { upsert: 1, patch: 4, remove: 0, set: 0 }, socketsLentos: 0, cierres4408: 0,
    },
  },
});

/** Fuentes de mentira: un runner vivo con ventanas que se consumen, hub con N conexiones y difusor con ventanas en cola. */
function montar(opciones: { vivo?: boolean; conexiones?: number; encendida?: boolean } = {}) {
  const ventanasVistas: VentanaDeVistas[] = [];
  const runner = {
    gameId: 'g1',
    metricas: () => ({ tick: 7, version: 9, colaPendiente: 0, relojDeMundoActivo: opciones.vivo ?? true }),
    tomarVentana: () => ({ ticksMs: [10, 20, 30], guardadoMs: [4], guardadoBytes: [1234] }),
  };
  const fuentes = {
    partidas: { abiertas: () => [runner] },
    hub: { conexionesAbiertas: () => opciones.conexiones ?? 0 },
    difusor: { metricas: () => ({ porCanal: { 'vista/mapa': { bytesFoto: { p50: 4000, max: 5000 } } } }), tomarVentana: () => ventanasVistas.shift() ?? ventanaVacia(), suscripcionesDe: () => ({ 'vista/mapa': { suscriptores: 3, claves: 2 } }) },
  };
  const reg = new RegistroDeCarga(almacen, fuentes as never, () => '2026-10-10T10:00:00.000Z');
  creados.push(reg);
  if (opciones.encendida ?? true) reg.activarPartida('g1'); // encender limpia las ventanas: se cargan después
  ventanasVistas.push(ventanaConMapa(), ventanaVacia());
  return reg;
}

describe('RegistroDeCarga', () => {
  it('anota una línea por muestra con formato, ventanas resumidas y suscripciones', async () => {
    const reg = montar();
    reg.muestrear();
    reg.muestrear(); // la segunda ventana del difusor está vacía: «desde la muestra anterior»
    const { muestras } = await reg.leer('g1');
    expect(muestras).toHaveLength(2);
    const [m1, m2] = muestras as [typeof muestras[0], typeof muestras[0]];
    expect(m1.formatoVersion).toBe(FORMATO_CARGA_VERSION);
    expect(m1).toMatchObject({ tipo: 'muestra', gameId: 'g1', tick: 7, version: 9, suscriptores: 3, claves: 2 });
    expect(m1.tickMs).toEqual({ n: 3, p50: 20, p95: 20, max: 30 });
    expect(m1.guardadoBytes).toBe(1234);
    expect(m1.canales['vista/mapa']).toMatchObject({ flushes: 2, bytesEnviados: 900, suscriptores: 3, claves: 2 });
    expect(m1.canales['vista/mapa']!.bytesFlush.max).toBe(300);
    expect(m1.canales['vista/mapa']!.bytesFotoVida).toBe(5000);
    expect(m2.canales['vista/mapa']!.flushes).toBe(0);
    expect(m1.costeMuestraMs).toBeGreaterThanOrEqual(0);
  });

  it('apagada por defecto: no muestrea ni activa los puntos calientes; se enciende y apaga en caliente y por proceso', async () => {
    const reg = montar({ encendida: false });
    expect(reg.activa).toBe(false);
    expect(medicion.activa).toBe(false);
    reg.muestrear();
    expect((await reg.leer('g1')).muestras).toHaveLength(0);

    reg.activarPartida('g1');
    expect(reg.activa).toBe(true);
    expect(medicion.activa).toBe(true);
    reg.muestrear();
    expect((await reg.leer('g1')).muestras).toHaveLength(1);

    reg.desactivarPartida('g1');
    expect(medicion.activa).toBe(false);

    reg.iniciar(60_000); // por env: todas las partidas
    expect(medicion.activa).toBe(true);
    reg.detener();
    expect(reg.activa).toBe(false);
    expect(medicion.activa).toBe(false);
  });

  it('no anota una partida ociosa (sin reloj de mundo ni conexiones) y sí con conexiones', async () => {
    const ocioso = montar({ vivo: false });
    ocioso.muestrear();
    expect((await ocioso.leer('g1')).muestras).toHaveLength(0);
    const conectado = montar({ vivo: false, conexiones: 1 });
    conectado.muestrear();
    expect((await conectado.leer('g1')).muestras).toHaveLength(1);
  });

  it('el playtestId se pone en cada muestra, filtra, y sobrevive a un reinicio', async () => {
    const reg = montar();
    reg.muestrear(); // sin prueba
    reg.fijarPlaytest('g1', 'prueba-A');
    reg.muestrear();
    expect((await reg.leer('g1', { playtestId: 'prueba-A' })).muestras).toHaveLength(1);
    expect((await reg.leer('g1')).muestras).toHaveLength(2);

    const reiniciado = montar(); // otro proceso, mismo almacén
    expect(await reiniciado.playtestDe('g1')).toBe('prueba-A');
    reiniciado.fijarPlaytest('g1', null);
    await reiniciado.drenar();
    expect(await montar().playtestDe('g1')).toBeNull();
  });

  it('una línea a medias no tumba la lectura', async () => {
    const reg = montar();
    reg.muestrear();
    await reg.drenar();
    await almacen.anexar(claveDeCarga('g1'), '{"formatoVersion":1,"tip');
    const { lineas, corruptas } = await leerCarga(almacen, 'g1');
    expect(lineas).toHaveLength(1);
    expect(corruptas).toBe(1);
  });
});

describe('podarCarga', () => {
  it('poda por edad y por tamaño, y conserva la última marca de playtest', async () => {
    const linea = (momento: string, extra: object = {}) => JSON.stringify({ formatoVersion: 1, tipo: 'muestra', gameId: 'g1', momento, ...extra }) + '\n';
    await almacen.escribir(
      claveDeCarga('g1'),
      JSON.stringify({ formatoVersion: 1, tipo: 'playtest', gameId: 'g1', momento: '2026-10-01T00:00:00.000Z', playtestId: 'vieja' }) + '\n' +
        linea('2026-10-01T00:00:01.000Z') + linea('2026-10-09T00:00:00.000Z') + linea('2026-10-09T00:00:10.000Z') + linea('2026-10-09T00:00:20.000Z')
    );
    const borradas = await podarCarga(almacen, 'g1', '2026-10-05T00:00:00.000Z', 2);
    const { lineas } = await leerCarga(almacen, 'g1');
    expect(lineas.map((l) => l.tipo)).toEqual(['playtest', 'muestra', 'muestra']);
    expect(borradas).toBe(2); // una por edad y una por tamaño
  });
});

describe('resumir', () => {
  it('serie vacía → ceros; si no, n/p50/p95/max', () => {
    expect(resumir([])).toEqual({ n: 0, p50: 0, p95: 0, max: 0 });
    expect(resumir([1, 2, 3, 4, 100])).toMatchObject({ n: 5, p50: 3, max: 100 });
  });
});
