import { describe, expect, it } from 'vitest';
import type { Point } from '../tipos';
import { anilloEn, FACTOR, type Campamento } from '../campamentos';
import type { NodoMetal } from '../metales';
import type { ClaseTerreno } from '../mundoTipos';
import {
  cantidadInicial, metalesARecursos, PARAMETROS_RECURSOS as P, RANGO_CANTIDAD, repartirRecursosNoMetalicos, type MuestreoRecursos,
} from '../recursos';

const ANCHO = 2667 * FACTOR, ALTO = 2000 * FACTOR;
const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

// Diez campamentos en rejilla, sin geografía real: se comprueban las reglas, no el relieve.
const campamentos: Campamento[] = [];
for (const y of [6000, 18000]) for (let i = 0; i < 5; i++) campamentos.push({ id: `campamento-${campamentos.length}`, posicion: { x: 3500 + i * 6300, y } });

// Relieve sintético: lagos, cimas, un risco y una zona sin acceso, con colinas y estepa onduladas.
const LAGO = { x: 9000, y: 12000 }, CIMA = { x: 20000, y: 12000 }, RISCO = { x: 14000, y: 6000 }, ISLA = { x: 27000, y: 17000 };
const altura = (p: Point) => 0.5 + 0.3 * Math.sin(p.x / 3100) * Math.cos(p.y / 2700);
const terreno = (p: Point): ClaseTerreno => {
  if (dist(p, LAGO) < 1500) return 'agua';
  if (dist(p, CIMA) < 1500) return 'cima';
  const h = altura(p);
  return h > 0.68 ? 'montana' : h > 0.55 ? 'colina' : 'llano';
};
const sintetico: MuestreoRecursos = {
  terreno,
  pendiente: p => (dist(p, RISCO) < 1200 ? 0.5 : 0.1),
  accesible: p => dist(p, ISLA) >= 1800,
  altura,
  geologia: p => 0.5 + 0.4 * Math.sin(p.x / 2300 + p.y / 3700),
  bosque: p => (Math.sin(p.x / 1700) > 0 ? 0.4 : 0),
  suelo: p => ({ pastos: 0.6 + 0.3 * Math.cos(p.y / 1900), estepa: Math.sin(p.x / 4100 + 1) > 0 }),
};

const recursos = repartirRecursosNoMetalicos(42, campamentos, sintetico, { ancho: ANCHO, alto: ALTO });
// La capa de la tierra lejana (ids `<tipo>-l<n>`) va aparte: el resto es el mínimo garantizado de antes.
const esCapaLejana = (r: { id: string }) => /-l\d+$/.test(r.id);
const base = recursos.filter(r => !esCapaLejana(r)), capa = recursos.filter(esCapaLejana);
const de = (tipo: string, anillo?: string) => base.filter(r => r.tipo === tipo && (!anillo || r.anillo === anillo));

describe('recursos no metálicos', () => {
  it('totales exactos de partida: piedra 5601, cobre 240, hierro 360, ganadería 400', () => {
    expect(de('piedra')).toHaveLength(5601);
    expect(de('cobre')).toHaveLength(240);
    expect(de('hierro')).toHaveLength(360);
    expect(de('ganaderia')).toHaveLength(400);
    expect(base).toHaveLength(6601);
    expect(new Set(recursos.map(r => r.id)).size).toBe(recursos.length);
  });
  it('el hierro es 1,5 veces el cobre', () => {
    expect(de('hierro').length).toBe(1.5 * de('cobre').length);
  });
  it('el reparto por anillo sigue el canon 1.9c', () => {
    expect([de('piedra', 'foco').length, de('piedra', 'frontera').length, de('piedra', 'lejana').length]).toEqual([3361, 2240, 0]);
    expect([de('cobre', 'foco').length, de('cobre', 'frontera').length, de('cobre', 'lejana').length]).toEqual([48, 192, 0]);
    expect([de('hierro', 'foco').length, de('hierro', 'frontera').length, de('hierro', 'lejana').length]).toEqual([72, 288, 0]);
    expect([de('ganaderia', 'foco').length, de('ganaderia', 'frontera').length, de('ganaderia', 'lejana').length]).toEqual([200, 0, 200]);
  });
  it('la capa lejana: solo cobre, hierro y piedra, todo en la tierra lejana, a su fracción de la densidad de la frontera', () => {
    expect(capa.length).toBeGreaterThan(0);
    for (const r of capa) {
      expect(r.anillo).toBe('lejana');
      expect(Object.keys(P.capaLejana.densidadRelativa)).toContain(r.tipo);
    }
    // La piedra a 1/10 y el cobre y el hierro a 1/4 de la frontera: en proporción, menos piedra lejana que frontera.
    expect(capa.filter(r => r.tipo === 'piedra').length).toBeLessThan(de('piedra', 'frontera').length);
  });
  it('cada recurso está en el anillo que declara', () => {
    for (const r of recursos) expect(anilloEn(campamentos, r.posicion).anillo).toBe(r.anillo);
  });
  it('ninguno en agua, cima, pendiente fuerte ni sitio sin acceso; el cobre y el hierro, en colina o montaña', () => {
    for (const r of recursos) {
      expect(['agua', 'cima']).not.toContain(terreno(r.posicion));
      expect(sintetico.pendiente(r.posicion)).toBeLessThanOrEqual(P.pendienteMaxima);
      expect(sintetico.accesible(r.posicion)).toBe(true);
      if (r.tipo === 'ganaderia') expect(terreno(r.posicion)).not.toBe('montana');
    }
  });
  it('el cobre y el hierro prefieren colina o montaña (el llano es el último recurso)', () => {
    const minerales = [...de('cobre'), ...de('hierro')];
    expect(minerales.filter(r => ['colina', 'montana'].includes(terreno(r.posicion))).length / minerales.length).toBeGreaterThan(0.9);
  });
  it('separaciones mínimas: mismo tipo (piedra 70, resto 250) y 45 entre tipos distintos', () => {
    let infracciones = 0;
    for (const [i, a] of recursos.entries()) for (const b of recursos.slice(i + 1)) {
      const minimo = a.tipo === b.tipo ? P.separacionMismoTipo[a.tipo as keyof typeof P.separacionMismoTipo] : P.separacionOtroTipo;
      if (dist(a.posicion, b.posicion) < minimo) infracciones++;
    }
    expect(infracciones).toBe(0);
  });
  it('los puntos ocupados (metales) también quedan a 45', () => {
    const ocupados = recursos.slice(0, 200).map(r => r.posicion);
    const otros = repartirRecursosNoMetalicos(42, campamentos, sintetico, { ancho: ANCHO, alto: ALTO }, ocupados);
    expect(otros.some(r => ocupados.some(q => dist(r.posicion, q) < P.separacionOtroTipo))).toBe(false);
  });
  it('determinista por semilla', () => {
    expect(repartirRecursosNoMetalicos(42, campamentos, sintetico, { ancho: ANCHO, alto: ALTO })).toEqual(recursos);
    expect(repartirRecursosNoMetalicos(43, campamentos, sintetico, { ancho: ANCHO, alto: ALTO })).not.toEqual(recursos);
  });
  it('escala por área: un mundo más pequeño tiene proporcionalmente menos', () => {
    const chico = repartirRecursosNoMetalicos(42, campamentos.slice(0, 3), sintetico, { ancho: 16002, alto: 12000 });
    expect(chico.filter(r => r.tipo === 'piedra' && !esCapaLejana(r))).toHaveLength(1400);
    expect(chico.filter(r => r.tipo === 'hierro' && !esCapaLejana(r))).toHaveLength(90);
  });
  it('si un tipo no cabe, lanza error en vez de entregar menos', () => {
    expect(() => repartirRecursosNoMetalicos(42, campamentos, { ...sintetico, geologia: () => 0 }, { ancho: ANCHO, alto: ALTO }, [], { ...P, calidadMinima: 0.9 })).toThrow(/faltan/);
  });
});

