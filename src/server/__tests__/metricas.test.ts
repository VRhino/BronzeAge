// Métricas de operación (Fase E3). Lo que se vigila aquí no es que los números existan, sino que **midan lo
// que dicen medir**: que la cola refleje trabajo realmente pendiente, que el cronómetro del tick cuente el
// tick completo, y que las ráfagas de catch-up queden registradas — que es la métrica que la re-medición de
// escala del 2026-09-05 señaló como la importante (doc 6 §1).
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { RegistroDePartidas } from '../registroDePartidas';
import { RegistroDeAuditoria } from '../auditoria';
import { HubDeDifusion } from '../difusion/hub';
import { recogerMetricas } from '../metricas';

let directorio: string;
let registro: RegistroDePartidas;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-metricas-'));
});

afterEach(async () => {
  await registro?.cerrar();
  await rm(directorio, { recursive: true, force: true });
});

/** Reloj de pared controlable, como en el resto de tests del servidor: el catch-up se mide contra él. */
function relojDesde(inicio: string) {
  let ms = new Date(inicio).getTime();
  return { ahora: () => new Date(ms).toISOString(), avanzar: (delta: number) => (ms += delta) };
}

function fuentes(extra: Partial<Parameters<typeof recogerMetricas>[0]> = {}) {
  return {
    partidas: registro,
    auditoria: new RegistroDeAuditoria(directorio, () => '2026-09-05T10:00:00.000Z'),
    hub: new HubDeDifusion(),
    ahora: () => '2026-09-05T10:00:00.000Z',
    ...extra,
  };
}

