// Una fuente del mapa, un dueño.
//
// Las zonas de influencia de facciones DISTINTAS se recortan entre sí y nunca se solapan (ver
// `engine/zones.ts`), así que dos rivales no pueden disputarse el mismo yacimiento. Pero dos asentamientos
// de la MISMA facción sí solapan zona, y la asignación de fuentes se calculaba mirando solo los edificios
// del propio asentamiento: los dos veían el mismo nodo libre y los dos le plantaban un extractor encima.
// El yacimiento se drenaba al doble de velocidad y ambos se quedaban sin recurso mucho antes de lo previsto.
//
// Estas pruebas fijan el comportamiento correcto: el conteo de fuentes tomadas es GLOBAL (ver
// `reclamosDeFuentes`) y se actualiza según se compromete cada obra, también dentro de un mismo tick.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { avanzarSimulacion, type EstadoSimulacion } from '../simulation';
import { reclamosDeFuentes } from '../construction';
import { evaluarViabilidadFundacion } from '../settlement';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

// Antes SEED=7: con el reordenamiento de PRNG de WORLDGEN_VERSION v6 (fertilidad antes que bosques, ver
// `worldgen/types.ts`) el mundo de esa seed desplazó los nodos minerales lejos del primer par de
// emplazamientos que encuentra el barrido de `dosAsentamientosDeLaMismaFaccion` — ninguno llegaba a
// construir un extractor en 200 ticks y el guard de "test vacío" (línea ~106) saltaba. 42 es la seed que ya
// usan los tests de determinismo del propio generador (ver `worldgen_caracterizacion.test.ts`) y sí produce
// mineral alcanzable para ambos asentamientos.
const SEED = 42;
const TICKS = 200;

/** Extractores (no Leñeras) agrupados por la fuente que explotan: fuenteId -> ids de asentamiento. */
function extractoresPorFuente(asentamientos: Asentamiento[]): Map<string, string[]> {
  const porFuente = new Map<string, string[]>();
  for (const asentamiento of asentamientos) {
    for (const edificio of asentamiento.edificios) {
      if (!edificio.fuenteId || edificio.tipo === 'lenera') continue;
      const lista = porFuente.get(edificio.fuenteId) ?? [];
      lista.push(asentamiento.id);
      porFuente.set(edificio.fuenteId, lista);
    }
  }
  return porFuente;
}

/**
 * Dos asentamientos de la MISMA facción, lo bastante cerca como para que sus zonas acaben solapándose
 * (el radio potencial crece hasta 60 en nivel 1 y más luego, ver ZONA_INFLUENCIA) pero sin que el segundo
 * caiga dentro de la zona inicial del primero, que lo impediría al fundar.
 */
function dosAsentamientosDeLaMismaFaccion() {
  const mapa = crearMapaDeterminista(SEED);
  // El cap de fundación en nivel 1 es de 1 asentamiento por Facción (CAP_FUNDACION_POR_NIVEL): se sube el
  // nivel de la Facción para poder fundar dos. Lo que se prueba aquí es la disputa de fuentes, no el cap.
  const facciones = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 3 } : f));

  // Par de emplazamientos separados 70 unidades: lo bastante lejos para que el segundo no caiga dentro de
  // la zona inicial del primero (radio 30) y lo bastante cerca para que ambas zonas acaben solapándose al
  // crecer (hasta 60 en nivel 1), que es cuando aparece la disputa por un mismo nodo. El primero se filtra
  // por `recomendable` (fundable + bosque alcanzable, igual que `posicionRecomendable`): desde Fase 0.1 el
  // mapa ya no tiene recursos repartidos uniformemente, así que el primer hueco fundable de la rejilla ya
  // no garantiza tener nada extraíble cerca — sin este filtro el test queda vacío (nunca se construye nada).
  const { ancho, alto } = mapa.limites;
  for (let x = 60; x < ancho - 100; x += 20) {
    for (let y = 60; y < alto - 100; y += 20) {
      if (!evaluarViabilidadFundacion(mapa, { x, y }, []).recomendable) continue;
      try {
        const primero = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, { x, y });
        const existentes = [primero.asentamiento];
        const segundo = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', existentes, 0, { x: x + 70, y });
        return { mapa, facciones, asentamientos: [primero.asentamiento, segundo.asentamiento] };
      } catch {
        continue;
      }
    }
  }
  throw new Error('No se encontró un par de emplazamientos válidos para el test — revisa la seed.');
}

