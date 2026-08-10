import { describe, expect, it } from 'vitest';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

describe('fixtures de test', () => {
  it('funda un asentamiento recomendable y avanza un tick sin explotar', () => {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);

    expect(asentamiento.edificios.length).toBeGreaterThan(0);

    const estado: EstadoSimulacion = {
      asentamientos: [asentamiento],
      facciones: faccionesTrasFundar,
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
    };

    const resultado = avanzarSimulacion(estado, mapa, 1);
    expect(resultado.asentamientos).toHaveLength(1);
  });
});
