// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `population.ts` migrado. Las 3 apariciones/muertes
// se disparan con condiciones deterministas (no dependen del rng) — se construye el `Asentamiento` justo en
// el borde de cada condición y se llama a la función a mano.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarNutricionPoblacion, crecerPoblacion, type PayloadHambrunaMuerte } from '../population';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

function edificioActivo(id: string, tipo: Edificio['tipo']): Edificio {
  return { id, tipo, posicion: { x: 0, y: 0 }, estado: 'activo', ambito: 'asentamiento' };
}

function base(): Asentamiento {
  const mapa = crearMapaDeterminista(7);
  return fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
}

describe('eventos de dominio — population.ts', () => {
  it('crecerPoblacion: primer edificio de transformación con Vivienda produce poblacion.primeros_artesanos', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      poblacion: { pesants: 0, artesanos: 0, nobleza: 0 },
      edificios: [edificioActivo('e1', 'fundicion'), edificioActivo('e2', 'vivienda')],
    };

    const resultado = crecerPoblacion(asentamiento, createRng(1));

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'poblacion.primeros_artesanos');
    expect(evento).toBeDefined();
    expect(resultado.poblacion.artesanos).toBe(1);
  });

  it('crecerPoblacion: Palacio + ciudadanía mínima produce poblacion.primeros_nobles', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      poblacion: { pesants: 0, artesanos: 0, nobleza: 0 },
      edificios: [edificioActivo('e1', 'palacio')],
      casasCompradas: ['j1', 'j2', 'j3'],
    };

    const resultado = crecerPoblacion(asentamiento, createRng(1));

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'poblacion.primeros_nobles');
    expect(evento).toBeDefined();
    expect(resultado.poblacion.nobleza).toBe(1);
  });

  it('avanzarNutricionPoblacion: nutrición ya en 0 y sin trigo produce poblacion.hambruna_muerte', () => {
    const asentamiento: Asentamiento = {
      ...base(),
      poblacion: { pesants: 10, artesanos: 10, nobleza: 0 },
      nutricionPoblacion: 0,
      almacen: { ...base().almacen, trigo: { cantidad: 0, capacidad: 1000 } },
    };

    const resultado = avanzarNutricionPoblacion(asentamiento);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('poblacion.hambruna_muerte');
    const p = evento.payload as PayloadHambrunaMuerte;
    expect(p.muertes).toBeGreaterThan(0);
    expect(p.muertesPesants + p.muertesArtesanos).toBe(p.muertes);
  });
});
