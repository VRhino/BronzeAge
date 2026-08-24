import { describe, expect, it } from 'vitest';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

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
      campamentosBandidos: [],
      bandidosProximoSpawnTick: 0,
    };

    const resultado = avanzarSimulacion(estado, mapa, contextoDeTest(1, createRng(1)));
    expect(resultado.asentamientos).toHaveLength(1);
  });
});
