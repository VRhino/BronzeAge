// Red de seguridad para la EXTRACCIÓN de la generación de mundo a una pieza aparte (`src/worldgen/`).
// Este test no valida que el mundo generado sea "bueno" — captura EXACTAMENTE el que produce el motor hoy,
// para que el refactor pueda demostrar que no cambió nada. La generación es determinista por seed (mulberry32,
// ver `rng.ts`), así que cualquier alteración del ORDEN de consumo del RNG (añadir/quitar/reordenar una
// llamada a `randRange` en cualquier punto del pipeline) desplaza todo lo generado después y aparece aquí.
//
// Dos capas complementarias, a propósito:
//  1. SNAPSHOT: la foto exacta. Detecta el cambio, pero no dice si es legítimo — regenerarlo es una decisión
//     consciente que debe justificarse en el commit.
//  2. INVARIANTES: el contrato que debe seguir cumpliéndose aunque el snapshot se regenere a propósito
//     (ids únicos para los `fuenteId` de los edificios, nodos dentro del mapa, fertilidad normalizada...).
import { describe, expect, it } from 'vitest';
import { evaluarBioma, evaluarElevacion, evaluarFertilidad, evaluarTerreno, generarMapa, type MapaGenerado } from '../../worldgen';
import {
  BOSQUE,
  LIVESTOCK,
  MAPA_DEFAULT,
  RECURSO_BIOMA_PERMITIDO,
  RECURSO_RAREZA,
  RECURSO_TIPOS_POR_RAREZA,
  RIOS,
} from '../../worldgen/config';

/** Seeds fijos: 1 es el de la partida por defecto (`gameStore`), 42 el de los tests de determinismo, 7 uno suelto. */
const SEEDS = [1, 42, 7];

/** Resolución del muestreo del campo de fertilidad/elevación. 20x20 = 400 puntos, suficiente para que dos
 * campos distintos no puedan coincidir por casualidad, y compacto de leer en el snapshot. */
const CELDAS_FERTILIDAD = 20;

function crear(seed: number): MapaGenerado {
  return generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed });
}

