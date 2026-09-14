// Regresión del rediseño "escuadrón por jugador" (Doc 2.1/2.5, a petición del usuario): un jugador reside
// en UN solo asentamiento — es lo que le permite tener como mucho un escuadrón de cada tropa (ver
// `Escuadron.heroeId`, domain/types.ts, y `reclutarTropa`, engine/tropas.ts). Antes de este cambio,
// `comprarCasa` solo impedía ciudadanía cruzada entre Facciones, no residencia cruzada entre asentamientos
// de la MISMA Facción.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { crearFacciones, crearMapaDeterminista, posicionRecomendable, instanteDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
import { cambiarResidencia, comprarCasa, FaccionInvalidaError } from '../faccion';

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

describe('cambiarResidencia (Doc 2.5/2.6, comando nuevo)', () => {
  it('mueve al jugador: fuera de la vieja (fundador y casa), dentro de la nueva', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    const conCargo: Asentamiento = { ...asentamientoA, cargos: { ...asentamientoA.cargos, gobernadorId: 'jugador-a' } };

    const { origen, destino } = cambiarResidencia(facciones, [conCargo, asentamientoB], asentamientoB.id, 'jugador-a');

    expect(origen.heroesFundadoresIds).not.toContain('jugador-a');
    expect(origen.casasCompradas).not.toContain('jugador-a');
    expect(origen.cargos.gobernadorId).toBeNull(); // cargo local vacío al mudarse
    expect(destino.casasCompradas).toContain('jugador-a');
  });

  it('los escuadrones posados en la residencia vieja NO se tocan', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    const conGuarnicion: Asentamiento = {
      ...asentamientoA,
      escuadrones: [
        { id: 'e1', nombre: 'x', heroeId: 'jugador-a', origen: 'pesants', cantidad: 20, veterania: 0, moral: 100, tropaId: 'milicia_lanceros' },
      ],
    };
    const { origen } = cambiarResidencia(facciones, [conGuarnicion, asentamientoB], asentamientoB.id, 'jugador-a');
    expect(origen.escuadrones).toHaveLength(1);
    expect(origen.escuadrones[0]!.heroeId).toBe('jugador-a');
  });

  it('un huérfano (sin residencia de la que salir) es rechazado', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    // jugador-c es ciudadano tras comprar casa en B; luego se le quita la casa (simulando conquista) → huérfano.
    const conC = comprarCasa(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-c');
    const bSinC: Asentamiento = { ...conC.asentamiento, casasCompradas: [] };
    expect(() => cambiarResidencia(conC.facciones, [asentamientoA, bSinC], asentamientoA.id, 'jugador-c')).toThrow(
      FaccionInvalidaError
    );
  });

  it('un jugador de otra Facción no puede residir aquí', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    expect(() => cambiarResidencia(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-de-faccion-2')).toThrow(
      FaccionInvalidaError
    );
  });
});
