// `idDeMapa`/`vistaAdminDeEstado` (Fase C11, doc 9: "el mapa deja de ser estado, es un asset"). Sobre estado
// GENUINO (misma fixture que el resto de `session/`), no un `MapaGenerado` fabricado a mano — así estas
// pruebas fallan de verdad si cambia la forma real de `GameSessionState.mapa`.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { anteponerEventos, idDeMapa, eventosDesde, MAX_EVENTOS_EN_MEMORIA, vistaAdminDeEstado, type EventoDominioConVersion } from '../estado';


describe('idDeMapa', () => {
  it('es estable: la misma partida produce el mismo id en llamadas repetidas', () => {
    const sesion = GameSession.crear('g1', { seed: 42 });
    const estado = sesion.getState();

    expect(idDeMapa(estado.mapa)).toBe(idDeMapa(estado.mapa));
  });

  it('no cambia al avanzar el tick: el mapa es inmutable durante toda la partida', () => {
    const sesion = GameSession.crear('g1', { seed: 42 });
    const idAntes = idDeMapa(sesion.getState().mapa);

    sesion.avanzarTick();

    expect(idDeMapa(sesion.getState().mapa)).toBe(idAntes);
  });

  it('dos seeds distintas producen ids distintos', () => {
    const a = GameSession.crear('a', { seed: 1 });
    const b = GameSession.crear('b', { seed: 2 });

    expect(idDeMapa(a.getState().mapa)).not.toBe(idDeMapa(b.getState().mapa));
  });

  it('incluye la seed en texto legible, no es un hash opaco', () => {
    const sesion = GameSession.crear('g1', { seed: 12345 });
    expect(idDeMapa(sesion.getState().mapa)).toContain('12345');
  });
});

describe('vistaAdminDeEstado', () => {
  it('quita `mapa` y añade `mapaId`, sin tocar el resto del estado', () => {
    const sesion = GameSession.crear('g1', { seed: 7 });
    const estado = sesion.getState();

    const vista = vistaAdminDeEstado(estado);

    expect(vista).not.toHaveProperty('mapa');
    expect(vista.mapaId).toBe(idDeMapa(estado.mapa));
    expect(vista.tick).toBe(estado.tick);
    expect(vista.asentamientos).toBe(estado.asentamientos);
  });
});

describe('eventosDesde (Fase C13: cursor incremental)', () => {
  it('cada evento lleva la version de la partida en la que se emitió', () => {
    const sesion = GameSession.crear('g1', { seed: 1 });
    const resultado = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: 'jugador-1' });

    expect(resultado.ok).toBe(true);
    expect(resultado.eventos).toHaveLength(1);
    expect(resultado.eventos[0]!.version).toBe(1);
  });

  it('desde=0 trae todo el historial, en orden cronológico (más viejo primero)', () => {
    const sesion = GameSession.crear('g1', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: 'a' });
    sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { actor: 'b' });

    const eventos = eventosDesde(sesion.getState(), 0);

    expect(eventos.map((e) => e.version)).toEqual([1, 2]);
  });

  it('desde=<version actual> no trae nada nuevo', () => {
    const sesion = GameSession.crear('g1', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: 'a' });

    expect(eventosDesde(sesion.getState(), sesion.getState().version)).toEqual([]);
  });

  it('desde=<version intermedia> trae solo lo posterior', () => {
    const sesion = GameSession.crear('g1', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: 'a' }); // version 1
    sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { actor: 'b' }); // version 2
    sesion.ejecutar(crearFaccion, { nombre: 'Esparta' }, { actor: 'c' }); // version 3

    const eventos = eventosDesde(sesion.getState(), 1);

    expect(eventos.map((e) => e.version)).toEqual([2, 3]);
  });
});

describe('anteponerEventos', () => {
  const ev = (version: number, n = 0): EventoDominioConVersion => ({ codigo: 'x', mensaje: `${version}-${n}`, momento: '', version });

  it('por debajo del tope antepone sin perder nada', () => {
    const r = anteponerEventos([ev(3), ev(3, 1)], [ev(2), ev(1)]);
    expect(r.map((e) => e.mensaje)).toEqual(['3-0', '3-1', '2-0', '1-0']);
  });

  it('en el tope recorta los más viejos y NUNCA parte una versión', () => {
    // Una versión por cada 2 eventos: el tope (par) cae entre versiones; con 3 por versión, hay que retroceder.
    const viejos = Array.from({ length: MAX_EVENTOS_EN_MEMORIA }, (_, i) => ev(Math.floor((MAX_EVENTOS_EN_MEMORIA - 1 - i) / 3) + 1, i));
    const nuevos = [ev(9999, 0), ev(9999, 1)];
    const r = anteponerEventos(nuevos, viejos);
    expect(r.length).toBeLessThanOrEqual(MAX_EVENTOS_EN_MEMORIA);
    expect(r.slice(0, 2)).toEqual(nuevos);
    const masVieja = r.at(-1)!.version;
    const quedanDeEsaVersion = r.filter((e) => e.version === masVieja).length;
    const teniaDeEsaVersion = [...nuevos, ...viejos].filter((e) => e.version === masVieja).length;
    expect(quedanDeEsaVersion, 'la versión más vieja que se conserva está completa').toBe(teniaDeEsaVersion);
  });
});
