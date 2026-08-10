// Integración: peaje de chokepoints cobrado de verdad al llegar una caravana comercial (Fase 0.3, Doc 1.5),
// vía `avanzarComercio` (`engine/trade.ts`) — a diferencia de `chokepoints.test.ts` (funciones puras de
// detección), esto verifica que el oro cambia de manos en el sitio correcto del flujo de llegada.
//
// `Mapa` sintético (mismo patrón que `world/__tests__/rutas.test.ts`): solo se stubean los dos métodos que
// esta ruta de código toca (`costeEnPunto`, `listarChokepoints`), no hace falta un mundo generado real.

import { describe, expect, it } from 'vitest';
import type { Asentamiento, Chokepoint, Faccion, Point, RecursoAlmacenado, ZonaInfluencia } from '../../domain/types';
import type { Caravana } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { CHOKEPOINTS_PEAJE } from '../../constants';
import { avanzarComercio } from '../trade';

function almacenCon(oro: number): Record<string, RecursoAlmacenado> {
  return { oro: { cantidad: oro, capacidad: 100000 } };
}

function asentamiento(id: string, faccionId: string, posicion: Point, oro: number): Asentamiento {
  return { id, faccionId, posicion, almacen: almacenCon(oro), politicasActivas: [] } as unknown as Asentamiento;
}

function mapaConChokepoint(chokepoints: Chokepoint[]): Mapa {
  return { costeEnPunto: () => 1, listarChokepoints: () => chokepoints } as unknown as Mapa;
}

describe('peaje de chokepoints al llegar una caravana comercial', () => {
  const origen = asentamiento('origen', 'faccion-origen', { x: 0, y: 0 }, 0);
  const destino = asentamiento('destino', 'faccion-destino', { x: 1000, y: 0 }, 100);
  const controlador = asentamiento('controlador-rival', 'faccion-rival', { x: 500, y: 0 }, 0);

  const chokepoint: Chokepoint = { id: 'cp1', posicion: { x: 500, y: 0 }, radio: 50 };
  const zonaDelRival: ZonaInfluencia = {
    asentamientoId: 'controlador-rival',
    poligono: [
      { x: 470, y: -30 },
      { x: 530, y: -30 },
      { x: 530, y: 30 },
      { x: 470, y: 30 },
    ],
  };

  // progreso casi 1: cualquier velocidad positiva basta para completar el tramo que falta y disparar la
  // llegada este mismo tick (ver `avanzarPosicionEnRuta`) — evita depender del valor exacto de
  // `CARAVANA_CATALOGO.comercial.velocidad`.
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

  it('cobra al destino y abona al asentamiento que controla el chokepoint que la ruta atraviesa', () => {
    const mapa = mapaConChokepoint([chokepoint]);
    const resultado = avanzarComercio(
      [origen, destino, controlador],
      [] as Faccion[],
      [caravanaCasiLlegando()],
      [],
      mapa,
      [],
      [zonaDelRival],
      1
    );

    const destinoFinal = resultado.asentamientos.find((a) => a.id === 'destino')!;
    const controladorFinal = resultado.asentamientos.find((a) => a.id === 'controlador-rival')!;

    expect(destinoFinal.almacen['oro']!.cantidad).toBe(100 - CHOKEPOINTS_PEAJE.oro);
    expect(controladorFinal.almacen['oro']!.cantidad).toBe(CHOKEPOINTS_PEAJE.oro);
    expect(resultado.eventos.some((e) => e.includes('Peaje') && e.includes('controlador-rival'))).toBe(true);
  });

  it('NO cobra peaje si nadie controla el chokepoint (sin zona que lo cubra)', () => {
    const mapa = mapaConChokepoint([chokepoint]);
    const resultado = avanzarComercio([origen, destino, controlador], [] as Faccion[], [caravanaCasiLlegando()], [], mapa, [], [], 1);

    const destinoFinal = resultado.asentamientos.find((a) => a.id === 'destino')!;
    expect(destinoFinal.almacen['oro']!.cantidad).toBe(100);
    expect(resultado.eventos.some((e) => e.includes('Peaje'))).toBe(false);
  });

  it('NO cobra peaje si el chokepoint controlado queda lejos de la ruta (fuera de su radio)', () => {
    const chokepointLejano: Chokepoint = { id: 'cp-lejos', posicion: { x: 500, y: 500 }, radio: 50 };
    const zonaLejana: ZonaInfluencia = {
      asentamientoId: 'controlador-rival',
      poligono: [
        { x: 470, y: 470 },
        { x: 530, y: 470 },
        { x: 530, y: 530 },
        { x: 470, y: 530 },
      ],
    };
    const mapa = mapaConChokepoint([chokepointLejano]);
    const resultado = avanzarComercio(
      [origen, destino, controlador],
      [] as Faccion[],
      [caravanaCasiLlegando()],
      [],
      mapa,
      [],
      [zonaLejana],
      1
    );

    const destinoFinal = resultado.asentamientos.find((a) => a.id === 'destino')!;
    expect(destinoFinal.almacen['oro']!.cantidad).toBe(100);
  });
});
