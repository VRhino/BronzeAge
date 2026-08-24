// `Mapa.contornosBosques` sobre mundos generados de verdad: la fusión tiene que aguantar los 170 discos
// solapados que produce `generarBosques`, no solo los casos de laboratorio de `poligonos.test.ts`.

import { describe, expect, it } from 'vitest';
import { generarMapa, MAPA_DEFAULT } from '../../worldgen';
import { crearMapa } from '../mapa';
import { pointInPolygon } from '../geometria';
import type { Point } from '../../domain/types';

const SEEDS = [1, 42, 7];

function mapaDeSeed(seed: number) {
  return crearMapa(generarMapa({ ...MAPA_DEFAULT, seed }));
}

/** ¿Cubre la silueta este punto? Los lazos exteriores y los huecos vienen con orientación opuesta, así que
 * un punto dentro de un número IMPAR de lazos está cubierto (misma regla que el relleno del canvas). */
function cubiertoPorSilueta(p: Point, contornos: readonly Point[][]): boolean {
  return contornos.filter((lazo) => pointInPolygon(p, lazo)).length % 2 === 1;
}

describe('Mapa.contornosBosques', () => {
  it.each(SEEDS)('produce menos siluetas que bosques, porque se solapan (seed %i)', (seed) => {
    const mapa = mapaDeSeed(seed);
    const contornos = mapa.contornosBosques();
    expect(contornos.length).toBeGreaterThan(0);
    expect(contornos.length).toBeLessThan(mapa.listarBosques().length);
  });

  it.each(SEEDS)('el centro de cada bosque cae dentro de la silueta (seed %i)', (seed) => {
    const mapa = mapaDeSeed(seed);
    const contornos = mapa.contornosBosques();
    const fuera = mapa.listarBosques().filter((b) => !cubiertoPorSilueta(b.centro, contornos));
    expect(fuera.map((b) => b.id)).toEqual([]);
  });

  it.each(SEEDS)('un punto lejos de todo bosque queda fuera de la silueta (seed %i)', (seed) => {
    const mapa = mapaDeSeed(seed);
    const contornos = mapa.contornosBosques();
    // Muestreo en rejilla: todo punto cubierto por la silueta debe tener un bosque encima de verdad, y
    // viceversa. Se deja un margen igual al paso de fusión (ancho/400) porque en el borde exacto la
    // discrepancia entre el círculo real y su contorno muestreado es esperable.
    const margen = MAPA_DEFAULT.ancho / 400 + 1;
    const bosques = mapa.listarBosques();
    let comprobados = 0;
    for (let x = 25; x < MAPA_DEFAULT.ancho; x += 97) {
      for (let y = 25; y < MAPA_DEFAULT.alto; y += 97) {
        const p = { x, y };
        const distancias = bosques.map((b) => Math.hypot(b.centro.x - p.x, b.centro.y - p.y) - b.radio);
        const holgura = Math.min(...distancias);
        if (Math.abs(holgura) <= margen) continue; // demasiado cerca del borde para exigir un veredicto
        expect(cubiertoPorSilueta(p, contornos)).toBe(holgura < 0);
        comprobados++;
      }
    }
    expect(comprobados).toBeGreaterThan(100);
  });

  it('devuelve la MISMA instancia en llamadas sucesivas: se cachea por mundo generado', () => {
    // Y se comparte entre fachadas distintas sobre el mismo mundo — cada foto del historial construye la
    // suya (ver `GameStore.getMapa`), y recalcular la unión al mover el slider sería un parón visible.
    const generado = generarMapa({ ...MAPA_DEFAULT, seed: 3 });
    const unMapa = crearMapa(generado);
    const otroMapa = crearMapa(generado);
    expect(unMapa.contornosBosques()).toBe(unMapa.contornosBosques());
    expect(otroMapa.contornosBosques()).toBe(unMapa.contornosBosques());
  });

  it('la misma seed da siempre la misma silueta', () => {
    const unos = crearMapa(generarMapa({ ...MAPA_DEFAULT, seed: 11 })).contornosBosques();
    const otros = crearMapa(generarMapa({ ...MAPA_DEFAULT, seed: 11 })).contornosBosques();
    expect(otros).toEqual(unos);
  });
});
