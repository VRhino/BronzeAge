// Los talleres respetan la reserva de la auto-construcción y se pueden parar receta a receta (Doc 4.2.1, decisión del
// usuario 2026-09-28). Medido en la Era I: una Armería vaciaba la madera de su plaza en medio día y la dejaba caer en
// ruinas por no poder pagar el mantenimiento — 13 plazas en dos semanas.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { avanzarConstruccion, reclamosDeFuentes } from '../construction';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

function conArmeria(madera: number, extra: Partial<Asentamiento> = {}): Asentamiento {
  const mapa = crearMapaDeterminista(7);
  const a = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
  const armeria = { ...a.edificios[0]!, id: 'armeria-1', tipo: 'armeria' as const, estado: 'activo' as const, nivelInterno: 1 };
  return {
    ...a,
    edificios: [...a.edificios, armeria],
    poblacion: { pesants: 100, artesanos: 50, nobleza: 0 },
    almacen: { ...a.almacen, madera: { cantidad: madera, capacidad: 10_000 } },
    autoConstruccionPausada: true,
    ...extra,
  };
}

function trasUnTick(a: Asentamiento) {
  const mapa = crearMapaDeterminista(7);
  return avanzarConstruccion(a, [], mapa, undefined, reclamosDeFuentes([a]), instanteDeTest(1), 0).asentamiento;
}

describe('talleres', () => {
  it('con madera de sobra, la Armería hace armas de madera', () => {
    expect(trasUnTick(conArmeria(1000)).almacen['armaMadera']?.cantidad ?? 0).toBeGreaterThan(0);
  });

  it('no baja la madera por debajo de la reserva de la construcción', () => {
    const tras = trasUnTick(conArmeria(3));
    expect(tras.almacen['armaMadera']?.cantidad ?? 0).toBe(0);
    expect(tras.almacen['madera']!.cantidad).toBeGreaterThanOrEqual(3);
  });

  it('una receta parada no se produce', () => {
    expect(trasUnTick(conArmeria(1000, { recetasPausadas: ['armaMadera'] })).almacen['armaMadera']?.cantidad ?? 0).toBe(0);
  });
});
