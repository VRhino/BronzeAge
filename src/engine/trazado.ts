import type { Asentamiento, Edificio, EdificioTipo, Point } from '../domain/types';
import { EDIFICIO_CATALOGO, EDIFICIO_TAMANO, PUESTO_MERCADO_FORMA, REJILLA_ASENTAMIENTO, TRAZADO } from '../constants';

/**
 * TRAZADO URBANO DINÁMICO de la Vista de Asentamiento (a petición del usuario).
 * Especificación completa: `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
 *
 * Tres pilares, y conviene tenerlos presentes antes de tocar nada de aquí:
 *
 * 1. LAS CALLES CORREN SOBRE LAS ARISTAS de la rejilla, no sobre celdas. Una arista separa dos celdas y no
 *    ocupa superficie construible. De ahí salen gratis dos garantías que antes había que validar a mano: un
 *    edificio no puede quedar encima de una calle (la calle no tiene celdas) y una calle no puede quedar
 *    encima de otra (la red es un Set de aristas, añadir una repetida es un no-op). Además, ningún tramo puede
 *    atravesar un edificio, porque una arista siempre va por el borde entre celdas.
 * 2. LA RED ES DERIVADA, NO PERSISTIDA. `redDeCalles` la reconstruye recorriendo `edificios` en su orden de
 *    construcción y aplicando el crecimiento paso a paso. Mismo input, mismo output: no hace falta guardarla
 *    en `Asentamiento` ni migrar partidas. El array de edificios ya está en orden de construcción (siempre se
 *    hace push/map, nunca se reordena).
 * 3. NADA SE PRE-GENERA AL FUNDAR. Un intento anterior trazaba toda la rejilla de calles al crear el
 *    asentamiento y fue rechazado: la ciudad nacía con su plano completo y además producía edificios encima de
 *    calles y granjas pegadas al centro. Aquí la única semilla es el PERÍMETRO DEL CENTRO URBANO; todo lo demás
 *    aparece cuando un edificio concreto lo necesita.
 *
 * Las manzanas no se dibujan ni se guardan: son los ciclos de la red, el espacio negativo que queda encerrado
 * cuando las calles cierran un anillo. Emergen de las reglas locales de `crecerRed` y `puntuarCandidato`.
 *
 * Nada aquí puede consumir `Math.random()`: rompería la reproducibilidad de la simulación. Toda variación por
 * asentamiento sale de `hashTexto` + `pseudoAleatorio` sobre su id.
 */

const T = REJILLA_ASENTAMIENTO.tamanoCelda;

export interface Celda {
  col: number;
  row: number;
}

export interface TamanoEdificio {
  ancho: number;
  alto: number;
}

/** Un tramo de calle o camino ya resuelto a coordenadas LOCALES — lo que `ui/canvas.ts` recibe para pintar,
 * sin saber nada de aristas ni de celdas (acoplamiento 0). */
export interface SegmentoTrazado {
  desde: Point;
  hasta: Point;
}

/** Calles (urbanas: forman filas y cierran manzanas) y caminos (rurales: solo conectan Granja/Corral con la
 * ciudad, ver §9 del doc) como conjuntos de claves de arista. Separados porque son clases distintas, no dos
 * grosores del mismo trazo. */
export interface RedDeCalles {
  calles: Set<string>;
  caminos: Set<string>;
}

// --- Rejilla: celdas, puntos y huellas ---

/** Edificios que viven en el espacio plano del asentamiento (coords locales) — los extractores del mapa
 * general (`ambito: 'mapa'`) no participan de esta rejilla. */
export function edificiosInternos(edificios: Edificio[]): Edificio[] {
  return edificios.filter((e) => (e.ambito ?? 'asentamiento') === 'asentamiento');
}

/**
 * Huella (ancho×alto en celdas) de un tipo de edificio. Granja es el único tipo cuyo tamaño depende del nivel
 * interno (1x1 → 6x6, ver `EDIFICIO_CATALOGO.granja.niveles`); el resto lo tiene fijo en `EDIFICIO_TAMANO`, y
 * un tipo ausente de esa tabla mide 1x1.
 */
export function tamanoEdificio(tipo: EdificioTipo, nivelInterno?: number): TamanoEdificio {
  if (tipo === 'granja') {
    const niveles = EDIFICIO_CATALOGO.granja.niveles as Record<number, { tamano?: TamanoEdificio }>;
    return niveles[nivelInterno ?? 1]?.tamano ?? { ancho: 1, alto: 1 };
  }
  // Puesto de Mercado: `nivelInterno` no es progresión, identifica qué FORMA tiene esta pieza de la zona.
  if (tipo === 'puestoMercado') return PUESTO_MERCADO_FORMA[nivelInterno ?? 1] ?? { ancho: 1, alto: 1 };
  return EDIFICIO_TAMANO[tipo] ?? { ancho: 1, alto: 1 };
}

export function tamanoDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno'>): TamanoEdificio {
  return tamanoEdificio(edificio.tipo, edificio.nivelInterno);
}

