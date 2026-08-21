import { describe, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { ZONA_INFLUENCIA } from '../../constants';
import { anadirEdificioManualmente } from '../construction';
import { anclaNacidaTrasSemilla } from '../trazado';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const SEED = 42;
const RECLAMOS_VACIOS = { nodos: new Set<string>(), lenerasPorBosque: new Map<string, number>() };

function base(nivel = 2) {
  const mapa = crearMapaDeterminista(SEED);
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const preparado: Asentamiento = {
    ...asentamiento,
    nivel,
    nivelActual: nivel,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[nivel] ?? asentamiento.radioPotencial,
    cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    almacen: {
      ...asentamiento.almacen,
      madera: { ...asentamiento.almacen.madera!, cantidad: 2000 },
      piedra: { ...asentamiento.almacen.piedra!, cantidad: 2000 },
      cobre: { ...asentamiento.almacen.cobre!, cantidad: 2000 },
    },
  };
  return { asentamiento: preparado, faccion: facciones[0]!, mapa };
}

describe('debug3', () => {
  it('fundicion', () => {
    const { asentamiento, faccion, mapa } = base(2);
    const conFundicion = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'fundicion', [], mapa, undefined, RECLAMOS_VACIOS);
    console.log('tipos:', conFundicion.edificios.map((e) => `${e.tipo}@(${e.posicion.x.toFixed(1)},${e.posicion.y.toFixed(1)})`));

    const fundicion = conFundicion.edificios.find((e) => e.tipo === 'fundicion')!;
    const direct = anclaNacidaTrasSemilla(conFundicion, conFundicion.edificios, fundicion, 'test-ancla-id');
    console.log('anclaNacidaTrasSemilla directo:', direct);
  });

  it('carpinteria nivel3', () => {
    const { asentamiento, faccion, mapa } = base(3);
    const conC = anadirEdificioManualmente(asentamiento, faccion, 'gobernador', 'carpinteria', [], mapa, undefined, RECLAMOS_VACIOS);
    console.log('tipos:', conC.edificios.map((e) => `${e.tipo}@(${e.posicion.x.toFixed(1)},${e.posicion.y.toFixed(1)})`));
    const carpinteria = conC.edificios.find((e) => e.tipo === 'carpinteria')!;
    console.log('categoria carpinteria activo? estado=', carpinteria.estado);
    const direct = anclaNacidaTrasSemilla(conC, conC.edificios, carpinteria, 'test-id');
    console.log('anclaNacidaTrasSemilla directo (carpinteria):', direct);
  });
});
