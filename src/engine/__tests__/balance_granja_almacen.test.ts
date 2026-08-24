// Dos reglas de balance pedidas por el usuario que no se ven en el snapshot de referencia (la partida de
// baseline no llega a mejorar Granjas ni a llenar el almacén dentro de sus ticks de corte), así que se fijan
// aquí explícitamente:
//
//  - El rinde de trigo de la Granja sube MUCHO más despacio que su costo: ×1 / ×1.5 / ×2 / ×3 sobre el nivel
//    1, no duplicando en cada salto.
//  - La capacidad de almacenamiento tiene techo: un tope de Almacenes por nivel de asentamiento.
import { beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { EDIFICIO_CATALOGO, NECESIDADES, NIVEL_ASENTAMIENTO, produccionTrigoDeGranja } from '../../constants';
import {
  alcanzoTopeDeAlmacenes,
  alcanzoTopeDeViviendas,
  anadirEdificioManualmente,
  ConstruccionManualInvalidaError,
  maximoViviendasPorNivel,
} from '../construction';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { createRng, type RandomFn } from '../../worldgen';
import { contextoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';

const SEED = 99;

describe('Granja: progresión del rinde de trigo', () => {
  it('multiplica sobre el NIVEL 1 (×1, ×1.5, ×2, ×3), no acumulando el doble en cada salto', () => {
    const base = produccionTrigoDeGranja(1);
    expect(base).toBe(EDIFICIO_CATALOGO.granja.produccionBaseTrigo);
    expect(produccionTrigoDeGranja(2)).toBeCloseTo(base * 1.5);
    expect(produccionTrigoDeGranja(3)).toBeCloseTo(base * 2);
    expect(produccionTrigoDeGranja(4)).toBeCloseTo(base * 3);
  });

  it('el costo sí duplica en cada salto, así que mejorar rinde cada vez menos por material invertido', () => {
    const niveles = EDIFICIO_CATALOGO.granja.niveles as Record<number, { costoMejora?: { madera?: number } }>;
    expect(niveles[2]!.costoMejora!.madera).toBe(EDIFICIO_CATALOGO.granja.costo.madera * 2);
    expect(niveles[3]!.costoMejora!.madera).toBe(EDIFICIO_CATALOGO.granja.costo.madera * 4);
    expect(niveles[4]!.costoMejora!.madera).toBe(EDIFICIO_CATALOGO.granja.costo.madera * 8);
  });

  it('sin nivel interno declarado rinde como nivel 1 (saves viejos sin `nivelInterno`)', () => {
    expect(produccionTrigoDeGranja(undefined)).toBe(produccionTrigoDeGranja(1));
  });
});

describe('Almacén: tope por nivel de asentamiento', () => {
  let rng: RandomFn;

  beforeEach(() => {
    rng = createRng(SEED);
  });

  function conAlmacenes(base: Asentamiento, cuantos: number, estado: Edificio['estado'] = 'activo'): Asentamiento {
    const almacenes: Edificio[] = Array.from({ length: cuantos }, (_, i) => ({
      id: `almacen-${base.id}-${i}`,
      tipo: 'almacen',
      posicion: { x: 30 + i * 12, y: 0 },
      estado,
      ticksRestantes: 0,
      ambito: 'asentamiento',
    }));
    return { ...base, edificios: [...base.edificios, ...almacenes] };
  }

  it('el tope se alcanza justo en el número declarado para el nivel', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    for (const [nivelTexto, tope] of Object.entries(NECESIDADES.maximoAlmacenesPorNivel)) {
      const nivel = Number(nivelTexto);
      // `alcanzoTopeDeAlmacenes` lee nivelActual (Doc Fase_0_5 §6.2), no nivel (nivelAlcanzado) — hay que
      // pisar los dos para simular "el asentamiento está en nivel X" a todos los efectos.
      const enNivel = { ...asentamiento, nivel, nivelActual: nivel };
      expect(alcanzoTopeDeAlmacenes(conAlmacenes(enNivel, tope - 1)), `nivel ${nivel}, ${tope - 1} almacenes`).toBe(false);
      expect(alcanzoTopeDeAlmacenes(conAlmacenes(enNivel, tope)), `nivel ${nivel}, ${tope} almacenes`).toBe(true);
    }
  });

  it('los que están en cola o en obra cuentan para el tope, no solo los activos', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const tope = NECESIDADES.maximoAlmacenesPorNivel[1]!;
    expect(alcanzoTopeDeAlmacenes(conAlmacenes({ ...asentamiento, nivel: 1 }, tope, 'en_cola'))).toBe(true);
  });

  it('la construcción manual rechaza el Almacén que pasaría del tope', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const tope = NECESIDADES.maximoAlmacenesPorNivel[1]!;
    const lleno: Asentamiento = {
      ...conAlmacenes({ ...asentamiento, nivel: 1 }, tope),
      cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' },
    };
    expect(() =>
      anadirEdificioManualmente(lleno, facs[0]!, 'gobernador', 'almacen', [], mapa, undefined, {
        nodos: new Set(),
        lenerasPorBosque: new Map(),
      })
    ).toThrow(ConstruccionManualInvalidaError);
  });

  /**
   * La auto-construcción de Almacenes solo se dispara con el recurso más lleno por encima de
   * `NECESIDADES.umbralAlmacenAmpliacion` (90%), y una partida normal nunca llega ahí — consume tan rápido
   * como produce. Así que la condición se fuerza: almacén a tope en todos los recursos. Sin esto el test
   * pasaría sin haber construido ni un Almacén, sin ejercer el tope.
   */
  function conAlmacenLleno(asentamiento: Asentamiento): Asentamiento {
    const almacen = Object.fromEntries(
      Object.entries(asentamiento.almacen).map(([recurso, r]) => [recurso, { ...r, cantidad: r.capacidad }])
    );
    return { ...asentamiento, almacen };
  }

  it('la auto-construcción tampoco pasa del tope, con el almacén desbordado tick tras tick', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicionRecomendable(mapa));
    let estado: EstadoSimulacion = {
      asentamientos: [conAlmacenLleno(asentamiento)],
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

    let maximoVisto = 0;
    for (let tick = 1; tick <= 200; tick++) {
      estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
      // Se vuelve a llenar cada tick: el consumo del propio tick lo bajaría del umbral enseguida.
      estado = { ...estado, asentamientos: estado.asentamientos.map(conAlmacenLleno) };
      for (const a of estado.asentamientos) {
        const almacenes = a.edificios.filter((e) => e.tipo === 'almacen').length;
        maximoVisto = Math.max(maximoVisto, almacenes);
        const tope = NECESIDADES.maximoAlmacenesPorNivel[a.nivel];
        if (tope === undefined) continue;
        expect(almacenes, `tick ${tick}, nivel ${a.nivel}`).toBeLessThanOrEqual(tope);
      }
    }
    // Tiene que llegar al tope, no solo construir alguno: si se quedara por debajo, el límite no se habría
    // puesto a prueba y el test volvería a ser vacuo.
    expect(maximoVisto, 'con el almacén lleno tiene que llegar al tope de su nivel').toBeGreaterThanOrEqual(
      NECESIDADES.maximoAlmacenesPorNivel[1]!
    );
  });
});

