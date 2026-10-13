import { describe, expect, it } from 'vitest';
import { crearCampoForestal, evaluarForestal, resumirSectoresForestales } from '../forestacionRegional';
import { crearRelieveRegional } from '../regional';
import { generarCampoElevacion } from '../elevacion';
import { createRng } from '../rng';
import type { RioZona } from '../tipos';

describe('forestación regional continua y permanente', () => {
  const config = { ancho: 32000, alto: 24000 };
  const elevacion = generarCampoElevacion(createRng(42), config);
  elevacion.regional = crearRelieveRegional(42, config);
  it('reproduce la seed y sobrevive a serialización sin depender de una malla de dibujo', () => {
    const campo = crearCampoForestal(42, []);
    expect(crearCampoForestal(42, [])).toEqual(campo);
    expect(crearCampoForestal(7, [])).not.toEqual(campo);
    const copia = JSON.parse(JSON.stringify(campo));
    let claros = 0, arbolados = 0, maxSalto = 0;
    for (let y = 1000; y < 24000; y += 1200) for (let x = 1000; x < 32000; x += 1200) {
      const p = { x, y }, f = evaluarForestal(campo, elevacion, p);
      expect(evaluarForestal(copia, elevacion, p)).toEqual(f);
      expect(f.densidad).toBeGreaterThanOrEqual(0); expect(f.densidad).toBeLessThanOrEqual(1);
      expect(f.costeMovimiento).toBeGreaterThanOrEqual(1); expect(f.costeMovimiento).toBeLessThanOrEqual(1.8);
      if (f.densidad >= 0.15) arbolados++; else claros++;
      maxSalto = Math.max(maxSalto, Math.abs(f.densidad - evaluarForestal(campo, elevacion, { x: x + 1, y }).densidad));
    }
    expect(claros).toBeGreaterThan(50); expect(arbolados).toBeGreaterThan(50);
    expect(maxSalto).toBeLessThan(0.03);
  });
  it('excluye agua y suaviza la vegetación en escarpes; una ribera favorece cobertura', () => {
    const rio = { puntos: [{ x: 1000, y: 0 }, { x: 1000, y: 5000 }] } as RioZona;
    const campo = crearCampoForestal(42, [rio]);
    const seco = crearCampoForestal(42, []), p = { x: 1000, y: 2000 };
    const muestra = { altura: 0.46, pendiente: 0.02, humedad: 0.4 };
    const ribera = evaluarForestal(campo, elevacion, p, muestra);
    expect(ribera.tipo).toBe('ribera');
    expect(ribera.densidad).toBeGreaterThan(evaluarForestal(seco, elevacion, p, muestra).densidad);
    expect(evaluarForestal(campo, elevacion, p, { ...muestra, altura: 0.37 }).densidad).toBe(0);
    expect(evaluarForestal(campo, elevacion, p, { ...muestra, pendiente: 0.7 }).densidad).toBe(0);
  });
  it('los sectores son únicos, no duplican superficie y sus resúmenes no consumen madera', () => {
    const campo = crearCampoForestal(42, []);
    const antes = JSON.stringify(campo);
    const sectores = resumirSectoresForestales(campo, elevacion, 2100, 1300);
    expect(new Set(sectores.map(s => s.id)).size).toBe(sectores.length);
    for (const s of sectores) {
      expect(s.x + s.ancho).toBeLessThanOrEqual(2100); expect(s.y + s.alto).toBeLessThanOrEqual(1300);
      expect(s.superficieEquivalente).toBeLessThanOrEqual(s.superficieArbolada);
      expect(s.superficieArbolada).toBeLessThanOrEqual(s.ancho * s.alto);
    }
    expect(resumirSectoresForestales(campo, elevacion, 2100, 1300)).toEqual(sectores);
    expect(JSON.stringify(campo)).toBe(antes);
  });
});
