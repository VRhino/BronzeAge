import { describe, expect, it, beforeAll } from 'vitest';
import { generarRelieve } from '../relieve';
import { evaluarSuelo, crearSuelosRegional } from '../suelosRegional';
import { evaluarElevacion, evaluarTerreno } from '../elevacion';
import { densidadForestal } from '../forestacionRegional';
import { puntoEnLago } from '../aguaRegional';
import { suavizarCauceRegional } from '../hidrografiaRegional';

describe('capas regionales 6', () => {
  it('suaviza curvas manteniendo los extremos y el descenso, sin mutar el trazado original', () => {
    const puntos = [{ x: 0, y: 0 }, { x: 100, y: 100 }, { x: 0, y: 200 }, { x: 100, y: 300 }, { x: 0, y: 400 }];
    const antes = JSON.stringify(puntos);
    const salida = suavizarCauceRegional(puntos, p => 1000 - p.y);
    expect(salida[0]).toEqual(puntos[0]); expect(salida.at(-1)).toEqual(puntos.at(-1));
    expect(JSON.stringify(puntos)).toBe(antes);
    const largo = (ps: typeof puntos) => ps.slice(1).reduce((s, p, i) => s + Math.hypot(p.x - ps[i]!.x, p.y - ps[i]!.y), 0);
    expect(largo(salida)).toBeLessThan(largo(puntos));
    for (let i = 1; i < salida.length; i++) expect(salida[i]!.y).toBeGreaterThan(salida[i - 1]!.y);
  });
  let mapa: ReturnType<typeof generarRelieve>;
  beforeAll(() => { mapa = generarRelieve(42, 32000, 24000); }, 60000);
  it('une afluentes sin duplicar aristas', () => {
    const aristas = new Set<string>();
    for (const rio of mapa.rios) {
      expect(rio.puntos.length).toBeGreaterThanOrEqual(2);
      expect(rio.orden).toBeGreaterThanOrEqual(1);
      for (let i = 1; i < rio.puntos.length; i++) {
        const clave = JSON.stringify([rio.puntos[i - 1], rio.puntos[i]]);
        expect(aristas.has(clave)).toBe(false); aristas.add(clave);
      }
    }
  });
  it('usa la orilla natural para agua, altura y exclusión forestal', () => {
    expect(mapa.lagos.length).toBeGreaterThan(0);
    for (const lago of mapa.lagos) {
      expect(lago.contorno!.length).toBeGreaterThan(20);
      expect(puntoEnLago(lago, lago.centro)).toBe(true);
      expect(evaluarTerreno(mapa.elevacion, lago.centro)).toBe('agua');
      expect(evaluarElevacion(mapa.elevacion, lago.centro)).toBe(lago.elevacionCentro);
      expect(densidadForestal(mapa.forestacion, mapa.elevacion, lago.centro)).toBe(0);
      expect(evaluarSuelo(mapa.suelos, mapa.elevacion, lago.centro).fertilidad).toBe(0);
      const distancias = lago.contorno!.map(p => Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y));
      expect(Math.max(...distancias) - Math.min(...distancias)).toBeGreaterThan(10);
    }
  });
  it('suelos serializables y deterministas', () => {
    expect(crearSuelosRegional(42, mapa.rios)).toEqual(mapa.suelos);
    const copia = JSON.parse(JSON.stringify(mapa.suelos));
    const tierra: { x: number; y: number }[] = [];
    for (let y = 400; y < 24000; y += 800) for (let x = 400; x < 32000; x += 800) {
      const p = { x, y };
      if (evaluarTerreno(mapa.elevacion, p) !== 'agua') tierra.push(p);
    }
    expect(tierra.length).toBeGreaterThanOrEqual(200);
    for (const p of tierra.slice(0, 200)) {
      const bosque = densidadForestal(mapa.forestacion, mapa.elevacion, p);
      const suelo = evaluarSuelo(mapa.suelos, mapa.elevacion, p, bosque);
      expect(evaluarSuelo(copia, mapa.elevacion, p, bosque)).toEqual(suelo);
      expect(suelo.fertilidad).toBeGreaterThanOrEqual(0); expect(suelo.fertilidad).toBeLessThanOrEqual(1);
      expect(suelo.pastos).toBeGreaterThanOrEqual(0); expect(suelo.pastos).toBeLessThanOrEqual(1);
    }
  });
});
