// Diario de comandos (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §5.1): una partida reconstruida desde el último
// guardado + el diario tiene que ser IDÉNTICA, byte a byte, a la que estaba en memoria al caer.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { RunnerDePartida } from '../runnerDePartida';
import { cargarPartida } from '../persistenciaPartida';
import { claveDeDiario, DiarioIrrepetibleError } from '../diarioDePartida';
import { GameSession } from '../../session/gameSession';
import { rechazo, type ManejadorComando } from '../../session/comandos/tipos';
import { CODIGOS_ERROR } from '../../session/comandos/codigosDeError';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const GAME = 'g-diario';

let directorio: string;
let almacen: ReturnType<typeof crearAlmacenEnDisco>;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-diario-'));
  almacen = crearAlmacenEnDisco(directorio);
});

afterEach(async () => {
  await rm(directorio, { recursive: true, force: true });
});

const abrir = () => RunnerDePartida.cargarOCrear(GAME, { seed: 11 }, { almacen, ahora: () => MOMENTO });
const bytes = (r: RunnerDePartida) => JSON.stringify(r.exportar());

async function heroe(r: RunnerDePartida, jugadorId: string): Promise<string> {
  const avatar = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };
  const creado = await r.ejecutar('crearHeroe', { displayName: jugadorId, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar }, jugadorId);
  // Sale del campamento: el guion también mueve columnas.
  await r.ejecutar('salirDelCampamento', { campamentoId: 'mercenarios-0', heroeId: creado.datos!.heroeId, escuadronIds: [], carga: {} }, creado.datos!.heroeId);
  return creado.datos!.heroeId;
}

/** Guion intercalado de varios actores, ticks (con el turno del NPC, que tira dados) y rechazos. Deja el
 * diario con comandos posteriores al último tick, que es lo que hay que recuperar. */
async function guion(r: RunnerDePartida): Promise<void> {
  await r.ejecutar('crearFaccionNpc', { nombre: 'Hatti' });
  const ana = await heroe(r, 'ana');
  const bea = await heroe(r, 'bea');
  const micenas = await r.ejecutar('crearFaccion', { nombre: 'Micenas' }, ana);
  await r.ejecutar('solicitarIngreso', { faccionId: micenas.datos!.faccionId }, bea);
  await r.ejecutar('pedirPrestamo', { tropaIds: ['milicia_lanceros'] }, ana);
  await r.ejecutar('marcharA', { heroeId: ana, objetivo: { tipo: 'punto', punto: { x: 900, y: 900 } } }, ana);
  for (let i = 0; i < 10; i++) await r.avanzarTick();
  expect((await r.ejecutar('solicitarIngreso', { faccionId: 'no-existe' }, 'cai')).ok).toBe(false);
  await r.ejecutar('responderSolicitud', { faccionId: micenas.datos!.faccionId, heroeId: bea, aceptar: true }, ana);
  for (let i = 0; i < 5; i++) await r.avanzarTick();
  await heroe(r, 'cai');
  await r.ejecutar('crearFaccion', { nombre: 'Ugarit' }, 'cai'); // sin héroe propio como actor: lo que diga el dominio
}

describe('diario de comandos — reconstrucción', () => {
  it('guardado + diario reconstruyen la partida byte a byte, y sigue igual después', async () => {
    const original = await abrir();
    await guion(original);
    const diario = await almacen.leer(claveDeDiario(GAME));
    expect(diario!.trim().split('\n').length).toBeGreaterThan(1); // hay algo que repasar

    // «Caída»: el original se abandona sin apagado limpio.
    const recuperada = await abrir();
    expect(bytes(recuperada)).toBe(bytes(original));

    for (const r of [original, recuperada]) {
      await r.avanzarTick();
      await heroe(r, 'dan');
    }
    expect(bytes(recuperada)).toBe(bytes(original));
  });

  it('el historial de eventos no se duplica al repasar lo que ya se había anexado', async () => {
    const original = await abrir();
    await guion(original);
    const recuperada = await abrir();
    await recuperada.avanzarTick();
    const versiones = (await recuperada.eventosDesde(0)).map((e) => e.version);
    expect(versiones).toEqual([...versiones].sort((a, b) => a - b));
    const lineas = (await almacen.leer(`${GAME}.eventos.jsonl`))!.trim().split('\n');
    expect(new Set(lineas).size).toBe(lineas.length);
  });

  it('una caída entre «guardar» y «vaciar» deja líneas ya guardadas: se saltan', async () => {
    const original = await abrir();
    await guion(original);
    const antes = (await almacen.leer(claveDeDiario(GAME)))!;
    await original.avanzarTick(); // guarda y vacía
    await almacen.anexar(claveDeDiario(GAME), antes); // como si el vaciado no hubiera llegado

    expect(bytes(await abrir())).toBe(bytes(original));
  });

  it('una última línea a medias es un corte de escritura y se descarta', async () => {
    const original = await abrir();
    await guion(original);
    await almacen.anexar(claveDeDiario(GAME), '{"v":99999,"t":"crearF');

    expect(bytes(await abrir())).toBe(bytes(original));
  });

  it('una línea que no reproduce su versión, o una ilegible en medio, impide abrir la partida', async () => {
    const original = await abrir();
    await guion(original);
    const bueno = (await almacen.leer(claveDeDiario(GAME)))!;
    const v = original.getState().version;

    await almacen.escribir(claveDeDiario(GAME), `${bueno}${JSON.stringify({ v: v + 5, t: 'avanzarTick', a: 'sistema' })}\n`);
    await expect(cargarPartida(almacen, GAME)).rejects.toThrow(DiarioIrrepetibleError);

    await almacen.escribir(claveDeDiario(GAME), `{roto\n${bueno}`);
    await expect(cargarPartida(almacen, GAME)).rejects.toThrow(DiarioIrrepetibleError);
  });

  it('crear una partida nueva vacía el diario de la anterior con el mismo gameId', async () => {
    const vieja = await abrir();
    await guion(vieja);
    const nueva = await RunnerDePartida.crearYPersistir(GAME, { seed: 11 }, { almacen, ahora: () => MOMENTO }, { forzar: true });
    expect(bytes(await abrir())).toBe(bytes(nueva));
  });
});

describe('GameSession.ejecutar — lo que no sube la versión no deja rastro', () => {
  it('un rechazo que tiró dados y pidió ids no mueve el RNG ni el generador de ids', () => {
    const tramposo: ManejadorComando<void, void> = (estado, _mapa, ctx) => {
      ctx.rng();
      ctx.ids.siguiente();
      return rechazo(estado, CODIGOS_ERROR.batallaNoExiste);
    };
    const sesion = GameSession.crear('g', { seed: 3 });
    const antes = JSON.stringify(sesion.exportar());
    expect(sesion.ejecutar(tramposo, undefined).ok).toBe(false);
    expect(JSON.stringify(sesion.exportar())).toBe(antes);
  });
});
