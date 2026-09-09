// Fase 0.1: 'cima' (banda de elevación más alta) es inhabitable — no se puede fundar ni extraer ahí. Este
// test cubre el contrato en los dos puntos donde se hace cumplir: `evaluarViabilidadFundacion` (lo que
// alimenta el aviso de la interfaz) y `fundarAsentamiento` (el bloqueo duro real).
import { describe, expect, it } from 'vitest';
import type { Mapa } from '../../world/mapa';
import { evaluarViabilidadFundacion, fundarAsentamiento, FundacionInvalidaError } from '../settlement';
import { crearFacciones, crearMapaDeterminista, instanteDeTest } from './fixtures';

const SEED = 1;

/** Busca un punto en 'cima' muestreando el mapa — con `ELEVACION.umbralCima` es una franja pequeña, pero
 * existe en un mapa 2000x2000 con esta seed; si esto llega a fallar, la seed/umbral cambiaron y hay que
 * revisar la calibración, no el test. */
function encontrarPuntoEnCima(mapa: Mapa): { x: number; y: number } {
  const paso = 20;
  for (let x = 0; x < mapa.limites.ancho; x += paso) {
    for (let y = 0; y < mapa.limites.alto; y += paso) {
      if (mapa.terrenoEn({ x, y }) === 'cima') return { x, y };
    }
  }
  throw new Error('No se encontró ningún punto en cima para esta seed/paso — revisar ELEVACION.umbralCima.');
}

describe('fundación en terreno de cima (Fase 0.1)', () => {
  it('evaluarViabilidadFundacion marca terrenoValido=false y fundable=false en cima', () => {
    const mapa = crearMapaDeterminista(SEED);
    const punto = encontrarPuntoEnCima(mapa);

    const v = evaluarViabilidadFundacion(mapa, punto, []);
    expect(v.terrenoValido).toBe(false);
    expect(v.fundable).toBe(false);
    expect(v.recomendable).toBe(false);
  });

  it('fundarAsentamiento rechaza duro una posición en cima, aunque esté dentro del mapa y libre', () => {
    const mapa = crearMapaDeterminista(SEED);
    const punto = encontrarPuntoEnCima(mapa);
    const facciones = crearFacciones();

    expect(() => fundarAsentamiento(mapa, facciones, 'faccion-1', punto, ['jugador-1'], [], instanteDeTest(0))).toThrow(
      FundacionInvalidaError
    );
  });

  it('un terreno normal (llano/colina/etc.) sigue siendo fundable — el bloqueo es solo de cima', () => {
    const mapa = crearMapaDeterminista(SEED);
    // (40,40) es donde ya buscan otros fixtures del motor — no es 'cima' para esta seed a esa escala.
    const punto = { x: 40, y: 40 };
    expect(mapa.terrenoEn(punto)).not.toBe('cima');

    const v = evaluarViabilidadFundacion(mapa, punto, []);
    expect(v.terrenoValido).toBe(true);
  });
});
