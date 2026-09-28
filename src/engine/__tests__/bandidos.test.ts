// Campamentos de bandidos (`engine/bandidos.ts`, Doc 1.9) — fusiona lo que antes eran dos archivos
// separados (revisión de duplicación 2026-08-25: mismo subsistema, `avanzarSpawnBandidos`), uno por cada
// bug/revisión real que motivó su prueba.
import { describe, expect, it } from 'vitest';
import { computeTodasLasZonas } from '../zones';
import { agendarReaparicionBandidos, avanzarSpawnBandidos } from '../bandidos';
import { evaluarViabilidadFundacion } from '../settlement';
import { CAMPAMENTOS_BANDIDOS } from '../../constants';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

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

    expect(avanzarSpawnBandidos([], zonas, [asentamiento], mapa, instanteDeTest(1)).campamentos, 'sin plazo, lo recibe ya').toHaveLength(1);

    const destruido = avanzarSpawnBandidos([], zonas, [asentamiento], mapa, instanteDeTest(1)).campamentos[0]!;
    const conPlazo = agendarReaparicionBandidos([asentamiento], destruido, instanteDeTest(10));
    const plazo = 10 + CAMPAMENTOS_BANDIDOS.respawnMinutos;
    expect(avanzarSpawnBandidos([], zonas, conPlazo, mapa, instanteDeTest(plazo - 1)).campamentos).toHaveLength(0);
    const repuesto = avanzarSpawnBandidos([], zonas, conPlazo, mapa, instanteDeTest(plazo)).campamentos;
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
      const resultado = avanzarSpawnBandidos(campamentos, zonas, asentamientos, mapa, instanteDeTest(tick));
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
