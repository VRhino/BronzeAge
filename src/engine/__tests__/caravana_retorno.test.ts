// Regresión (a petición del usuario, ver Correcciones): una caravana comercial que entrega en `destino` ya
// NO se teletransporta de vuelta a `origen` — debe recorrer la ruta de vuelta tick a tick, igual que fue.
// Antes, `avanzarCaravanas` (engine/trade.ts) la reseteaba a `estado: 'disponible'` con
// `posicionActual: origen.posicion` en el MISMO tick de la entrega.

import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion, Point, RecursoAlmacenado } from '../../domain/types';
import type { Caravana } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { avanzarComercio } from '../trade';

function almacenCon(oro: number): Record<string, RecursoAlmacenado> {
  return { oro: { cantidad: oro, capacidad: 100000 } };
}

function asentamiento(id: string, posicion: Point): Asentamiento {
  return { id, faccionId: 'faccion-1', posicion, almacen: almacenCon(0), politicasActivas: [] } as unknown as Asentamiento;
}

const origen = asentamiento('origen', { x: 0, y: 0 });
const destino = asentamiento('destino', { x: 1000, y: 0 });
const mapaLlano: Mapa = { costeEnPunto: () => 1, listarChokepoints: () => [] } as unknown as Mapa;

function caravanaCasiLlegando(): Caravana {
  return {
    id: 'caravana-1',
    tipo: 'comercial',
    origenAsentamientoId: 'origen',
    destinoAsentamientoId: 'destino',
    contenido: {},
    posicionActual: { x: 999, y: 0 },
    progreso: 0.9999999,
    estado: 'en_transito',
    ruta: [origen.posicion, destino.posicion],
  };
}

function avanzar(caravanas: Caravana[]) {
  return avanzarComercio([origen, destino], [] as Faccion[], caravanas, [], mapaLlano, [], [], 1);
}

describe('retorno real de una caravana comercial tras entregar', () => {
  it('al entregar pasa a "retornando" en destino, NO a "disponible" en origen', () => {
    const resultado = avanzar([caravanaCasiLlegando()]);
    const caravana = resultado.caravanas.find((c) => c.id === 'caravana-1')!;

    expect(caravana.estado).toBe('retornando');
    expect(caravana.posicionActual).toEqual(destino.posicion);
    expect(caravana.progreso).toBe(0);
  });

  it('en el siguiente tick avanza gradualmente hacia origen, sin saltar directo a él', () => {
    const trasEntrega = avanzar([caravanaCasiLlegando()]).caravanas;
    const trasUnTick = avanzar(trasEntrega).caravanas.find((c) => c.id === 'caravana-1')!;

    expect(trasUnTick.estado).toBe('retornando');
    expect(trasUnTick.posicionActual.x).toBeLessThan(destino.posicion.x);
    expect(trasUnTick.posicionActual.x).toBeGreaterThan(origen.posicion.x);
    expect(trasUnTick.progreso).toBeGreaterThan(0);
    expect(trasUnTick.progreso).toBeLessThan(1);
  });

  it('no puede reasignarse a un nuevo envío mientras está retornando (no cuenta como "disponible")', () => {
    const trasEntrega = avanzar([caravanaCasiLlegando()]).caravanas;
    const caravana = trasEntrega.find((c) => c.id === 'caravana-1')!;
    expect(caravana.estado).not.toBe('disponible');
  });

  it('tras completar el viaje de vuelta, llega de verdad a origen y solo entonces vuelve a "disponible"', () => {
    let caravanas = avanzar([caravanaCasiLlegando()]).caravanas;
    for (let i = 0; i < 200 && caravanas.find((c) => c.id === 'caravana-1')!.estado !== 'disponible'; i++) {
      caravanas = avanzar(caravanas).caravanas;
    }
    const caravanaFinal = caravanas.find((c) => c.id === 'caravana-1')!;

    expect(caravanaFinal.estado).toBe('disponible');
    expect(caravanaFinal.posicionActual).toEqual(origen.posicion);
  });
});
