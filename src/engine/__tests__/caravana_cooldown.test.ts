// Cooldown de creación de caravanas (a petición del usuario): tras crear una caravana (Fundación o comercial)
// desde un asentamiento, hay que esperar `CARAVANA_COOLDOWN.ticksCooldown` ticks antes de poder crear otra
// desde el mismo asentamiento — evita spam de creación cuando una caravana recién salida es destruida
// (bandidos, intercepción) y el cupo/recursos vuelven a estar disponibles de inmediato. Ver
// `Docs/3_Sistema_Economico_y_Comercio.md` y `engine/asentamientoQuery.ts` (`puedeCrearCaravana`).
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { CARAVANA_COOLDOWN } from '../../constants';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';
import { construirCaravanaComercial, CaravanaInvalidaError } from '../trade';
import { lanzarCaravanaFundacion, ExpansionInvalidaError } from '../expansion';

function asentamientoConMercado(): Asentamiento {
  const mapa = crearMapaDeterminista(1);
  const facciones = crearFacciones();
  const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  return {
    ...asentamiento,
    almacen: { ...asentamiento.almacen, madera: { cantidad: 1000, capacidad: 2000 } },
    edificios: [
      ...asentamiento.edificios,
      {
        id: 'mercado-test',
        tipo: 'mercado',
        posicion: { x: 100, y: 100 },
        estado: 'activo',
        ticksRestantes: 0,
        ambito: 'asentamiento',
      },
    ],
  };
}

describe('cooldown de creación de caravanas — caravana comercial (construirCaravanaComercial)', () => {
  it('rechaza crear una segunda caravana antes de que pase el cooldown', () => {
    const asentamiento = asentamientoConMercado();
    const r1 = construirCaravanaComercial(asentamiento, [], 0, 0);
    expect(() => construirCaravanaComercial(r1.asentamiento, [r1.caravana], 1, 1)).toThrow(CaravanaInvalidaError);
  });

  it('permite crear otra en cuanto pasa CARAVANA_COOLDOWN.ticksCooldown ticks', () => {
    const asentamiento = asentamientoConMercado();
    const r1 = construirCaravanaComercial(asentamiento, [], 0, 0);
    expect(() =>
      construirCaravanaComercial(r1.asentamiento, [r1.caravana], CARAVANA_COOLDOWN.ticksCooldown, 1)
    ).not.toThrow();
  });

  it('registra el tick de creación en ultimaCaravanaCreadaEnTick', () => {
    const asentamiento = asentamientoConMercado();
    const r1 = construirCaravanaComercial(asentamiento, [], 5, 0);
    expect(r1.asentamiento.ultimaCaravanaCreadaEnTick).toBe(5);
  });

  it('un asentamiento que nunca creó ninguna no está en cooldown', () => {
    const asentamiento = asentamientoConMercado();
    expect(() => construirCaravanaComercial(asentamiento, [], 0, 0)).not.toThrow();
  });
});

describe('cooldown de creación de caravanas — Caravana de Fundación (lanzarCaravanaFundacion)', () => {
  function contextoNivel2() {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 3 } : f));
    const { asentamiento, facciones: trasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const abundante: Asentamiento = {
      ...asentamiento,
      nivel: 2,
      nivelActual: 2,
      almacen: Object.fromEntries(
        Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: 5000, capacidad: 5000 }])
      ),
    };
    const faccion = trasFundar.find((f) => f.id === 'faccion-1')!;
    const destino = posicionRecomendable(mapa, [abundante]);
    return { mapa, asentamiento: abundante, faccion, destino };
  }

  it('rechaza lanzar una segunda Caravana de Fundación antes de que pase el cooldown', () => {
    const { mapa, asentamiento, faccion, destino } = contextoNivel2();
    const r1 = lanzarCaravanaFundacion(mapa, asentamiento, faccion, destino, [asentamiento], [], 1, 0, 0);
    expect(() =>
      lanzarCaravanaFundacion(mapa, r1.origenActualizado, faccion, destino, [asentamiento], [r1.caravana], 1, 1, 1)
    ).toThrow(ExpansionInvalidaError);
  });

  it('permite lanzar otra en cuanto pasa CARAVANA_COOLDOWN.ticksCooldown ticks', () => {
    const { mapa, asentamiento, faccion, destino } = contextoNivel2();
    const r1 = lanzarCaravanaFundacion(mapa, asentamiento, faccion, destino, [asentamiento], [], 1, 0, 0);
    expect(() =>
      lanzarCaravanaFundacion(
        mapa,
        r1.origenActualizado,
        faccion,
        destino,
        [asentamiento],
        [r1.caravana],
        1,
        CARAVANA_COOLDOWN.ticksCooldown,
        1
      )
    ).not.toThrow();
  });
});