/**
 * Centro del rectángulo `tamano` cuya esquina mínima (menor col, menor row) es `celdaMin` — el valor que se
 * guarda en `Edificio.posicion`. Para una huella 1x1 da exactamente el centro de la celda, así que los
 * edificios de una celda conservan las posiciones de siempre y las partidas guardadas no se mueven.
 */
export function puntoDeRectangulo(celdaMin: Celda, tamano: TamanoEdificio): Point {
  return {
    x: (celdaMin.col + tamano.ancho / 2) * T,
    y: (celdaMin.row + tamano.alto / 2) * T,
  };
}

/**
 * Inversa de `puntoDeRectangulo`: la esquina mínima del rectángulo centrado en `posicion`. El redondeo es
 * exacto tanto para lados pares como impares (el centro cae en un vértice o en el centro de una celda, y
 * `posicion / T - lado / 2` da un entero en ambos casos).
 *
 * El Centro Urbano es la ÚNICA excepción del sistema: su posición `(0,0)` no es su centro sino el VÉRTICE de
 * su esquina inferior izquierda (sur-oeste: +y = sur), a petición explícita del usuario. Con 3x3 eso lo deja
 * ocupando las columnas 0..2 y las filas -3..-1.
 */
export function celdaMinimaDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion'>): Celda {
  const tamano = tamanoDeEdificio(edificio);
  if (edificio.tipo === 'centroUrbano') return { col: 0, row: -tamano.alto };
  return {
    col: Math.round(edificio.posicion.x / T - tamano.ancho / 2),
    row: Math.round(edificio.posicion.y / T - tamano.alto / 2),
  };
}

function claveCelda(col: number, row: number): string {
  return `${col},${row}`;
}

/** Celdas que ocupa un edificio — su rectángulo completo, no solo la celda de su posición. */
export function celdasDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion'>): Celda[] {
  const tamano = tamanoDeEdificio(edificio);
  const min = celdaMinimaDeEdificio(edificio);
  const celdas: Celda[] = [];
  for (let dc = 0; dc < tamano.ancho; dc++) {
    for (let dr = 0; dr < tamano.alto; dr++) celdas.push({ col: min.col + dc, row: min.row + dr });
  }
  return celdas;
}

/** Celdas ocupadas por todos los edificios internos, como set de claves "col,row" para lookup O(1). */
function celdasOcupadas(edificios: Edificio[], excluirId?: string): Set<string> {
  const set = new Set<string>();
  for (const e of edificiosInternos(edificios)) {
    if (excluirId && e.id === excluirId) continue;
    for (const c of celdasDeEdificio(e)) set.add(claveCelda(c.col, c.row));
  }
  return set;
}

// --- Aristas: la red de calles vive sobre las líneas de la rejilla ---
//
// Un vértice (i, j) está en el punto local (i·T, j·T). Una arista unitaria es horizontal `H i,j` (de (i,j) a
// (i+1,j)) o vertical `V i,j` (de (i,j) a (i,j+1)).

function aristaH(i: number, j: number): string {
  return `H${i},${j}`;
}

function aristaV(i: number, j: number): string {
  return `V${i},${j}`;
}

/** Las 2·(ancho+alto) aristas que rodean un rectángulo de celdas. La conexión de un edificio a la red es
 * exactamente "alguna de estas está en la red" — vale igual para un 1x1 que para el 6x6 de Granja nivel 4, sin
 * ninguna regla de fachada aparte. */
function aristasDeRectangulo(min: Celda, tamano: TamanoEdificio): string[] {
  const aristas: string[] = [];
  for (let dc = 0; dc < tamano.ancho; dc++) {
    aristas.push(aristaH(min.col + dc, min.row));
    aristas.push(aristaH(min.col + dc, min.row + tamano.alto));
  }
  for (let dr = 0; dr < tamano.alto; dr++) {
    aristas.push(aristaV(min.col, min.row + dr));
    aristas.push(aristaV(min.col + tamano.ancho, min.row + dr));
  }
  return aristas;
}

export function aristasDePerimetro(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion'>): string[] {
  return aristasDeRectangulo(celdaMinimaDeEdificio(edificio), tamanoDeEdificio(edificio));
}

interface Vertice {
  i: number;
  j: number;
}

/** Los dos extremos de una arista, a partir de su clave. */
function extremosDeArista(clave: string): [Vertice, Vertice] {
  const horizontal = clave[0] === 'H';
  const [i, j] = clave.slice(1).split(',').map(Number) as [number, number];
  return horizontal ? [{ i, j }, { i: i + 1, j }] : [{ i, j }, { i, j: j + 1 }];
}

function verticesDeRed(red: RedDeCalles): Vertice[] {
  const vistos = new Set<string>();
  const vertices: Vertice[] = [];
  for (const clave of [...red.calles, ...red.caminos]) {
    for (const v of extremosDeArista(clave)) {
      const k = `${v.i},${v.j}`;
      if (vistos.has(k)) continue;
      vistos.add(k);
      vertices.push(v);
    }
  }
  return vertices;
}

/** Aristas de un trayecto en "L" entre dos vértices: primero el tramo horizontal, después el vertical. Solo
 * ángulos de 90°, nunca una diagonal — la petición original del usuario sobre las calles. */
