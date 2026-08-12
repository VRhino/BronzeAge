// Regresión (bug real detectado con un save del usuario, tick 69): un Camino Comercial ya construido entre
// dos asentamientos se reutiliza para AMBAS direcciones de trueque (Doc 1.6, `asignarCaravanasATrueque`,
// engine/trade.ts). `buscarCamino` empareja el par en cualquier orden pero devuelve `puntos` siempre en el
// orden en que se guardó (`asentamientoAId` -> `asentamientoBId`, ver engine/caminos.ts) — si no se reorienta,
// una caravana que viaja en sentido CONTRARIO recibe una polilínea invertida: `progreso: 0` cae en el punto
// del camino más cercano al DESTINO, no al propio origen, y la caravana "salta" hasta allí en su primer paso
// de movimiento (ver `avanzarPosicionEnRuta`, engine/movimiento.ts, que siempre mide desde `ruta[0]`).

import { describe, expect, it } from 'vitest';
import type { Asentamiento, CaminoComercial, Faccion, Point, RecursoAlmacenado } from '../../domain/types';
import type { Mapa } from '../../world/mapa';
import { avanzarComercio, construirCaravanaComercial, proponerTrueque } from '../trade';

function almacen(recursos: Record<string, number>): Record<string, RecursoAlmacenado> {
  const out: Record<string, RecursoAlmacenado> = {};
  for (const [r, cantidad] of Object.entries(recursos)) out[r] = { cantidad, capacidad: 100000 };
  return out;
}

function asentamiento(id: string, posicion: Point, recursos: Record<string, number>): Asentamiento {
  return {
    id,
    faccionId: `faccion-${id}`,
    posicion,
    almacen: almacen(recursos),
    politicasActivas: [],
    edificios: [{ id: `mercado-${id}`, tipo: 'mercado', posicion, estado: 'activo', ticksRestantes: 0, nivelInterno: 1 }],
  } as unknown as Asentamiento;
}

const posA: Point = { x: 0, y: 0 };
const posB: Point = { x: 1000, y: 0 };
const mapaLlano: Mapa = { costeEnPunto: () => 1, listarChokepoints: () => [] } as unknown as Mapa;

// Camino ya construido A -> B (mismo patrón que un save real: la infraestructura persiste independientemente
// de qué lado la use después) — sus `puntos` están ordenados desde `posA` hacia `posB`.
const caminoAB: CaminoComercial = {
  id: 'camino-A-B',
  asentamientoAId: 'A',
  asentamientoBId: 'B',
  puntos: [posA, { x: 500, y: 0 }, posB],
};

describe('orientación de la ruta al reutilizar un Camino Comercial existente', () => {
  it('una caravana que viaja B -> A recibe la polilínea invertida, no la original A -> B', () => {
    const a = asentamiento('A', posA, { madera: 50, cobre: 200 });
    const b = asentamiento('B', posB, { madera: 50, oro: 1000 });

    // Cada asentamiento construye su propia caravana (mismo patrón que el save real: una por lado).
    const { asentamiento: aTrasConstruir, caravana: caravanaA } = construirCaravanaComercial(a, [], 0, 0);
    const { asentamiento: bTrasConstruir, caravana: caravanaB } = construirCaravanaComercial(b, [caravanaA], 0, 1);

    // Trueque A<->B: A entrega cobre a B (caravana de A viaja A->B, a favor del camino), B entrega oro a A
    // (caravana de B viaja B->A, EN CONTRA del orden guardado del camino) — el mismo patrón de dos trueques
    // opuestos por el mismo camino que expuso el bug en la partida real.
    const acuerdo = proponerTrueque([aTrasConstruir, bTrasConstruir], 'A', 'B', 'cobre', 'oro', 50, 20, 0, 0);

    const resultado = avanzarComercio(
      [aTrasConstruir, bTrasConstruir],
      [] as Faccion[],
      [caravanaA, caravanaB],
      [acuerdo],
      mapaLlano,
      [caminoAB],
      [],
      1
    );

    const cA = resultado.caravanas.find((c) => c.id === caravanaA.id)!;
    const cB = resultado.caravanas.find((c) => c.id === caravanaB.id)!;

    expect(cA.estado).toBe('en_transito');
    expect(cB.estado).toBe('en_transito');

    // Caravana de A (viaja A->B, a favor del camino): ruta empieza en su propio origen.
    expect(cA.ruta?.[0]).toEqual(posA);
    // Caravana de B (viaja B->A, en CONTRA del camino guardado): antes del fix, esto también daba `posA`
    // (el bug) — corregido, debe empezar en su propio origen, `posB`.
    expect(cB.ruta?.[0]).toEqual(posB);

    // Y por tanto, en el primer tick de movimiento, NINGUNA de las dos aparece ya en la posición de la otra.
    expect(cA.posicionActual).not.toEqual(cB.posicionActual);
    // La caravana de B se movió gradualmente desde su origen (B, x=1000), no saltó hacia x=0.
    expect(cB.posicionActual.x).toBeGreaterThan(900);
  });
});
