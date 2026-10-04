// Campamentos de bandidos (`engine/bandidos.ts`, Doc 1.9) — fusiona lo que antes eran dos archivos
// separados (revisión de duplicación 2026-08-25: mismo subsistema, `avanzarSpawnBandidos`), uno por cada
// bug/revisión real que motivó su prueba.
import { describe, expect, it } from 'vitest';
import { createRng } from '../../worldgen';
import { computeTodasLasZonas } from '../zones';
import { agendarReaparicionBandidos, avanzarSpawnBandidos, botinDeBandidos } from '../bandidos';
import { colocarCampamentosIniciales } from '../mercenarios';
import { evaluarViabilidadFundacion } from '../settlement';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, heroeDePrueba, instanteDeTest } from './fixtures';

const SEED = 42;

// ---------------------------------------------------------------------------------------------------------
// Reaparición por asentamiento (Doc 1.9; decisión del usuario 2026-09-28): cada uno lleva su plazo, y el suyo
// reaparece junto a él. Con un plazo único para todo el mundo, el campamento nuevo iba siempre al primer asentamiento
// de la lista, y en la Era I medida la Facción 1 se quedaba con la experiencia de todos los bandidos del mapa.
// ---------------------------------------------------------------------------------------------------------
describe('reaparición de campamentos de bandidos', () => {
  it('no repone el de un asentamiento antes de su plazo, y lo repone justo al cumplirse', () => {
    const mapa = crearMapaDeterminista(SEED);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', [], 0);
    const zonas = computeTodasLasZonas([asentamiento]);

    expect(avanzarSpawnBandidos([], zonas, [asentamiento], mapa, instanteDeTest(1), createRng(1)).campamentos, 'sin plazo, lo recibe ya').toHaveLength(1);

    const destruido = avanzarSpawnBandidos([], zonas, [asentamiento], mapa, instanteDeTest(1), createRng(1)).campamentos[0]!;
    const conPlazo = agendarReaparicionBandidos([asentamiento], destruido, instanteDeTest(10));
    const plazo = 10 + CAMPAMENTOS_BANDIDOS.respawnMinutos;
    expect(avanzarSpawnBandidos([], zonas, conPlazo, mapa, instanteDeTest(plazo - 1), createRng(1)).campamentos).toHaveLength(0);
    const repuesto = avanzarSpawnBandidos([], zonas, conPlazo, mapa, instanteDeTest(plazo), createRng(1)).campamentos;
    expect(repuesto).toHaveLength(1);
    expect(repuesto[0]?.asentamientoId).toBe(asentamiento.id);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Uno por asentamiento, siempre (antes `bandidos_spawn_por_asentamiento.test.ts`)
//
// Bug real detectado jugando (a petición del usuario): con varios asentamientos cercanos entre sí, un
// criterio anterior de "cobertura por distancia" (cualquier campamento a ≤600u ya "atendía" a un
// asentamiento vecino) dejaba a esos vecinos sin campamento propio para siempre — con 3 asentamientos
// activos solo llegaba a aparecer 1, nunca los 3 que exige el diseño (Doc 1.9: "UNO por asentamiento").
// Este test fija el comportamiento correcto: cada asentamiento recibe SIEMPRE su propio campamento
// (`asentamientoId`), sin importar lo cerca que esté de otro ya atendido.
// ---------------------------------------------------------------------------------------------------------
describe('spawn de campamentos de bandidos: uno por asentamiento, siempre', () => {
  // Techo generoso de ticks para dejar que el respawn (uno por tick, ver `avanzarSpawnBandidos`) alcance a
  // cubrir los 3 asentamientos aunque el primer bosque libre de cada uno tarde varios intentos en aparecer.
  const TICKS_MAXIMOS = 500;

  /**
   * Tres asentamientos de Facciones DISTINTAS, apretados en fila (70 unidades entre cada par — igual que
   * `reclamo_de_fuentes.test.ts`: bastante para no caer dentro de la zona inicial del vecino, bastante poco
   * para que los 3 hubieran quedado "cubiertos" por un único campamento bajo el criterio de distancia viejo).
   */
  function tresAsentamientosApretados() {
    const mapa = crearMapaDeterminista(SEED);
    const facciones = crearFacciones();
    const { ancho, alto } = mapa.limites;

    for (let x = 60; x < ancho - 200; x += 20) {
      for (let y = 60; y < alto - 100; y += 20) {
        if (!evaluarViabilidadFundacion(mapa, { x, y }, []).recomendable) continue;
        try {
          const primero = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, { x, y });
          const existentes1 = [primero.asentamiento];
          const segundo = fundarAsentamientoDeTest(mapa, facciones, 'faccion-2', existentes1, 0, { x: x + 70, y });
          const existentes2 = [...existentes1, segundo.asentamiento];
          const tercero = fundarAsentamientoDeTest(mapa, facciones, 'faccion-3', existentes2, 0, { x: x + 140, y });
          return { mapa, asentamientos: [primero.asentamiento, segundo.asentamiento, tercero.asentamiento] };
        } catch {
          continue;
        }
      }
    }
    throw new Error('No se encontró un trío de emplazamientos apretados válido para el test — revisa la seed.');
  }

  it('cubre a los 3 asentamientos con su propio campamento aunque estén muy cerca entre sí', () => {
    const { mapa, asentamientos } = tresAsentamientosApretados();

    let campamentos: ReturnType<typeof avanzarSpawnBandidos>['campamentos'] = [];
    for (let tick = 1; tick <= TICKS_MAXIMOS && campamentos.length < asentamientos.length; tick++) {
      const zonas = computeTodasLasZonas(asentamientos);
      const resultado = avanzarSpawnBandidos(campamentos, zonas, asentamientos, mapa, instanteDeTest(tick), createRng(1));
      campamentos = resultado.campamentos;
    }

    expect(campamentos).toHaveLength(asentamientos.length);
    const asentamientosCubiertos = new Set(campamentos.map((c) => c.asentamientoId));
    for (const asentamiento of asentamientos) {
      expect(asentamientosCubiertos.has(asentamiento.id)).toBe(true);
    }
    // Ningún asentamiento recibe un segundo campamento mientras otro se queda sin ninguno.
    expect(asentamientosCubiertos.size).toBe(asentamientos.length);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Bandidos de los campamentos de mercenarios (D21, D37, D42, D28) y su botín (D22, D26, D27).
// ---------------------------------------------------------------------------------------------------------
describe('el anillo de un campamento de mercenarios', () => {
  const mapa = crearMapaDeterminista(SEED);
  const [mercenarios] = colocarCampamentosIniciales(mapa, instanteDeTest(0));
  const conResidentes = (n: number) => ({ ...mercenarios!, residentesIds: Array.from({ length: n }, (_, i) => `h${i}`) });

  it('aparecen según los residentes sin asentamiento, uno cada plazo, con nivel y a la distancia del anillo', () => {
    const rng = createRng(1);
    const m = conResidentes(3); // 3 sin Facción → 2 campamentos
    const primero = avanzarSpawnBandidos([], [], [], mapa, instanteDeTest(0), rng, [m], []);
    expect(primero.campamentos).toHaveLength(1);
    const c = primero.campamentos[0]!;
    expect(c.campamentoMercenariosId).toBe(m.id);
    expect([1, 2, 3]).toContain(c.nivel);
    expect(c.poder).toBe(CAMPAMENTOS_BANDIDOS.niveles[c.nivel].poder);
    const d = Math.hypot(c.posicion.x - m.posicion.x, c.posicion.y - m.posicion.y);
    expect(d).toBeGreaterThanOrEqual(CAMPAMENTOS_BANDIDOS.anillo.radioMin);
    expect(d).toBeLessThanOrEqual(CAMPAMENTOS_BANDIDOS.anillo.radioMax);

    const plazo = CAMPAMENTOS_BANDIDOS.anillo.reaparicionMinutos;
    const antes = avanzarSpawnBandidos(primero.campamentos, [], [], mapa, instanteDeTest(plazo - 1), rng, primero.mercenarios, []);
    expect(antes.campamentos, 'antes del plazo, no').toHaveLength(1);
    const segundo = avanzarSpawnBandidos(primero.campamentos, [], [], mapa, instanteDeTest(plazo), rng, primero.mercenarios, []);
    expect(segundo.campamentos).toHaveLength(2);
    const tercero = avanzarSpawnBandidos(segundo.campamentos, [], [], mapa, instanteDeTest(2 * plazo), rng, segundo.mercenarios, []);
    expect(tercero.campamentos, 'ya están los que tocan').toHaveLength(2);
  });

  it('siempre al menos uno, aunque no haya nadie', () => {
    expect(avanzarSpawnBandidos([], [], [], mapa, instanteDeTest(0), createRng(1), [conResidentes(0)], []).campamentos).toHaveLength(1);
  });
});

describe('el botín de bandidos', () => {
  const t = (minutos: number) => instanteDeTest(minutos);
  const oroDe = (destruidos: number, nivel: 1 | 2 | 3 = 1) => {
    const previas = Array.from({ length: destruidos }, (_, i) => t(i));
    const heroe = heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'c' }, { bandidosDestruidosEn: previas });
    return botinDeBandidos([heroe], ['h1'], nivel, t(60)).oro['h1'];
  };

  it('oro del nivel por héroe, a su oro de botín, para cada héroe de la columna', () => {
    const heroes = ['h1', 'h2'].map((id) => heroeDePrueba(id, { tipo: 'columna', ejercitoId: 'c' }));
    const r = botinDeBandidos(heroes, ['h1', 'h2'], 2, t(0));
    expect(r.heroes.map((h) => h.oroDeBotin)).toEqual([CAMPAMENTOS_BANDIDOS.niveles[2].oroPorHeroe, CAMPAMENTOS_BANDIDOS.niveles[2].oroPorHeroe]);
    expect(r.heroes[0]!.almacenPersonal?.['oro'], 'no al almacén').toBeUndefined();
  });

  it('rendimientos decrecientes en 24 h (D26): completo, luego a menos, luego solo experiencia', () => {
    const { completas, caidaPorCada, soloExperienciaDesde } = CAMPAMENTOS_BANDIDOS.rendimientos;
    const base = CAMPAMENTOS_BANDIDOS.niveles[1].oroPorHeroe;
    expect(oroDe(completas - 1)).toBe(base);
    expect(oroDe(completas)).toBe(Math.round(base * (1 - caidaPorCada)));
    expect(oroDe(soloExperienciaDesde - 1)).toBe(0);
    // Lo de hace más de 24 h no cuenta.
    const viejo = heroeDePrueba('h1', { tipo: 'columna', ejercitoId: 'c' }, { bandidosDestruidosEn: Array.from({ length: 20 }, () => t(0)) });
    expect(botinDeBandidos([viejo], ['h1'], 1, t(25 * 60)).oro['h1']).toBe(base);
  });
});