describe('cantidades iniciales', () => {
  it('enteras y dentro del rango de su tipo, también la ganadería', () => {
    for (const r of recursos) {
      const { min, max } = RANGO_CANTIDAD[r.tipo], f = esCapaLejana(r) ? P.capaLejana.factorCantidad : 1;
      expect(Number.isInteger(r.cantidadInicial)).toBe(true);
      expect(r.cantidadInicial).toBeGreaterThanOrEqual(Math.round(min * f));
      expect(r.cantidadInicial).toBeLessThanOrEqual(Math.round(max * f));
    }
  });
  it('recorren el rango y dependen solo de (tipo, id, semilla)', () => {
    const piedras = de('piedra').map(r => r.cantidadInicial);
    expect(Math.min(...piedras)).toBeLessThan(700);
    expect(Math.max(...piedras)).toBeGreaterThan(1100);
    expect(cantidadInicial('oro', 'oro-lejos-3', 7)).toBe(cantidadInicial('oro', 'oro-lejos-3', 7));
    expect(RANGO_CANTIDAD.ganaderia).toEqual({ min: 10, max: 40 });
  });
});

describe('metalesARecursos', () => {
  const nodos: NodoMetal[] = [
    { id: 'estano-foco-0', tipo: 'estano', posicion: { x: 1, y: 2 }, anillo: 'foco', cantidadFactor: 1 / 3 },
    { id: 'oro-frontera-0', tipo: 'oro', posicion: { x: 3, y: 4 }, anillo: 'frontera', cantidadFactor: 1 },
    { id: 'estano-veta-0-1', tipo: 'estano', posicion: { x: 5, y: 6 }, anillo: 'lejana', cantidadFactor: 1, vetaId: 'veta-0' },
  ];
  const res = metalesARecursos(nodos, 9);
  it('conserva id, tipo, posición, anillo y veta', () => {
    expect(res.map(r => [r.id, r.tipo, r.posicion, r.anillo, r.vetaId])).toEqual(nodos.map(n => [n.id, n.tipo, n.posicion, n.anillo, n.vetaId]));
    expect('vetaId' in res[0]!).toBe(false);
  });
  it('aplica cantidadFactor: el foco lleva un tercio del rango y el resto el rango completo', () => {
    expect(res[0]!.cantidadInicial).toBeGreaterThanOrEqual(Math.round(RANGO_CANTIDAD.estano.min / 3));
    expect(res[0]!.cantidadInicial).toBeLessThanOrEqual(Math.round(RANGO_CANTIDAD.estano.max / 3));
    for (const r of res.slice(1)) {
      expect(r.cantidadInicial).toBeGreaterThanOrEqual(RANGO_CANTIDAD[r.tipo].min);
      expect(r.cantidadInicial).toBeLessThanOrEqual(RANGO_CANTIDAD[r.tipo].max);
    }
    expect(metalesARecursos(nodos, 9)).toEqual(res);
  });
});
