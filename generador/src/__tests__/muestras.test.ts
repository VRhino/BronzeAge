import { describe, expect, it } from 'vitest';
import { crearRelieveRegional } from '../regional';
import { evaluarTerreno, evaluarElevacion, type CampoElevacion } from '../elevacion';
import { esTransitable } from '../costeMovimiento';
import { crearCampoForestal, densidadForestal } from '../forestacionRegional';
import { crearSuelosRegional, fertilidadSuelo } from '../suelosRegional';
import { muestrearRelieve } from '../muestras';
import { muestrearCeldas } from '../celdas';

const limites = { ancho: 6000, alto: 4000 };
const elevacion: CampoElevacion = { regional: crearRelieveRegional(42, { ancho: 32000, alto: 24000 }) };
const forestacion = crearCampoForestal(42, []);
const suelos = crearSuelosRegional(42, []);
const puntos = Array.from({ length: 60 }, (_, i) => ({ x: 500 + (i * 2371) % 31000, y: 400 + (i * 1913) % 23000 }));

describe('muestras de relieve compartidas', () => {
  it('evaluarTerreno y esTransitable con altura dan lo mismo que sin ella', () => {
    for (const p of puntos) {
      const altura = evaluarElevacion(elevacion, p);
      expect(evaluarTerreno(elevacion, p, altura)).toBe(evaluarTerreno(elevacion, p));
      expect(esTransitable(elevacion, p, altura)).toBe(esTransitable(elevacion, p));
    }
  });
  it('fertilidadSuelo y densidadForestal con muestra dan exactamente lo mismo que sin ella', () => {
    for (const p of puntos) {
      const m = muestrearRelieve(elevacion, p), bosque = densidadForestal(forestacion, elevacion, p);
      expect(densidadForestal(forestacion, elevacion, p, m)).toBe(bosque);
      expect(fertilidadSuelo(suelos, elevacion, p, m)).toBe(fertilidadSuelo(suelos, elevacion, p));
    }
  });
  it('muestrearCeldas coincide con el cálculo directo en el centro de cada celda', () => {
    const lado = 48, c = muestrearCeldas({ elevacion, forestacion }, limites, lado);
    expect(c.columnas).toBe(Math.ceil(limites.ancho / lado));
    expect(c.filas).toBe(Math.ceil(limites.alto / lado));
    for (const i of [0, 1, 77, 1234, c.columnas * c.filas - 1, 5000, 7777]) {
      const p = { x: ((i % c.columnas) + 0.5) * lado, y: (Math.floor(i / c.columnas) + 0.5) * lado }, m = muestrearRelieve(elevacion, p);
      expect(c.indice(p)).toBe(i);
      expect(c.altura[i]).toBe(m.altura); expect(c.pendiente[i]).toBe(m.pendiente); expect(c.humedad[i]).toBe(m.humedad);
      expect(c.densidad[i]).toBe(densidadForestal(forestacion, elevacion, p));
      expect(c.transitable[i]).toBe(esTransitable(elevacion, p) ? 1 : 0);
    }
  });
  it('indice solo acepta el centro exacto de una celda', () => {
    const c = muestrearCeldas({ elevacion, forestacion }, limites, 48);
    expect(c.indice({ x: 24, y: 24 })).toBe(0);
    expect(c.indice({ x: 24.0001, y: 24 })).toBe(-1);
    expect(c.indice({ x: 10, y: 10 })).toBe(-1);
    expect(c.indice({ x: -24, y: 24 })).toBe(-1);
    expect(c.indice({ x: 1e9, y: 24 })).toBe(-1);
  });
});
