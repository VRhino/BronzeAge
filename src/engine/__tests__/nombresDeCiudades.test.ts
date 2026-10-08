// Los nombres de los asentamientos que se fundan: ciudades de la época, sin repetirse en una partida.
import { describe, expect, it } from 'vitest';
import { NOMBRES_DE_CIUDADES, nombreDeCiudadLibre } from '../nombresDeCiudades';

const sinTildes = (n: string) => n.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

describe('NOMBRES_DE_CIUDADES', () => {
  it('son al menos cien, sin repetidos ni vacíos (tampoco sin tildes)', () => {
    expect(NOMBRES_DE_CIUDADES.length).toBeGreaterThanOrEqual(100);
    expect(NOMBRES_DE_CIUDADES.every((n) => n.trim() === n && n.length > 0)).toBe(true);
    expect(new Set(NOMBRES_DE_CIUDADES.map(sinTildes)).size).toBe(NOMBRES_DE_CIUDADES.length);
  });
});

describe('nombreDeCiudadLibre', () => {
  it('es de la lista y el mismo id sobre las mismas plazas da el mismo nombre', () => {
    const nombre = nombreDeCiudadLibre([], 'asentamiento-0');
    expect(NOMBRES_DE_CIUDADES).toContain(nombre);
    expect(nombreDeCiudadLibre([], 'asentamiento-0')).toBe(nombre);
  });

  it('nunca da uno que la partida ya use, con o sin tildes o mayúsculas', () => {
    const usados: string[] = [];
    for (let i = 0; i < NOMBRES_DE_CIUDADES.length; i++) {
      const nombre = nombreDeCiudadLibre(usados, `asentamiento-${i}`);
      expect(nombre, `el ${i + 1}.º de ${NOMBRES_DE_CIUDADES.length}`).toBeDefined();
      expect(usados.map(sinTildes)).not.toContain(sinTildes(nombre!));
      usados.push(nombre!);
    }
    // Agotados todos, se queda sin nombre (el asentamiento usa su id).
    expect(nombreDeCiudadLibre(usados, 'asentamiento-x')).toBeUndefined();
    expect(nombreDeCiudadLibre([NOMBRES_DE_CIUDADES[0]!.toUpperCase(), undefined], 'a')).not.toBe(NOMBRES_DE_CIUDADES[0]);
  });

  it('ids distintos reparten por toda la lista, no siempre el primero', () => {
    const distintos = new Set(Array.from({ length: 60 }, (_, i) => nombreDeCiudadLibre([], `asentamiento-${i}`)));
    expect(distintos.size).toBeGreaterThan(30);
  });
});
