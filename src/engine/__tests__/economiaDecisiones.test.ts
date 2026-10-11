// Decisiones del usuario del 2026-10-11 sobre la economía (Docs/Mechanics/Economia_Construccion.md): las piezas gratuitas cuentan
// la mitad en el mantenimiento, un almacén lleno PARA la producción en vez de desperdiciarla, la Gran Fundición cuenta como mano
// de obra de artesanos, y las mejoras piden "algún edificio de ese nivel", no "el primero".
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { MANTENIMIENTO } from '../../constants';
import { avanzarConstruccion, anadirEdificioManualmente, reclamosDeFuentes } from '../construction';
import { calcularCostoMantenimiento } from '../mantenimiento';
import { ratioManoObraArtesanos } from '../asentamientoQuery';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
import { TODAS_LAS_TECNOLOGIAS } from '../tecnologia';

function plaza(): Asentamiento {
  const mapa = crearMapaDeterminista(7);
  return fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
}

const edificio = (id: string, tipo: Edificio['tipo'], extra: Partial<Edificio> = {}): Edificio => ({
  id, tipo, posicion: { x: 0, y: 0 }, estado: 'activo', ambito: 'asentamiento', ...extra,
});

const tick = (a: Asentamiento) =>
  avanzarConstruccion(a, [], crearMapaDeterminista(7), undefined, reclamosDeFuentes([a]), instanteDeTest(1), 0, TODAS_LAS_TECNOLOGIAS).asentamiento;

describe('mantenimiento de las piezas gratuitas', () => {
  it('los puestos del Mercado y las anclas cuentan `pesoPiezaGratuita` de un edificio', () => {
    const base = plaza();
    const extra = (tipo: Edificio['tipo']) =>
      calcularCostoMantenimiento({ ...base, edificios: [...base.edificios, ...Array.from({ length: 10 }, (_, i) => edificio(`x-${i}`, tipo))] }, base).madera! -
      calcularCostoMantenimiento(base, base).madera!;
    for (const tipo of MANTENIMIENTO.piezasGratuitas) expect(extra(tipo)).toBeCloseTo(extra('almacen') * MANTENIMIENTO.pesoPiezaGratuita);
  });
});

describe('almacén lleno', () => {
  it('una receta no consume insumos que no se van a poder guardar', () => {
    const a = plaza();
    const armeria = edificio('armeria-1', 'armeria', { nivelInterno: 1 });
    const lleno: Asentamiento = {
      ...a,
      edificios: [...a.edificios, armeria],
      poblacion: { pesants: 100, artesanos: 50, nobleza: 0 },
      almacen: { ...a.almacen, piedra: { cantidad: 0, capacidad: 10_000 }, madera: { cantidad: 1000, capacidad: 10_000 }, armaMadera: { cantidad: 100, capacidad: 100 } },
      autoConstruccionPausada: true,
    };
    const tras = tick(lleno);
    expect(tras.almacen['armaMadera']!.cantidad).toBe(100);
    expect(tras.almacen['madera']!.cantidad).toBe(1000);
    expect(tras.edificios.find((e) => e.id === 'armeria-1')!.pausadoPorAlmacenLleno).toBe(true);
  });

  it('un extractor con el almacén lleno no gasta el yacimiento', () => {
    const mapa = crearMapaDeterminista(7);
    const a = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
    const nodo = mapa.nodosEnRadio(a.posicion, 400, { tipo: 'piedra', conStock: true })[0]!;
    expect(nodo, 'el mundo de prueba tiene piedra al alcance').toBeDefined();
    const cantera = edificio('cantera-1', 'cantera', { fuenteId: nodo.id, ambito: 'mapa', posicion: nodo.posicion });
    const lleno: Asentamiento = {
      ...a,
      edificios: [...a.edificios, cantera],
      poblacion: { pesants: 500, artesanos: 0, nobleza: 0 },
      almacen: { ...a.almacen, piedra: { cantidad: 400, capacidad: 400 } },
      autoConstruccionPausada: true,
    };
    const antes = mapa.stock(nodo.id);
    avanzarConstruccion(lleno, [], mapa, undefined, reclamosDeFuentes([lleno]), instanteDeTest(1), 0, TODAS_LAS_TECNOLOGIAS);
    expect(mapa.stock(nodo.id)).toBe(antes);
  });
});

describe('Gran Fundición', () => {
  it('cuenta en la mano de obra de artesanos', () => {
    const a = plaza();
    const con: Asentamiento = { ...a, edificios: [...a.edificios, edificio('gf-1', 'granFundicion')], poblacion: { pesants: 100, artesanos: 6, nobleza: 0 } };
    expect(ratioManoObraArtesanos(con)).toBeCloseTo(6 / 12);
  });
});

describe('Maravilla', () => {
  it('no se puede encolar mientras el servidor no esté en la Era V', () => {
    const a = plaza();
    const faccion = crearFacciones()[0]!;
    const mapa = crearMapaDeterminista(7);
    const conRecursos: Asentamiento = {
      ...a,
      nivel: 5,
      nivelActual: 5,
      cargos: { ...a.cargos, gobernadorId: 'g' },
      almacen: Object.fromEntries(Object.entries(a.almacen).map(([r, v]) => [r, { ...v, cantidad: 9000, capacidad: 9000 }])),
    };
    const construir = (era?: number) =>
      anadirEdificioManualmente(conRecursos, faccion, 'gobernador', 'maravilla', [], mapa, undefined, reclamosDeFuentes([conRecursos]), TODAS_LAS_TECNOLOGIAS, 0, 0, false, era);
    expect(() => construir(3)).toThrow(/Era/);
  });
});
