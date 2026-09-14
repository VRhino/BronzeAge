// Regresión del rediseño "escuadrón por jugador" (Doc 2.5, a petición del usuario): antes, `reclutarTropa`
// fusionaba por `tropaId` a nivel de ASENTAMIENTO, así que dos jugadores reclutando la misma tropa en el
// mismo asentamiento terminaban compartiendo un solo escuadrón — el bug real reportado en la UI de Combate
// (dos escuadrones de 25 aparecían como un único chip de 50, inseleccionable de forma independiente).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, ContenedorEscuadron, Ejercito, Heroe } from '../../domain/types';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, heroesCon, posicionRecomendable, instanteDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
import { reclutarTropa, ReclutamientoInvalidoError } from '../tropas';
import { campamentoDe } from '../tropa';

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

/** Los dos residentes y un ciudadano de faccion-1 que reside en otra parte, todos sin tropa. */
const HEROES: Heroe[] = heroesCon([], ['jugador-a', 'jugador-b', 'jugador-c-de-faccion-1']);

describe('reclutarTropa — escuadrones por jugador (Doc 2.5)', () => {
  it('dos heroes reclutando la misma tropa en el mismo asentamiento crean DOS escuadrones separados', () => {
    const asentamiento = asentamientoDeTest();

    const trasA = reclutarTropa(asentamiento, HEROES, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);
    const trasB = reclutarTropa(trasA.asentamiento, trasA.heroes, [], 'jugador-b', 'faccion-1', 'milicia_lanceros', 'pesants', 1);

    const campamento = campamentoDe(trasB.asentamiento, trasB.heroes);
    expect(campamento).toHaveLength(2);
    const deA = campamento.find((e) => e.heroeId === 'jugador-a');
    const deB = campamento.find((e) => e.heroeId === 'jugador-b');
    expect(deA?.cantidad).toBe(25);
    expect(deB?.cantidad).toBe(25);
    expect(deA?.id).not.toBe(deB?.id);
  });

  it('reclutar de nuevo repone solo el faltante hasta el tope, sin crear un segundo escuadrón', () => {
    const asentamiento = asentamientoDeTest();
    const trasReclutar = reclutarTropa(asentamiento, HEROES, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);

    // Simula bajas de combate: el escuadrón de jugador-a queda en 20/25.
    const conBajas = trasReclutar.heroes.map((h) => ({ ...h, escuadrones: h.escuadrones.map((e) => ({ ...e, cantidad: 20 })) }));

    const repuesto = reclutarTropa(conRecursos(trasReclutar.asentamiento, 10, 100), conBajas, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 1);

    const suyas = repuesto.heroes.find((h) => h.id === 'jugador-a')!.escuadrones;
    expect(suyas).toHaveLength(1);
    expect(suyas[0]!.cantidad).toBe(25);
    // Repone 5 unidades: paga 5 pesants (no 25) y 5 * 2 madera = 10 madera (mismo costo por soldado).
    expect(repuesto.asentamiento.poblacion.pesants).toBe(5);
    expect(repuesto.asentamiento.almacen.madera!.cantidad).toBe(90);
  });

  it('reclutar un escuadrón ya al tope se rechaza', () => {
    const asentamiento = asentamientoDeTest();
    const tras = reclutarTropa(asentamiento, HEROES, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);

    expect(() => reclutarTropa(tras.asentamiento, tras.heroes, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 1)).toThrow(
      ReclutamientoInvalidoError
    );
  });

  it('un jugador de OTRA Facción no puede reclutar (Doc 5.4, revisión 2026-09-08)', () => {
    const asentamiento = asentamientoDeTest(); // faccion-1
    expect(() => reclutarTropa(asentamiento, HEROES, [], 'jugador-forastero', 'faccion-2', 'milicia_lanceros', 'pesants', 0)).toThrow(
      /No puedes reclutar aquí/
    );
  });

  it('un ciudadano de la MISMA Facción que no reside solo puede REPONER su columna a la puerta', () => {
    const asentamiento = asentamientoDeTest(); // faccion-1, residentes jugador-a/jugador-b
    const c = 'jugador-c-de-faccion-1';
    // Escuadrón nuevo → rechazado.
    expect(() => reclutarTropa(asentamiento, HEROES, [], c, 'faccion-1', 'milicia_lanceros', 'pesants', 0)).toThrow(/solo puedes reponer/);
    // Pero SÍ puede reponer la escuadra que lleva en su columna, plantada a la puerta.
    const suya = escuadronDePrueba('esc-c', c, 'milicia_lanceros', 10, { contenedor: { tipo: 'ejercito', ejercitoId: 'col-c' } });
    const aLaPuerta = [{ id: 'col-c', posicionActual: asentamiento.posicion } as Ejercito];
    const repuesto = reclutarTropa(asentamiento, heroesCon([suya], ['jugador-a']), aLaPuerta, c, 'faccion-1', 'milicia_lanceros', 'pesants', 0);
    expect(repuesto.heroes.find((h) => h.id === c)!.escuadrones[0]!.cantidad).toBe(25);
  });

  // Doc 2.5 (confirmado 2026-09-13): UNA escuadra por `tropaId` en toda la partida, y solo se repone donde está.
  const fuera: [string, ContenedorEscuadron, string][] = [
    ['de escolta en una caravana', { tipo: 'escolta', caravanaId: 'car-1' }, 'la escolta de la caravana car-1'],
    ['dentro de un ejército', { tipo: 'ejercito', ejercitoId: 'ej-1' }, 'el ejército ej-1'],
  ];
  it.each(fuera)('con la escuadra %s, reclutar ese tipo en casa se rechaza', (_donde, contenedor, ubicacion) => {
    const casa = asentamientoDeTest();
    const tras = reclutarTropa(casa, HEROES, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0);
    const conLaSuyaFuera = tras.heroes.map((h) => ({ ...h, escuadrones: h.escuadrones.map((e) => ({ ...e, contenedor })) }));

    expect(() => reclutarTropa(casa, conLaSuyaFuera, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 1)).toThrow(
      new RegExp(`Ya tienes una escuadra de .*: está en ${ubicacion}`)
    );
    // La regla es por jugador: otro residente sí recluta esa tropa.
    expect(() => reclutarTropa(casa, conLaSuyaFuera, [], 'jugador-b', 'faccion-1', 'milicia_lanceros', 'pesants', 1)).not.toThrow();
  });

  it('reclutar ya no exige un General asignado (Doc 2.5 gana la ambigüedad frente a Doc 2.2)', () => {
    const asentamiento = asentamientoDeTest();
    expect(asentamiento.cargos.generalId).toBeNull();

    expect(() => reclutarTropa(asentamiento, HEROES, [], 'jugador-a', 'faccion-1', 'milicia_lanceros', 'pesants', 0)).not.toThrow();
  });
});