describe('reclamo de fuentes del mapa', () => {
  let restaurarMathRandom: () => void;

  beforeEach(() => {
    restaurarMathRandom = mockMathRandomDeterminista(SEED);
  });

  afterEach(() => {
    restaurarMathRandom();
  });

  it('ningún yacimiento acaba explotado por dos asentamientos a la vez', () => {
    const { mapa, facciones, asentamientos } = dosAsentamientosDeLaMismaFaccion();

    let estado: EstadoSimulacion = {
      asentamientos,
      facciones,
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
    };

    for (let tick = 1; tick <= TICKS; tick++) {
      estado = avanzarSimulacion(estado, mapa, tick);

      for (const [fuenteId, duenos] of extractoresPorFuente(estado.asentamientos)) {
        const distintos = new Set(duenos);
        expect(distintos.size, `tick ${tick}: el yacimiento ${fuenteId} lo explotan ${[...distintos].join(' y ')}`).toBe(1);
        // Tampoco dos extractores del MISMO asentamiento sobre el mismo nodo.
        expect(duenos, `tick ${tick}: ${fuenteId} tiene ${duenos.length} extractores`).toHaveLength(1);
      }
    }

    // El test sería vacuo si nunca se hubiera construido un extractor.
    const fuentes = extractoresPorFuente(estado.asentamientos);
    expect(fuentes.size).toBeGreaterThan(0);
  });

  it('ningún bosque supera su capacidad de Leñeras sumando todos los asentamientos', () => {
    const { mapa, facciones, asentamientos } = dosAsentamientosDeLaMismaFaccion();

    let estado: EstadoSimulacion = {
      asentamientos,
      facciones,
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
    };

    let lenerasVistas = 0;
    for (let tick = 1; tick <= TICKS; tick++) {
      estado = avanzarSimulacion(estado, mapa, tick);

      const { lenerasPorBosque } = reclamosDeFuentes(estado.asentamientos);
      lenerasVistas = Math.max(lenerasVistas, [...lenerasPorBosque.values()].reduce((a, b) => a + b, 0));
      for (const [bosqueId, cantidad] of lenerasPorBosque) {
        expect(cantidad, `tick ${tick}: el bosque ${bosqueId} tiene ${cantidad} Leñeras`).toBeLessThanOrEqual(
          mapa.capacidadLeneras(bosqueId)
        );
      }
    }

    expect(lenerasVistas).toBeGreaterThan(0);
  });

  it('reclamosDeFuentes agrega los edificios de todos los asentamientos', () => {
    const { asentamientos } = dosAsentamientosDeLaMismaFaccion();
    const conExtractor: Asentamiento[] = [
      {
        ...asentamientos[0]!,
        edificios: [
          ...asentamientos[0]!.edificios,
          { id: 'e1', tipo: 'cantera', posicion: { x: 0, y: 0 }, estado: 'activo', ticksRestantes: 0, fuenteId: 'nodo-x' },
          { id: 'e2', tipo: 'lenera', posicion: { x: 0, y: 0 }, estado: 'activo', ticksRestantes: 0, fuenteId: 'bosque-y' },
        ],
      },
      {
        ...asentamientos[1]!,
        edificios: [
          ...asentamientos[1]!.edificios,
          { id: 'e3', tipo: 'lenera', posicion: { x: 0, y: 0 }, estado: 'en_cola', ticksRestantes: 5, fuenteId: 'bosque-y' },
        ],
      },
    ];

    const reclamos = reclamosDeFuentes(conExtractor);
    expect(reclamos.nodos.has('nodo-x')).toBe(true);
    // Las Leñeras se cuentan, no se marcan: un bosque grande admite varias (a diferencia de un yacimiento).
    expect(reclamos.lenerasPorBosque.get('bosque-y')).toBe(2);
    // Cuenta también lo que todavía está en cola: ya está pagado y tiene la fuente asignada.
    expect([...reclamos.nodos].every((id) => typeof id === 'string')).toBe(true);
  });
});
