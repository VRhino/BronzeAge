// Informe de carga (03 §14): agregación de ventanas, comparación con el sondeo y candidatos de §13.
import { describe, expect, it } from 'vitest';
import type { EntradaAuditoria } from '../auditoria';
import { aMarkdown, generarInforme, UMBRALES } from '../informeDeCarga';
import type { MuestraDeCanal, MuestraDeCarga } from '../telemetria';

const r = (p50: number, p95 = p50, max = p95, n = 1) => ({ n, p50, p95, max });
const canal = (o: Partial<MuestraDeCanal> = {}): MuestraDeCanal => ({
  flushes: 10, bytesEnviados: 600_000, bytesFlush: r(1000), bytesFoto: r(0, 0, 0, 0), bytesFotoVida: 0, msProyeccion: r(1), msDiff: r(1), msSerializacion: r(0.1),
  fotos: { alta: 0, resync: 0, lento: 0, visibilidad: 0 }, cambios: { upsert: 0, patch: 0, remove: 0, set: 0 }, socketsLentos: 0, cierres4408: 0, suscriptores: 2, claves: 1, ...o,
});
const muestra = (i: number, canales: Record<string, MuestraDeCanal>, o: Partial<MuestraDeCarga> = {}): MuestraDeCarga => ({
  formatoVersion: 1, tipo: 'muestra', momento: `2026-10-10T10:00:${String(i * 10).padStart(2, '0')}.000Z`, gameId: 'g1', ventanaMs: i === 0 ? 0 : 10_000,
  tick: i, version: i, conexiones: 2, colaPendiente: 0, suscriptores: 2, claves: 1, tickMs: r(10, 20, 30), guardadoMs: r(5), guardadoBytes: 2000,
  canales, memoriaMb: { rss: 100 + i, heapUsado: 50 }, bucleMs: { p50: 1, p99: 5, max: 9 }, costeMuestraMs: 0.1, ...o,
});

describe('generarInforme', () => {
  it('agrega ventanas, calcula bytes/min por cliente y compara con el sondeo', () => {
    // 600 000 B en 10 s con 2 clientes = 1 800 000 B/min/cliente. Foto de admin de 275 KiB a 1 sondeo/3 s = 5 632 000 B/min.
    const adm = canal({ bytesFoto: r(281_600) });
    const i = generarInforme([0, 1, 2].map((n) => muestra(n, { 'admin/partida': adm })));
    expect(i.muestras).toBe(3);
    expect(i.duracionMin).toBeCloseTo(20 / 60, 1);
    expect(i.tickMs).toEqual({ p50: 10, p95: 20, max: 30 });
    expect(i.memoriaMb).toEqual({ rssInicial: 100, rssFinal: 102, rssMax: 102 });
    expect(i.canales['admin/partida']!.bytesMinPorCliente).toBe(1_800_000);
    expect(i.contraSondeo.admin!.sondeoBytesMin).toBe(5_632_000);
    expect(i.contraSondeo.admin!.razon).toBeCloseTo(0.32, 2);
  });

  it('activa los candidatos de §13 solo al superar el umbral', () => {
    const sano = generarInforme([0, 1].map((n) => muestra(n, { 'vista/mapa': canal({ bytesEnviados: 1000 }) })));
    expect(sano.candidatos).toEqual([]);

    const apurado = generarInforme(
      [0, 1].map((n) =>
        muestra(
          n,
          { 'vista/mapa': canal({ bytesEnviados: 1000, msProyeccion: r(2, UMBRALES.msProyeccionP95 + 1), fotos: { alta: 1, resync: 3, lento: 2, visibilidad: 0 }, socketsLentos: 1 }) },
          { bucleMs: { p50: 1, p99: 80, max: 90 } }
        )
      )
    );
    expect(apurado.candidatos.map((c) => c.id).sort()).toEqual(['13.3', '13.4', '13.6', '13.8']);
  });

  it('resume la auditoría: rechazos por causa y comandos más caros', () => {
    const e = (comando: string, ms: number, o: Partial<EntradaAuditoria> = {}): EntradaAuditoria =>
      ({ formatoVersion: 1, momento: 'x', gameId: 'g1', actor: 'a', comando, resultado: 'aceptado', ms, bytes: 100 * ms, ...o });
    const i = generarInforme([muestra(1, {})], [e('fundar', 5), e('fundar', 50), e('construir', 2), e('construir', 1, { resultado: 'rechazado', causa: 'dominio' })]);
    expect(i.auditoria!.rechazos).toEqual({ dominio: 1 });
    expect(i.auditoria!.masCaros[0]).toMatchObject({ comando: 'fundar', n: 2, msMax: 50, bytesMax: 5000 });
    expect(aMarkdown(i)).toContain('fundar');
  });

  it('sin muestras no revienta', () => {
    const i = generarInforme([]);
    expect(i.muestras).toBe(0);
    expect(aMarkdown(i)).toContain('Ningún umbral superado');
  });
});
