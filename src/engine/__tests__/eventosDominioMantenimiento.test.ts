// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `mantenimiento.ts` migrado. Ambas funciones son
// puras y directamente testeables — se construye el `Asentamiento` en el estado exacto que dispara cada
// código y se llama a mano, sin correr un tick completo ni encadenar muchos.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import {
  avanzarMantenimiento,
  avanzarNivelAsentamiento,
  type PayloadAsentamientoRuinas,
  type PayloadMantenimientoColapsado,
  type PayloadMantenimientoDeficit,
  type PayloadMantenimientoRecuperado,
  type PayloadNivelSubio,
} from '../mantenimiento';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

function edificioActivo(id: string, tipo: Edificio['tipo']): Edificio {
  return { id, tipo, posicion: { x: 0, y: 0 }, estado: 'activo', ambito: 'asentamiento' };
}

function base(): Asentamiento {
  const mapa = crearMapaDeterminista(7);
  return fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
}

describe('eventos de dominio — mantenimiento.ts', () => {
  it('avanzarNivelAsentamiento: gates de nivel 2 cumplidos produce asentamiento.nivel_subio', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      poblacion: { pesants: 200, artesanos: 0, nobleza: 0 },
      edificios: [edificioActivo('e1', 'cantera'), edificioActivo('e2', 'lenera'), edificioActivo('e3', 'granja')],
    };

    const resultado = avanzarNivelAsentamiento(asentamiento);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('asentamiento.nivel_subio');
    const p = evento.payload as PayloadNivelSubio;
    expect(p.asentamientoId).toBe(asentamiento.id);
    expect(p.nivelNuevo).toBe(2);
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