function distancia(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Foto compacta y legible del mundo. Se serializa a strings con precisión fija en vez de volcar los objetos
 * crudos para que el diff del snapshot señale la línea exacta que cambió (qué nodo se movió, qué bosque
 * cambió de radio) en vez de un muro de JSON. 6 decimales es de sobra: un cambio real en el pipeline no
 * mueve un valor 1e-6, lo manda a otra parte del mapa.
 */
function digest(world: MapaGenerado) {
  const bosques = world.bosques.map(
    (b) => `${b.id} c=(${b.centro.x.toFixed(6)},${b.centro.y.toFixed(6)}) r=${b.radio.toFixed(6)} d=${b.densidad.toFixed(6)}`
  );

  const nodos = world.nodos.map(
    (n) => `${n.id} ${n.tipo}/${n.rareza} p=(${n.posicion.x.toFixed(6)},${n.posicion.y.toFixed(6)}) q=${n.cantidadInicial}`
  );

  const conteoPorTipo: Record<string, number> = {};
  for (const n of world.nodos) conteoPorTipo[n.tipo] = (conteoPorTipo[n.tipo] ?? 0) + 1;

  // Campo de fertilidad muestreado en el centro de cada celda, una fila por línea.
  const paso = world.config.ancho / CELDAS_FERTILIDAD;
  const fertilidad = Array.from({ length: CELDAS_FERTILIDAD }, (_, fila) =>
    Array.from({ length: CELDAS_FERTILIDAD }, (_, col) =>
      evaluarFertilidad(world.fertilidad, { x: (col + 0.5) * paso, y: (fila + 0.5) * paso }).toFixed(6)
    ).join(' ')
  );

  // `placeWithSpacing` evita los bosques por rejection sampling, pero si tras 30 intentos no encuentra hueco
  // coloca igualmente (mapa saturado). Cuántos nodos acaban dentro de un bosque es por tanto una CONSECUENCIA
  // del algoritmo, no una garantía — se captura como número para que un cambio de estrategia de colocación
  // se note aquí en vez de pasar desapercibido.
  const nodosDentroDeBosque = world.nodos.filter((n) =>
    world.bosques.some((b) => distancia(n.posicion, b.centro) < b.radio)
  ).length;

  // Campo de elevación muestreado igual que fertilidad — misma rejilla, mismo criterio de precisión.
  const elevacion = Array.from({ length: CELDAS_FERTILIDAD }, (_, fila) =>
    Array.from({ length: CELDAS_FERTILIDAD }, (_, col) =>
      evaluarElevacion(world.elevacion, { x: (col + 0.5) * paso, y: (fila + 0.5) * paso }).toFixed(6)
    ).join(' ')
  );

  const rios = world.rios.map((r) => {
    const extremo = r.puntos[r.puntos.length - 1]!;
    return `${r.id} n=${r.puntos.length} fin=(${extremo.x.toFixed(6)},${extremo.y.toFixed(6)}) lago=${r.terminaEnLago}`;
  });

  // Cumplimiento de bioma por tipo de recurso: informativo, NO se asume 100% — `colocarConEspaciado` tiene
  // el mismo fallback de "mapa saturado, coloca igual" que ya usa `nodosDentroDeBosque` arriba.
  const cumplimientoBiomaPorTipo: Record<string, string> = {};
  for (const tipo of Object.keys(RECURSO_BIOMA_PERMITIDO)) {
    const deEsteTipo = world.nodos.filter((n) => n.tipo === tipo);
    if (deEsteTipo.length === 0) continue;
    const permitido = RECURSO_BIOMA_PERMITIDO[tipo]!;
    const enBioma = deEsteTipo.filter((n) => permitido.includes(evaluarBioma(world.elevacion, world.fertilidad, world.rios, n.posicion))).length;
    cumplimientoBiomaPorTipo[tipo] = `${enBioma}/${deEsteTipo.length}`;
  }

  return { conteoPorTipo, nodosDentroDeBosque, cumplimientoBiomaPorTipo, bosques, nodos, rios, fertilidad, elevacion };
}

describe('caracterización de la generación de mundo', () => {
  for (const seed of SEEDS) {
    it(`seed ${seed} produce exactamente el mismo mundo que hoy`, () => {
      expect(digest(crear(seed))).toMatchSnapshot();
    });
  }

  it('dos generaciones con el mismo seed son idénticas (incluido el campo de fertilidad)', () => {
    const a = crear(42);
    const b = crear(42);
    expect(digest(b)).toEqual(digest(a));
  });

  it('seeds distintos producen mundos distintos', () => {
    expect(digest(crear(42))).not.toEqual(digest(crear(43)));
  });
});

describe('invariantes de la generación de mundo', () => {
  // Contrato que debe seguir cumpliéndose aunque los snapshots de arriba se regeneren a propósito.

  it('los ids de nodo y de bosque son únicos', () => {
    // Crítico: los edificios guardan `fuenteId` apuntando a un nodo o bosque (ver `Edificio.fuenteId`), y
    // la producción los resuelve por id. Dos fuentes con el mismo id harían que una cantera extrajera del
    // yacimiento equivocado.
    for (const seed of SEEDS) {
      const world = crear(seed);
      const idsNodo = world.nodos.map((n) => n.id);
      const idsBosque = world.bosques.map((b) => b.id);
      expect(new Set(idsNodo).size).toBe(idsNodo.length);
      expect(new Set(idsBosque).size).toBe(idsBosque.length);
    }
  });

  it('todo nodo cae dentro de los límites del mapa y tiene cantidad positiva', () => {
    for (const seed of SEEDS) {
      const world = crear(seed);
      for (const nodo of world.nodos) {
        expect(nodo.posicion.x).toBeGreaterThanOrEqual(0);
        expect(nodo.posicion.x).toBeLessThanOrEqual(world.config.ancho);
        expect(nodo.posicion.y).toBeGreaterThanOrEqual(0);
        expect(nodo.posicion.y).toBeLessThanOrEqual(world.config.alto);
        expect(nodo.cantidadInicial).toBeGreaterThan(0);
      }
    }
  });

  it('la cantidad de nodos por tipo es la que dicta la configuración de rareza', () => {
    for (const seed of SEEDS) {
      const world = crear(seed);
      for (const [rareza, tipos] of Object.entries(RECURSO_TIPOS_POR_RAREZA)) {
        const esperados = RECURSO_RAREZA[rareza as keyof typeof RECURSO_RAREZA].cantidadBase;
        for (const tipo of tipos) {
          expect(world.nodos.filter((n) => n.tipo === tipo)).toHaveLength(esperados);
        }
      }
      expect(world.nodos.filter((n) => n.tipo === 'livestock')).toHaveLength(LIVESTOCK.cantidadBase);
    }
  });

  it('trigo y madera NO generan nodos (vienen de fertilidad y de bosques, Doc 1.4)', () => {
    for (const seed of SEEDS) {
      const world = crear(seed);
      expect(world.nodos.some((n) => n.tipo === 'trigo' || n.tipo === 'madera')).toBe(false);
    }
  });

  it('los bosques respetan cantidad, rango de radio y rango de densidad', () => {
    for (const seed of SEEDS) {
      const world = crear(seed);
      expect(world.bosques).toHaveLength(BOSQUE.cantidad);
      for (const bosque of world.bosques) {
        expect(bosque.radio).toBeGreaterThanOrEqual(BOSQUE.radioMin);
        expect(bosque.radio).toBeLessThanOrEqual(BOSQUE.radioMax);
        expect(bosque.densidad).toBeGreaterThanOrEqual(BOSQUE.densidadMin);
        expect(bosque.densidad).toBeLessThanOrEqual(BOSQUE.densidadMax);
      }
    }
  });

  it('el campo de fertilidad devuelve valores normalizados 0-1 en todo el mapa, incluido fuera de límites', () => {
    // Fuera de límites importa: `mejorPuntoFertilidadCercano` (settlement.ts) y `sitioMejorFertilidad`
    // (construction.ts) muestrean anillos alrededor del asentamiento que pueden salirse del mapa.
    for (const seed of SEEDS) {
      const world = crear(seed);
      const puntos = [
        { x: 0, y: 0 },
        { x: world.config.ancho, y: world.config.alto },
        { x: -200, y: -200 },
        { x: world.config.ancho + 200, y: world.config.alto + 200 },
        { x: 500.5, y: 123.25 },
      ];
      for (const p of puntos) {
        const valor = evaluarFertilidad(world.fertilidad, p);
        expect(valor).toBeGreaterThanOrEqual(0);
        expect(valor).toBeLessThanOrEqual(1);
      }
    }
  });

  it('el campo de fertilidad tiene variación real (no es una constante)', () => {
    const world = crear(1);
    const muestras = Array.from({ length: 200 }, (_, i) =>
      evaluarFertilidad(world.fertilidad, { x: (i * 37) % world.config.ancho, y: (i * 53) % world.config.alto })
    );
    const min = Math.min(...muestras);
    const max = Math.max(...muestras);
    expect(max - min).toBeGreaterThan(0.2);
  });

  // --- Fase 0.1: relieve, ríos y bioma ---

  it('el campo de elevación devuelve valores normalizados 0-1 en todo el mapa, incluido fuera de límites', () => {
    for (const seed of SEEDS) {
      const world = crear(seed);
      const puntos = [
        { x: 0, y: 0 },
        { x: world.config.ancho, y: world.config.alto },
        { x: -200, y: -200 },
        { x: world.config.ancho + 200, y: world.config.alto + 200 },
        { x: 500.5, y: 123.25 },
      ];
      for (const p of puntos) {
        const valor = evaluarElevacion(world.elevacion, p);
        expect(valor).toBeGreaterThanOrEqual(0);
        expect(valor).toBeLessThanOrEqual(1);
      }
    }
  });

  it('el campo de elevación tiene variación real (no es una constante)', () => {
    const world = crear(1);
    const muestras = Array.from({ length: 200 }, (_, i) =>
      evaluarElevacion(world.elevacion, { x: (i * 37) % world.config.ancho, y: (i * 53) % world.config.alto })
    );
    const min = Math.min(...muestras);
    const max = Math.max(...muestras);
    expect(max - min).toBeGreaterThan(0.2);
  });

  it('terrenoEn/evaluarBioma siempre devuelven un valor definido, incluido fuera de límites', () => {
    const TERRENOS = ['agua', 'costa', 'llano', 'colina', 'montana', 'cima'];
    const BIOMAS = ['agua', 'costa', 'estepa', 'llanuraFertil', 'colina', 'montana', 'cima'];
    for (const seed of SEEDS) {
      const world = crear(seed);
      const puntos = [
        { x: 0, y: 0 },
        { x: world.config.ancho, y: world.config.alto },
        { x: -200, y: -200 },
        { x: world.config.ancho + 200, y: world.config.alto + 200 },
        { x: 500.5, y: 123.25 },
      ];
      for (const p of puntos) {
        expect(TERRENOS).toContain(evaluarTerreno(world.elevacion, p));
        expect(BIOMAS).toContain(evaluarBioma(world.elevacion, world.fertilidad, world.rios, p));
      }
    }
  });

  it('hay exactamente RIOS.cantidad ríos y todo punto de todo río cae dentro del mapa', () => {
    // >=1 es la garantía estructural (`generarRios` siempre empuja el nacimiento); un río de un solo punto
    // es un nacimiento sin pendiente clara alrededor (colocación condicionada al gradiente, ver `rios.ts`,
    // o el fallback de "mapa saturado" que comparte con `colocarConEspaciado`) — no se renderiza como línea
    // (`canvas.ts` lo salta), pero no es un error de generación.
    for (const seed of SEEDS) {
      const world = crear(seed);
      expect(world.rios).toHaveLength(RIOS.cantidad);
      for (const rio of world.rios) {
        expect(rio.puntos.length).toBeGreaterThanOrEqual(1);
        for (const p of rio.puntos) {
          expect(p.x).toBeGreaterThanOrEqual(0);
          expect(p.x).toBeLessThanOrEqual(world.config.ancho);
          expect(p.y).toBeGreaterThanOrEqual(0);
          expect(p.y).toBeLessThanOrEqual(world.config.alto);
        }
      }
    }
  });
});