function aristasDeCaminoEnL(desde: Vertice, hasta: Vertice): string[] {
  const aristas: string[] = [];
  const paso = (a: number, b: number) => (a < b ? 1 : -1);
  for (let i = desde.i; i !== hasta.i; i += paso(desde.i, hasta.i)) {
    aristas.push(aristaH(Math.min(i, i + paso(desde.i, hasta.i)), desde.j));
  }
  for (let j = desde.j; j !== hasta.j; j += paso(desde.j, hasta.j)) {
    aristas.push(aristaV(hasta.i, Math.min(j, j + paso(desde.j, hasta.j))));
  }
  return aristas;
}

// --- Aleatoriedad determinista por asentamiento ---

/** Hash determinista de texto -> entero (FNV-1a) — el motor no puede consumir `Math.random()`, así que
 * cualquier variación por asentamiento necesita una semilla estable derivada de texto (aquí, su id). */
export function hashTexto(texto: string): number {
  let h = 2166136261;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Pseudo-aleatorio determinista en [0,1) a partir de un entero. */
export function pseudoAleatorio(semilla: number): number {
  const x = Math.sin(semilla * 12.9898) * 43758.5453;
  return x - Math.floor(x);
}

/**
 * Largo máximo (en celdas) que puede alcanzar una fila de edificios sobre la misma calle antes de que el
 * siguiente obligue a abrir una transversal — el momento en que el anillo se cierra y nace la manzana. Es una
 * constante sorteada por asentamiento, pero la calle SOLO aparece cuando una fila llega de verdad a ese largo:
 * ahí está la diferencia con el plano pre-generado que se descartó.
 */
function largoMaxFila(asentamientoId: string): number {
  const rango = TRAZADO.largoFilaMax - TRAZADO.largoFilaMin + 1;
  return TRAZADO.largoFilaMin + Math.floor(pseudoAleatorio(hashTexto(`${asentamientoId}-largo-fila`)) * rango);
}

// --- Barrios: cada categoría crece en una dirección cardinal aleatoria por asentamiento ---

const DIRECCIONES_CARDINALES = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'] as const;
type DireccionCardinal = (typeof DIRECCIONES_CARDINALES)[number];

/** Vector unitario (en celdas) de cada dirección cardinal — +x = Este, +y = Sur (mismo criterio que el resto
 * de coordenadas locales, ver `Edificio.posicion`). */
const VECTOR_DIRECCION: Record<DireccionCardinal, { dx: number; dy: number }> = {
  N: { dx: 0, dy: -1 },
  NE: { dx: 1, dy: -1 },
  E: { dx: 1, dy: 0 },
  SE: { dx: 1, dy: 1 },
  S: { dx: 0, dy: 1 },
  SW: { dx: -1, dy: 1 },
  W: { dx: -1, dy: 0 },
  NW: { dx: -1, dy: -1 },
};

const ANGULO_DIRECCION: Record<DireccionCardinal, number> = Object.fromEntries(
  DIRECCIONES_CARDINALES.map((d) => [d, Math.atan2(VECTOR_DIRECCION[d].dy, VECTOR_DIRECCION[d].dx)])
) as Record<DireccionCardinal, number>;

/** Categoría funcional (barrio) de cada tipo de edificio "urbano". Ausente = sin barrio fijo: Granja/Corral
 * van a las afueras y Palacio junto al centro (ver `sitiosParaTipo`); Centro Urbano es el origen;
 * mina/minaCobre/minaEstano/cantera viven en el mapa general (`ambito: 'mapa'`), fuera de esta rejilla. */
export type CategoriaAsentamiento = 'residencial' | 'industria' | 'militar' | 'mercado' | 'almacenaje';

/** Categoría (barrio) de cada tipo — exportado para que `app/gameStore.ts` pueda pasárselo a `ui/canvas.ts`
 * (acoplamiento 0: la UI solo lee este dato, nunca lo decide). */
export const CATEGORIA_POR_TIPO: Partial<Record<EdificioTipo, CategoriaAsentamiento>> = {
  vivienda: 'residencial',
  almacen: 'almacenaje',
  // Leñera/Corral: extractores locales sin barrio temático claro — se agrupan con Almacén (logística de
  // recursos), decisión editorial simple para no inventar una sexta categoría.
  lenera: 'almacenaje',
  corral: 'almacenaje',
  fundicion: 'industria',
  curtiduria: 'industria',
  armeria: 'industria',
  granFundicion: 'industria',
  maravilla: 'industria',
  barracon: 'militar',
  galeriaDeTiro: 'militar',
  carpinteria: 'militar',
  muralla: 'militar',
  mercado: 'mercado',
  // Los puestos comparten el barrio del Mercado: así la acreción que ya existe (`distanciaAlBarrio`) los
  // agrupa alrededor de la pieza principal sola, sin ninguna regla nueva de "quedar pegados".
  puestoMercado: 'mercado',
};

const CATEGORIAS_ASENTAMIENTO: CategoriaAsentamiento[] = ['residencial', 'industria', 'militar', 'mercado', 'almacenaje'];

/**
 * Reparto de direcciones cardinales por categoría, ALEATORIO por asentamiento (a petición del usuario) pero
 * determinista y sin persistir nada: se baraja `DIRECCIONES_CARDINALES` (Fisher-Yates con semilla derivada de
 * `asentamientoId`) y se asignan las 5 primeras, en orden fijo, a las 5 categorías — mismo id, mismo reparto,
 * siempre; dos asentamientos distintos casi seguro difieren.
 */
function direccionesDelAsentamiento(asentamientoId: string): Record<CategoriaAsentamiento, DireccionCardinal> {
  const direcciones = [...DIRECCIONES_CARDINALES] as DireccionCardinal[];
  for (let i = direcciones.length - 1; i > 0; i--) {
    const j = Math.floor(pseudoAleatorio(hashTexto(`${asentamientoId}-direccion-${i}`)) * (i + 1));
    [direcciones[i], direcciones[j]] = [direcciones[j]!, direcciones[i]!];
  }
  const resultado = {} as Record<CategoriaAsentamiento, DireccionCardinal>;
  CATEGORIAS_ASENTAMIENTO.forEach((categoria, indice) => {
    resultado[categoria] = direcciones[indice]!;
  });
  return resultado;
}

/** Ángulo (radianes) asignado a cada categoría para ESTE asentamiento — expuesto para `app/gameStore.ts`. */
export function angulosDeBarrios(asentamientoId: string): Record<CategoriaAsentamiento, number> {
  const direcciones = direccionesDelAsentamiento(asentamientoId);
  const resultado = {} as Record<CategoriaAsentamiento, number>;
  for (const categoria of CATEGORIAS_ASENTAMIENTO) {
    resultado[categoria] = ANGULO_DIRECCION[direcciones[categoria]];
  }
  return resultado;
}

/** Diferencia angular mínima entre dos ángulos (radianes), siempre en `[0, π]`. */
function diferenciaAngular(a: number, b: number): number {
  let diff = Math.abs(a - b) % (Math.PI * 2);
  if (diff > Math.PI) diff = Math.PI * 2 - diff;
  return diff;
}

/** Media cuña por dirección: 8 direcciones cubriendo 360° sin huecos ni solapes = 45° cada una. */
const MEDIA_CUÑA_DIRECCION = Math.PI / 8;

/** Tipos que se colocan FUERA de la trama urbana y se conectan por camino, no por calle (§8-9 del doc). */
const TIPOS_AFUERAS = new Set<EdificioTipo>(['granja', 'corral']);

export function esDeAfueras(tipo: EdificioTipo): boolean {
  return TIPOS_AFUERAS.has(tipo);
}

/**
 * Hasta dónde llegan las afueras. Nunca menos que `radioAfuerasMin + anchoBandaAfueras`, aunque la zona de
 * influencia sea más chica: el campo de una ciudad está FUERA de su zona de influencia, y si el tope fuera el
 * de la zona, al fundar (radio inicial 30, radio vedado 60) no habría ningún hueco válido para la Granja
 * inicial y caería al fallback del origen, encima del Centro Urbano.
 */
function radioMaximoAfueras(radioPotencial: number): number {
  return Math.max(radioPotencial, TRAZADO.radioAfuerasMin + TRAZADO.anchoBandaAfueras);
}

// --- Crecimiento de la red ---

/**
 * ¿La línea `indice` (una columna o una fila de la rejilla) es un BORDE DE MANZANA en este asentamiento?
 *
 * Las manzanas miden `maxFila` celdas, con un desfase propio de cada asentamiento para que dos ciudades no
 * tengan los bloques alineados a las mismas columnas absolutas. Es un criterio POSICIONAL, y esa es justo la
 * parte que costó acertar: probé disparar la transversal por largo de hilera (`>= maxFila`), por módulo de ese
 * largo, y con un veto de separación mínima entre cruces. Los tres dependen del ORDEN en que se construyeron
 * los edificios, y como una hilera crece por los dos extremos y se vuelve a medir entera cada vez, las
 * transversales salían amontonadas o no salían casi nunca — medido: entre 2 y 6 manzanas cerradas en una
 * ciudad de 50 edificios, a veces con bloques de un solo edificio de ancho.
 *
 * Ojo con lo que esto NO es: no hay ningún plano previo. Nada se traza al fundar; esta función solo dice por
 * dónde CAERÍA la transversal el día que un edificio la necesite. Si la ciudad nunca crece hacia esa columna,
 * esa calle no llega a existir.
 */
function esBordeDeManzana(indice: number, paso: number, desfase: number): boolean {
  return (((indice - desfase) % paso) + paso) % paso === 0;
}

/**
 * Fondo de una manzana, en celdas: DOS hileras de edificios entre dos calles paralelas, una mirando a cada
 * calle. Sale directamente del invariante de §3 (todo edificio toca calle): detrás de una segunda hilera ya no
 * se puede construir sin traer otra calle, así que la manzana no puede ser más honda. Con esto los bloques
 * quedan de `maxFila` × 2 celdas — alargados, como una manzana de verdad, y no un cuadrado de 6x6 lleno de
 * callejones interiores para dar salida a los edificios del centro.
 */
const FONDO_MANZANA = 2;

/** Distancia Manhattan de un vértice al rectángulo de un edificio (0 si cae dentro o sobre su borde). */
function distanciaVerticeARectangulo(v: Vertice, min: Celda, tamano: TamanoEdificio): number {
  const dx = Math.max(min.col - v.i, 0, v.i - (min.col + tamano.ancho));
  const dy = Math.max(min.row - v.j, 0, v.j - (min.row + tamano.alto));
  return dx + dy;
}

/**
 * Conecta un edificio que quedó suelto: busca el vértice de red más cercano, traza hasta él un camino en "L"
 * sobre el retículo y añade además una arista de su perímetro (la fachada). Las aristas nuevas van a `destino`
 * — calles para lo urbano, caminos para Granja/Corral.
 *
 * No hace falta comprobar que el trayecto no atraviese nada: una arista siempre corre por el borde entre dos
 * celdas, así que un tramo jamás puede pasar por encima de un edificio.
 */
function conectarEdificio(edificio: Edificio, red: RedDeCalles, destino: Set<string>): void {
  const min = celdaMinimaDeEdificio(edificio);
  const tamano = tamanoDeEdificio(edificio);
  const vertices = verticesDeRed(red);
  if (vertices.length === 0) return;

  let mejor = vertices[0]!;
  let mejorDistancia = Infinity;
  for (const v of vertices) {
    const d = distanciaVerticeARectangulo(v, min, tamano);
    if (d < mejorDistancia) {
      mejor = v;
      mejorDistancia = d;
    }
  }

  // Vértice del perímetro más cercano al de la red: el punto por donde entra la calle.
  const perimetro: Vertice[] = [];
  for (let dc = 0; dc <= tamano.ancho; dc++) {
    perimetro.push({ i: min.col + dc, j: min.row });
    perimetro.push({ i: min.col + dc, j: min.row + tamano.alto });
  }
  for (let dr = 1; dr < tamano.alto; dr++) {
    perimetro.push({ i: min.col, j: min.row + dr });
    perimetro.push({ i: min.col + tamano.ancho, j: min.row + dr });
  }
  let entrada = perimetro[0]!;
  let entradaDistancia = Infinity;
  for (const v of perimetro) {
    const d = Math.abs(v.i - mejor.i) + Math.abs(v.j - mejor.j);
    if (d < entradaDistancia) {
      entrada = v;
      entradaDistancia = d;
    }
  }

  for (const arista of aristasDeCaminoEnL(mejor, entrada)) destino.add(arista);

  // Fachada: la arista del perímetro que sale del vértice de entrada. Sin ella el edificio tendría la calle
  // "tocándole una esquina", que no cuenta como conexión (§3 del doc pide una ARISTA, no un vértice).
  //
  // Entre las candidatas se prefiere la que PROLONGA una calle ya existente en línea recta. No es cosmético:
  // eligiendo cualquiera, muchas fachadas salían perpendiculares a la hilera y se colaban como cruces entre
  // dos casas vecinas, partiendo la fila en trozos de 2-3. Con eso el contador de fila nunca llegaba al máximo
  // y la transversal no saltaba casi nunca — la ciudad quedaba en un árbol de calles, sin manzanas cerradas.
  const candidatas = aristasDeRectangulo(min, tamano).filter((a) =>
    extremosDeArista(a).some((v) => v.i === entrada.i && v.j === entrada.j)
  );
  const prolongaCalle = (clave: string): boolean => {
    const horizontal = clave[0] === 'H';
    const [i, j] = clave.slice(1).split(',').map(Number) as [number, number];
    const vecinas = horizontal ? [aristaH(i - 1, j), aristaH(i + 1, j)] : [aristaV(i, j - 1), aristaV(i, j + 1)];
    return vecinas.some((v) => red.calles.has(v) || red.caminos.has(v));
  };
  const fachada = candidatas.find(prolongaCalle) ?? candidatas[0];
  if (fachada) destino.add(fachada);
}

/**
 * La red de calles y caminos del asentamiento, reconstruida desde cero (§2 del doc: derivada, no persistida).
 *
 * Semilla: el PERÍMETRO DEL CENTRO URBANO. Nace ya siendo un anillo cerrado —el CU es la manzana cero— y
 * ofrece frente de calle en las cuatro direcciones desde el primer tick. Eso es lo que evita el bloqueo que
 * hundió el intento anterior, donde algunos barrios no tenían ningún sitio viable al principio y el
 * asentamiento no llegaba a crecer.
 *
 * Después, por cada edificio en orden de construcción:
 *  1. si alguna arista de su perímetro ya está en la red, no se añade nada (el caso común de la ciudad
 *     compacta: el edificio nació pegado a una calle que ya existía);
 *  2. si no, se extiende la red más cercana hasta él (`conectarEdificio`);
 *  3. si su fachada completa una fila que alcanzó `largoMaxFila`, se añade además TODO su perímetro: eso es la
 *     transversal en su extremo más la calle de fondo, y con ella el anillo se cierra. Ahí nace la manzana, y
 *     las celdas de detrás pasan a tener frente de calle propio para la siguiente hilera.
 */
export function redDeCalles(asentamientoId: string, edificios: Edificio[]): RedDeCalles {
  const red: RedDeCalles = { calles: new Set(), caminos: new Set() };
  const internos = edificiosInternos(edificios);
  const centro = internos.find((e) => e.tipo === 'centroUrbano');
  if (!centro) return red;

  for (const arista of aristasDePerimetro(centro)) red.calles.add(arista);
  const maxFila = largoMaxFila(asentamientoId);
  const desfase = Math.floor(pseudoAleatorio(hashTexto(`${asentamientoId}-desfase-manzana`)) * maxFila);

  // Ocupación acumulada: solo los edificios YA procesados. El replay tiene que ver la ciudad como estaba en el
  // momento de construir cada uno, no como está al final — si no, dejaría de ser un crecimiento paso a paso.
  const ocupadas = new Set<string>();
  for (const c of celdasDeEdificio(centro)) ocupadas.add(claveCelda(c.col, c.row));

  for (const edificio of internos) {
    if (edificio.tipo === 'centroUrbano') continue;
    const min = celdaMinimaDeEdificio(edificio);
    const tamano = tamanoDeEdificio(edificio);
    for (const c of celdasDeEdificio(edificio)) ocupadas.add(claveCelda(c.col, c.row));

    const destino = esDeAfueras(edificio.tipo) ? red.caminos : red.calles;
    const perimetro = aristasDePerimetro(edificio);
    const yaConectado = perimetro.some((a) => red.calles.has(a) || red.caminos.has(a));
    if (!yaConectado) conectarEdificio(edificio, red, destino);

    // Transversal + calle de fondo: solo para lo urbano. Un camino rural no forma manzanas (§9).
    if (esDeAfueras(edificio.tipo)) continue;

    // Manzanas: de los cuatro lados del edificio, se convierten en calle los que caen sobre una línea de borde
    // de manzana — transversales cada `maxFila` columnas, calles de hilera cada `FONDO_MANZANA` filas. El
    // edificio aporta solo SU trozo de esas líneas; la calle entera se va formando conforme la ciudad crece
    // hacia ahí, y si nunca crece, nunca existe.
    if (esBordeDeManzana(min.col, maxFila, desfase)) {
      for (let dr = 0; dr < tamano.alto; dr++) red.calles.add(aristaV(min.col, min.row + dr));
    }
    if (esBordeDeManzana(min.col + tamano.ancho, maxFila, desfase)) {
      for (let dr = 0; dr < tamano.alto; dr++) red.calles.add(aristaV(min.col + tamano.ancho, min.row + dr));
    }
    if (esBordeDeManzana(min.row, FONDO_MANZANA, desfase)) {
      for (let dc = 0; dc < tamano.ancho; dc++) red.calles.add(aristaH(min.col + dc, min.row));
    }
    if (esBordeDeManzana(min.row + tamano.alto, FONDO_MANZANA, desfase)) {
      for (let dc = 0; dc < tamano.ancho; dc++) red.calles.add(aristaH(min.col + dc, min.row + tamano.alto));
    }
  }
  return red;
}

/** La red resuelta a segmentos en coordenadas LOCALES, listos para dibujar — `ui/canvas.ts` nunca ve una
 * arista ni una celda. */
export function segmentosDeRed(red: RedDeCalles): { calles: SegmentoTrazado[]; caminos: SegmentoTrazado[] } {
  const aSegmento = (clave: string): SegmentoTrazado => {
    const [a, b] = extremosDeArista(clave);
    return { desde: { x: a.i * T, y: a.j * T }, hasta: { x: b.i * T, y: b.j * T } };
  };
  return {
    calles: [...red.calles].map(aSegmento),
    // Una arista que ya es calle no se dibuja también como camino: la calle manda.
    caminos: [...red.caminos].filter((a) => !red.calles.has(a)).map(aSegmento),
  };
}

// --- Colocación ---

interface Candidato {
  min: Celda;
  punto: Point;
  /** 0 = continúa una fila sobre una calle existente; 1 = tiene frente de calle; 2 = pared con pared con algún
   * edificio; 3 = suelto (habrá que estirar la red hasta él). Menor es mejor. */
  nivel: number;
}

/**
 * ¿La arista `arista` del candidato es una calle que YA sirve de fachada a un vecino inmediato en la misma
 * hilera? Es decir: ¿colocarse aquí continúa una fila en vez de empezar una suelta?
 */
function continuaFila(arista: string, min: Celda, tamano: TamanoEdificio, ocupadas: Set<string>, calles: Set<string>): boolean {
  if (!calles.has(arista)) return false;
  const horizontal = arista[0] === 'H';
  const [i, j] = arista.slice(1).split(',').map(Number) as [number, number];
  if (horizontal) {
    const fila = j === min.row ? min.row : min.row + tamano.alto - 1;
    return (
      (calles.has(aristaH(i - 1, j)) && ocupadas.has(claveCelda(i - 1, fila))) ||
      (calles.has(aristaH(i + 1, j)) && ocupadas.has(claveCelda(i + 1, fila)))
    );
  }
  const columna = i === min.col ? min.col : min.col + tamano.ancho - 1;
  return (
    (calles.has(aristaV(i, j - 1)) && ocupadas.has(claveCelda(columna, j - 1))) ||
    (calles.has(aristaV(i, j + 1)) && ocupadas.has(claveCelda(columna, j + 1)))
  );
}

function distanciaAlOrigen(p: Point): number {
  return Math.hypot(p.x, p.y);
}

/**
 * Todos los rectángulos libres donde cabe `tamano`, con su nivel de preferencia ya calculado.
 *
 * `direccion` acota la búsqueda a la cuña de 45° del barrio; `null` = 360° (Granja/Corral/Palacio). La
 * PRIORIDAD BARRIO VS. FILA (§5 del doc) se resuelve aquí: los candidatos de nivel 0 y 1 —los que continúan
 * algo ya empezado— NO se filtran por cuña, así que una fila arrancada se completa aunque su último edificio
 * caiga fuera del ángulo exacto. Sin eso, las manzanas quedarían cortadas por un borde invisible.
 */
function candidatosLibres(
  radioPotencial: number,
  direccion: DireccionCardinal | null,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  red: RedDeCalles,
  distanciaMinima: number
): Candidato[] {
  const maxCeldas = Math.ceil(radioPotencial / T) + 1;
  const anguloObjetivo = direccion ? ANGULO_DIRECCION[direccion] : null;
  const candidatos: Candidato[] = [];

  for (let col = -maxCeldas; col <= maxCeldas; col++) {
    for (let row = -maxCeldas; row <= maxCeldas; row++) {
      const min: Celda = { col, row };
      const punto = puntoDeRectangulo(min, tamano);
      const distancia = distanciaAlOrigen(punto);
      if (distancia < distanciaMinima || distancia > radioPotencial) continue;

      let libre = true;
      for (let dc = 0; dc < tamano.ancho && libre; dc++) {
        for (let dr = 0; dr < tamano.alto && libre; dr++) {
          if (ocupadas.has(claveCelda(col + dc, row + dr))) libre = false;
        }
      }
      if (!libre) continue;

      const aristas = aristasDeRectangulo(min, tamano);
      const conFrente = aristas.some((a) => red.calles.has(a));
      let nivel: number;
      if (conFrente) {
        // Nivel 0 = además CONTINÚA UNA FILA: da a la misma calle que un vecino inmediato, pared con pared.
        // Sin esta distinción la ciudad crece como un borrón compacto —cada edificio se pega donde le queda
        // más cerca del barrio— y las hileras nunca llegan al largo que dispara la transversal: medido, solo
        // 3 de 50 edificios cerraban manzana. Preferir la continuación es lo que alinea las fachadas.
        nivel = aristas.some((a) => continuaFila(a, min, tamano, ocupadas, red.calles)) ? 0 : 1;
      } else {
        // Pared con pared: alguna celda inmediatamente adyacente al rectángulo está ocupada.
        let pegado = false;
        for (let dc = 0; dc < tamano.ancho && !pegado; dc++) {
          if (ocupadas.has(claveCelda(col + dc, row - 1)) || ocupadas.has(claveCelda(col + dc, row + tamano.alto))) pegado = true;
        }
        for (let dr = 0; dr < tamano.alto && !pegado; dr++) {
          if (ocupadas.has(claveCelda(col - 1, row + dr)) || ocupadas.has(claveCelda(col + tamano.ancho, row + dr))) pegado = true;
        }
        nivel = pegado ? 2 : 3;
      }

      const desvio = anguloObjetivo === null ? 0 : diferenciaAngular(Math.atan2(punto.y, punto.x), anguloObjetivo);
      if (nivel === 3 && desvio > MEDIA_CUÑA_DIRECCION) continue;
      // Los niveles 0-2 continúan algo ya empezado, así que se saltan la cuña estricta (§5 del doc: la fila
      // manda dentro de una manzana). Pero no al punto de saltar al otro lado de la ciudad: se admite hasta
      // 90° de desvío, que es el margen para doblar una esquina de manzana.
      if (nivel !== 3 && desvio > Math.PI / 2) continue;
      candidatos.push({ min, punto, nivel });
    }
  }
  return candidatos;
}

function porDistanciaAlOrigen(candidatos: Candidato[], masLejos: boolean): Candidato[] {
  return [...candidatos].sort((a, b) =>
    masLejos ? distanciaAlOrigen(b.punto) - distanciaAlOrigen(a.punto) : distanciaAlOrigen(a.punto) - distanciaAlOrigen(b.punto)
  );
}

/**
 * Huecos donde puede ir un edificio de tipo `tipo`, EN ORDEN DE PREFERENCIA y ya recortados al mejor nivel
 * disponible (§5 del doc: se prefiere siempre lo que ya está conectado).
 *
 * - Granja y Corral: a las afueras. 360°, distancia mínima `TRAZADO.radioAfuerasMin`, y prefieren el hueco MÁS
 *   LEJANO — eso es lo que las mantiene en el borde de la ciudad según crece, no un radio calculado.
 * - Palacio: 360°, el más CERCANO al centro (junto al centro de poder), dentro de la trama urbana.
 * - Resto: la cuña de dirección de su barrio, prefiriendo quedar pegado a lo que ese barrio ya construyó
 *   (acreción) y, en su defecto, lo más cerca del Centro Urbano.
 *
 * `ampliado` devuelve TODOS los huecos del barrio, no solo los del mejor nivel. Lo usa la política "Líneas de
 * Producción" (`sitioEnBarrioLineaProduccion`, construction.ts): recortada al mejor nivel se quedaba con uno o
 * dos huecos y la política dejaba de tener efecto medible. Es deliberado que ahí la logística pueda ganarle a
 * la compacidad —para eso existe la política— y no rompe nada del trazado: el hueco suelto que elija se
 * conectará igual con su tramo de calle en `redDeCalles`, y el solape sigue siendo imposible.
 *
 * Solo necesita `id` + `radioPotencial` de `asentamiento` (narrowing deliberado) para poder reutilizarse en
 * `engine/settlement.ts` durante la FUNDACIÓN, antes de que exista un `Asentamiento` completo.
 */
export function sitiosParaTipo(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  ocupados: Edificio[],
  tipo: EdificioTipo,
  nivelInterno?: number,
  ampliado = false
): Point[] {
  const tamano = tamanoEdificio(tipo, nivelInterno);
  const ocupadas = celdasOcupadas(ocupados);
  const red = redDeCalles(asentamiento.id, ocupados);

  if (esDeAfueras(tipo)) {
    const candidatos = candidatosLibres(radioMaximoAfueras(asentamiento.radioPotencial), null, tamano, ocupadas, red, TRAZADO.radioAfuerasMin);
    return porDistanciaAlOrigen(candidatos, true).map((c) => c.punto);
  }
  if (tipo === 'palacio') {
    const candidatos = candidatosLibres(asentamiento.radioPotencial, null, tamano, ocupadas, red, 0);
    return porDistanciaAlOrigen(candidatos, false).map((c) => c.punto);
  }

  const categoria = CATEGORIA_POR_TIPO[tipo];
  if (!categoria) return [];
  const direccion = direccionesDelAsentamiento(asentamiento.id)[categoria];
  const candidatos = candidatosLibres(asentamiento.radioPotencial, direccion, tamano, ocupadas, red, 0);
  if (candidatos.length === 0) return [];

  const mejorNivel = Math.min(...candidatos.map((c) => c.nivel));
  const delMejorNivel = ampliado ? candidatos : candidatos.filter((c) => c.nivel === mejorNivel);

  const existentesBarrio = edificiosInternos(ocupados).filter((e) => CATEGORIA_POR_TIPO[e.tipo] === categoria);
  if (existentesBarrio.length === 0) return porDistanciaAlOrigen(delMejorNivel, false).map((c) => c.punto);

  return [...delMejorNivel]
    .sort((a, b) => distanciaAlBarrio(a.punto, existentesBarrio) - distanciaAlBarrio(b.punto, existentesBarrio))
    .map((c) => c.punto);
}

function distanciaAlBarrio(punto: Point, existentes: Edificio[]): number {
  let minimo = Infinity;
  for (const e of existentes) {
    const d = Math.hypot(punto.x - e.posicion.x, punto.y - e.posicion.y);
    if (d < minimo) minimo = d;
  }
  return minimo;
}

/** El mejor hueco para `tipo`, o `null` si no cabe en ningún sitio. */
export function sitioParaTipo(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  ocupados: Edificio[],
  tipo: EdificioTipo,
  nivelInterno?: number
): Point | null {
  return sitiosParaTipo(asentamiento, ocupados, tipo, nivelInterno)[0] ?? null;
}

/**
 * Reubica un edificio cuya huella acaba de crecer (hoy solo Granja, §7 del doc): busca el hueco de afueras más
 * cercano posible a donde estaba, sin exigir que quepa en su sitio actual.
 *
 * LA MEJORA MANDA SOBRE LA CERCANÍA: si el único hueco libre está en el extremo opuesto del mapa, se muda
 * igual. Devuelve `null` solo cuando NO existe ningún hueco para el tamaño nuevo en todo el asentamiento —
 * ahí la mejora no puede aplicarse sin romper la invariante de "ningún edificio encima de otro", que es lo
 * único que no se negocia.
 */
export function reubicarPorTamano(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  edificio: Edificio,
  todos: Edificio[],
  nivelInternoNuevo: number
): Point | null {
  const tamano = tamanoEdificio(edificio.tipo, nivelInternoNuevo);
  const ocupadas = celdasOcupadas(todos, edificio.id);
  const red = redDeCalles(asentamiento.id, todos);
  const afueras = esDeAfueras(edificio.tipo);
  const distanciaMinima = afueras ? TRAZADO.radioAfuerasMin : 0;
  const radioMaximo = afueras ? radioMaximoAfueras(asentamiento.radioPotencial) : asentamiento.radioPotencial;
  const candidatos = candidatosLibres(radioMaximo, null, tamano, ocupadas, red, distanciaMinima);
  if (candidatos.length === 0) return null;

  let mejor = candidatos[0]!;
  let mejorDistancia = Infinity;
  for (const c of candidatos) {
    const d = Math.hypot(c.punto.x - edificio.posicion.x, c.punto.y - edificio.posicion.y);
    if (d < mejorDistancia) {
      mejor = c;
      mejorDistancia = d;
    }
  }
  return mejor.punto;
}
