// Composición del estado del cliente admin a partir de la versión ligera + el detalle (`cliente/src/app/estadoEnVivo.ts`, 03 §2.1).
import { describe, expect, it } from 'vitest';
import { AlmacenDeCanal } from '../../../../cliente/src/app/almacenDeCanal';
import { fusionarEstado } from '../../../../cliente/src/app/estadoEnVivo';
import type { EstadoAdmin } from '../../estado';

const base = {
  asentamientos: [
    { id: 'a1', nombre: 'Viejo', almacen: { madera: { cantidad: 1, capacidad: 9 } }, edificios: [{ id: 'b1', tipo: 'granja', ambito: 'asentamiento' }, { id: 'm1', tipo: 'muralla', ambito: 'mapa' }], recintos: [] },
  ],
  campamentosMercenarios: [{ id: 'c1', mercado: { hierro: 3 }, fondos: {}, residentesIds: [] }],
  heroes: [],
  historialHeroes: { h1: [{ momento: 'x', mensaje: 'viejo' }] },
  instante: 1,
  version: 1,
  relojDeMundoIntervaloMs: null,
  acuerdos: [{ id: 'ac-http' }],
} as unknown as EstadoAdmin;

function partida(datos: Record<string, unknown>): AlmacenDeCanal {
  const a = new AlmacenDeCanal([], { titulos: 'tituloId' });
  a.cargarFoto(7, 700, datos);
  return a;
}

const ligero = {
  asentamientos: [{ id: 'a1', nombre: 'Nuevo', edificios: [{ id: 'm1', tipo: 'muralla', ambito: 'mapa', estado: 'activo' }], resumen: { edificiosActivos: 4 } }],
  campamentosMercenarios: [{ id: 'c1', residentesIds: ['h1'] }],
  heroes: [{ id: 'h1' }],
  ejercitos: [],
};

describe('cliente admin: estado en vivo', () => {
  it('sin detalle: lo ligero fresco encima, el interior de la última lectura completa debajo', () => {
    const e = fusionarEstado(base, partida(ligero), () => undefined, 5000);
    const a = e.asentamientos[0]! as unknown as { nombre: string; almacen: unknown; edificios: { id: string; estado?: string }[]; resumen: unknown };
    expect(a.nombre).toBe('Nuevo');
    expect(a.almacen).toEqual({ madera: { cantidad: 1, capacidad: 9 } });
    expect(a.edificios.map((x) => x.id).sort()).toEqual(['b1', 'm1']);
    expect(a.edificios.find((x) => x.id === 'm1')!.estado).toBe('activo');
    expect(a.resumen).toEqual({ edificiosActivos: 4 });
    expect(e.campamentosMercenarios[0]!.mercado).toEqual({ hierro: 3 });
    expect(e.campamentosMercenarios[0]!.residentesIds).toEqual(['h1']);
    expect(e.relojDeMundoIntervaloMs).toBe(5000);
    expect(e.instante).toBe(700);
    expect(e.version).toBe(7);
  });

  it('con detalle: el almacén y los edificios salen del canal de detalle', () => {
    const det = new AlmacenDeCanal(['asentamiento']);
    det.cargarFoto(3, 700, { asentamiento: { id: 'a1', nombre: 'Nuevo', almacen: { madera: { cantidad: 50, capacidad: 99 } } }, edificios: [{ id: 'b1', tipo: 'granja', estado: 'activo' }] });
    const e = fusionarEstado(base, partida(ligero), (c) => (c === 'admin/asentamiento/a1' ? det : undefined), 5000);
    const a = e.asentamientos[0]! as unknown as { almacen: { madera: { cantidad: number } }; edificios: { id: string }[]; resumen: unknown };
    expect(a.almacen.madera.cantidad).toBe(50);
    expect(a.edificios.map((x) => x.id)).toEqual(['b1']);
    expect(a.resumen).toEqual({ edificiosActivos: 4 });
  });

  it('el historial de un héroe viaja por su canal; lo que el canal no lleva se conserva', () => {
    const det = new AlmacenDeCanal(['heroe']);
    det.cargarFoto(1, 700, { heroe: { id: 'h1' }, historial: [{ id: 'z', momento: 'y', mensaje: 'nuevo' }] });
    const e = fusionarEstado(base, partida(ligero), (c) => (c === 'admin/heroe/h1' ? det : undefined), undefined);
    expect(e.historialHeroes['h1']![0]!.mensaje).toBe('nuevo');
    expect((e as unknown as { acuerdos: unknown[] }).acuerdos).toEqual([{ id: 'ac-http' }]);
    expect(e.relojDeMundoIntervaloMs).toBeNull();
  });

  it('un asentamiento que la lectura completa no conocía sale con almacén vacío, no roto', () => {
    const p = partida({ ...ligero, asentamientos: [{ id: 'a2', nombre: 'Recién fundada', edificios: [], resumen: {} }] });
    const a = fusionarEstado(base, p, () => undefined, 5000).asentamientos[0]! as unknown as { almacen: unknown; recintos: unknown };
    expect(a.almacen).toEqual({});
    expect(a.recintos).toEqual([]);
  });
});
