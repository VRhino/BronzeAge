import { describe, expect, it } from 'vitest';
import { fertilidadOliva, fertilidadVid } from '../cultivos';
import { distanciaCostaRegional, crearRelieveRegional } from '../regional';
import { evaluarTerreno, type CampoElevacion } from '../elevacion';
import { muestrearRelieve, type MuestraRelieve } from '../muestras';

/** Una ladera soleada y seca de tierra baja: buena para la vid y, cerca del mar, para la oliva. */
const ladera: MuestraRelieve = { altura: 0.5, pendiente: 0.2, humedad: 0.3 };

describe('fertilidad de la vid', () => {
  it('es 0 en el agua y en las cumbres, aunque lo demás sea ideal', () => {
    expect(fertilidadVid(ladera, 'costa')).toBeGreaterThan(0.5);
    expect(fertilidadVid(ladera, 'agua')).toBe(0);
    expect(fertilidadVid({ ...ladera, altura: 0.69 }, 'cima')).toBe(0);
  });
  it('prefiere la ladera suave a la tierra plana y a la pared, y lo seco a lo húmedo', () => {
    const v = (m: Partial<MuestraRelieve>) => fertilidadVid({ ...ladera, ...m }, 'colina');
    expect(v({})).toBeGreaterThan(v({ pendiente: 0 }));
    expect(v({})).toBeGreaterThan(v({ pendiente: 0.65 }));
    expect(v({})).toBeGreaterThan(v({ humedad: 0.9 }));
  });
});

describe('fertilidad de la oliva', () => {
  it('es mayor cerca de la costa que tierra adentro a igual altura', () => {
    expect(fertilidadOliva(ladera, 'llano', 500)).toBeGreaterThan(fertilidadOliva(ladera, 'llano', 3500));
    expect(fertilidadOliva(ladera, 'llano', 3500)).toBeGreaterThan(fertilidadOliva(ladera, 'llano', 9000));
    expect(fertilidadOliva(ladera, 'llano', 9000)).toBe(0);
  });
  it('es 0 en el agua y cae con la altura y la humedad', () => {
    expect(fertilidadOliva(ladera, 'agua', 0)).toBe(0);
    const o = (m: Partial<MuestraRelieve>) => fertilidadOliva({ ...ladera, ...m }, 'llano', 500);
    expect(o({})).toBeGreaterThan(o({ altura: 0.58 }));
    expect(o({})).toBeGreaterThan(o({ humedad: 0.9 }));
  });
});

describe('capas sobre el relieve de la semilla 42', () => {
  const regional = crearRelieveRegional(42, { ancho: 32000, alto: 24000 });
  const elevacion: CampoElevacion = { regional };
  const puntos = Array.from({ length: 400 }, (_, i) => ({ x: 300 + (i * 2371) % 31400, y: 300 + (i * 1913) % 23400 }));
  it('quedan en 0..1, la vid es 0 en agua y cumbres y la oliva en agua, y ambas dan algo en tierra', () => {
    let vid = 0, oliva = 0;
    for (const p of puntos) {
      const m = muestrearRelieve(elevacion, p), terreno = evaluarTerreno(elevacion, p, m.altura);
      const v = fertilidadVid(m, terreno), o = fertilidadOliva(m, terreno, distanciaCostaRegional(regional, p));
      for (const x of [v, o]) { expect(x).toBeGreaterThanOrEqual(0); expect(x).toBeLessThanOrEqual(1); }
      if (terreno === 'agua' || terreno === 'cima') expect(v).toBe(0);
      if (terreno === 'agua') expect(o).toBe(0);
      if (v > 0) vid++;
      if (o > 0) oliva++;
    }
    expect(vid).toBeGreaterThan(0);
    expect(oliva).toBeGreaterThan(0);
  });
});
