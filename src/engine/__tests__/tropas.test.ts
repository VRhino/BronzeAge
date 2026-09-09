// Regresión del rediseño "escuadrón por jugador" (Doc 2.5, a petición del usuario): antes, `reclutarTropa`
// fusionaba por `tropaId` a nivel de ASENTAMIENTO, así que dos jugadores reclutando la misma tropa en el
// mismo asentamiento terminaban compartiendo un solo escuadrón — el bug real reportado en la UI de Combate
// (dos escuadrones de 25 aparecían como un único chip de 50, inseleccionable de forma independiente).
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { crearFacciones, crearMapaDeterminista, posicionRecomendable, instanteDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
import { reclutarTropa, ReclutamientoInvalidoError } from '../tropas';

/** Los asentamientos recién fundados no tienen suficiente población/madera para reclutar (Doc 1.3: 20
 * pesants, 50 madera iniciales — Milicia de lanceros exige 25 pesants) — para un test de UNIDAD del motor de
 * reclutamiento, no de la curva de arranque completa, se sobreescriben directamente esos dos campos. */
function conRecursos(asentamiento: Asentamiento, pesants: number, madera: number): Asentamiento {
  return {
    ...asentamiento,
    poblacion: { ...asentamiento.poblacion, pesants },
    almacen: { ...asentamiento.almacen, madera: { ...asentamiento.almacen.madera!, cantidad: madera } },
  };
}

function asentamientoDeTest(): Asentamiento {
  const mapa = crearMapaDeterminista(1);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento } = fundarAsentamiento(mapa, facciones, 'faccion-1', posicion, ['jugador-a', 'jugador-b'], [], instanteDeTest(0));
  // 60, no 50: con el pool de reclutamiento acotado por mano de obra (`poblacionDisponibleParaReclutar`,
  // engine/asentamientoQuery.ts), el asentamiento de fundación ya tiene una Granja activa reservando 4 pesants
  // — 50 solo alcanzaba para UN escuadrón de 25, y este fixture lo comparten dos tests que reclutan dos veces.
  return conRecursos(asentamiento, 60, 200);
}

describe('reclutarTropa — escuadrones por jugador (Doc 2.5)', () => {
  it('dos jugadores reclutando la misma tropa en el mismo asentamiento crean DOS escuadrones separados', () => {
    const asentamiento = asentamientoDeTest();

    const trasA = reclutarTropa(asentamiento, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);
    const trasB = reclutarTropa(trasA, 'jugador-b', 'faccion-1', 'milicia_lanceros', 'pesants', 1);

    expect(trasB.escuadrones).toHaveLength(2);
    const deA = trasB.escuadrones.find((e) => e.jugadorId === 'jugador-a');
    const deB = trasB.escuadrones.find((e) => e.jugadorId === 'jugador-b');
    expect(deA?.cantidad).toBe(25);
    expect(deB?.cantidad).toBe(25);
    expect(deA?.id).not.toBe(deB?.id);
  });

  it('reclutar de nuevo repone solo el faltante hasta el tope, sin crear un segundo escuadrón', () => {
    const asentamiento = asentamientoDeTest();
    const trasReclutar = reclutarTropa(asentamiento, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);

    // Simula bajas de combate: el escuadrón de jugador-a queda en 20/25.
    const conBajas = conRecursos(
      { ...trasReclutar, escuadrones: trasReclutar.escuadrones.map((e) => ({ ...e, cantidad: 20 })) },
      10,
      100
    );

    const repuesto = reclutarTropa(conBajas, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 1);

    expect(repuesto.escuadrones).toHaveLength(1);
    expect(repuesto.escuadrones[0]!.cantidad).toBe(25);
    // Repone 5 unidades: paga 5 pesants (no 25) y 5 * 2 madera = 10 madera (mismo costo por soldado).
    expect(repuesto.poblacion.pesants).toBe(5);
    expect(repuesto.almacen.madera!.cantidad).toBe(90);
  });

  it('reclutar un escuadrón ya al tope se rechaza', () => {
    const asentamiento = asentamientoDeTest();
    const trasReclutar = reclutarTropa(asentamiento, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);

    expect(() => reclutarTropa(trasReclutar, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 1)).toThrow(ReclutamientoInvalidoError);
  });

  it('un jugador de OTRA Facción no puede reclutar (Doc 5.4, revisión 2026-09-08)', () => {
    const asentamiento = asentamientoDeTest(); // faccion-1
    expect(() => reclutarTropa(asentamiento, 'jugador-forastero', 'faccion-2', 'milicia_lanceros', 'pesants', 0)).toThrow(
      /No puedes reclutar aquí/
    );
  });

  it('un ciudadano de la MISMA Facción que no reside solo puede REPONER, no reclutar un escuadrón nuevo', () => {
    const asentamiento = asentamientoDeTest(); // faccion-1, residentes jugador-a/jugador-b
    // Escuadrón nuevo → rechazado.
    expect(() => reclutarTropa(asentamiento, 'jugador-c-de-faccion-1', 'faccion-1', 'milicia_lanceros', 'pesants', 0)).toThrow(
      /solo puedes reponer/
    );
    // Pero SÍ puede reponer uno que ya tiene posado aquí.
    const conEscuadronDeC: Asentamiento = {
      ...asentamiento,
      escuadrones: [
        {
          id: 'esc-c',
          nombre: 'Milicia de jugador-c',
          jugadorId: 'jugador-c-de-faccion-1',
          origen: 'pesants',
          cantidad: 10,
          veterania: 0,
          moral: 100,
          tropaId: 'milicia_lanceros',
        },
      ],
    };
    const repuesto = reclutarTropa(conEscuadronDeC, 'jugador-c-de-faccion-1', 'faccion-1', 'milicia_lanceros', 'pesants', 0);
    expect(repuesto.escuadrones[0]!.cantidad).toBe(25);
  });

  it('reclutar ya no exige un General asignado (Doc 2.5 gana la ambigüedad frente a Doc 2.2)', () => {
    const asentamiento = asentamientoDeTest();
    expect(asentamiento.cargos.generalId).toBeNull();

    expect(() => reclutarTropa(asentamiento, 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0)).not.toThrow();
  });
});