describe('recogerMetricas', () => {
  it('un proceso sin partidas abiertas no falla y lo dice', async () => {
    registro = new RegistroDePartidas(directorio);
    const m = recogerMetricas(fuentes());

    expect(m.proceso.partidasAbiertas).toBe(0);
    expect(m.partidas).toEqual([]);
    expect(m.proceso.memoriaMb.rss).toBeGreaterThan(0);
  });

  it('una partida abierta aparece con su tick, version y conexiones', async () => {
    registro = new RegistroDePartidas(directorio);
    await registro.abrir('g1', { seed: 42 });

    const m = recogerMetricas(fuentes());

    expect(m.proceso.partidasAbiertas).toBe(1);
    expect(m.partidas[0]).toMatchObject({ gameId: 'g1', tick: 0, colaPendiente: 0, ticksEjecutados: 0, conexiones: 0 });
    // Sin `intervaloTickMs` no hay reloj de mundo: es el default opt-in de C12/D5, y la métrica debe decirlo.
    expect(m.partidas[0]!.relojDeMundoActivo).toBe(false);
  });

  it('el cronometro del tick mide el tick COMPLETO y acumula media y maximo', async () => {
    registro = new RegistroDePartidas(directorio);
    const runner = await registro.abrir('g1', { seed: 42 });
    await runner.avanzarTick();
    await runner.avanzarTick();

    const p = recogerMetricas(fuentes()).partidas[0]!;
    expect(p.ticksEjecutados).toBe(2);
    expect(p.tickMsUltimo).toBeGreaterThan(0);
    expect(p.tickMsMedio).toBeGreaterThan(0);
    // El máximo no puede ser menor que la media: es la comprobación barata de que no se están mezclando.
    expect(p.tickMsMaximo).toBeGreaterThanOrEqual(p.tickMsMedio);
  });

  it('la cola refleja trabajo pendiente mientras lo hay, y vuelve a cero al drenar', async () => {
    registro = new RegistroDePartidas(directorio);
    const runner = await registro.abrir('g1', { seed: 42 });

    // Tres ticks encolados sin esperar: la cola es serial, así que hay pendientes de verdad.
    const enVuelo = [runner.avanzarTick(), runner.avanzarTick(), runner.avanzarTick()];
    expect(recogerMetricas(fuentes()).partidas[0]!.colaPendiente).toBeGreaterThan(0);

    await Promise.all(enVuelo);
    expect(recogerMetricas(fuentes()).partidas[0]!.colaPendiente).toBe(0);
  });

  it('registra el tamano de la rafaga de catch-up — la metrica que motivo todo esto', async () => {
    // Es el escenario del doc 6 §1: proceso caído, reabierto tras un hueco, y el mundo se pone al día en
    // ráfaga por la cola serial. Sin esta métrica, esos minutos solo se ven como "el servidor no responde".
    const reloj = relojDesde('2026-09-05T10:00:00.000Z');
    registro = new RegistroDePartidas(directorio, undefined, reloj.ahora);
    const runner = await registro.abrir('g1', { seed: 42 });

    reloj.avanzar(5 * 60_000); // cinco minutos de mundo adeudados
    runner.iniciarRelojDeMundo(60_000);
    await runner.esperarColaVacia();

    const p = recogerMetricas(fuentes({ ahora: reloj.ahora })).partidas[0]!;
    expect(p.ultimaRafagaTicks).toBe(5);
    expect(p.mayorRafagaTicks).toBe(5);
    expect(p.tick).toBe(5);
    expect(p.relojDeMundoActivo).toBe(true);
    runner.detenerRelojDeMundo();
  });

  it('`mayorRafagaTicks` conserva el pico aunque la ultima rafaga sea menor', async () => {
    const reloj = relojDesde('2026-09-05T10:00:00.000Z');
    registro = new RegistroDePartidas(directorio, undefined, reloj.ahora);
    const runner = await registro.abrir('g1', { seed: 42 });

    reloj.avanzar(4 * 60_000);
    runner.iniciarRelojDeMundo(60_000);
    await runner.esperarColaVacia();
    reloj.avanzar(1 * 60_000);
    await runner.esperarColaVacia();

    const p = recogerMetricas(fuentes({ ahora: reloj.ahora })).partidas[0]!;
    // Un pico que solo se ve en la media queda escondido; el propósito del máximo es justo no perderlo.
    expect(p.mayorRafagaTicks).toBe(4);
    runner.detenerRelojDeMundo();
  });

  it('cuenta los comandos por resultado, con las causas separadas', async () => {
    registro = new RegistroDePartidas(directorio);
    const auditoria = new RegistroDeAuditoria(directorio, () => '2026-09-05T10:00:00.000Z');
    const base = { gameId: 'g1', actor: 'ana', comando: 'crearFaccion' } as const;
    auditoria.registrar({ ...base, resultado: 'aceptado' });
    auditoria.registrar({ ...base, resultado: 'rechazado', causa: 'autorizacion' });
    auditoria.registrar({ ...base, resultado: 'rechazado', causa: 'autorizacion' });
    auditoria.registrar({ ...base, resultado: 'rechazado', causa: 'dominio' });
    await auditoria.drenar();

    const m = recogerMetricas(fuentes({ auditoria }));
    // Separadas y no agregadas en "rechazados": un pico de `autorizacion` es moderación, uno de `esquema` es
    // un cliente roto. Agregarlas perdería justo lo que hace útil el número.
    expect(m.comandos).toEqual({ aceptados: 1, autorizacion: 2, esquema: 0, dominio: 1, persistencia: 0 });
    expect(m.auditoriaFallida).toBe(0);
  });

  it('el recuento no depende de que la escritura en disco funcione', async () => {
    // Cuenta lo que el servidor DECIDIÓ, no lo que se pudo registrar: son dos preguntas distintas, y la
    // segunda ya la responde `auditoriaFallida`.
    registro = new RegistroDePartidas(directorio);
    const auditoria = new RegistroDeAuditoria(directorio, () => '2026-09-05T10:00:00.000Z');
    auditoria.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado' });

    // Sin drenar: la escritura ni siquiera ha ocurrido todavía y el recuento ya es correcto.
    expect(recogerMetricas(fuentes({ auditoria })).comandos.aceptados).toBe(1);
    await auditoria.drenar();
  });

  it('los contadores que devuelve son copias, no las internas', async () => {
    registro = new RegistroDePartidas(directorio);
    const auditoria = new RegistroDeAuditoria(directorio, () => '2026-09-05T10:00:00.000Z');
    auditoria.registrar({ gameId: 'g1', actor: 'ana', comando: 'crearFaccion', resultado: 'aceptado' });
    await auditoria.drenar();

    const m = recogerMetricas(fuentes({ auditoria }));
    m.comandos.aceptados = 999;

    expect(recogerMetricas(fuentes({ auditoria })).comandos.aceptados).toBe(1);
  });
});
