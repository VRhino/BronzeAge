// Doc 4.2.1, a petición del usuario: dos mecánicas nuevas sobre edificios de transformación.
//
// 1) Gate de materia prima (solo auto-construcción, ver `tieneInsumoDeArranque`): Fundición/Curtiduría no se
//    auto-proponen si el asentamiento no tiene YA en almacén al menos un insumo directo de su receta de
//    nivel 1 — antes se construían igual y se quedaban produciendo 0 para siempre (ver comentario de
//    `RECETA_ARMA_MADERA` en constants.ts: solo ~6% de los asentamientos nace con nodo de cobre/livestock).
// 2) Líneas de producción (ver `factorPorDistancia`/`factorLineaProduccion`): la distancia dentro del
//    asentamiento entre un transformador y la fuente más cercana de cada insumo penaliza cuánto produce ese
//    tick, nunca cuánto consume por unidad.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio, RecursoAlmacenado } from '../../domain/types';
import type { RecetaProduccion } from '../../constants';
import { LINEAS_PRODUCCION } from '../../constants';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { factorLineaProduccion, factorPorDistancia, sitioConcentrico, sitioConcentricoLineaProduccion, tieneInsumoDeArranque } from '../construction';
import { activarPolitica, lineasProduccionPriorizadas } from '../politicas';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

const SEED = 42;

function conAlmacen(asentamiento: Asentamiento, valores: Partial<Record<string, number>>): Asentamiento {
  const almacen: Record<string, RecursoAlmacenado> = { ...asentamiento.almacen };
  for (const [recurso, cantidad] of Object.entries(valores)) {
    almacen[recurso] = { cantidad: cantidad ?? 0, capacidad: almacen[recurso]?.capacidad ?? 200 };
  }
  return { ...asentamiento, almacen };
}

function edificio(tipo: Edificio['tipo'], posicion: { x: number; y: number }, overrides: Partial<Edificio> = {}): Edificio {
  return { id: `test-${tipo}-${posicion.x}-${posicion.y}`, tipo, posicion, estado: 'activo', ticksRestantes: 0, ...overrides };
}

describe('gate de materia prima para auto-construcción de transformación', () => {
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
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const restaurarMathRandom = mockMathRandomDeterminista(SEED);
    try {
      const { asentamiento: base } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
      const asentamiento = conAlmacen(base, { cobre: 10, piedra: 200 });
      let estado: EstadoSimulacion = {
        asentamientos: [asentamiento],
        facciones,
        caravanas: [],
        acuerdos: [],
        ordenes: [],
        relaciones: [],
        titulos: [],
        caminos: [],
        campamentosBandidos: [],
        bandidosProximoSpawnTick: 0,
      };

      // 100, no 40: presupuesto con margen sobre el sitio de fundación real de este seed (ver comentario
      // equivalente más abajo, en el test de líneas de producción) — evita que el test dependa del filo
      // exacto de cuántos ticks tarda la auto-construcción en llegar a Fundición para este mundo concreto.
      let fundicionVista = false;
      for (let tick = 1; tick <= 100; tick++) {
        estado = avanzarSimulacion(estado, mapa, tick);
        const propios = estado.asentamientos[0]!.edificios;
        expect(propios.some((e) => e.tipo === 'curtiduria'), `tick ${tick}: Curtiduría apareció sin livestock en almacén`).toBe(false);
        if (propios.some((e) => e.tipo === 'fundicion')) fundicionVista = true;
      }

      expect(fundicionVista, 'Fundición nunca se auto-construyó en 100 ticks pese a tener cobre y piedra disponibles').toBe(true);
    } finally {
      restaurarMathRandom();
    }
  });
});

