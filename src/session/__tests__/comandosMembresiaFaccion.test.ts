// Membresía de Facción (a petición del usuario, 2026-08-27): crear una Facción asigna ciudadanía automática
// a quien la crea; un jugador con Facción no puede crear otra hasta que pase el cooldown de
// `CIUDADANIA.cooldownCreacionFaccionDias` desde su ÚLTIMA salida, y solo si no pertenece ya a ninguna;
// `unirseAFaccion`/`dejarFaccion` cubren entrar en una Facción existente y abandonar la propia.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion, type PayloadFaccionCreada } from '../comandos/crearFaccion';
import { unirseAFaccion, type PayloadFaccionUnion } from '../comandos/unirseAFaccion';
import { dejarFaccion, type PayloadFaccionAbandonada } from '../comandos/dejarFaccion';
import { asignarRey } from '../comandos/cargos';
import { CIUDADANIA } from '../../constants';

const MOMENTO = '2026-01-01T00:00:00.000Z';
const OPC = { momento: MOMENTO, actor: 'jugador-a' };

function momentoMasDias(dias: number): string {
  return new Date(new Date(MOMENTO).getTime() + dias * 24 * 60 * 60 * 1000).toISOString();
}

describe('crearFaccion — ciudadanía automática', () => {
  it('quien crea la Facción queda como ciudadano de inmediato, sin fundar ni comprar casa', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);

    expect(r.ok).toBe(true);
    const faccion = sesion.getState().facciones.find((f) => f.id === r.datos!.faccionId)!;
    expect(faccion.ciudadanosIds).toEqual([OPC.actor]);
  });

  it('el evento lleva el fundador en el payload', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    expect((r.eventos[0]!.payload as PayloadFaccionCreada).fundadorId).toBe(OPC.actor);
  });
});

describe('crearFaccion — un jugador, una Facción', () => {
  it('rechaza crear una segunda Facción mientras siga en la primera', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    const segunda = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC);

    expect(segunda.ok).toBe(false);
    expect(segunda.codigoError).toBe('faccion.ya_pertenece');
    expect(sesion.getState().facciones).toHaveLength(1); // nada se creó
  });

  it('un actor distinto SÍ puede crear su propia Facción en paralelo', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    const otra = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' });

    expect(otra.ok).toBe(true);
    expect(sesion.getState().facciones).toHaveLength(2);
  });
});

describe('crearFaccion — cooldown tras abandonar', () => {
  it(`rechaza crear antes de que pasen los ${CIUDADANIA.cooldownCreacionFaccionDias} días desde la última salida`, () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    sesion.ejecutar(dejarFaccion, {}, OPC);

    const reintento = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, momento: momentoMasDias(1) });
    expect(reintento.ok).toBe(false);
    expect(reintento.codigoError).toBe('faccion.cooldown_creacion');
  });

  it('permite crear de nuevo justo al cumplirse el cooldown', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    sesion.ejecutar(dejarFaccion, {}, OPC);

    const reintento = sesion.ejecutar(
      crearFaccion,
      { nombre: 'Troya' },
      { ...OPC, momento: momentoMasDias(CIUDADANIA.cooldownCreacionFaccionDias) }
    );
    expect(reintento.ok).toBe(true);
  });

  it('un jugador que nunca abandonó ninguna Facción no arrastra cooldown', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    expect(r.ok).toBe(true);
  });
});

describe('unirseAFaccion', () => {
  it('otorga ciudadanía de una Facción existente sin necesidad de comprar casa', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;

    const r = sesion.ejecutar(unirseAFaccion, { faccionId }, { ...OPC, actor: 'jugador-b' });
    expect(r.ok).toBe(true);
    const faccion = sesion.getState().facciones.find((f) => f.id === faccionId)!;
    expect(faccion.ciudadanosIds).toContain('jugador-b');

    const e = r.eventos[0]!;
    expect(e.codigo).toBe('faccion.ciudadania_union');
    expect(e.payload as PayloadFaccionUnion).toEqual({ faccionId, jugadorId: 'jugador-b' });
  });

  it('rechaza unirse si ya es ciudadano de OTRA Facción', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    const otraId = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;

    const r = sesion.ejecutar(unirseAFaccion, { faccionId: otraId }, OPC); // jugador-a ya es de Micenas
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('faccion.ya_pertenece');
  });

  it('unirse a la Facción de la que ya se es ciudadano es idempotente: no versiona', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    const antes = sesion.getState().version;

    const r = sesion.ejecutar(unirseAFaccion, { faccionId }, OPC);
    expect(r.ok).toBe(true);
    expect(r.version).toBe(antes);
    expect(r.eventos).toHaveLength(0);
  });

  it('Facción inexistente: rechazo estándar `faccion.no_existe`', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(unirseAFaccion, { faccionId: 'no-existe' }, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('faccion.no_existe');
  });
});

describe('dejarFaccion', () => {
  it('quita la ciudadanía y libera al jugador para unirse a otra sin esperar', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;

    const r = sesion.ejecutar(dejarFaccion, {}, OPC);
    expect(r.ok).toBe(true);
    const faccion = sesion.getState().facciones.find((f) => f.id === faccionId)!;
    expect(faccion.ciudadanosIds).not.toContain(OPC.actor);

    const e = r.eventos[0]!;
    expect(e.codigo).toBe('faccion.abandonada');
    expect(e.payload as PayloadFaccionAbandonada).toEqual({ faccionId, jugadorId: OPC.actor });

    const otraId = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;
    const union = sesion.ejecutar(unirseAFaccion, { faccionId: otraId }, OPC); // sin cooldown: unirse no lo tiene
    expect(union.ok).toBe(true);
  });

  it('libera el trono si el que se va era Rey', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    sesion.ejecutar(asignarRey, { faccionId, jugadorId: OPC.actor }, OPC);
    expect(sesion.getState().facciones[0]!.reyId).toBe(OPC.actor);

    sesion.ejecutar(dejarFaccion, {}, OPC);
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)!.reyId).toBeNull();
  });

  it('rechaza si el actor no es ciudadano de ninguna Facción', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(dejarFaccion, {}, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('faccion.no_pertenece');
  });
});
