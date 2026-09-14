// Fixtures compartidas para los tests de regresión del motor: construyen mapa/facción/asentamiento
// usando las funciones REALES del motor (generarMapa/crearFaccion/fundarAsentamiento), nunca objetos
// inventados a mano — así un test que pasa hoy sigue significando "el motor real produce esto".
import type { Asentamiento, Escuadron, Faccion, Heroe, UbicacionHeroe } from '../../domain/types';
import { instante, type Instante } from '../../domain/tiempo';
import { generarMapa, MAPA_DEFAULT, type RandomFn } from '../../worldgen';
import { crearMapa, type Mapa } from '../../world/mapa';
import { LIDERAZGO, SIMULACION } from '../../constants';
import { crearFaccion } from '../faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../settlement';
import type { ContextoSimulacion, EstadoSimulacion } from '../simulation';
import { PROGRESION_INICIAL } from '../tropas';
import { progresionInicial } from '../heroe';

/** Un héroe humano de prueba. Su `jugadorId` es su propio id, así que un test actúa con `{ actor: id }`. */
export function heroeDePrueba(id: string, ubicacion: UbicacionHeroe, extra: Partial<Heroe> = {}): Heroe {
  return {
    id,
    jugadorId: id,
    controlador: 'humano',
    displayName: id,
    classDefinitionId: 'Spear',
    genero: 'masculino',
    avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
    liderazgoBase: LIDERAZGO.base,
    ubicacion,
    escuadrones: [],
    ...progresionInicial(id),
    ...extra,
  };
}

/** Un escuadrón de prueba, por defecto en el campamento de su héroe. */
export function escuadronDePrueba(
  id: string,
  heroeId: string,
  tropaId = 'milicia_lanceros',
  cantidad = 10,
  extra: Partial<Escuadron> = {}
): Escuadron {
  return {
    id,
    nombre: tropaId,
    heroeId,
    origen: 'pesants',
    cantidad,
    ...PROGRESION_INICIAL,
    moral: 100,
    tropaId,
    contenedor: { tipo: 'campamento' },
    enGuarnicion: false,
    ...extra,
  };
}

/** Los héroes dueños de estas escuadras —uno por `heroeId`, con las suyas dentro—, más los indicados sin tropa.
 * Las escuadras viven en su héroe (`engine/tropa.ts`): sin su registro no hay dónde guardarlas. */
export function heroesCon(escuadrones: readonly Escuadron[], sinTropa: readonly string[] = []): Heroe[] {
  const porHeroe = new Map<string, Escuadron[]>(sinTropa.map((id) => [id, []]));
  for (const e of escuadrones) porHeroe.set(e.heroeId, [...(porHeroe.get(e.heroeId) ?? []), e]);
  return [...porHeroe].map(([id, suyas]) => heroeDePrueba(id, { tipo: 'desconectado', punto: { x: 0, y: 0 } }, { escuadrones: suyas }));
}

export function crearMapaDeterminista(seed: number): Mapa {
  return crearMapa(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed }));
}

const EPOCA_MS = new Date(SIMULACION.epocaInicial).getTime();

/** Instante de mundo de un `tick`, misma fórmula que `instanteDeTick` (`session/estado.ts`, Fase D / doc 10)
 * — para que un test que razona en ticks pueda pasar el `Instante` que el motor ahora espera. */
export function instanteDeTest(tick = 0): Instante {
  return instante(EPOCA_MS + tick * SIMULACION.duracionTickMs);
}

/**
 * `ContextoSimulacion` para tests, con `instante`/`momento` DERIVADOS DEL TICK y nunca del reloj real: el
 * motor tiene que ser reproducible (ver `determinismo.test.ts`), así que un `Date.now()` aquí haría divergir
 * dos corridas idénticas. Pasar el MISMO `rng` en todos los ticks de una corrida — es una secuencia con
 * estado, no una fábrica.
 */
export function contextoDeTest(tick: number, rng: RandomFn): ContextoSimulacion {
  const i = instanteDeTest(tick);
  return { instante: i, momento: new Date(i).toISOString(), rng };
}

/**
 * Barre una grilla regular buscando una posición "recomendable" (fundable + bosque alcanzable, ver
 * `evaluarViabilidadFundacion`) — evita que los tests dependan de que el seed elegido a mano tenga
 * un bosque cerca del origen (0,0).
 */
export function posicionRecomendable(
  mapa: Mapa,
  asentamientosExistentes: Asentamiento[] = [],
  paso = 40
): { x: number; y: number } {
  for (let x = paso; x < mapa.limites.ancho; x += paso) {
    for (let y = paso; y < mapa.limites.alto; y += paso) {
      const posicion = { x, y };
      if (evaluarViabilidadFundacion(mapa, posicion, asentamientosExistentes).recomendable) return posicion;
    }
  }
  throw new Error('No se encontró posición recomendable en la grilla de test — revisa el seed/paso.');
}

/** Funda un asentamiento de un solo jugador en una posición recomendable (o la indicada), vía el motor real. */
export function fundarAsentamientoDeTest(
  mapa: Mapa,
  facciones: Faccion[],
  faccionId: string,
  asentamientosExistentes: Asentamiento[],
  tickActual = 0,
  posicion?: { x: number; y: number }
): { asentamiento: Asentamiento; facciones: Faccion[] } {
  const pos = posicion ?? posicionRecomendable(mapa, asentamientosExistentes);
  return fundarAsentamiento(mapa, facciones, faccionId, pos, [`jugador-${faccionId}-1`], asentamientosExistentes, instanteDeTest(tickActual));
}

export function crearFacciones(): Faccion[] {
  return [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
}

/**
 * `EstadoSimulacion` de test: `asentamientos`/`facciones` son lo único que varía de un test a otro en toda
 * la suite (revisión de duplicación 2026-08-25) — los otros 8 campos SIEMPRE arrancan vacíos/en cero, y ese
 * objeto literal de 10 campos estaba copiado tal cual en 13+ archivos. `overrides` cubre el caso — hoy
 * inexistente, pero no imposible — de un test que necesite arrancar con caravanas u órdenes ya puestas.
 */
export function crearEstadoDeTest(
  asentamientos: Asentamiento[],
  facciones: Faccion[],
  overrides: Partial<Omit<EstadoSimulacion, 'asentamientos' | 'facciones'>> = {}
): EstadoSimulacion {
  return {
    asentamientos,
    facciones,
    caravanas: [],
    ejercitos: [],
    memoriaPorFaccion: {},
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    caminos: [],
    campamentosBandidos: [],
    bandidosProximoSpawnEn: instanteDeTest(0),
    heroes: [],
    ...overrides,
  };
}
