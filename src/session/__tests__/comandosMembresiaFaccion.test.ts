// Membresía de Facción (a petición del usuario, 2026-08-27): crear una Facción asigna ciudadanía automática
// a quien la crea; un jugador con Facción no puede crear otra hasta que pase el cooldown de
// `CIUDADANIA.cooldownCreacionFaccionDias` desde su ÚLTIMA salida, y solo si no pertenece ya a ninguna;
// `solicitarIngreso` + `responderSolicitud` (el Rey decide, D46) y `dejarFaccion` cubren entrar en una Facción existente y abandonar la propia.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearFaccion, type PayloadFaccionCreada } from '../comandos/crearFaccion';
import { responderSolicitud, solicitarIngreso, type PayloadSolicitudRespondida } from '../comandos/ingresoEnFaccion';
import { dejarFaccion, type PayloadFaccionAbandonada } from '../comandos/dejarFaccion';
import { CIUDADANIA, SIMULACION } from '../../constants';

const OPC = { actor: 'jugador-a' };

const TICKS_POR_DIA = (24 * 60 * 60 * 1000) / SIMULACION.duracionTickMs;

/**
 * Adelanta la partida `dias` de MUNDO (Fase D: 1 tick = 1 minuto real, así que el tiempo de mundo es función
 * del tick). Se hace por `exportar`/`importar` en vez de ejecutar `dias × 1440` ticks reales — el cooldown
 * solo mira `instanteDeTick(estado.tick)`, no lo que pasó entre medias.
 */
function adelantarDias(sesion: GameSession, dias: number): GameSession {
  const exportada = sesion.exportar();
  return GameSession.importar({ ...exportada, state: { ...exportada.state, tick: dias * TICKS_POR_DIA } });
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

  it('quien crea la Facción queda además como su Rey (2026-09-10: siempre hay Rey)', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    expect(sesion.getState().facciones.find((f) => f.id === r.datos!.faccionId)!.reyId).toBe(OPC.actor);
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
  it(`rechaza crear antes de que pasen los ${CIUDADANIA.cooldownCreacionFaccionDias} días de MUNDO desde la última salida`, () => {
    let sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    sesion.ejecutar(dejarFaccion, {}, OPC);

    sesion = adelantarDias(sesion, 1);
    const reintento = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC);
    expect(reintento.ok).toBe(false);
    expect(reintento.codigoError).toBe('faccion.cooldown_creacion');
  });

  it('permite crear de nuevo justo al cumplirse el cooldown', () => {
    let sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    sesion.ejecutar(dejarFaccion, {}, OPC);

    sesion = adelantarDias(sesion, CIUDADANIA.cooldownCreacionFaccionDias);
    const reintento = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC);
    expect(reintento.ok).toBe(true);
  });

  it('un jugador que nunca abandonó ninguna Facción no arrastra cooldown', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    expect(r.ok).toBe(true);
  });
});

/** `heroeId` pide entrar y el Rey (`jugador-a`, que la creó) responde. */
function entra(sesion: GameSession, faccionId: string, heroeId: string, aceptar = true) {
  sesion.ejecutar(solicitarIngreso, { faccionId }, { actor: heroeId });
  return sesion.ejecutar(responderSolicitud, { faccionId, heroeId, aceptar }, OPC);
}

describe('ingreso por solicitud (D46)', () => {
  it('pedir entra en la lista; el Rey acepta y es ciudadano', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;

    expect(sesion.ejecutar(solicitarIngreso, { faccionId }, { actor: 'jugador-b' }).ok).toBe(true);
    expect(sesion.getState().facciones[0]!.solicitudesIds).toEqual(['jugador-b']);
    expect(sesion.getState().facciones[0]!.ciudadanosIds, 'pedir no da ciudadanía').not.toContain('jugador-b');

    const r = sesion.ejecutar(responderSolicitud, { faccionId, heroeId: 'jugador-b', aceptar: true }, OPC);
    expect(r.ok).toBe(true);
    const faccion = sesion.getState().facciones[0]!;
    expect(faccion.ciudadanosIds).toContain('jugador-b');
    expect(faccion.solicitudesIds).toEqual([]);
    expect(r.eventos[0]!.codigo).toBe('faccion.ciudadania_union');
    expect(r.eventos[0]!.payload as PayloadSolicitudRespondida).toEqual({ faccionId, heroeId: 'jugador-b', aceptada: true });
  });

  it('denegada, sale de la lista y no entra', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    expect(entra(sesion, faccionId, 'jugador-b', false).ok).toBe(true);
    expect(sesion.getState().facciones[0]).toMatchObject({ solicitudesIds: [] });
    expect(sesion.getState().facciones[0]!.ciudadanosIds).not.toContain('jugador-b');
  });

  it('aceptado en una, sus solicitudes en otras caen', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const micenas = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    const troya = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { actor: 'jugador-c' }).datos!.faccionId;
    sesion.ejecutar(solicitarIngreso, { faccionId: troya }, { actor: 'jugador-b' });
    entra(sesion, micenas, 'jugador-b');
    expect(sesion.getState().facciones.find((f) => f.id === troya)!.solicitudesIds).toEqual([]);
  });

  it('rechaza pedir siendo ya ciudadano, y una Facción que no existe', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    const otraId = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;
    expect(sesion.ejecutar(solicitarIngreso, { faccionId: otraId }, OPC).codigoError).toBe('faccion.ya_pertenece');
    expect(sesion.ejecutar(solicitarIngreso, { faccionId: 'no-existe' }, { actor: 'jugador-z' }).codigoError).toBe('faccion.no_existe');
  });

  it('no se responde a quien no ha pedido', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    expect(sesion.ejecutar(responderSolicitud, { faccionId, heroeId: 'jugador-b', aceptar: true }, OPC).ok).toBe(false);
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
    expect(e.payload as PayloadFaccionAbandonada).toEqual({ faccionId, heroeId: OPC.actor });

    const otraId = sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { ...OPC, actor: 'jugador-b' }).datos!.faccionId;
    expect(entra(sesion, otraId, OPC.actor).ok, 'sin cooldown: entrar no lo tiene').toBe(true);
  });

  it('el trono queda vacío solo si se va el último ciudadano', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    expect(sesion.getState().facciones[0]!.reyId).toBe(OPC.actor); // Rey desde la creación

    sesion.ejecutar(dejarFaccion, {}, OPC);
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)!.reyId).toBeNull();
  });

  it('sucesión: si se va el Rey y quedan ciudadanos, el trono pasa al siguiente', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const faccionId = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC).datos!.faccionId;
    entra(sesion, faccionId, 'jugador-b');

    sesion.ejecutar(dejarFaccion, {}, OPC); // se va el Rey (el creador)
    expect(sesion.getState().facciones.find((f) => f.id === faccionId)!.reyId).toBe('jugador-b');
  });

  it('rechaza si el actor no es ciudadano de ninguna Facción', () => {
    const sesion = GameSession.crear('t', { seed: 1 });
    const r = sesion.ejecutar(dejarFaccion, {}, OPC);
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('faccion.no_pertenece');
  });
});
