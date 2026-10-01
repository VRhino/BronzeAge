// Cantería (Doc 6.6): la tecnología de la Era I que multiplica por 1,5 lo que sacan las Canteras de la Facción.
import { describe, expect, it } from 'vitest';
import { TECNOLOGIAS } from '../../constants';
import { factorProduccionTecnologica, produccionPorMinuto } from '../asentamientoQuery';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, prepararParaSubirANivel2 } from './fixtures';

const mapa = crearMapaDeterminista(7);
const plaza = prepararParaSubirANivel2(fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []).asentamiento, mapa);
const piedraPorMinuto = (adoptadas: Parameters<typeof produccionPorMinuto>[3]) =>
  produccionPorMinuto(plaza, mapa, [], adoptadas).find((i) => i.tipo === 'cantera')!.cantidadPorMinuto;

describe('Cantería', () => {
  it('es de la Era I y multiplica solo la piedra', () => {
    expect(TECNOLOGIAS.canteria.era).toBe('reinos_palaciales');
    expect(factorProduccionTecnologica(['canteria'], 'piedra')).toBe(1.5);
    expect(factorProduccionTecnologica(['canteria'], 'oro')).toBe(1);
    expect(factorProduccionTecnologica([], 'piedra')).toBe(1);
  });

  it('con ella adoptada, las Canteras de la plaza rinden un 50 % más', () => {
    expect(piedraPorMinuto(['canteria'])).toBeCloseTo(piedraPorMinuto([]) * 1.5);
  });
});