describe('factor de distancia de líneas de producción', () => {
  it('sin penalización por debajo del umbral, factor mínimo a partir de la distancia máxima', () => {
    expect(factorPorDistancia(0)).toBe(1);
    expect(factorPorDistancia(LINEAS_PRODUCCION.distanciaSinPenalizacion)).toBe(1);
    expect(factorPorDistancia(LINEAS_PRODUCCION.distanciaMaxima)).toBeCloseTo(LINEAS_PRODUCCION.factorMinimo);
    expect(factorPorDistancia(LINEAS_PRODUCCION.distanciaMaxima + 1000)).toBeCloseTo(LINEAS_PRODUCCION.factorMinimo);
  });

  it('decae monótonamente entre los dos umbrales', () => {
    const medio = (LINEAS_PRODUCCION.distanciaSinPenalizacion + LINEAS_PRODUCCION.distanciaMaxima) / 2;
    const factorMedio = factorPorDistancia(medio);
    expect(factorMedio).toBeLessThan(1);
    expect(factorMedio).toBeGreaterThan(LINEAS_PRODUCCION.factorMinimo);
    expect(factorPorDistancia(medio - 10)).toBeGreaterThan(factorMedio);
    expect(factorPorDistancia(medio + 10)).toBeLessThan(factorMedio);
  });

  it('mide contra la fuente activa más cercana de ese insumo, no cualquier edificio del tipo', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const fundicion = edificio('fundicion', { x: 0, y: 0 });
    const receta: RecetaProduccion = { produce: 'lingoteCobre', produccionBase: 5, consumePorUnidad: { cobre: 2 } };

    const minaCerca = edificio('minaCobre', { x: 5, y: 0 });
    const minaLejos = edificio('minaCobre', { x: 500, y: 0 });
    const conDosMinas = { ...asentamiento, edificios: [minaLejos, minaCerca] };
    expect(factorLineaProduccion(fundicion, receta, conDosMinas)).toBe(factorPorDistancia(5));

    // Una mina en_cola (todavía no activa) no cuenta como fuente real.
    const soloEnCola = { ...asentamiento, edificios: [{ ...minaCerca, estado: 'en_cola' as const }] };
    expect(factorLineaProduccion(fundicion, receta, soloEnCola)).toBe(factorPorDistancia(LINEAS_PRODUCCION.distanciaEstandarSinFuente));
  });

  it('sin ninguna fuente propia del insumo, usa la distancia estándar placeholder', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const fundicion = edificio('fundicion', { x: 0, y: 0 });
    const receta: RecetaProduccion = { produce: 'lingoteCobre', produccionBase: 5, consumePorUnidad: { cobre: 2 } };
    const sinMinas = { ...asentamiento, edificios: [] };
    expect(factorLineaProduccion(fundicion, receta, sinMinas)).toBe(factorPorDistancia(LINEAS_PRODUCCION.distanciaEstandarSinFuente));
  });

  it('la fuente de un insumo intermedio es el transformador que lo fabrica, no un extractor', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const armeria = edificio('armeria', { x: 0, y: 0 });
    // armaCobre consume lingoteCobre (intermedio, lo fabrica Fundición) + madera (crudo, lo fabrica Leñera).
    const receta: RecetaProduccion = { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 } };
    const fundicionCerca = edificio('fundicion', { x: 10, y: 0 });
    const con = { ...asentamiento, edificios: [fundicionCerca] };
    // Sin Leñera, madera cae al placeholder de "sin fuente" — peor que la distancia a la Fundición cercana.
    expect(factorLineaProduccion(armeria, receta, con)).toBe(factorPorDistancia(LINEAS_PRODUCCION.distanciaEstandarSinFuente));
  });

  it('con varios insumos, manda el más penalizado (eslabón más débil), no un promedio', () => {
    const { asentamiento } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    const armeria = edificio('armeria', { x: 0, y: 0 });
    const receta: RecetaProduccion = { produce: 'armaCobre', produccionBase: 3, consumePorUnidad: { lingoteCobre: 1, madera: 1 } };
    const fundicionCerca = edificio('fundicion', { x: 10, y: 0 }); // lingoteCobre: factor alto
    const leneraLejos = edificio('lenera', { x: 1000, y: 0 }); // madera: factor mínimo (el eslabón débil)
    const con = { ...asentamiento, edificios: [fundicionCerca, leneraLejos] };
    expect(factorLineaProduccion(armeria, receta, con)).toBeCloseTo(LINEAS_PRODUCCION.factorMinimo);
  });
});

