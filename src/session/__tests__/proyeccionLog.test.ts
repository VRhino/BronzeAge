// El log en texto dejó de ser estado persistido y pasó a derivarse de `eventosDominio` (`proyectarLog`).
// Dos cosas que proteger:
//
//  1. Que el prefijo del asentamiento vuelva. Cuando el tick pasó a alimentar el log desde `eventosDominio`
//     —cuyos `mensaje` NO llevan prefijo, porque el id va aparte en `asentamientoId`— la consola empezó a
//     mostrar "granja completado." sin decir de cuál. Regresión real, arreglada en la proyección.
//  2. Que un evento GLOBAL (facción, diplomacia, mercado) NO se prefije: no pertenece a ningún asentamiento.
import { describe, expect, it } from 'vitest';
import { proyectarLog } from '../estado';
import type { EventoDominio } from '../../domain/eventos';

const MOMENTO = '2026-01-01T00:00:00.000Z';

function ev(mensaje: string, asentamientoId?: string): EventoDominio {
  return { codigo: 'prueba', mensaje, momento: MOMENTO, tick: 7, asentamientoId };
}

describe('proyectarLog', () => {
  it('prefija con el id del asentamiento los eventos que le pertenecen', () => {
    expect(proyectarLog([ev('granja completado.', 'asent-1')])).toEqual([{ tick: 7, mensaje: 'asent-1: granja completado.' }]);
  });

  it('deja intactos los eventos globales, que no pertenecen a ningún asentamiento', () => {
    expect(proyectarLog([ev('Se crea la Facción "Micenas".')])).toEqual([{ tick: 7, mensaje: 'Se crea la Facción "Micenas".' }]);
  });

  it('conserva el orden y el tick de cada evento', () => {
    const proyectado = proyectarLog([ev('primero', 'a'), ev('segundo'), ev('tercero', 'b')]);
    expect(proyectado.map((e) => e.mensaje)).toEqual(['a: primero', 'segundo', 'b: tercero']);
    expect(proyectado.every((e) => e.tick === 7)).toBe(true);
  });
});
