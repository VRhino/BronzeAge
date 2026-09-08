// Recaudación de oro por población (Doc 4.1, bloque "economía del oro"): espejo de `consumoComidaPoblacion`,
// signo opuesto. Estos tests fijan el contrato: Σ(habitantes_clase × tasa_clase), Nobleza > Artesanos >
// Pesants por cabeza, y el llamador (avanzarSimulacion) la suma al almacén respetando la capacidad.
import { describe, expect, it } from 'vitest';
import { IMPUESTOS } from '../../constants';
import { recaudacionOro } from '../population';
import { agregarRecurso } from '../almacen';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';

function asentamientoDeTest() {
  const mapa = crearMapaDeterminista(42);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);
  return asentamiento;
}

describe('recaudacionOro', () => {
  it('suma por clase con la tasa propia de cada una', () => {
    const base = asentamientoDeTest();
    const a = { ...base, poblacion: { pesants: 100, artesanos: 10, nobleza: 2 } };
    expect(recaudacionOro(a)).toBeCloseTo(
      100 * IMPUESTOS.tasaPesants + 10 * IMPUESTOS.tasaArtesanos + 2 * IMPUESTOS.tasaNobleza
    );
  });

  it('población 0 recauda 0', () => {
    const a = { ...asentamientoDeTest(), poblacion: { pesants: 0, artesanos: 0, nobleza: 0 } };
    expect(recaudacionOro(a)).toBe(0);
  });

  it('Nobleza rinde más per cápita que Artesanos, y Artesanos más que Pesants', () => {
    const base = asentamientoDeTest();
    const soloP = { ...base, poblacion: { pesants: 1, artesanos: 0, nobleza: 0 } };
    const soloA = { ...base, poblacion: { pesants: 0, artesanos: 1, nobleza: 0 } };
    const soloN = { ...base, poblacion: { pesants: 0, artesanos: 0, nobleza: 1 } };
    expect(recaudacionOro(soloN)).toBeGreaterThan(recaudacionOro(soloA));
    expect(recaudacionOro(soloA)).toBeGreaterThan(recaudacionOro(soloP));
  });

  it('el oro recaudado respeta la capacidad del almacén (el sobrante se pierde)', () => {
    const base = asentamientoDeTest();
    const almacenCasiLleno = { ...base.almacen, oro: { cantidad: 399.5, capacidad: 400 } };
    const conRecaudacion = agregarRecurso(almacenCasiLleno, 'oro', 100);
    expect(conRecaudacion['oro']!.cantidad).toBe(400);
  });
});
