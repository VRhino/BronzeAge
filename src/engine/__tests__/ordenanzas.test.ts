// Cada ordenanza de trazado del Maestro de Obras lleva un beneficio propio (Doc 4.4, 2026-10-02): sin él competían en
// desventaja contra Vía Rápida. Aquí solo que cada una mueve su factor y ninguna toca el de las demás.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { POLITICA_CATALOGO } from '../../constants';
import { factorComisionExterna, factorCrecimientoPoblacion, factorProduccionTalleres, factorTiempoMuralla } from '../politicas';

const con = (...ids: string[]) => ({ politicasActivas: ids.map((politicaId) => ({ politicaId })) }) as unknown as Asentamiento;

describe('ordenanzas de trazado', () => {
  it('cada una mueve su propio factor', () => {
    expect(factorTiempoMuralla(con('postura_defensiva'))).toBeLessThan(1);
    expect(factorComisionExterna(con('arterias_comerciales'))).toBeLessThan(1);
    expect(factorProduccionTalleres(con('barrios_gremiales'))).toBeGreaterThan(1);
    expect(factorCrecimientoPoblacion(con('plazas_mayores'))).toBeGreaterThan(1);
  });

  it('sin ordenanza activa todos valen 1, y una no toca los factores de las otras', () => {
    const ninguna = con();
    expect([factorTiempoMuralla(ninguna), factorComisionExterna(ninguna), factorProduccionTalleres(ninguna), factorCrecimientoPoblacion(ninguna)]).toEqual([1, 1, 1, 1]);
    const defensiva = con('postura_defensiva');
    expect([factorComisionExterna(defensiva), factorProduccionTalleres(defensiva), factorCrecimientoPoblacion(defensiva)]).toEqual([1, 1, 1]);
  });

  it('las cuatro ordenanzas del catálogo tienen un efecto', () => {
    const efectos = POLITICA_CATALOGO.filter((p) => 'perfilTrazado' in p).map((p) => Object.keys(p).filter((k) => k.startsWith('factor')));
    expect(efectos).toHaveLength(4);
    for (const e of efectos) expect(e.length).toBeGreaterThan(0);
  });
});
