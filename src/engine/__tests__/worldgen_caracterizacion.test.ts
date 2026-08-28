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
import type { RegionId } from '../../domain/types';
import { evaluarBioma, evaluarElevacion, evaluarFertilidad, evaluarTerreno, generarMapa, REGIONES, type MapaGenerado } from '../../worldgen';
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
    return `${r.id} n=${r.puntos.length} fin=(${extremo.x.toFixed(6)},${extremo.y.toFixed(6)}) lago=${r.terminaEnLago} navegable=${r.navegable}`;
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

  it('solo los ríos con desembocadura real pueden ser navegables (comercio fluvial, fases futuras)', () => {
    // Un río atrapado en un lago de montaña (`terminaEnLago`) no tiene por dónde salir un barco — nunca debe
    // salir navegable, sin importar cuán largo sea (ver comentario de `RIOS.proporcionNavegable`).
    for (const seed of SEEDS) {
      const world = crear(seed);
      for (const rio of world.rios) {
        if (rio.terminaEnLago) expect(rio.navegable).toBe(false);
      }
    }
  });
});

describe('generación regional (Fase 0.2 — región geográfica opcional)', () => {
  // Rejilla más fina que `CELDAS_FERTILIDAD`: aquí lo que importa es medir composición de terreno (% agua,
  // % montaña...) sobre una muestra grande, no capturar un snapshot legible línea a línea.
  const CELDAS_COMPOSICION = 60;

  function composicionDeTerreno(world: MapaGenerado): Record<string, number> {
    const conteo: Record<string, number> = {};
    const pasoX = world.config.ancho / CELDAS_COMPOSICION;
    const pasoY = world.config.alto / CELDAS_COMPOSICION;
    for (let fila = 0; fila < CELDAS_COMPOSICION; fila++) {
      for (let col = 0; col < CELDAS_COMPOSICION; col++) {
        const t = evaluarTerreno(world.elevacion, { x: (col + 0.5) * pasoX, y: (fila + 0.5) * pasoY });
        conteo[t] = (conteo[t] ?? 0) + 1;
      }
    }
    return conteo;
  }

  it('sin `region`, generarMapa produce EXACTAMENTE el mismo mundo que antes de Fase 0.2', () => {
    // La garantía de compatibilidad más importante de esta feature: `region` es opcional y su ausencia no
    // debe desplazar ni un bit de lo que ya generaba el motor (mismo consumo de RNG, mismo `evaluarElevacion`
    // sin mezcla). Comparado contra el propio `crear(seed)` de arriba, que nunca pasa `region`.
    for (const seed of SEEDS) {
      const libre = crear(seed);
      const explicito = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: undefined });
      expect(explicito).toEqual(libre);
    }
  });

  const TODAS_LAS_REGIONES = Object.keys(REGIONES) as RegionId[];

  it.each(TODAS_LAS_REGIONES)('%s es determinista por seed, igual que el mundo libre', (region) => {
    for (const seed of SEEDS) {
      const a = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region });
      const b = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region });
      expect(b).toEqual(a);
    }
  });

  it.each(TODAS_LAS_REGIONES)('%s da seeds DISTINTAS entre sí (la guía no aplasta toda la variedad)', (region) => {
    const a = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: 1, region });
    const b = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: 2, region });
    expect(a).not.toEqual(b);
  });

  it.each(TODAS_LAS_REGIONES)(
    '%s no rompe la generación de bosques/nodos (misma cantidad que el mundo libre) ni deja nodos fuera del mapa',
    (region) => {
      // La región solo sesga ELEVACIÓN (y, para Nilo/Mesopotamia, añade ríos troncales aparte — ver el
      // describe de abajo) — bosques/nodos (que vienen de `config.ts`, no de la elevación) deben seguir
      // intactos en cantidad aunque el terreno sobre el que caen sea distinto.
      for (const seed of SEEDS) {
        const world = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region });
        expect(world.bosques).toHaveLength(BOSQUE.cantidad);
        for (const nodo of world.nodos) {
          expect(nodo.posicion.x).toBeGreaterThanOrEqual(0);
          expect(nodo.posicion.x).toBeLessThanOrEqual(world.config.ancho);
        }
      }
    }
  );

  /** Composición agregada por bandas de terreno "que se puede pisar" vs "agua/costa", para comparar el
   * carácter de una región contra el mundo libre en las mismas seeds. */
  function resumenTerreno(world: MapaGenerado) {
    const c = composicionDeTerreno(world);
    return {
      montanoso: (c.colina ?? 0) + (c.montana ?? 0) + (c.cima ?? 0),
      agua: (c.agua ?? 0) + (c.costa ?? 0),
      llano: c.llano ?? 0,
    };
  }

  it('greciaContinental sale más montañosa y con más agua que el mundo libre, para las mismas seeds', () => {
    // No es un valor exacto (dependería de la calibración fina de `regiones.ts`, que puede seguir
    // ajustándose) — es el contrato de fondo: la región tiene que notarse, no solo existir. Medido en
    // `regiones.ts` (comentario de `GRECIA_CONTINENTAL`): colina+montaña+cima ~55-58% vs ~26-28% libre.
    for (const seed of SEEDS) {
      const libre = resumenTerreno(crear(seed));
      const grecia = resumenTerreno(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: 'greciaContinental' }));
      expect(grecia.montanoso).toBeGreaterThan(libre.montanoso);
      expect(grecia.agua).toBeGreaterThan(libre.agua);
    }
  });

  it('anatolia sale más montañosa que el mundo libre (cordilleras norte/sur), con meseta abierta de por medio', () => {
    for (const seed of SEEDS) {
      const libre = resumenTerreno(crear(seed));
      const anatolia = resumenTerreno(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: 'anatolia' }));
      expect(anatolia.montanoso).toBeGreaterThan(libre.montanoso);
      // A diferencia de Grecia, la meseta interior deja bastante 'llano' real — no es solo montaña.
      expect(anatolia.llano).toBeGreaterThan(20);
    }
  });

  it('egeo sale abrumadoramente más acuático que el mundo libre (archipiélago)', () => {
    for (const seed of SEEDS) {
      const libre = resumenTerreno(crear(seed));
      const egeo = resumenTerreno(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: 'egeo' }));
      expect(egeo.agua).toBeGreaterThan(libre.agua * 2);
    }
  });

  it.each(['nilo', 'mesopotamia'] as const)(
    '%s sale más llana y con MENOS montaña que el mundo libre (valle fluvial en llanura árida)',
    (region) => {
      for (const seed of SEEDS) {
        const libre = resumenTerreno(crear(seed));
        const mundo = resumenTerreno(generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region }));
        expect(mundo.llano).toBeGreaterThan(libre.llano);
        expect(mundo.montanoso).toBeLessThan(libre.montanoso);
      }
    }
  );
});

