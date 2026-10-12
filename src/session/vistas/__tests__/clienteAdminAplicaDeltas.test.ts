// El almacén de canal del cliente admin (`cliente/src/app/almacenDeCanal.ts`) debe reconstruir exactamente la foto del servidor aplicando sus
// deltas: se prueba contra el diff REAL (`diferenciar`), no contra deltas escritos a mano, para que un cambio de formato en uno rompa aquí.
import { describe, expect, it } from 'vitest';
import { AlmacenDeCanal } from '../../../../cliente/src/app/almacenDeCanal';
import { diferenciar, type Foto } from '../diff';
import { analizarCanal, definicionDe } from '../segmentos';

function canal(nombre: string) {
  const c = analizarCanal(nombre, { tipo: 'admin' });
  if (!c) throw new Error(`canal ${nombre}`);
  return definicionDe(c).esquema;
}

/** Lo que el cliente ve del almacén, en la misma forma que la foto (arrays para colecciones, objetos para únicas y bloques). */
function leer(almacen: AlmacenDeCanal, foto: Foto, esquema: ReturnType<typeof canal>): Foto {
  const salida: Foto = {};
  for (const k of Object.keys(foto)) {
    if (k in esquema.colecciones) salida[k] = almacen.lista(k);
    else if (esquema.unicas?.includes(k)) salida[k] = almacen.unica(k);
    else salida[k] = almacen.bloque(k);
  }
  return salida;
}

const orden = (v: unknown): unknown => (Array.isArray(v) ? [...v].sort((a, b) => String((a as { id?: string }).id).localeCompare(String((b as { id?: string }).id))) : v);
const igualesSinOrden = (a: Foto, b: Foto) => {
  const A = Object.fromEntries(Object.entries(a).map(([k, v]) => [k, orden(v)]));
  const B = Object.fromEntries(Object.entries(b).map(([k, v]) => [k, orden(v)]));
  expect(A).toEqual(B);
};

describe('cliente admin: aplicar deltas del servidor', () => {
  it('admin/partida: patch de lo móvil, alta, baja, bloque y escalar', () => {
    const esquema = canal('admin/partida');
    const antes: Foto = {
      heroes: [{ id: 'h1', nombre: 'A' }, { id: 'h2', nombre: 'B' }],
      ejercitos: [{ id: 'e1', progreso: 0.1, posicionActual: { x: 1, y: 1 }, ruta: [{ x: 0, y: 0 }, { x: 10, y: 10 }] }],
      titulos: [{ tituloId: 't1', dueno: 'h1' }],
      facciones: [{ id: 'f1', nombre: 'F' }],
      tecnologia: { era: 'bronce' },
      mapaId: 'm1',
    };
    const despues: Foto = {
      heroes: [{ id: 'h1', nombre: 'A2' }, { id: 'h3', nombre: 'C' }],
      ejercitos: [{ id: 'e1', progreso: 0.2, posicionActual: { x: 2, y: 2 }, ruta: [{ x: 0, y: 0 }, { x: 10, y: 10 }], avancePorTick: 0.1 }],
      titulos: [{ tituloId: 't1', dueno: 'h3' }],
      facciones: [{ id: 'f1', nombre: 'F' }],
      tecnologia: { era: 'hierro' },
      mapaId: 'm1',
    };
    const almacen = new AlmacenDeCanal([], { titulos: 'tituloId' });
    almacen.cargarFoto(1, 100, antes);
    const d = diferenciar(antes, despues, esquema);
    expect(almacen.aplicarDelta({ desde: 1, hasta: 2, instante: 200, cambios: d.cambios as never, escalares: d.escalares })).toBe('aplicado');
    igualesSinOrden(leer(almacen, despues, esquema), despues);
    expect(almacen.version).toBe(2);
    expect(almacen.instante).toBe(200);
  });

  it('una colección sin cambios entrega el mismo array', () => {
    const esquema = canal('admin/partida');
    const antes: Foto = { heroes: [{ id: 'h1' }], facciones: [{ id: 'f1', nombre: 'F' }] };
    const despues: Foto = { heroes: [{ id: 'h1', x: 1 }], facciones: [{ id: 'f1', nombre: 'F' }] };
    const almacen = new AlmacenDeCanal();
    almacen.cargarFoto(1, 1, antes);
    const facciones = almacen.lista('facciones');
    const d = diferenciar(antes, despues, esquema);
    almacen.aplicarDelta({ desde: 1, hasta: 2, instante: 2, cambios: d.cambios as never });
    expect(almacen.lista('facciones')).toBe(facciones);
    expect(almacen.lista('heroes')).not.toBe(facciones);
  });

  it('admin/asentamiento: la entidad única y sus edificios', () => {
    const esquema = canal('admin/asentamiento/a1');
    const antes: Foto = { asentamiento: { id: 'a1', almacen: { madera: { cantidad: 5 } } }, edificios: [{ id: 'b1', estado: 'en_construccion' }] };
    const despues: Foto = { asentamiento: { id: 'a1', almacen: { madera: { cantidad: 9 } } }, edificios: [{ id: 'b1', estado: 'activo' }, { id: 'b2', estado: 'en_cola' }] };
    const almacen = new AlmacenDeCanal(['asentamiento']);
    almacen.cargarFoto(10, 1, antes);
    const d = diferenciar(antes, despues, esquema);
    expect(almacen.aplicarDelta({ desde: 10, hasta: 11, instante: 2, cambios: d.cambios as never })).toBe('aplicado');
    igualesSinOrden(leer(almacen, despues, esquema), despues);
  });

  it('un hueco no se aplica y un duplicado se ignora', () => {
    const almacen = new AlmacenDeCanal();
    almacen.cargarFoto(5, 1, { heroes: [{ id: 'h1' }] });
    expect(almacen.aplicarDelta({ desde: 7, hasta: 8, instante: 2, cambios: [] })).toBe('hueco');
    expect(almacen.aplicarDelta({ desde: 3, hasta: 5, instante: 2, cambios: [] })).toBe('duplicado');
    expect(almacen.aplicarDelta({ desde: 5, hasta: 6, instante: 2, cambios: [{ t: 'heroes', id: 'zz', op: 'patch', v: { x: 1 } }] })).toBe('hueco');
    expect(almacen.version).toBe(5);
  });

  it('un patch con null borra la clave', () => {
    const almacen = new AlmacenDeCanal();
    almacen.cargarFoto(1, 1, { heroes: [{ id: 'h1', a: 1, b: 2 }] });
    almacen.aplicarDelta({ desde: 1, hasta: 2, instante: 2, cambios: [{ t: 'heroes', id: 'h1', op: 'patch', v: { a: null, b: 3 } }] });
    expect(almacen.lista('heroes')).toEqual([{ id: 'h1', b: 3 }]);
  });
});
