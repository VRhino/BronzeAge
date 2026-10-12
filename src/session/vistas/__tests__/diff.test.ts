import { describe, expect, it } from 'vitest';
import { aplicarDiferencia, clavePorId, diferenciar, iguales, type EsquemaDeVista, type Foto } from '../diff';

const ESQUEMA: EsquemaDeVista = {
  colecciones: { ejercitos: clavePorId, asentamientosConocidos: ((e: { asentamientoId: string }) => e.asentamientoId) as never },
  mapas: ['nombresDeCompaneros'],
  escalares: ['instante', 'estadoMapa'],
};

const ruta = [{ x: 0, y: 0 }, { x: 100, y: 0 }];
const foto = (extra: Foto = {}): Foto => ({
  instante: 10,
  estadoMapa: 'a',
  ejercitos: [{ id: 'e1', progreso: 0.1, ruta, posicionActual: { x: 10, y: 0 } }, { id: 'e2', progreso: 0.5, ruta, posicionActual: { x: 50, y: 0 } }],
  asentamientosConocidos: [{ asentamientoId: 'a1', nombre: 'Micenas' }],
  nombresDeCompaneros: { h1: 'Ana' },
  exploracion: { celdas: [1, 2, 3] },
  ...extra,
});

const ordenada = (f: Foto): unknown =>
  JSON.parse(JSON.stringify(f, (_k, v) => (Array.isArray(v) && v.length > 0 && typeof v[0] === 'object' && v[0] && ('id' in v[0] || 'asentamientoId' in v[0]) ? [...v].sort((a, b) => String(a.id ?? a.asentamientoId).localeCompare(String(b.id ?? b.asentamientoId))) : v)));

describe('diferenciar', () => {
  it('fotos iguales no producen cambios', () => {
    expect(diferenciar(foto(), foto(), ESQUEMA)).toEqual({ cambios: [], escalares: {} });
  });

  it('lo que se mueve va en un patch que no repite la ruta', () => {
    const antes = foto();
    const despues = foto({ ejercitos: [{ id: 'e1', progreso: 0.2, ruta, posicionActual: { x: 20, y: 0 } }, antes.ejercitos instanceof Array ? antes.ejercitos[1] : null] });
    const d = diferenciar(antes, despues, ESQUEMA);
    expect(d.cambios).toEqual([{ t: 'ejercitos', id: 'e1', op: 'patch', v: { progreso: 0.2, posicionActual: { x: 20, y: 0 } } }]);
  });

  it('quita, inserta y retoca, con los remove primero', () => {
    const antes = foto();
    const despues = foto({ ejercitos: [{ id: 'e2', progreso: 0.6, ruta, posicionActual: { x: 60, y: 0 } }, { id: 'e3', progreso: 0, ruta, posicionActual: { x: 0, y: 0 } }] });
    const d = diferenciar(antes, despues, ESQUEMA);
    expect(d.cambios.map((c) => `${c.op}:${'id' in c ? c.id : ''}`)).toEqual(['remove:e1', 'patch:e2', 'upsert:e3']);
  });

  it('los escalares viajan aparte, solo si cambian', () => {
    const d = diferenciar(foto(), foto({ instante: 11 }), ESQUEMA);
    expect(d).toEqual({ cambios: [], escalares: { instante: 11 } });
  });

  it('un bloque sin clave se sustituye entero si cambia', () => {
    const d = diferenciar(foto(), foto({ exploracion: { celdas: [1, 2, 3, 4] } }), ESQUEMA);
    expect(d.cambios).toEqual([{ t: 'exploracion', op: 'set', v: { celdas: [1, 2, 3, 4] } }]);
  });

  it('los mapas id → valor se difieren por clave', () => {
    const d = diferenciar(foto(), foto({ nombresDeCompaneros: { h2: 'Bea' } }), ESQUEMA);
    expect(d.cambios).toEqual([
      { t: 'nombresDeCompaneros', id: 'h1', op: 'remove' },
      { t: 'nombresDeCompaneros', id: 'h2', op: 'upsert', v: 'Bea' },
    ]);
  });

  it('una entidad con un null legítimo se manda entera, para no confundirlo con un borrado', () => {
    const antes = foto({ ejercitos: [{ id: 'e1', persiguiendo: undefined, objetivo: { x: 1 } }] });
    const despues = foto({ ejercitos: [{ id: 'e1', persiguiendo: null, objetivo: { x: 2 } }] });
    const d = diferenciar(antes, despues, ESQUEMA);
    expect(d.cambios).toEqual([{ t: 'ejercitos', id: 'e1', op: 'upsert', v: { id: 'e1', persiguiendo: null, objetivo: { x: 2 } } }]);
  });

  it('una clave que desaparece de la entidad se borra con null', () => {
    const antes = foto({ ejercitos: [{ id: 'e1', persiguiendo: { id: 'x' }, progreso: 0 }] });
    const despues = foto({ ejercitos: [{ id: 'e1', progreso: 0 }] });
    expect(diferenciar(antes, despues, ESQUEMA).cambios).toEqual([{ t: 'ejercitos', id: 'e1', op: 'patch', v: { persiguiendo: null } }]);
  });
});

describe('aplicarDiferencia (el cliente)', () => {
  it('foto + diferencia ≡ foto nueva, en una secuencia de cambios variados', () => {
    const pasos: Foto[] = [
      foto(),
      foto({ instante: 11, ejercitos: [{ id: 'e1', progreso: 0.2, ruta, posicionActual: { x: 20, y: 0 } }] }),
      foto({ instante: 12, estadoMapa: 'b', ejercitos: [{ id: 'e1', progreso: 0.3, ruta, posicionActual: { x: 30, y: 0 } }, { id: 'e9', progreso: 0, ruta: [], posicionActual: { x: 1, y: 1 } }] }),
      foto({ instante: 13, ejercitos: [], asentamientosConocidos: [], exploracion: { celdas: [] }, nombresDeCompaneros: {} }),
    ];
    let cliente = pasos[0]!;
    for (let i = 1; i < pasos.length; i++) {
      cliente = aplicarDiferencia(cliente, diferenciar(pasos[i - 1]!, pasos[i]!, ESQUEMA), ESQUEMA);
      expect(ordenada(cliente)).toEqual(ordenada(pasos[i]!));
    }
  });

  it('un patch sobre una entidad que no existe es un hueco', () => {
    expect(() => aplicarDiferencia(foto({ ejercitos: [] }), { cambios: [{ t: 'ejercitos', id: 'e1', op: 'patch', v: { progreso: 1 } }], escalares: {} }, ESQUEMA)).toThrow('hueco');
  });

  it('no muta la foto de entrada', () => {
    const antes = foto();
    const copia = JSON.stringify(antes);
    aplicarDiferencia(antes, diferenciar(antes, foto({ instante: 99, ejercitos: [] }), ESQUEMA), ESQUEMA);
    expect(JSON.stringify(antes)).toBe(copia);
  });
});

describe('iguales', () => {
  it('compara en profundidad y distingue arrays de objetos', () => {
    expect(iguales({ a: [1, { b: 2 }] }, { a: [1, { b: 2 }] })).toBe(true);
    expect(iguales({ a: [1] }, { a: { 0: 1 } })).toBe(false);
    expect(iguales(null, {})).toBe(false);
  });
});
