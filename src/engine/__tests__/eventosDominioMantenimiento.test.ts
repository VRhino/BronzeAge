// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `mantenimiento.ts` migrado (el evento de subida de
// nivel se fue con la subida manual a `ascenso.test.ts`). Las funciones son
// puras y directamente testeables — se construye el `Asentamiento` en el estado exacto que dispara cada
// código y se llama a mano, sin correr un tick completo ni encadenar muchos.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import {
  avanzarMantenimiento,
  type PayloadAsentamientoRuinas,
  type PayloadMantenimientoColapsado,
  type PayloadMantenimientoDeficit,
  type PayloadMantenimientoRecuperado,
} from '../mantenimiento';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

function base(): Asentamiento {
  const mapa = crearMapaDeterminista(7);
  return fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
}

describe('eventos de dominio — mantenimiento.ts', () => {
  it('avanzarMantenimiento: bajo ocupación reciente NO degrada aunque haya déficit (Ocupacion §2.4)', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      fundadoEn: instanteDeTest(0),
      medidorMantenimiento: MANTENIMIENTO.medidorInicial,
      almacen: { ...base().almacen, madera: { cantidad: 0, capacidad: 1000 } },
      ocupacionHasta: instanteDeTest(MANTENIMIENTO.graciaMinutos + 100),
    };

    const resultado = avanzarMantenimiento(asentamiento, undefined, instanteDeTest(MANTENIMIENTO.graciaMinutos + 1));

    expect(resultado.destruido).toBe(false);
    expect(resultado.eventos).toHaveLength(0);
    expect(resultado.asentamiento.medidorMantenimiento).toBe(MANTENIMIENTO.medidorInicial);
  });

  it('avanzarMantenimiento: sin madera para pagar produce mantenimiento.deficit', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      fundadoEn: instanteDeTest(0),
      medidorMantenimiento: MANTENIMIENTO.medidorInicial,
      almacen: { ...base().almacen, madera: { cantidad: 0, capacidad: 1000 } },
    };

    const resultado = avanzarMantenimiento(asentamiento, undefined, instanteDeTest(MANTENIMIENTO.graciaMinutos + 1));

    expect(resultado.destruido).toBe(false);
    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'mantenimiento.deficit');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadMantenimientoDeficit;
    expect(p.medidor).toBeLessThan(MANTENIMIENTO.medidorInicial);
  });

  it('avanzarMantenimiento: medidor a 0 con nivelActual > 1 produce mantenimiento.colapsado (no destruye)', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      fundadoEn: instanteDeTest(0),
      nivel: 2,
      nivelActual: 2,
      medidorMantenimiento: 0.1,
      almacen: { ...base().almacen, madera: { cantidad: 0, capacidad: 1000 } },
    };

    const resultado = avanzarMantenimiento(asentamiento, undefined, instanteDeTest(MANTENIMIENTO.graciaMinutos + 1));

    expect(resultado.destruido).toBe(false);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('mantenimiento.colapsado');
    const p = evento.payload as PayloadMantenimientoColapsado;
    expect(p.nivelActualAnterior).toBe(2);
    expect(p.nivelActualNuevo).toBe(1);
  });

  it('avanzarMantenimiento: medidor a 0 con nivelActual == 1 produce asentamiento.ruinas (destruye)', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      fundadoEn: instanteDeTest(0),
      nivel: 1,
      nivelActual: 1,
      medidorMantenimiento: 0.1,
      almacen: { ...base().almacen, madera: { cantidad: 0, capacidad: 1000 } },
    };

    const resultado = avanzarMantenimiento(asentamiento, undefined, instanteDeTest(MANTENIMIENTO.graciaMinutos + 1));

    expect(resultado.destruido).toBe(true);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('asentamiento.ruinas');
    const p = evento.payload as PayloadAsentamientoRuinas;
    expect(p.faltantes.length).toBeGreaterThan(0);
    expect(p.faltantes[0]!.recurso).toBe('madera');
    expect(p.fundadoEn).toBe(instanteDeTest(0));
    expect(p.duro).toBe((MANTENIMIENTO.graciaMinutos + 1) * 60_000); // graciaMinutos+1 ticks de mundo, en ms
  });

  it('avanzarMantenimiento: pago íntegro + racha suficiente produce mantenimiento.recuperado', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      fundadoEn: instanteDeTest(0),
      nivel: 2,
      nivelActual: 1,
      medidorMantenimiento: MANTENIMIENTO.medidorInicial,
      rachaMantenimientoSano: MANTENIMIENTO.minutosSanosParaRecuperarNivel - 1,
      almacen: { ...base().almacen, madera: { cantidad: 1000, capacidad: 1000 } },
    };

    const resultado = avanzarMantenimiento(asentamiento, undefined, instanteDeTest(MANTENIMIENTO.graciaMinutos + 1));

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('mantenimiento.recuperado');
    const p = evento.payload as PayloadMantenimientoRecuperado;
    expect(p.nivelActualNuevo).toBe(2);
  });
});
