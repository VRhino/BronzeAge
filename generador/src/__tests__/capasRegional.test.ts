import { describe, expect, it, beforeAll } from 'vitest';
import { generarRegionalVisor } from '../../scripts/visor-worldgen.regional';
import { evaluarSuelo, crearSuelosRegional, generarRecursosRegional } from '../suelosRegional';
import { evaluarElevacion, evaluarTerreno } from '../elevacion';
import { evaluarForestal } from '../forestacionRegional';
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
  let mapa: ReturnType<typeof generarRegionalVisor>;
  beforeAll(() => { mapa = generarRegionalVisor(42, 32000, 24000); }, 60000);
  it('une afluentes sin duplicar aristas y conserva una red acíclica', () => {
    const aristas = new Set<string>(), rios = new Map(mapa.rios.map(r => [r.id, r]));
    expect(mapa.rios.some(r => r.desembocaEn)).toBe(true);
    expect(mapa.rios.some(r => r.longitud > 6000)).toBe(true);
    for (const rio of mapa.rios) {
      expect(rio.ancho).toBeGreaterThan(0);
      for (let i = 1; i < rio.puntos.length; i++) {
        const clave = JSON.stringify([rio.puntos[i - 1], rio.puntos[i]]);
        expect(aristas.has(clave)).toBe(false); aristas.add(clave);
      }
      const visitados = new Set<string>(); let actual = rio;
      while (actual.desembocaEn) {
        expect(visitados.has(actual.id)).toBe(false); visitados.add(actual.id);
        const siguiente = rios.get(actual.desembocaEn)!;
        expect(siguiente).toBeDefined();
        expect(actual.puntos.at(-1)).toEqual(siguiente.puntos[0]);
        expect(siguiente.cuenca).toBe(actual.cuenca);
        expect(siguiente.orden).toBeGreaterThanOrEqual(actual.orden);
        actual = siguiente;
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
      expect(evaluarForestal(mapa.forestacion, mapa.elevacion, lago.centro).densidad).toBe(0);
      expect(evaluarSuelo(mapa.suelos, mapa.elevacion, lago.centro).fertilidad).toBe(0);
      const distancias = lago.contorno!.map(p => Math.hypot(p.x - lago.centro.x, p.y - lago.centro.y));
      expect(Math.max(...distancias) - Math.min(...distancias)).toBeGreaterThan(10);
    }
  });
  it('suelos serializables, recursos deterministas y sin candidatos aislados publicados', () => {
    expect(crearSuelosRegional(42, mapa.rios)).toEqual(mapa.suelos);
    const candidatos = generarRecursosRegional(42, mapa.suelos, mapa.elevacion, mapa.forestacion, 4, mapa.conexion.tieneAcceso);
    expect(candidatos.filter(r => !mapa.estrategia.recursosAislados.includes(r.id))).toEqual(mapa.recursos);
    expect(new Set(mapa.recursos.map(r => r.id)).size).toBe(mapa.recursos.length);
    expect(new Set(mapa.recursos.map(r => r.tipo)).size).toBe(5);
    for (const r of mapa.recursos) {
      expect(evaluarTerreno(mapa.elevacion, r.posicion)).not.toBe('agua');
      const bosque = evaluarForestal(mapa.forestacion, mapa.elevacion, r.posicion).densidad;
      const suelo = evaluarSuelo(mapa.suelos, mapa.elevacion, r.posicion, bosque);
      expect(evaluarSuelo(JSON.parse(JSON.stringify(mapa.suelos)), mapa.elevacion, r.posicion, bosque)).toEqual(suelo);
      expect(suelo.fertilidad).toBeGreaterThanOrEqual(0); expect(suelo.fertilidad).toBeLessThanOrEqual(1);
      expect(suelo.pastos).toBeGreaterThanOrEqual(0); expect(suelo.pastos).toBeLessThanOrEqual(1);
    }
    for (const paso of mapa.estrategia.pasos) if (paso.factor !== null) expect(paso.factor).toBeGreaterThanOrEqual(1 - 1e-8);
    expect(mapa.estrategia.pasos[9]!.recorrido).not.toBeNull();
  });
  it('la densidad conserva las cuotas de estaño y las fuentes no metálicas anteriores', () => {
    const anterior = generarRecursosRegional(42, mapa.suelos, mapa.elevacion, mapa.forestacion, 1, mapa.conexion.tieneAcceso);
    const actual = generarRecursosRegional(42, mapa.suelos, mapa.elevacion, mapa.forestacion, 4, mapa.conexion.tieneAcceso);
    expect(actual.filter(r => r.tipo === 'estano')).toHaveLength(56);
    expect(anterior.filter(r => r.tipo === 'estano')).toHaveLength(56);
    for (const r of anterior.filter(r=>r.tipo!=='cobre'&&r.tipo!=='estano'&&!r.id.startsWith('piedra-enclave-'))) expect(actual.find(a => a.id === r.id)).toEqual(r);
    expect(actual.filter(r => r.tipo === 'piedra').length).toBeGreaterThanOrEqual(560);
    expect(actual.filter(r => r.tipo === 'cobre').length).toBe(240);
    expect(new Set(actual.map(r => r.id)).size).toBe(actual.length);
    expect(() => generarRecursosRegional(42, mapa.suelos, mapa.elevacion, mapa.forestacion, 3)).toThrow();
  });
});
