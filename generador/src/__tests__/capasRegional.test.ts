import { describe, expect, it, beforeAll } from 'vitest';
import { generarRelieve } from '../relieve';
import { fertilidadSuelo, crearSuelosRegional } from '../suelosRegional';
import { evaluarElevacion, evaluarTerreno } from '../elevacion';
import { densidadForestal } from '../forestacionRegional';
import { puntoEnLago } from '../aguaRegional';
import { anchoDeCaudal, generarHidrografiaRegional, suavizarCauceRegional } from '../hidrografiaRegional';
import { crearRelieveRegional } from '../regional';
import type { CampoElevacion } from '../elevacion';

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
  beforeAll(() => { mapa = generarRelieve(42, 32000, 24000); }, 180000);
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
  it('el caudal no baja aguas abajo, suma en las confluencias y el ancho sigue a su raíz', () => {
    const inicio = new Map<string, number>(), llegan = new Map<string, number>();
    const clave = (p: { x: number; y: number }) => `${p.x},${p.y}`;
    for (const rio of mapa.rios) {
      expect(rio.caudal.length).toBe(rio.puntos.length);
      // el ancho sale del caudal; el estuario solo lo ensancha
      rio.anchos.forEach((a, i) => expect(a).toBeGreaterThanOrEqual(anchoDeCaudal(rio.caudal[i]!) - 1e-9));
      for (let i = 1; i < rio.caudal.length; i++) expect(rio.caudal[i]!).toBeGreaterThanOrEqual(rio.caudal[i - 1]!);
      // las trenzas son brazos del mismo río (salen de él y vuelven), no afluentes: no cuentan en las confluencias
      if (rio.id.includes('-trenza-')) continue;
      inicio.set(clave(rio.puntos[0]!), rio.caudal[0]!);
      const fin = clave(rio.puntos.at(-1)!);
      llegan.set(fin, (llegan.get(fin) ?? 0) + rio.caudal.at(-1)!);
    }
    // En una confluencia, el tramo que sale lleva al menos lo que traen los que entran (el afluente llega con lo suyo).
    for (const [p, q] of llegan) if (inicio.has(p)) expect(inicio.get(p)! + 1e-6).toBeGreaterThanOrEqual(q);
  });
  it('fase 5: las trenzas salen de un río y vuelven a él, los brazos muertos son agua y la erosión no toca los cauces', () => {
    const trenzas = mapa.rios.filter(r => r.id.includes('-trenza-')), clave = (p: { x: number; y: number }) => `${p.x},${p.y}`;
    expect(trenzas.length).toBeGreaterThan(0);
    for (const t of trenzas) {
      const madre = mapa.rios.find(r => t.id.startsWith(`${r.id}-trenza-`))!;
      const enMadre = new Set(madre.puntos.map(clave));
      expect(enMadre.has(clave(t.puntos[0]!))).toBe(true);
      expect(enMadre.has(clave(t.puntos.at(-1)!))).toBe(true);
    }
    // los brazos muertos, donde los haya, son agua (que salgan depende de que un cuello se cierre: lo mira la prueba siguiente)
    const muertos = mapa.lagos.filter(l => l.id.startsWith('lago-brazo-'));
    for (const l of muertos) expect(evaluarTerreno(mapa.elevacion, l.centro)).toBe('agua');
    const sinErosion = { ...mapa.elevacion, erosion: undefined };
    expect(mapa.elevacion.erosion).toBeDefined();
    // (la erosión se apaga a más de la diagonal de su celda de la orilla: nada, salvo redondeo de coma flotante)
    for (const r of mapa.rios) for (const p of r.puntos) expect(Math.abs(evaluarElevacion(mapa.elevacion, p) - evaluarElevacion(sinErosion, p))).toBeLessThan(1e-6);
  });
  it('fase 5: los meandros que migran dejan brazos muertos (en alguna de tres semillas)', () => {
    let muertos = 0;
    for (const semilla of [1, 7, 42]) {
      const campo: CampoElevacion = { regional: crearRelieveRegional(semilla, { ancho: 32004, alto: 24000 }) };
      muertos += generarHidrografiaRegional(semilla, campo).lagos.filter(l => l.id.startsWith('lago-brazo-')).length;
    }
    expect(muertos).toBeGreaterThan(0);
  }, 180000);
  it('usa la orilla natural para agua, altura y exclusión forestal', () => {
    expect(mapa.lagos.length).toBeGreaterThan(0);
    for (const lago of mapa.lagos) {
      expect(lago.contorno!.length).toBeGreaterThan(20);
      expect(puntoEnLago(lago, lago.centro)).toBe(true);
      expect(evaluarTerreno(mapa.elevacion, lago.centro)).toBe('agua');
      expect(evaluarElevacion(mapa.elevacion, lago.centro)).toBe(lago.elevacionCentro);
      expect(densidadForestal(mapa.forestacion, mapa.elevacion, lago.centro)).toBe(0);
      expect(fertilidadSuelo(mapa.suelos, mapa.elevacion, lago.centro)).toBe(0);
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
      const fertilidad = fertilidadSuelo(mapa.suelos, mapa.elevacion, p);
      expect(fertilidadSuelo(copia, mapa.elevacion, p)).toEqual(fertilidad);
      expect(fertilidad).toBeGreaterThanOrEqual(0); expect(fertilidad).toBeLessThanOrEqual(1);
    }
  });
});
