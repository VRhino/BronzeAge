// Reglas puras de la intel de las tabernas (`engine/intel.ts`, Doc 5.12.10): precio, cupo, cooldown y qué entra en un Informe.
import { describe, expect, it } from 'vitest';
import { INTEL } from '../../constants';
import { instante, minutos, sumar } from '../../domain/tiempo';
import type { Asentamiento, Heroe } from '../../domain/types';
import { comprarMirada, IntelInvalidaError, levantarInforme, miradasActivasDe, precioInforme, precioMirada, validarInforme, type PeticionMirada } from '../intel';

const AHORA = instante(1_000_000_000);
const LIMITES = { ancho: 2000, alto: 2000 };

function peticion(parcial: Partial<PeticionMirada> = {}): PeticionMirada {
  return { id: 'm1', faccionId: 'f1', origenId: 'plaza-1', cupo: 1, centro: { x: 1000, y: 1000 }, ojosPropios: [{ x: 1000, y: 1000 }], limites: LIMITES, ahora: AHORA, ...parcial };
}

describe('precioMirada', () => {
  it('es la base más un tanto por la distancia al ojo propio más cercano', () => {
    expect(precioMirada({ x: 1000, y: 1000 }, [{ x: 1000, y: 1000 }])).toBe(INTEL.mirada.oroBase);
    expect(precioMirada({ x: 1000, y: 1000 }, [{ x: 0, y: 1000 }, { x: 1500, y: 1000 }])).toBe(Math.ceil(INTEL.mirada.oroBase + INTEL.mirada.oroPorUnidad * 500));
  });
});

describe('comprarMirada', () => {
  it('abre un ojo con el radio y la duración de la tarifa, y deja libre la zona tras el cooldown', () => {
    const r = comprarMirada([], peticion());
    expect(r.mirada).toMatchObject({ faccionId: 'f1', origenId: 'plaza-1', radio: INTEL.mirada.radio, compradaEn: AHORA });
    expect(r.mirada.expiraEn).toBe(sumar(AHORA, minutos(INTEL.mirada.duracionMinutos)));
    expect(r.mirada.libreEn).toBe(sumar(r.mirada.expiraEn, minutos(INTEL.mirada.cooldownMinutos)));
    expect(r.precio).toBe(INTEL.mirada.oroBase);
  });

  it('rechaza más Miradas abiertas que el cupo de esa taberna, pero otra taberna tiene el suyo', () => {
    const una = comprarMirada([], peticion()).miradas;
    expect(() => comprarMirada(una, peticion({ id: 'm2', centro: { x: 100, y: 100 } }))).toThrow(IntelInvalidaError);
    expect(() => comprarMirada(una, peticion({ id: 'm2', centro: { x: 100, y: 100 }, origenId: 'plaza-2' }))).not.toThrow();
    expect(() => comprarMirada(una, peticion({ id: 'm2', centro: { x: 100, y: 100 }, cupo: 2 }))).not.toThrow();
  });

  it('una zona recién mirada no se repite hasta pasado el cooldown, aunque la Mirada ya haya caducado', () => {
    const una = comprarMirada([], peticion()).miradas[0]!;
    const caducada = sumar(una.expiraEn, minutos(1));
    expect(() => comprarMirada([una], peticion({ id: 'm2', ahora: caducada }))).toThrow(IntelInvalidaError);
    const libre = sumar(una.libreEn, minutos(1));
    const r = comprarMirada([una], peticion({ id: 'm2', ahora: libre }));
    expect(r.miradas.map((m) => m.id)).toEqual(['m2']); // la vieja ya se ha purgado
  });

  it('el cooldown es de la Facción: otra puede mirar la misma zona', () => {
    const una = comprarMirada([], peticion()).miradas;
    expect(() => comprarMirada(una, peticion({ id: 'm2', faccionId: 'f2', origenId: 'plaza-9' }))).not.toThrow();
  });

  it('rechaza una zona fuera del mapa y una taberna sin cupo', () => {
    expect(() => comprarMirada([], peticion({ centro: { x: -1, y: 10 } }))).toThrow(IntelInvalidaError);
    expect(() => comprarMirada([], peticion({ centro: { x: 10, y: 2001 } }))).toThrow(IntelInvalidaError);
    expect(() => comprarMirada([], peticion({ cupo: 0 }))).toThrow(IntelInvalidaError);
  });
});