function dist(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

describe('política "Líneas de Producción" del Maestro de Obras', () => {
  it('lineasProduccionPriorizadas es false por defecto y true tras activar la política', () => {
    const { asentamiento: base, facciones } = fundarAsentamientoDeTest(crearMapaDeterminista(SEED), crearFacciones(), 'faccion-1', []);
    expect(lineasProduccionPriorizadas(base)).toBe(false);

    const conCargo = { ...base, cargos: { ...base.cargos, maestroObrasId: 'jugador-test' } };
    const faccion = facciones.find((f) => f.id === 'faccion-1')!;
    const conPolitica = activarPolitica(conCargo, faccion, 'maestroObras', 'lineas_produccion', 1);
    expect(lineasProduccionPriorizadas(conPolitica)).toBe(true);
  });

  it('sitioConcentricoLineaProduccion nunca elige un hueco peor que el que elegiría sitioConcentrico', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento: base } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    // Vista de Asentamiento: la optimización de "líneas de producción" solo aplica a fuentes del MISMO espacio
    // (interno). La Curtiduría (interna) consume `livestock`, que produce el Corral (interno). Colocamos el
    // Corral pegado al borde del espacio plano local: el primer hueco de sitioConcentrico (anillo interior)
    // queda necesariamente más lejos de él que el mejor hueco posible.
    const corral: Edificio = {
      id: 'corral-test',
      tipo: 'corral',
      posicion: { x: base.radioPotencial * 0.95, y: 0 }, // coords LOCALES (origen = Centro Urbano)
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
      fuenteId: 'nodo-livestock-inexistente', // basta el tipo/estado para `fuentesDeRecurso`; no se extrae aquí
    };
    const conCorral = { ...base, edificios: [...base.edificios, corral] };

    const plano = sitioConcentrico(conCorral, conCorral.edificios);
    const optimizado = sitioConcentricoLineaProduccion(conCorral, conCorral.edificios, 'curtiduria');
    expect(plano).not.toBeNull();
    expect(optimizado).not.toBeNull();

    const distPlano = dist(plano!, corral.posicion);
    const distOptimizado = dist(optimizado!, corral.posicion);
    expect(distOptimizado).toBeLessThanOrEqual(distPlano);
    expect(factorPorDistancia(distOptimizado)).toBeGreaterThanOrEqual(factorPorDistancia(distPlano));
  });

  it('con la política activa, la auto-construcción sitúa Fundición cerca de la mina; sin ella, en el hueco genérico', () => {
    // Mockeado como el resto de tests "en simulación real" de este archivo (ver arriba): sin esto, la
    // varianza de `Math.random()` en el crecimiento de población (`population.ts`) puede retrasar lo
    // suficiente la disponibilidad de mano de obra como para que Fundición no se proponga dentro del
    // presupuesto de ticks — el test no verifica timing de población, solo DÓNDE se sitúa Fundición.
    const restaurarMathRandom = mockMathRandomDeterminista(SEED);
    try {
      const mapa = crearMapaDeterminista(SEED);
      const facciones = crearFacciones();
      const { asentamiento: base, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
      const minaCobre: Edificio = {
        id: 'mina-test',
        tipo: 'minaCobre',
        posicion: { x: base.posicion.x + base.radioPotencial * 0.95, y: base.posicion.y },
        estado: 'activo',
        ticksRestantes: 0,
      };

      function fundicionPropuesta(conPolitica: boolean): { x: number; y: number } {
        let asentamiento: Asentamiento = {
          ...base,
          edificios: [...base.edificios, minaCobre],
          almacen: { ...base.almacen, cobre: { cantidad: 10, capacidad: 200 }, piedra: { cantidad: 200, capacidad: 200 } },
        };
        if (conPolitica) {
          asentamiento = { ...asentamiento, cargos: { ...asentamiento.cargos, maestroObrasId: 'jugador-test' } };
          const faccion = facs.find((f) => f.id === 'faccion-1')!;
          asentamiento = activarPolitica(asentamiento, faccion, 'maestroObras', 'lineas_produccion', 1);
        }
        let estado: EstadoSimulacion = {
          asentamientos: [asentamiento],
          facciones: facs,
          caravanas: [],
          acuerdos: [],
          ordenes: [],
          relaciones: [],
          titulos: [],
          caminos: [],
          campamentosBandidos: [],
          bandidosProximoSpawnTick: 0,
        };
        // 100, no 40: con el sitio de fundación real de esta seed, ambas ramas tardan ~76-77 ticks en
        // encontrarle sitio a Fundición detrás de Armería (solo una transformación en vuelo a la vez, ver
        // comentario en `construction.ts`) — margen para que no dependa del filo exacto del fixture.
        for (let tick = 1; tick <= 100; tick++) {
          estado = avanzarSimulacion(estado, mapa, tick);
          const fundicion = estado.asentamientos[0]!.edificios.find((e) => e.tipo === 'fundicion');
          if (fundicion) return fundicion.posicion;
        }
        throw new Error('Fundición nunca se propuso en 100 ticks — revisa el fixture del test.');
      }

      const posicionConPolitica = fundicionPropuesta(true);
      const posicionSinPolitica = fundicionPropuesta(false);

      expect(dist(posicionConPolitica, minaCobre.posicion)).toBeLessThanOrEqual(dist(posicionSinPolitica, minaCobre.posicion));
    } finally {
      restaurarMathRandom();
    }
  });
});
