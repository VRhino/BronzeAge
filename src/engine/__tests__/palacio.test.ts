// Palacio con niveles (Doc 4.2.1): se construye en el nivel 2 y el nivel 5 pide el Palacio 3 (Doc 4.5).
import { describe, expect, it } from 'vitest';
import { EDIFICIO_CATALOGO, POLITICAS } from '../../constants';
import { slotsDisponibles } from '../politicas';
import type { Asentamiento } from '../../domain/types';
import { evaluarGatesDeNivel } from '../mantenimiento';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

function conPalacio(nivelInterno: number): Asentamiento {
  const a = fundarAsentamientoDeTest(crearMapaDeterminista(7), crearFacciones(), 'faccion-1', []).asentamiento;
  const palacio = { id: 'palacio-t', tipo: 'palacio' as const, posicion: { x: 30, y: 0 }, estado: 'activo' as const, ambito: 'asentamiento' as const, nivelInterno };
  return { ...a, nivel: 4, nivelActual: 4, edificios: [...a.edificios, palacio], poblacion: { pesants: 5000, artesanos: 5000, nobleza: 0 } };
}

describe('Palacio', () => {
  it('se construye desde el nivel 2 con cupo de 80 / 240 / 400 nobles por nivel', () => {
    expect(EDIFICIO_CATALOGO.palacio.requisitoNivelAsentamientoConstruccion).toBe(2);
    expect([1, 2, 3].map((n) => EDIFICIO_CATALOGO.palacio.niveles[n]!.capacidadNobles)).toEqual([80, 240, 400]);
  });

  it('el nivel 5 pide el Palacio de nivel 3', () => {
    expect(evaluarGatesDeNivel(conPalacio(2), 5)!.edificios).toBe(false);
    expect(evaluarGatesDeNivel(conPalacio(3), 5)!.edificios).toBe(true);
  });
});

describe('Sala del Consejo (Doc 4.4)', () => {
  it('da una ranura de política más al Gobernador, por encima del máximo', () => {
    const { base, maximo } = POLITICAS.slotsPorCargo.gobernador;
    expect(slotsDisponibles('gobernador', 1)).toBe(base);
    expect(slotsDisponibles('gobernador', 1, true)).toBe(base + POLITICAS.slotSalaConsejo);
    expect(slotsDisponibles('gobernador', 99, true)).toBe(maximo + POLITICAS.slotSalaConsejo);
    expect(slotsDisponibles('tesorero', 1, true)).toBe(POLITICAS.slotsPorCargo.tesorero.base);
  });
});
