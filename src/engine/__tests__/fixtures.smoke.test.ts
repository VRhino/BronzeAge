import { describe, expect, it } from 'vitest';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

describe('fixtures de test', () => {
  it('funda un asentamiento recomendable y avanza un tick sin explotar', () => {
    const mapa = crearMapaDeterminista(1);
    const facciones = crearFacciones();
    const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);

    expect(asentamiento.edificios.length).toBeGreaterThan(0);

    const estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);

    const resultado = avanzarSimulacion(estado, mapa, contextoDeTest(1, createRng(1)));
    expect(resultado.asentamientos).toHaveLength(1);
  });
});
