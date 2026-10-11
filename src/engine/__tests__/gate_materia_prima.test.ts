// Doc 4.2.1: gate de materia prima (solo auto-construcción, ver `tieneInsumoDeArranque`): Fundición/Curtiduría no se
// auto-proponen si el asentamiento no tiene YA en almacén al menos un insumo directo de su receta de nivel 1 — antes
// se construían igual y se quedaban produciendo 0 para siempre (solo ~6% de los asentamientos nace con nodo de cobre/livestock).
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Asentamiento, RecursoAlmacenado } from '../../domain/types';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { tieneInsumoDeArranque } from '../construction';
import { acelerarObras, contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

const SEED = 42;

function conAlmacen(asentamiento: Asentamiento, valores: Partial<Record<string, number>>): Asentamiento {
  const almacen: Record<string, RecursoAlmacenado> = { ...asentamiento.almacen };
  for (const [recurso, cantidad] of Object.entries(valores)) {
    almacen[recurso] = { cantidad: cantidad ?? 0, capacidad: almacen[recurso]?.capacidad ?? 200 };
  }
  return { ...asentamiento, almacen };
}


describe('gate de materia prima para auto-construcción de transformación', () => {
  // Regla de construcción, no de ritmo: obras aceleradas para crecer la ciudad (ver `acelerarObras`).
  let restaurarObras: () => void;
  beforeAll(() => {
    restaurarObras = acelerarObras();
  });
  afterAll(() => restaurarObras());

  it('Fundición no pasa el gate sin cobre ni estaño en almacén', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const sinMinerales = conAlmacen(asentamiento, { cobre: 0, estano: 0 });
    expect(tieneInsumoDeArranque(sinMinerales, 'fundicion')).toBe(false);
  });

  it('Fundición pasa el gate con cobre en almacén, sin importar si vino de extracción propia o trueque', () => {
    // La receta de NIVEL 1 de Fundición solo consume cobre (lingoteCobre <- cobre) — estaño entra recién en
    // nivel 2 (lingoteEstano), así que el gate de arranque (que solo mira el nivel 1) no lo exige todavía.
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    expect(tieneInsumoDeArranque(conAlmacen(asentamiento, { cobre: 1 }), 'fundicion')).toBe(true);
  });

  it('Curtiduría no pasa el gate sin livestock en almacén', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    expect(tieneInsumoDeArranque(conAlmacen(asentamiento, { livestock: 0 }), 'curtiduria')).toBe(false);
    expect(tieneInsumoDeArranque(conAlmacen(asentamiento, { livestock: 1 }), 'curtiduria')).toBe(true);
  });

  it('Armería queda exenta en la práctica: su receta de nivel 1 incluye armaMadera (solo madera)', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const sinNada = conAlmacen(asentamiento, { cobre: 0, estano: 0, livestock: 0, madera: 10 });
    expect(tieneInsumoDeArranque(sinNada, 'armeria')).toBe(true);
  });

  it('un edificio sin recetas (ej. Carpintería) no tiene insumo de arranque que exigir', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    expect(tieneInsumoDeArranque(conAlmacen(asentamiento, { madera: 0, piedra: 0 }), 'carpinteria')).toBe(true);
  });

  it('en simulación real: con cobre ya en almacén, Fundición se auto-construye; sin livestock, Curtiduría nunca', () => {
    // Inyecta cobre (simula stock de trueque) + piedra (evita que el gate se confunda con la restricción,
    // ya existente, de que el asentamiento no pueda pagar el COSTO de construcción — esa es otra condición,
    // no la que este gate prueba). Livestock se deja en 0 a propósito: Curtiduría nunca debe aparecer.
    // nivel: 2 forzado (Doc Fase_0_6): Fundición/Curtiduría ahora exigen nivel de asentamiento 2 para su
    // construcción BASE — este test prueba el gate de INSUMO, no el de nivel, así que arranca ya en nivel 2.
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const rng = createRng(SEED);
    const { asentamiento: base } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const asentamiento = conAlmacen({ ...base, nivel: 2, nivelActual: 2 }, { cobre: 10, piedra: 200 });
    let estado = crearEstadoDeTest([asentamiento], facciones);

    // 150, no 40: presupuesto con margen sobre el sitio de fundación real de este seed (ver comentario
    // equivalente más abajo, en el test de líneas de producción) — evita que el test dependa del filo
    // exacto de cuántos ticks tarda la auto-construcción en llegar a Fundición para este mundo concreto.
    // Subido de 100 a 150 (Etapa 3, anclas y satélites): la separación mínima que ahora se exige para no
    // sembrar un ancla mal colocada (§5.4) hace que encontrar sitio tarde algún tick más de lo habitual.
    let fundicionVista = false;
    for (let tick = 1; tick <= 150; tick++) {
      estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      const propios = estado.asentamientos[0]!.edificios;
      expect(propios.some((e) => e.tipo === 'curtiduria'), `tick ${tick}: Curtiduría apareció sin livestock en almacén`).toBe(false);
      if (propios.some((e) => e.tipo === 'fundicion')) fundicionVista = true;
    }

    expect(fundicionVista, 'Fundición nunca se auto-construyó en 100 ticks pese a tener cobre y piedra disponibles').toBe(true);
  });
});