describe('ríos troncales (Fase 0.2 — Nilo/Mesopotamia, ver RegionGeografica.riosTroncales)', () => {
  const CASOS: { region: RegionId; ids: string[] }[] = [
    { region: 'nilo', ids: ['nilo'] },
    { region: 'mesopotamia', ids: ['tigris', 'eufrates'] },
  ];

  function longitudRio(puntos: { x: number; y: number }[]): number {
    let total = 0;
    for (let i = 0; i < puntos.length - 1; i++) total += Math.hypot(puntos[i + 1]!.x - puntos[i]!.x, puntos[i + 1]!.y - puntos[i]!.y);
    return total;
  }

  it.each(CASOS)('$region genera sus ríos troncales, navegables, con desembocadura real y cruzando casi todo el mapa', ({ region, ids }) => {
    for (const seed of SEEDS) {
      const world = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region });
      for (const id of ids) {
        const rio = world.rios.find((r) => r.id === id);
        expect(rio, `${region}/seed ${seed}: falta el río troncal '${id}'`).toBeDefined();
        expect(rio!.navegable).toBe(true);
        expect(rio!.terminaEnLago).toBe(false);
        // >90% del alto del mapa: los troncales autorados van de borde norte a borde sur (ver `NILO`/
        // `MESOPOTAMIA` en regiones.ts), no en diagonal — cruzan de un extremo al otro, no son un afluente.
        expect(longitudRio(rio!.puntos)).toBeGreaterThan(world.config.alto * 0.9);
      }
    }
  });

  it('mesopotamia: los ríos troncales no rompen el cupo normal de RIOS.cantidad (se suman aparte)', () => {
    for (const seed of SEEDS) {
      const world = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: 'mesopotamia' });
      expect(world.rios).toHaveLength(RIOS.cantidad + 2);
    }
  });

  it('nilo: el río troncal no rompe el cupo normal de RIOS.cantidad (se suma aparte)', () => {
    for (const seed of SEEDS) {
      const world = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed, region: 'nilo' });
      expect(world.rios).toHaveLength(RIOS.cantidad + 1);
    }
  });
});
