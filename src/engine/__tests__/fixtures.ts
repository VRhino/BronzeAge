// Fixtures compartidas para los tests de regresión del motor: construyen mapa/facción/asentamiento
// usando las funciones REALES del motor (generarMapa/crearFaccion/fundarAsentamiento), nunca objetos
// inventados a mano — así un test que pasa hoy sigue significando "el motor real produce esto".
import type { Asentamiento, Edificio, EdificioTipo, Escuadron, Faccion, Heroe, UbicacionHeroe } from '../../domain/types';
import { instante, type Instante } from '../../domain/tiempo';
import { generarMapa, MAPA_DEFAULT, type RandomFn } from '../../worldgen';
import { crearMapa, type Mapa } from '../../world/mapa';
import { EDIFICIO_CATALOGO, LIDERAZGO, MURALLA, SIMULACION } from '../../constants';
import { crearFaccion } from '../faccion';
import { evaluarViabilidadFundacion, fundarAsentamiento } from '../settlement';
import type { ContextoSimulacion, EstadoSimulacion } from '../simulation';
import { PROGRESION_INICIAL } from '../tropas';
import { progresionInicial } from '../heroe';
import { estadoTecnologiaInicial, TODAS_LAS_TECNOLOGIAS } from '../tecnologia';
import type { EstadoTecnologia } from '../../domain/types';

/** Estado de tecnología con todo el catálogo adoptado por esas Facciones. */
export function conTodoAdoptado(estado: EstadoTecnologia, facciones: readonly Faccion[]): EstadoTecnologia {
  const todas = [...TODAS_LAS_TECNOLOGIAS];
  return { ...estado, porFaccion: Object.fromEntries(facciones.map((f) => [f.id, { aparecidas: todas, adoptadas: todas }])) };
}

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
 * Barre una grilla regular buscando una posición "recomendable" (fundable + bosque libre + piedra en el radio,
 * ver `evaluarViabilidadFundacion`) — evita que los tests dependan de que el seed elegido a mano tenga
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
  const fundado = fundarAsentamiento(mapa, facciones, faccionId, pos, [`jugador-${faccionId}-1`], asentamientosExistentes, instanteDeTest(tickActual));
  // Sin la protección con la que nace (Doc 5.12.9): la suite asedia plazas recién fundadas; quien la prueba, la pone.
  const { protegidaHasta: _proteccion, ...sinProteccion } = fundado.asentamiento;
  return { ...fundado, asentamiento: sinProteccion };
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
    // Todas adoptadas: la suite del motor prueba cada regla por separado; la tecnología, en `tecnologia.test.ts`.
    tecnologia: conTodoAdoptado(estadoTecnologiaInicial(instante(0)), facciones),
    acuerdos: [],
    ordenes: [],
    relaciones: [],
    titulos: [],
    campamentosBandidos: [],
    campamentosMercenarios: [],
    heroes: [],
    ...overrides,
  };
}

/**
 * `asentamiento` con todo lo que pide subir a nivel 2 (engine/ascenso.ts): 200 pesants y 3 tipos de extracción
 * (gate), almacén que cubre la obra, y producción de sobra para el mantenimiento del nivel 2 — 2 Canteras, 2 Minas y 2
 * Leñeras sobre fuentes REALES de `mapa`, porque la solvencia se calcula con `produccionPorMinuto`, que lee el mapa.
 */
export function prepararParaSubirANivel2(asentamiento: Asentamiento, mapa: Mapa): Asentamiento {
  const piedra = mapa.listarNodos().filter((n) => n.tipo === 'piedra');
  const oro = mapa.listarNodos().filter((n) => n.tipo === 'oro');
  const bosques = mapa.listarBosques();
  const extractor = (id: string, tipo: EdificioTipo, fuenteId: string): Edificio => ({
    id,
    tipo,
    posicion: { x: 0, y: 0 },
    estado: 'activo',
    ambito: 'mapa',
    fuenteId,
  });
  const almacen = { ...asentamiento.almacen };
  for (const recurso of ['madera', 'piedra', 'oro']) almacen[recurso] = { cantidad: 5000, capacidad: 10000 };
  return {
    ...asentamiento,
    poblacion: { ...asentamiento.poblacion, pesants: 200 },
    almacen,
    edificios: [
      ...asentamiento.edificios,
      extractor(`${asentamiento.id}-c1`, 'cantera', piedra[0]!.id),
      extractor(`${asentamiento.id}-c2`, 'cantera', piedra[1]!.id),
      extractor(`${asentamiento.id}-m1`, 'mina', oro[0]!.id),
      extractor(`${asentamiento.id}-m2`, 'mina', oro[1]!.id),
      extractor(`${asentamiento.id}-l1`, 'lenera', bosques[0]!.id),
      extractor(`${asentamiento.id}-l2`, 'lenera', bosques[1]!.id),
    ],
  };
}

/**
 * Divide por `factor` los tiempos de obra del catálogo y de la muralla, y devuelve cómo restaurarlos. Para tests que
 * necesitan una ciudad CRECIDA y miran QUÉ se construye y DÓNDE, no CUÁNDO (trazado, murallas, perfiles, gates de
 * construcción): desde el 2026-09-26 las obras tardan horas (`Ritmo_Crecimiento_Asentamientos.md` §11), y crecer una
 * ciudad a tiempo real haría la suite inasumible. Con el 60 por defecto los tiempos vuelven a rondar los de antes.
 * **Los tests de ritmo no deben usarlo.** Uso: `beforeAll(() => { restaurar = acelerarObras(); })` y
 * `afterAll(() => restaurar())`, o envolviendo un solo test con try/finally.
 */
export function acelerarObras(factor = 60): () => void {
  const catalogo = EDIFICIO_CATALOGO as Record<string, { tiempoConstruccionMinutos: number }>;
  const obras = Object.entries(catalogo).map(([tipo, def]) => [tipo, def.tiempoConstruccionMinutos] as const);
  const celdas = { ...MURALLA.minutosPorCelda };
  for (const [tipo, minutos] of obras) catalogo[tipo]!.tiempoConstruccionMinutos = Math.max(1, Math.round(minutos / factor));
  for (const [nivel, minutos] of Object.entries(celdas)) MURALLA.minutosPorCelda[Number(nivel)] = Math.max(1, Math.round(minutos / factor));
  return () => {
    for (const [tipo, minutos] of obras) catalogo[tipo]!.tiempoConstruccionMinutos = minutos;
    Object.assign(MURALLA.minutosPorCelda, celdas);
  };
}