// Bug detectado por el usuario: no existía ningún límite de Viviendas por nivel de asentamiento — el tope se
// deriva de cuántos pesants exige el SIGUIENTE nivel (`NIVEL_ASENTAMIENTO.requisitos[nivel+1].pesants`) entre
// la capacidad de una Vivienda (`EDIFICIO_CATALOGO.vivienda.capacidadPesants`), ver `maximoViviendasPorNivel`.
describe('Vivienda: tope por nivel de asentamiento', () => {
  function conViviendas(base: Asentamiento, cuantas: number, estado: Edificio['estado'] = 'activo'): Asentamiento {
    const viviendas: Edificio[] = Array.from({ length: cuantas }, (_, i) => ({
      id: `vivienda-${base.id}-${i}`,
      tipo: 'vivienda',
      posicion: { x: 30 + i * 12, y: 0 },
      estado,
      ticksRestantes: 0,
      ambito: 'asentamiento',
    }));
    return { ...base, edificios: [...base.edificios, ...viviendas] };
  }

  it('el tope de nivel 1 es el mismo cálculo que hizo el usuario: 200 pesants ÷ 15/Vivienda = 14', () => {
    expect(NIVEL_ASENTAMIENTO.requisitos[2]!.pesants).toBe(200);
    expect(EDIFICIO_CATALOGO.vivienda.capacidadPesants).toBe(15);
    expect(maximoViviendasPorNivel(1)).toBe(14);
  });

  // Bug detectado por el usuario: el primer cálculo del tope solo miraba pesants (nivel 2→3: 500/15=34
  // Viviendas), lo que daba una capacidad de artesanos de 34×5=170 — por debajo de los 200 que exige nivel 3,
  // un DEADLOCK real (nunca se podía subir a nivel 3). Este test fija el invariante para TODOS los niveles
  // con requisito siguiente: el tope de Viviendas debe dar cupo suficiente para AMBOS, pesants y artesanos,
  // del gate al que apunta — así una futura recalibración de cifras no puede reintroducir el mismo deadlock.
  it('el tope de Viviendas nunca deja sin cupo suficiente ni a pesants ni a artesanos del siguiente nivel', () => {
    for (const nivelTexto of Object.keys(NIVEL_ASENTAMIENTO.requisitos)) {
      const nivelSiguiente = Number(nivelTexto);
      const nivel = nivelSiguiente - 1;
      const requisito = NIVEL_ASENTAMIENTO.requisitos[nivelSiguiente]!;
      const tope = maximoViviendasPorNivel(nivel);
      const capacidadPesants = tope * EDIFICIO_CATALOGO.vivienda.capacidadPesants;
      const capacidadArtesanos = tope * EDIFICIO_CATALOGO.vivienda.capacidadArtesanos;
      expect(capacidadPesants, `nivel ${nivel}: cupo de pesants`).toBeGreaterThanOrEqual(requisito.pesants);
      expect(capacidadArtesanos, `nivel ${nivel}: cupo de artesanos`).toBeGreaterThanOrEqual(requisito.artesanos);
    }
  });

  it('el tope se alcanza justo en el número calculado para cada nivel con requisito de nivel siguiente', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento: fundado } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    // Fundar ya trae `FUNDACION.viviendasIniciales` Viviendas activas — se descartan para partir de 0 y que
    // `conViviendas` controle el total exacto que se está probando.
    const asentamiento = { ...fundado, edificios: fundado.edificios.filter((e) => e.tipo !== 'vivienda') };

    for (const nivelTexto of Object.keys(NIVEL_ASENTAMIENTO.requisitos)) {
      const nivelSiguiente = Number(nivelTexto);
      const nivel = nivelSiguiente - 1;
      const tope = maximoViviendasPorNivel(nivel);
      const enNivel = { ...asentamiento, nivel, nivelActual: nivel };
      expect(alcanzoTopeDeViviendas(conViviendas(enNivel, tope - 1)), `nivel ${nivel}, ${tope - 1} viviendas`).toBe(false);
      expect(alcanzoTopeDeViviendas(conViviendas(enNivel, tope)), `nivel ${nivel}, ${tope} viviendas`).toBe(true);
    }
  });

  it('los que están en cola o en obra cuentan para el tope, no solo los activos', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento: fundado } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const asentamiento = { ...fundado, edificios: fundado.edificios.filter((e) => e.tipo !== 'vivienda') };
    const tope = maximoViviendasPorNivel(1);
    expect(alcanzoTopeDeViviendas(conViviendas({ ...asentamiento, nivel: 1, nivelActual: 1 }, tope, 'en_cola'))).toBe(true);
  });

  it('la construcción manual rechaza la Vivienda que pasaría del tope', () => {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { asentamiento: fundado, facciones: facs } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const base = { ...fundado, edificios: fundado.edificios.filter((e) => e.tipo !== 'vivienda') };
    const tope = maximoViviendasPorNivel(1);
    const lleno: Asentamiento = {
      ...conViviendas({ ...base, nivel: 1, nivelActual: 1 }, tope),
      cargos: { ...base.cargos, gobernadorId: 'jugador-faccion-1-1' },
    };
    expect(() =>
      anadirEdificioManualmente(lleno, facs[0]!, 'gobernador', 'vivienda', [], mapa, undefined, {
        nodos: new Set(),
        lenerasPorBosque: new Map(),
      })
    ).toThrow(ConstruccionManualInvalidaError);
  });

  /**
   * La auto-construcción llega al mismo `alcanzoTopeDeViviendas` que la manual (ver `evaluarNecesidades`,
   * construction.ts) — no se ejercita aquí con una simulación completa porque a nivel 1 el espacio físico del
   * trazado urbano (`radioPotencial`, independiente de este cambio) ya limita cuántas Viviendas caben mucho
   * antes de llegar al tope de población (14): en la práctica, a nivel 1 se topa primero con "sin sitio
   * libre" que con este tope. El tope de población importa a partir de niveles con más radio/espacio.
   */
  it('la auto-construcción de Vivienda deja de proponerse en cuanto se alcanza el tope', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento: fundado } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const tope = maximoViviendasPorNivel(1);
    const asentamiento = {
      ...conViviendas({ ...fundado, nivel: 1, nivelActual: 1, edificios: fundado.edificios.filter((e) => e.tipo !== 'vivienda') }, tope),
      poblacion: { ...fundado.poblacion, pesants: 100000 },
    };
    expect(alcanzoTopeDeViviendas(asentamiento)).toBe(true);
  });
});
