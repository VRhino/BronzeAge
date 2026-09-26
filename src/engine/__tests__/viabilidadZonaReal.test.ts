// La viabilidad de un sitio mira la zona REAL con la que nacería (círculo recortado contra los rivales), no el
// círculo crudo (engine/settlement.ts, 2026-09-26). El batch lo destapó: 26 de 28 asentamientos que murieron por
// madera tenían un bosque dentro del círculo al fundar y ninguno dentro de su zona recortada.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Point } from '../../domain/types';
import { ZONA_INFLUENCIA } from '../../constants';
import { evaluarViabilidadFundacion } from '../settlement';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const R = ZONA_INFLUENCIA.radioInicial;

describe('viabilidad de fundación: la madera se mira en la zona real', () => {
  it('un bosque que solo toca el círculo, y que un rival deja fuera del recorte, ya no cuenta', () => {
    const mapa = crearMapaDeterminista(7);
    const bosques = mapa.listarBosques();
    // Un sitio al este de un bosque, con el círculo inicial entrando 5 unidades en él, y ningún otro bosque a mano:
    // así el único bosque que puede decidir el resultado es ese.
    const caso = bosques
      .map((b) => ({ b, p: { x: b.centro.x + b.radio + R - 5, y: b.centro.y } as Point }))
      .find(({ p }) => bosques.filter((o) => Math.hypot(o.centro.x - p.x, o.centro.y - p.y) < R + o.radio).length === 1);
    expect(caso, 'la semilla 7 tiene que dar al menos un bosque aislado').toBeDefined();
    const { p } = caso!;

    // Un vecino al oeste, del lado del bosque, con radio 60 a 70 unidades: su frontera cae a 70 × 30/90 ≈ 23 de p,
    // antes del borde del bosque (a 25). p queda fuera de su círculo, así que el sitio sigue siendo libre.
    const base = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento;
    const vecino = (faccionId: string): Asentamiento => ({
      ...base,
      id: 'vecino',
      faccionId,
      posicion: { x: p.x - 70, y: p.y },
      radioPotencial: 60,
      edificios: [],
    });

    expect(evaluarViabilidadFundacion(mapa, p, [], 'faccion-1').bosqueLibreAlcanzable, 'sin vecinos, el bosque cuenta').toBe(true);
    expect(evaluarViabilidadFundacion(mapa, p, [vecino('faccion-2')], 'faccion-1').bosqueLibreAlcanzable, 'un rival lo recorta').toBe(false);
    // Un asentamiento de la MISMA Facción no recorta (Doc 1.2): el bosque vuelve a contar.
    expect(evaluarViabilidadFundacion(mapa, p, [vecino('faccion-1')], 'faccion-1').bosqueLibreAlcanzable).toBe(true);
    // Sin saber quién funda, todo vecino se trata como rival.
    expect(evaluarViabilidadFundacion(mapa, p, [vecino('faccion-1')]).bosqueLibreAlcanzable).toBe(false);
  });
});