describe('miradasActivasDe', () => {
  it('solo las abiertas de las Facciones que cuentan', () => {
    const a = comprarMirada([], peticion()).mirada;
    const b = comprarMirada([], peticion({ id: 'm2', faccionId: 'f2' })).mirada;
    expect(miradasActivasDe([a, b], new Set(['f1']), AHORA)).toEqual([a]);
    expect(miradasActivasDe([a, b], new Set(['f1', 'f2']), AHORA)).toHaveLength(2);
    expect(miradasActivasDe([a, b], new Set(['f1', 'f2']), a.expiraEn)).toEqual([]);
  });
});

describe('informe de plaza', () => {
  const plaza = {
    id: 'plaza-rival',
    faccionId: 'f2',
    nombre: 'Troya',
    nivel: 3,
    posicion: { x: 500, y: 500 },
    almacen: { oro: { cantidad: 9999, capacidad: 9999 } },
    cargos: { gobernadorId: 'x' },
    edificios: [
      { id: 'e1', tipo: 'centroUrbano', posicion: { x: 0, y: 0 }, estado: 'activo' },
      { id: 'e2', tipo: 'barracon', posicion: { x: 3, y: 0 }, estado: 'activo', nivelInterno: 2 },
      { id: 'e3', tipo: 'granja', posicion: { x: 6, y: 0 }, estado: 'en_construccion' },
      { id: 'e4', tipo: 'vivienda', posicion: { x: 9, y: 0 }, estado: 'en_cola' },
    ],
    recintos: [{ id: 'r', nivel: 2, celdas: [{ col: 0, row: 0, clase: 'muro' }], avance: 0, comprometidoEn: AHORA }],
  } as unknown as Asentamiento;

  it('cuesta según el nivel de la plaza mirada', () => {
    expect(precioInforme(plaza)).toBe(INTEL.informe.oroPorNivel * 3);
  });

  it('cuenta el layout y la defensa, sin el almacén, los cargos ni lo que aún está en cola', () => {
    const informe = levantarInforme(plaza, [] as Heroe[], AHORA);
    expect(informe).toMatchObject({ asentamientoId: 'plaza-rival', faccionId: 'f2', nombre: 'Troya', nivel: 3, conocidoEn: AHORA });
    expect(informe.edificios.map((e) => e.tipo)).toEqual(['centroUrbano', 'barracon', 'granja']);
    expect(informe.edificios[1]).toMatchObject({ nivelInterno: 2 });
    expect(informe.recintos).toEqual([{ nivel: 2, celdas: [{ col: 0, row: 0, clase: 'muro' }], avance: 0 }]);
    expect(JSON.stringify(informe)).not.toMatch(/almacen|cargos|oro/);
  });

  it('no se pide de la propia Facción, ni de una plaza que no se conoce, ni dos veces seguidas', () => {
    expect(() => validarInforme(plaza, 'f2', true, undefined, AHORA)).toThrow(IntelInvalidaError);
    expect(() => validarInforme(plaza, 'f1', false, undefined, AHORA)).toThrow(IntelInvalidaError);
    const previo = levantarInforme(plaza, [], AHORA);
    expect(() => validarInforme(plaza, 'f1', true, previo, sumar(AHORA, minutos(INTEL.informe.cooldownMinutos - 1)))).toThrow(IntelInvalidaError);
    expect(validarInforme(plaza, 'f1', true, previo, sumar(AHORA, minutos(INTEL.informe.cooldownMinutos + 1)))).toBe(precioInforme(plaza));
  });
});
