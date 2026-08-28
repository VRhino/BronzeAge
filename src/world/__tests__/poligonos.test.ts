import { describe, expect, it } from 'vitest';
import type { Point } from '../../domain/types';
import { areaFirmada, formaCirculo, unirFormas, unirPoligonos } from '../poligonos';

function cuadrado(x: number, y: number, lado: number): Point[] {
  return [
    { x, y },
    { x: x + lado, y },
    { x: x + lado, y: y + lado },
    { x, y: y + lado },
  ];
}

/** Área total ocupada por los lazos: los exteriores suman y los agujeros restan (orientación opuesta). */
function areaTotal(lazos: Point[][]): number {
  return lazos.reduce((total, lazo) => total + areaFirmada(lazo), 0);
}

describe('unirPoligonos', () => {
  it('sin formas devuelve nada', () => {
    expect(unirPoligonos([], { paso: 1 })).toEqual([]);
  });

  it('ignora polígonos degenerados (menos de 3 puntos)', () => {
    expect(unirPoligonos([[{ x: 0, y: 0 }], []], { paso: 1 })).toEqual([]);
  });

  it('un cuadrado suelto sale como un único lazo con su área', () => {
    const lazos = unirPoligonos([cuadrado(0, 0, 100)], { paso: 2 });
    expect(lazos).toHaveLength(1);
    expect(Math.abs(areaTotal(lazos))).toBeCloseTo(100 * 100, -2);
  });

  it('dos cuadrados separados salen como dos lazos', () => {
    const lazos = unirPoligonos([cuadrado(0, 0, 60), cuadrado(200, 0, 60)], { paso: 2 });
    expect(lazos).toHaveLength(2);
  });

  it('dos cuadrados solapados se fusionan en UN lazo, sin contar el solape dos veces', () => {
    // Este es el caso del bug: antes cada silueta se dibujaba entera y el solape quedaba doblemente pintado.
    const lazos = unirPoligonos([cuadrado(0, 0, 100), cuadrado(50, 0, 100)], { paso: 2 });
    expect(lazos).toHaveLength(1);
    // Unión = 150x100, no 2 * 100x100.
    expect(Math.abs(areaTotal(lazos))).toBeCloseTo(150 * 100, -2.5);
  });

  it('un polígono contenido en otro no añade contorno propio', () => {
    const lazos = unirPoligonos([cuadrado(0, 0, 200), cuadrado(50, 50, 40)], { paso: 2 });
    expect(lazos).toHaveLength(1);
    expect(Math.abs(areaTotal(lazos))).toBeCloseTo(200 * 200, -2.5);
  });

  it('formas idénticas (caso degenerado para un clipper booleano) dan un solo lazo', () => {
    const lazos = unirPoligonos([cuadrado(0, 0, 80), cuadrado(0, 0, 80), cuadrado(0, 0, 80)], { paso: 2 });
    expect(lazos).toHaveLength(1);
  });
});

describe('unirFormas con círculos', () => {
  it('dos círculos que se solapan dan un lazo; separados, dos', () => {
    const juntos = unirFormas([formaCirculo({ x: 0, y: 0 }, 50), formaCirculo({ x: 60, y: 0 }, 50)], { paso: 2 });
    expect(juntos).toHaveLength(1);

    const sueltos = unirFormas([formaCirculo({ x: 0, y: 0 }, 50), formaCirculo({ x: 300, y: 0 }, 50)], { paso: 2 });
    expect(sueltos).toHaveLength(2);
  });

  it('un círculo suelto conserva su área', () => {
    const lazos = unirFormas([formaCirculo({ x: 500, y: 500 }, 120)], { paso: 2 });
    expect(lazos).toHaveLength(1);
    expect(Math.abs(areaTotal(lazos))).toBeCloseTo(Math.PI * 120 * 120, -3);
  });

  it('un anillo de círculos deja un AGUJERO: lazo interior con orientación opuesta al exterior', () => {
    // 10 círculos en corona: se tocan entre sí pero dejan el claro del centro sin cubrir.
    const radioCorona = 100;
    const formas = Array.from({ length: 10 }, (_, i) => {
      const a = (i / 10) * Math.PI * 2;
      return formaCirculo({ x: Math.cos(a) * radioCorona, y: Math.sin(a) * radioCorona }, 40);
    });
    const lazos = unirFormas(formas, { paso: 2 });
    expect(lazos).toHaveLength(2);

    const areas = lazos.map(areaFirmada);
    // Signos opuestos = el canvas recorta el agujero solo, con la regla de relleno `nonzero`.
    expect(Math.sign(areas[0]!)).toBe(-Math.sign(areas[1]!));
    // El área neta (exterior menos agujero) es menor que la del lazo exterior por sí solo.
    expect(Math.abs(areaTotal(lazos))).toBeLessThan(Math.max(...areas.map(Math.abs)));
  });
});

describe('opciones de unirFormas', () => {
  it('la tolerancia recorta puntos sin deformar la silueta', () => {
    const forma = [formaCirculo({ x: 0, y: 0 }, 150)];
    const detallado = unirFormas(forma, { paso: 2 })[0]!;
    const simplificado = unirFormas(forma, { paso: 2, tolerancia: 1 })[0]!;

    expect(simplificado.length).toBeLessThan(detallado.length);
    expect(Math.abs(areaFirmada(simplificado))).toBeCloseTo(Math.abs(areaFirmada(detallado)), -3);
  });

  it('areaMinima descarta las motas', () => {
    const grande = formaCirculo({ x: 0, y: 0 }, 100);
    const mota = formaCirculo({ x: 600, y: 600 }, 4);
    expect(unirFormas([grande, mota], { paso: 2 })).toHaveLength(2);
    expect(unirFormas([grande, mota], { paso: 2, areaMinima: 500 })).toHaveLength(1);
  });

  it('un paso absurdamente fino no cuelga: se degrada la resolución en vez de reventar la rejilla', () => {
    const lazos = unirFormas([formaCirculo({ x: 0, y: 0 }, 5000)], { paso: 0.01 });
    expect(lazos).toHaveLength(1);
  });
});
