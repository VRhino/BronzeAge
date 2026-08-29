// Regresión del rediseño "escuadrón por jugador" (Doc 2.1/2.5, a petición del usuario): un jugador reside
// en UN solo asentamiento — es lo que le permite tener como mucho un escuadrón de cada tropa (ver
// `Escuadron.jugadorId`, domain/types.ts, y `reclutarTropa`, engine/tropas.ts). Antes de este cambio,
// `comprarCasa` solo impedía ciudadanía cruzada entre Facciones, no residencia cruzada entre asentamientos
// de la MISMA Facción.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { crearFacciones, crearMapaDeterminista, posicionRecomendable, instanteDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
import { comprarCasa, FaccionInvalidaError } from '../faccion';

/** El cap de fundación en nivel 1 es 1 asentamiento por Facción (`CAP_FUNDACION_POR_NIVEL`, constants.ts) —
 * para probar residencia cruzada entre DOS asentamientos de la misma Facción, se sube el nivel a mano tras
 * fundar el primero (no es una regla que este test esté validando, solo un requisito previo del motor). */
function fundarDosAsentamientosDeFaccion(): { asentamientoA: Asentamiento; asentamientoB: Asentamiento; facciones: Faccion[] } {
  const mapa = crearMapaDeterminista(1);
  const facciones = crearFacciones();

  const { asentamiento: asentamientoA, facciones: faccionesTrasA } = fundarAsentamiento(
    mapa,
    facciones,
    'faccion-1',
    posicionRecomendable(mapa),
    ['jugador-a'],
    [],
    instanteDeTest(0)
  );
  const faccionesNivel2 = faccionesTrasA.map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
  const { asentamiento: asentamientoB, facciones: faccionesTrasB } = fundarAsentamiento(
    mapa,
    faccionesNivel2,
    'faccion-1',
    posicionRecomendable(mapa, [asentamientoA]),
    ['jugador-b'],
    [asentamientoA],
    instanteDeTest(0)
  );

  return { asentamientoA, asentamientoB, facciones: faccionesTrasB };
}

describe('comprarCasa — residencia única (Doc 2.1)', () => {
  it('un jugador que ya reside en otro asentamiento no puede comprar casa en uno nuevo', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();

    expect(() => comprarCasa(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-a')).toThrow(
      FaccionInvalidaError
    );
  });

  it('un jugador nuevo de la misma Facción sí puede comprar casa en otro asentamiento', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();

    const resultado = comprarCasa(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-c');
    expect(resultado.asentamiento.casasCompradas).toContain('jugador-c');
  });
});
