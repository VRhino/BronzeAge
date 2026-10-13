import { describe, expect, it } from 'vitest';
import { generarRegionalVisor } from '../../scripts/visor-worldgen.regional';

describe('regresiones de abastecimiento regional', () => {
  for (const seed of [99, 2026]) it(`seed ${seed}: acceso continental y prospección coherente`, () => {
    const mapa = generarRegionalVisor(seed, 32000, 24000);
    expect(mapa.conexion.conectadas).toBe(16);
    expect(mapa.estrategia.areas.every(a => a.acceso)).toBe(true);
    // Seed 99 descartaba todos los recursos; seed 2026 quedaba con un área aislada.
    expect(mapa.recursos.length).toBeGreaterThan(200);
    const balance = mapa.estrategia.balance;
    const tinDepositos = mapa.recursos.filter(r => r.tipo === 'estano');
    expect(tinDepositos.filter(r => r.categoriaEstano === 'aislado')).toHaveLength(32);
    expect(tinDepositos.filter(r => r.categoriaEstano === 'cluster')).toHaveLength(24);
    expect(new Set(tinDepositos.filter(r => r.categoriaEstano === 'cluster').map(r => r.area)).size).toBe(4);
    expect(balance.estano.regionesConNodos).toBe(16);
    for (const area of mapa.areas) {
      expect(tinDepositos.filter(r => r.area === area.id && r.categoriaEstano === 'aislado')).toHaveLength(2);
      expect(mapa.recursos.some(r => r.tipo === 'cobre' && r.area === area.id)).toBe(true);
    }
    for (let i=0;i<tinDepositos.length;i++) for(let j=i+1;j<tinDepositos.length;j++) {
      const a=tinDepositos[i]!.posicion,b=tinDepositos[j]!.posicion;
      expect(Math.hypot(a.x-b.x,a.y-b.y)).toBeGreaterThanOrEqual(1100);
    }
    expect(balance.estano.regionesConAccesoMenor).toBe(16);
    expect(balance.estano.regionesConAlternativa).toBe(16);
    expect(balance.estano.clusters).toBe(4);
    expect(balance.estano.cuotaGrandes).toBeCloseTo(24/56);
    const copper = balance.concentracion.find(c=>c.tipo==='cobre')!;
    expect(copper.regiones).toBe(16);
    expect(copper.maxFraccion).toBeLessThanOrEqual(0.12);
    for (const area of mapa.estrategia.areas) {
      expect(area.estano.fuenteCercana).not.toBe(area.estano.fuenteAlternativa);
      expect(area.estano.alternativa).not.toBeNull();
    }
    expect(balance.regiones).toHaveLength(16);
    expect(balance.sectoresOportunidad).toBeGreaterThan(0);
    expect(balance.regiones.reduce((n, r) => n + r.sectoresOportunidad, 0)).toBe(balance.sectoresOportunidad);
    for (const region of balance.regiones) {
      expect(region.abastecidaKm2).toBeLessThanOrEqual(region.habitableKm2 + 1e-8);
      expect(region.habitableKm2).toBeLessThanOrEqual(region.superficieKm2 + 1e-8);
      const locales = mapa.recursos.filter(r => r.area === region.area);
      expect(Object.values(region.recursos).reduce((a, b) => a + b, 0)).toBe(locales.length);
    }
    const tin = balance.concentracion.find(c => c.tipo === 'estano')!;
    expect(tin.total).toBeGreaterThan(0);
    expect(tin.regiones).toBeGreaterThanOrEqual(3);
    expect(tin.maxFraccion).toBeLessThanOrEqual(0.5);
  }, 60000);
});
