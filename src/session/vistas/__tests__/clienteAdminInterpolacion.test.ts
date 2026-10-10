// Interpolación de lo móvil en el mapa del cliente admin (`cliente/src/ui/interpolacion.ts`, 03 §4.1).
import { describe, expect, it } from 'vitest';
import { interpolar, puntoEnRuta } from '../../../../cliente/src/ui/interpolacion';

const ruta = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }];

describe('cliente admin: interpolación de móviles', () => {
  it('puntoEnRuta sigue la polilínea por longitud', () => {
    expect(puntoEnRuta(ruta, 0)).toEqual({ x: 0, y: 0 });
    expect(puntoEnRuta(ruta, 0.25)).toEqual({ x: 5, y: 0 });
    expect(puntoEnRuta(ruta, 0.75)).toEqual({ x: 10, y: 5 });
    expect(puntoEnRuta(ruta, 1)).toEqual({ x: 10, y: 10 });
  });

  it('avanza la fracción del tick sobre la ruta y se capa en 1', () => {
    const m = { id: 'e1', ruta, progreso: 0.25, avancePorTick: 0.25, posicionActual: { x: 5, y: 0 } };
    expect(interpolar([m], 0.5)[0]!.posicionActual).toEqual({ x: 7.5, y: 0 });
    expect(interpolar([m], 5)[0]!.progreso).toBe(0.5);
  });

  it('devuelve el mismo array si nada se mueve', () => {
    const quieto = [{ ruta, progreso: 0.25, avancePorTick: 0, posicionActual: { x: 5, y: 0 } }];
    expect(interpolar(quieto, 0.5)).toBe(quieto);
    const sinRuta = [{ posicionActual: { x: 1, y: 1 } }];
    expect(interpolar(sinRuta, 0.5)).toBe(sinRuta);
  });

  it('no mueve lo que no cuadra con su ruta (retorno, ruta cambiada)', () => {
    const raro = [{ ruta, progreso: 0.25, avancePorTick: 0.1, posicionActual: { x: 9, y: 9 } }];
    expect(interpolar(raro, 0.5)).toBe(raro);
  });

  it('un avance negativo (retorna por la misma ruta) retrocede', () => {
    const m = { ruta, progreso: 0.75, avancePorTick: -0.25, posicionActual: { x: 10, y: 5 } };
    expect(interpolar([m], 1)[0]!.posicionActual).toEqual({ x: 10, y: 0 });
  });
});
