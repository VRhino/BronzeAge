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

export function tamanoDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'rotado'>): TamanoEdificio {
  const tamano = tamanoEdificio(edificio.tipo, edificio.nivelInterno);
  return edificio.rotado ? { ancho: tamano.alto, alto: tamano.ancho } : tamano;
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
export function celdaMinimaDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): Celda {
  const tamano = tamanoDeEdificio(edificio);
  if (edificio.tipo === 'centroUrbano') return { col: 0, row: -tamano.alto };
  return {
    col: Math.round(edificio.posicion.x / T - tamano.ancho / 2),
    row: Math.round(edificio.posicion.y / T - tamano.alto / 2),
  };
}

/** Rectángulo (en celdas) que ocupa un edificio — mismo concepto que `celdasDeEdificio` pero como caja de una
 * pieza, no como lista de celdas. Base de toda comparación borde-a-borde entre anclas y satélites (§5.3, §5.7). */
export interface RectanguloCeldas {
  minCol: number;
  minRow: number;
  ancho: number;
  alto: number;
}

function rectanguloDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): RectanguloCeldas {
  const min = celdaMinimaDeEdificio(edificio);
  const tamano = tamanoDeEdificio(edificio);
  return { minCol: min.col, minRow: min.row, ancho: tamano.ancho, alto: tamano.alto };
}

/**
 * Centro geométrico (coords locales) de un rectángulo de celdas. Para el Centro Urbano, cuyo `posicion` es el
 * VÉRTICE de su esquina y no su centro (excepción documentada arriba), `centroDeRectangulo(rectanguloDeEdificio(cu))`
 * da el punto correcto sin ningún caso especial adicional — `celdaMinimaDeEdificio` ya resuelve esa excepción.
 */
function centroDeRectangulo(r: RectanguloCeldas): Point {
  return puntoDeRectangulo({ col: r.minCol, row: r.minRow }, { ancho: r.ancho, alto: r.alto });
}

/**
 * Hueco real, BORDE A BORDE, entre dos rectángulos de celdas — Chebyshev (la diagonal cuenta como 1 paso, no
 * √2, mismo criterio que el resto de `trazado.ts` ya usa en celdas enteras): 0 si se tocan o se solapan: si no,
 * el mayor entre el hueco de columnas y el de filas.
 *
 * Es la métrica que faltaba en dos sitios (bug medido por el usuario, ver `Consideraciones/
 * Vista_Asentamiento_Trazado_Urbano.md` §"Etapa 2"): la zona de seguridad entre anclas (§5.7) y la atracción
 * dura (§5.3) medían antes distancia CENTRO a CENTRO, así que el propio tamaño de cada edificio se comía el
 * presupuesto sin que el código lo supiera — un Mercado (3x2) y un Centro Urbano (3x3) con los bordes ya
 * tocándose seguían midiendo ~3.35 celdas de centro a centro, muy por encima de cualquier piso razonable.
 */
export function gapCeldas(a: RectanguloCeldas, b: RectanguloCeldas): number {
  const gapCols = Math.max(0, a.minCol - (b.minCol + b.ancho), b.minCol - (a.minCol + a.ancho));
  const gapRows = Math.max(0, a.minRow - (b.minRow + b.alto), b.minRow - (a.minRow + a.alto));
  return Math.max(gapCols, gapRows);
}

/**
 * Largo (en celdas) del tramo de borde realmente compartido entre dos rectángulos que se TOCAN (`gapCeldas`
 * 0) — Etapa 4, punto 2, a petición del usuario: `gapCeldas` por sí solo no distingue tocar por una sola
 * esquina de compartir un lado entero, así que la atracción dura podía preferir un candidato en diagonal sobre
 * uno pegado de lado a lado. 0 si no comparten ningún tramo de lado (incluida la diagonal pura). Solo tiene
 * sentido como desempate SECUNDARIO tras `gapCeldas`, no lo sustituye.
 */
function bordeCompartido(a: RectanguloCeldas, b: RectanguloCeldas): number {
  const tocaVertical = a.minRow + a.alto === b.minRow || b.minRow + b.alto === a.minRow;
  const tocaHorizontal = a.minCol + a.ancho === b.minCol || b.minCol + b.ancho === a.minCol;
  if (tocaVertical) {
    const solape = Math.min(a.minCol + a.ancho, b.minCol + b.ancho) - Math.max(a.minCol, b.minCol);
    if (solape > 0) return solape;
  }
  if (tocaHorizontal) {
    const solape = Math.min(a.minRow + a.alto, b.minRow + b.alto) - Math.max(a.minRow, b.minRow);
    if (solape > 0) return solape;
  }
  return 0;
}

function claveCelda(col: number, row: number): string {
  return `${col},${row}`;
}

function distanciaEntrePuntos(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** Celdas que ocupa un edificio — su rectángulo completo, no solo la celda de su posición. */
export function celdasDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): Celda[] {
  const tamano = tamanoDeEdificio(edificio);
  const min = celdaMinimaDeEdificio(edificio);
  const celdas: Celda[] = [];
  for (let dc = 0; dc < tamano.ancho; dc++) {
    for (let dr = 0; dr < tamano.alto; dr++) celdas.push({ col: min.col + dc, row: min.row + dr });
  }
  return celdas;
}

/** Celdas ocupadas por todos los edificios internos, como set de claves "col,row" para lookup O(1). */
export function celdasOcupadas(edificios: Edificio[], excluirId?: string): Set<string> {
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

export function aristasDePerimetro(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): string[] {
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

// --- Dirección: el asentamiento entero gira sus 8 direcciones cardinales un poco (Etapa 5) ---

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

/** Categoría funcional de cada tipo de edificio "urbano". Ausente = sin categoría fija: Granja/Corral van a
 * las afueras y Palacio junto al centro (ver `sitiosParaTipo`); Centro Urbano es el origen;
 * mina/minaCobre/minaEstano/cantera viven en el mapa general (`ambito: 'mapa'`), fuera de esta rejilla. */
export type CategoriaAsentamiento = 'residencial' | 'industria' | 'militar' | 'mercado' | 'almacenaje' | 'carpinteria';

/** Categoría de cada tipo — exportado para que `app/gameStore.ts` pueda pasárselo a `ui/canvas.ts`
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
  // Mismo truco que puestoMercado/mercado (Etapa 3, §9): categoría propia cuyo único ancla es la Carpintería
  // misma, así el mecanismo de ancla de una sola instancia sirve sin tocarlo para pegar los talleres a ella.
  tallerCarpinteria: 'carpinteria',
};

/**
 * Ancla PRIMARIA de cada categoría — la que ya existe (o se construye normalmente) sin pasar por el árbol de
 * anclas (Etapa 5). Residencial, mercado y carpinteria la tienen: Centro Urbano nace con la fundación, Mercado
 * y Carpintería son construcción normal por cola (aunque, como son ancla de su propia categoría y no tienen a
 * qué atraerse, su propia colocación también pasa por el árbol de anclas — ver `sitiosParaTipo`). Militar e
 * industria NO tienen ancla primaria — nacen enteramente por el árbol, ver `ANCLA_SATURACION_POR_CATEGORIA` y
 * `crearAnclaNueva`.
 */
export const ANCLA_PRIMARIA_POR_CATEGORIA: Partial<Record<CategoriaAsentamiento, EdificioTipo>> = {
  residencial: 'centroUrbano',
  mercado: 'mercado',
  carpinteria: 'carpinteria',
};

/**
 * Anclas DE SATURACIÓN de cada categoría (lista en vez de tipo único desde la Etapa 4, a petición del usuario)
 * — los tipos entre los que se sortea (determinista, ver `crearAnclaNueva`) cuando hace falta una ancla nueva
 * de esta categoría. Militar e industria siguen con un solo tipo — mismo mecanismo, sin caso especial, listas
 * de un elemento. Mercado y carpinteria no tienen entrada aquí a propósito: son su propia ancla primaria y
 * nunca necesitan una ancla adicional.
 */
export const ANCLA_SATURACION_POR_CATEGORIA: Partial<Record<CategoriaAsentamiento, EdificioTipo[]>> = {
  residencial: ['plaza', 'pozo', 'parque'],
  militar: ['plazaDeArmas'],
  industria: ['patioDeGremios'],
};

/** Unión de anclas primarias y de saturación — `redDeCalles` las usa para decidir a quién le toca el anillo
 * completo de calle de §5.2 (ver más abajo), y `crearAnclaNueva`/`sitiosParaTipo` para la separación entre
 * anclas. Centro Urbano ya tenía ese trato desde el principio (era la semilla de la red entera); esto lo
 * extiende a todas las anclas reales y de saturación, primarias o repetibles. */
export const ANCLAS_REALES = new Set<EdificioTipo>([
  ...Object.values(ANCLA_PRIMARIA_POR_CATEGORIA),
  ...Object.values(ANCLA_SATURACION_POR_CATEGORIA).flat(),
]);

/** Tipos de ancla (primaria, si existe, seguida de TODAS las de saturación) válidos para que un satélite de
 * esta categoría se atraiga a ellos (Lógica 2 — `anclaMasCercana` dentro de `sitiosParaTipo`), y entre los que
 * se sortea al crear una ancla nueva de esta categoría (`crearAnclaNueva`). */
function tiposAnclaDe(categoria: CategoriaAsentamiento): EdificioTipo[] {
  const tipos: EdificioTipo[] = [];
  const primaria = ANCLA_PRIMARIA_POR_CATEGORIA[categoria];
  if (primaria) tipos.push(primaria);
  tipos.push(...(ANCLA_SATURACION_POR_CATEGORIA[categoria] ?? []));
  return tipos;
}

/** Tipo de ancla a crear para `categoria` cuando hace falta una nueva (Etapa 5, usado por `asegurarAnclaPara`
 * en construction.ts) — sorteo determinista entre `ANCLA_SATURACION_POR_CATEGORIA` (residencial: plaza/pozo/
 * parque; militar/industria: un solo tipo, sorteo no-op) sembrado por `semillaId` (el id que va a tener la
 * ancla nueva, ya único y determinista). `null` si la categoría no tiene ancla de saturación (mercado: es su
 * propia ancla primaria, se resuelve dentro de `sitiosParaTipo` directamente, nunca llega aquí). */
export function tipoAnclaParaCategoria(categoria: CategoriaAsentamiento, semillaId: string): EdificioTipo | null {
  const opciones = ANCLA_SATURACION_POR_CATEGORIA[categoria];
  if (!opciones || opciones.length === 0) return null;
  return opciones[Math.floor(pseudoAleatorio(hashTexto(semillaId)) * opciones.length)]!;
}

/**
 * Instancia más cercana a `punto`, de cualquiera de `tipos`, entre los edificios internos ya construidos —
 * Lógica 2 (satélites): decide a qué instancia de ancla se atrae un edificio normal de una categoría. `punto`
 * es una referencia fija (el origen del asentamiento) para las búsquedas de colocación. NO decide dónde nace
 * una ancla nueva — eso es responsabilidad de `semillaActiva` (Etapa 5, Lógica 1: árbol único de anclas de
 * TODOS los tipos, sin distinguir categoría).
 */
function anclaMasCercana(tipos: EdificioTipo[], punto: Point, edificios: Edificio[]): Edificio | undefined {
  if (tipos.length === 0) return undefined;
  let mejor: Edificio | undefined;
  let mejorDistancia = Infinity;
  for (const e of edificiosInternos(edificios)) {
    if (!tipos.includes(e.tipo)) continue;
    const d = distanciaEntrePuntos(punto, e.posicion);
    if (d < mejorDistancia) {
      mejor = e;
      mejorDistancia = d;
    }
  }
  return mejor;
}

/** Media ranura de dirección: 8 direcciones cubriendo 360° sin huecos ni solapes = 45° cada una. */
const MEDIA_RANURA_DIRECCION = Math.PI / 8;

/**
 * Rotación (radianes) del eje de las 8 direcciones cardinales, determinista por asentamiento (a petición del
 * usuario: "movemos algunos grados... así es más difícil encontrar ciudades iguales") — aplica a las 8
 * ranuras de crecimiento del árbol único de anclas (`direccionesRotadas`, Etapa 5). El rango se limita a
 * `[0°, 45°)`: más allá de una ranura completa el resultado sería indistinguible de otra rotación (cada una
 * de las 8 direcciones ya cubre 45° del círculo), así que no aportaría variedad nueva, solo complejidad.
 */
function anguloRotacionEje(asentamientoId: string): number {
  return pseudoAleatorio(hashTexto(`${asentamientoId}-rotacion-eje`)) * MEDIA_RANURA_DIRECCION * 2;
}

/** Las 8 direcciones cardinales con el eje ya rotado para este asentamiento (`anguloRotacionEje`), listas para
 * proyectar una ranura de ancla nueva (Etapa 5, `crearAnclaNueva`). */
export function direccionesRotadas(asentamientoId: string): { vector: Point; angulo: number }[] {
  const rotacion = anguloRotacionEje(asentamientoId);
  return DIRECCIONES_CARDINALES.map((d) => {
    const angulo = ANGULO_DIRECCION[d] + rotacion;
    return { vector: { x: Math.cos(angulo), y: Math.sin(angulo) }, angulo };
  });
}

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
export const FONDO_MANZANA = 2;

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
 *  2.5. si además es un ANCLA REAL (Mercado, Carpintería — Etapa 2 de "anclas y satélites", §5.2 del doc), se
 *     añade TODO su perímetro, no solo la fachada de entrada: nace con su propio anillo cerrado, igual que el
 *     Centro Urbano, y ese es el frente que sus satélites llenan primero por atracción dura (§5.3, `sitiosParaTipo`).
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

    // §5.2 del doc: todo ANCLA REAL siembra su anillo completo, no solo la fachada de entrada — mismo trato
    // que ya tenía el Centro Urbano desde el principio, generalizado a Mercado y Carpintería.
    if (ANCLAS_REALES.has(edificio.tipo)) {
      for (const arista of perimetro) red.calles.add(arista);
    }

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

export interface TrazadoAsentamiento {
  calles: SegmentoTrazado[];
  caminos: SegmentoTrazado[];
  /** Rectángulo (coords locales) que ocupa cada edificio, por `id`. */
  huellas: Record<string, { x: number; y: number; ancho: number; alto: number }>;
}

/**
 * Trazado urbano de UN asentamiento, ya resuelto a coordenadas locales para dibujar — orquesta
 * `redDeCalles`/`segmentosDeRed`/`edificiosInternos`/`celdaMinimaDeEdificio`/`tamanoDeEdificio`, las mismas
 * piezas que antes solo combinaba `cliente/src/app/gameStore.ts` (Fase C10, doc 9 T2a: es una consulta de UN
 * asentamiento propio, así que vivir en el motor —no en la capa de aplicación de un cliente concreto— es lo
 * que permite que el servidor la sirva ya resuelta sin que el cliente necesite `engine/trazado` para nada.
 * `cliente/` sigue con su propia copia hasta que se reescriba sin `@motor/*`, fuera de alcance de este hito).
 */
export function trazadoParaAsentamiento(asentamiento: Asentamiento): TrazadoAsentamiento {
  const { calles, caminos } = segmentosDeRed(redDeCalles(asentamiento.id, asentamiento.edificios));
  const huellas: TrazadoAsentamiento['huellas'] = {};
  for (const edificio of edificiosInternos(asentamiento.edificios)) {
    const min = celdaMinimaDeEdificio(edificio);
    const tamano = tamanoDeEdificio(edificio);
    huellas[edificio.id] = { x: min.col * T, y: min.row * T, ancho: tamano.ancho * T, alto: tamano.alto * T };
  }
  return { calles, caminos, huellas };
}

// --- Colocación ---

interface Candidato {
  min: Celda;
  punto: Point;
  /** 0 = continúa una fila sobre una calle existente; 1 = tiene frente de calle; 2 = pared con pared con algún
   * edificio; 3 = suelto (habrá que estirar la red hasta él). Menor es mejor. */
  nivel: number;
  /** true si este candidato usa la huella GIRADA (ancho↔alto) del tipo — Etapa 4, punto 1: orientación
   * intercambiable, mismo criterio de calidad (nivel/hueco/borde) para las dos orientaciones. */
  rotado: boolean;
}

/** Semilla determinista por candidato (posición + orientación) — desempate final cuando nivel, hueco y borde
 * compartido ya empataron, para que dos huecos u orientaciones igual de buenos no resuelvan siempre al mismo
 * (Etapa 4, punto 1: "más aleatoriedad" en la silueta de cada ciudad, a petición del usuario). Determinista:
 * misma celda y misma orientación siempre dan la misma semilla. */
function semillaCandidato(c: Candidato): number {
  return pseudoAleatorio(hashTexto(`${c.min.col},${c.min.row},${c.rotado}`));
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

const ORIGEN_RECT: RectanguloCeldas = { minCol: 0, minRow: 0, ancho: 0, alto: 0 };

function distanciaAlOrigen(p: Point): number {
  return Math.hypot(p.x, p.y);
}

/**
 * Todos los rectángulos libres donde cabe `tamano`, con su nivel de preferencia ya calculado.
 *
 * `referencia` es el rectángulo contra el que se miden radio y distancia (por su CENTRO, ver
 * `centroDeRectangulo`) — `ORIGEN_RECT` (tamaño cero en el origen, así que su centro es el origen mismo) para
 * las búsquedas relativas a la ciudad (afueras/palacio/almacén/leñera), o el rectángulo de un ancla real para
 * la atracción dura (`sitiosPorAtraccionDura`, que además filtra el resultado por HUECO real, no por esta
 * distancia al centro — ver `gapCeldas`). Sin filtro de dirección (Etapa 5: el reparto de barrio por cuña
 * desapareció, todas las búsquedas son 360°).
 */
function candidatosLibres(
  referencia: RectanguloCeldas,
  radioPotencial: number,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  red: RedDeCalles,
  distanciaMinima: number,
  rotado = false
): Candidato[] {
  const centro = centroDeRectangulo(referencia);
  const maxCeldas = Math.ceil(radioPotencial / T) + 1;
  const centroCol = Math.round(centro.x / T);
  const centroRow = Math.round(centro.y / T);
  const candidatos: Candidato[] = [];

  for (let col = centroCol - maxCeldas; col <= centroCol + maxCeldas; col++) {
    for (let row = centroRow - maxCeldas; row <= centroRow + maxCeldas; row++) {
      const min: Celda = { col, row };
      const punto = puntoDeRectangulo(min, tamano);
      const distancia = distanciaEntrePuntos(punto, centro);
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

      candidatos.push({ min, punto, nivel, rotado });
    }
  }
  return candidatos;
}

/** Tipos cuyo tamaño NUNCA debe intercambiarse (Etapa 4, punto 1): `granja` es una progresión real (no
 * cosmética) y `puestoMercado` usa `nivelInterno` para identificar una FORMA concreta de la zona (incluida la
 * asimétrica 3x2, forma 2) — girarla daría otra pieza, no variedad visual. Ningún otro tipo tiene tamaño
 * especial (ver `tamanoEdificio`), así que el resto puede ofrecerse girado sin caso especial. */
const TIPOS_SIN_ROTACION = new Set<EdificioTipo>(['granja', 'puestoMercado']);

export function permiteRotacion(tipo: EdificioTipo, tamano: TamanoEdificio): boolean {
  return !TIPOS_SIN_ROTACION.has(tipo) && tamano.ancho !== tamano.alto;
}

/** Igual que `candidatosLibres`, pero además ofrece la huella GIRADA (ancho↔alto) como candidatos adicionales
 * cuando `permitirRotacion` es true y el tamaño no es cuadrado — Etapa 4, punto 1. El desempate entre
 * orientaciones lo decide quien llama (mismo criterio de nivel/hueco/borde que ya se aplica a la orientación
 * normal, más `semillaCandidato` como último desempate). */
function candidatosConOrientaciones(
  referencia: RectanguloCeldas,
  radioPotencial: number,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  red: RedDeCalles,
  distanciaMinima: number,
  permitirRotacion: boolean
): Candidato[] {
  const normales = candidatosLibres(referencia, radioPotencial, tamano, ocupadas, red, distanciaMinima, false);
  if (!permitirRotacion || tamano.ancho === tamano.alto) return normales;
  const girado: TamanoEdificio = { ancho: tamano.alto, alto: tamano.ancho };
  const girados = candidatosLibres(referencia, radioPotencial, girado, ocupadas, red, distanciaMinima, true);
  return [...normales, ...girados];
}

/** Tamaño real de un candidato, según si usa la orientación normal o la girada (ver `Candidato.rotado`). */
function tamanoDeCandidato(base: TamanoEdificio, c: Candidato): TamanoEdificio {
  return c.rotado ? { ancho: base.alto, alto: base.ancho } : base;
}

function porDistanciaAlOrigen(candidatos: Candidato[], masLejos: boolean): Candidato[] {
  return [...candidatos].sort((a, b) => {
    const porDistancia = masLejos
      ? distanciaAlOrigen(b.punto) - distanciaAlOrigen(a.punto)
      : distanciaAlOrigen(a.punto) - distanciaAlOrigen(b.punto);
    return porDistancia || semillaCandidato(a) - semillaCandidato(b);
  });
}

/**
 * Atracción dura (Lógica 2 — satélites de un ancla): el hueco más pegado posible al ANCLA real de la
 * categoría, buscando en anillos concéntricos por HUECO real (borde a borde, `gapCeldas` — no distancia centro
 * a centro, ver más abajo por qué) — arranca en el anillo (hueco 0, tocando) y se expande de `FONDO_MANZANA`
 * en `FONDO_MANZANA`, capado en `radioMaximoNucleo = separacionMinimaAnclas / 2` (así dos núcleos vecinos
 * nunca se invaden). En cuanto un anillo ofrece algún hueco de nivel 0 o 1 (conectado — nivel 2/3 no cuenta,
 * eso es saturación, no un hueco válido), se detiene ahí. Nunca filtra por dirección: la prioridad es solo
 * cercanía al ancla.
 *
 * BUG medido por el usuario y corregido en su momento: la versión anterior medía la distancia de cada
 * candidato al CENTRO del ancla, así que el propio tamaño del ancla (p.ej. el Mercado, 3x2) ya se comía buena
 * parte del radio disponible antes de llegar a ningún candidato real. Aquí el candidato se escanea con margen
 * de sobra alrededor del ancla y el filtro real —a qué anillo pertenece cada uno— usa `gapCeldas` entre el
 * rectángulo del candidato y el del ancla.
 *
 * `ampliado` (política "Líneas de Producción", ver `sitioEnBarrioLineaProduccion` en construction.ts): en vez
 * de detenerse en el primer anillo con hueco de nivel 0/1 y devolver solo ESE anillo, devuelve TODOS los
 * candidatos dentro del tope (`radioMaximoNucleoCeldas`), de cualquier nivel — así la política de logística
 * tiene un conjunto real donde elegir por distancia a la fuente de sus insumos, no un solo hueco ya decidido.
 *
 * Devuelve `[]` si el núcleo está saturado (ningún candidato dentro del tope, o ninguno con hueco de nivel
 * 0/1 cuando `ampliado` es false). Quien llama (Etapa 5: `sitiosParaTipo`) no tiene ningún fallback que
 * ofrecer — la ancla alcanzable debe garantizarse ANTES de pedir sitio (`asegurarAnclaPara`, construction.ts).
 */
export function sitiosPorAtraccionDura(
  ancla: Edificio,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  red: RedDeCalles,
  permitirRotacion = false,
  ampliado = false
): { punto: Point; rotado: boolean }[] {
  const rectAncla = rectanguloDeEdificio(ancla);
  const radioMaximoNucleoCeldas = TRAZADO.separacionMinimaAnclas / 2;

  // Escaneo generoso: cubre el tope de verdad más el propio tamaño del ancla y del satélite, para que ningún
  // candidato válido quede fuera por culpa del radio de escaneo — el tope real lo decide el hueco, más abajo.
  const margenCeldas = radioMaximoNucleoCeldas + Math.max(rectAncla.ancho, rectAncla.alto) + Math.max(tamano.ancho, tamano.alto);
  const candidatosConHueco = candidatosConOrientaciones(rectAncla, margenCeldas * T, tamano, ocupadas, red, 0, permitirRotacion).map((c) => {
    const rectCandidato: RectanguloCeldas = { minCol: c.min.col, minRow: c.min.row, ...tamanoDeCandidato(tamano, c) };
    return { ...c, rectCandidato, hueco: gapCeldas(rectCandidato, rectAncla) };
  });

  if (ampliado) {
    return candidatosConHueco
      .filter((c) => c.hueco <= radioMaximoNucleoCeldas)
      .map((c) => ({ punto: c.punto, rotado: c.rotado }));
  }

  for (let radioCeldas = 0; radioCeldas <= radioMaximoNucleoCeldas; radioCeldas += FONDO_MANZANA) {
    const enEsteAnillo = candidatosConHueco.filter((c) => c.hueco <= radioCeldas);
    if (enEsteAnillo.length === 0) continue;

    const mejorNivel = Math.min(...enEsteAnillo.map((c) => c.nivel));
    if (mejorNivel > 1) continue; // solo pared/suelto dentro del tope: saturado, no un hueco válido.

    return enEsteAnillo
      .filter((c) => c.nivel === mejorNivel)
      .sort(
        (a, b) =>
          a.hueco - b.hueco ||
          bordeCompartido(b.rectCandidato, rectAncla) - bordeCompartido(a.rectCandidato, rectAncla) ||
          semillaCandidato(a) - semillaCandidato(b)
      )
      .map((c) => ({ punto: c.punto, rotado: c.rotado }));
  }
  return [];
}

/**
 * Instancia de `categoria` a la que atraerse AHORA MISMO (Lógica 2, reemplaza el uso antiguo de
 * `anclaMasCercana` en `sitiosParaTipo`): recorre las instancias no marcadas `anclaLlena` de más cerca a más
 * lejos del origen y prueba cada una con la MISMA `sitiosPorAtraccionDura` que hará después la colocación
 * real — la primera con hueco gana. Las que se prueban y no tienen hueco quedan en `anclasRecienLlenas`, que
 * el llamante (`asegurarAnclaPara`, construction.ts) debe persistir como `anclaLlena: true` ANTES de pedir
 * sitio, igual que `crearAnclaNueva`/`anclasRecienSaturadas` hace para el árbol de anclas (Lógica 1).
 *
 * Antes de esto, `anclaMasCercana` siempre volvía a la instancia más cercana al ORIGEN sin memoria de si
 * tenía hueco — con Centro Urbano fijo en el origen, eso significaba que una vez su núcleo se llenaba (nada
 * libera celdas), ninguna otra instancia de su categoría se volvía a consultar jamás: el asentamiento
 * fabricaba anclas nuevas sin parar en su lugar en vez de reutilizar las que ya tenían hueco de sobra (bug
 * detectado por el usuario jugando con el laboratorio visual).
 */
export function anclaActivaParaCategoria(
  categoria: CategoriaAsentamiento,
  tipo: EdificioTipo,
  nivelInterno: number | undefined,
  edificios: Edificio[],
  red: RedDeCalles
): { instancia: Edificio | null; anclasRecienLlenas: string[] } {
  const tamano = tamanoEdificio(tipo, nivelInterno);
  const permitirRotacion = permiteRotacion(tipo, tamano);
  const ocupadas = celdasOcupadas(edificios);
  const tipos = tiposAnclaDe(categoria);
  const candidatos = edificiosInternos(edificios)
    .filter((e) => tipos.includes(e.tipo) && !e.anclaLlena)
    .sort((a, b) => distanciaAlOrigen(a.posicion) - distanciaAlOrigen(b.posicion));

  const anclasRecienLlenas: string[] = [];
  for (const candidato of candidatos) {
    if (sitiosPorAtraccionDura(candidato, tamano, ocupadas, red, permitirRotacion).length > 0) {
      return { instancia: candidato, anclasRecienLlenas };
    }
    anclasRecienLlenas.push(candidato.id);
  }
  return { instancia: null, anclasRecienLlenas };
}

/**
 * Huecos donde puede ir un edificio de tipo `tipo`, EN ORDEN DE PREFERENCIA (Etapa 5: sin reparto de barrio —
 * toda categoría con ancla exige una ancla alcanzable ya garantizada por quien llama).
 *
 * - Granja y Corral: a las afueras. 360°, distancia mínima `TRAZADO.radioAfuerasMin`, y prefieren el hueco MÁS
 *   LEJANO — eso es lo que las mantiene en el borde de la ciudad según crece, no un radio calculado.
 * - Palacio, Almacén y Leñera: 360°, el más CERCANO al centro, dentro de la trama urbana, sin ancla propia (a
 *   petición del usuario: Almacén/Leñera llenan huecos libres desde Centro Urbano hacia afuera, sin cuña).
 * - Mercado: es su propia ancla primaria — no tiene a qué atraerse, nace directamente por el árbol único de
 *   anclas (`crearAnclaNueva`), igual que cualquier otra ancla (Etapa 5, Lógica 1). Carpintería, en cambio, es
 *   categoría `militar` (satélite de Plaza de Armas, como Barracón) — solo `tallerCarpinteria` usa a
 *   Carpintería como su propia ancla, y eso vive en `crearTalleresDeCarpinteria`, construction.ts.
 * - Resto (con categoría y ancla de saturación — residencial/militar/industria): atracción dura a la instancia
 *   ALCANZABLE Y CON HUECO más cercana al origen (`anclaMasCercana`, filtrada por `!anclaLlena` — Lógica 2).
 *   Si no hay ninguna, devuelve `[]` — quien llama (`asegurarAnclaPara`, construction.ts) debe haber
 *   garantizado una instancia usable (`anclaActivaParaCategoria`) ANTES de pedir sitio.
 *
 * `ampliado` (política "Líneas de Producción", `sitioEnBarrioLineaProduccion` en construction.ts): pide TODOS
 * los candidatos dentro del núcleo del ancla, no solo el mejor — ver `sitiosPorAtraccionDura`.
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
): { punto: Point; rotado: boolean }[] {
  const tamano = tamanoEdificio(tipo, nivelInterno);
  const ocupadas = celdasOcupadas(ocupados);
  const red = redDeCalles(asentamiento.id, ocupados);
  const aPunto = (candidatos: Candidato[]): { punto: Point; rotado: boolean }[] =>
    candidatos.map((c) => ({ punto: c.punto, rotado: c.rotado }));

  if (esDeAfueras(tipo)) {
    const candidatos = candidatosLibres(ORIGEN_RECT, radioMaximoAfueras(asentamiento.radioPotencial), tamano, ocupadas, red, TRAZADO.radioAfuerasMin);
    return aPunto(porDistanciaAlOrigen(candidatos, true));
  }
  if (tipo === 'palacio' || tipo === 'almacen' || tipo === 'lenera') {
    const candidatos = candidatosLibres(ORIGEN_RECT, asentamiento.radioPotencial, tamano, ocupadas, red, 0);
    return aPunto(porDistanciaAlOrigen(candidatos, false));
  }

  const categoria = CATEGORIA_POR_TIPO[tipo];
  if (!categoria) return [];

  // Mercado/Carpintería son su propia ancla primaria (Etapa 5): nacen por el árbol único de anclas igual que
  // cualquier otra. El id es solo semilla determinista para esta consulta — el llamante genera el id real al
  // comprometer la construcción (`crearEdificioEnCola`, construction.ts).
  if (ANCLA_PRIMARIA_POR_CATEGORIA[categoria] === tipo) {
    const resultado = crearAnclaNueva(asentamiento.id, ocupados, tipo, `consulta-${tipo}`);
    return resultado ? [{ punto: resultado.nuevaAncla.posicion, rotado: resultado.nuevaAncla.rotado ?? false }] : [];
  }

  // Orientación intercambiable ancho↔alto (Etapa 4, punto 1): `granja`/`puestoMercado` nunca la usan (ver
  // `permiteRotacion`), el resto la ofrece siempre que su huella no sea cuadrada.
  const permitirRotacion = permiteRotacion(tipo, tamano);

  const anclaInstancia = anclaMasCercana(
    tiposAnclaDe(categoria),
    centroDeRectangulo(ORIGEN_RECT),
    ocupados.filter((e) => !e.anclaLlena)
  );
  if (!anclaInstancia) return [];
  return sitiosPorAtraccionDura(anclaInstancia, tamano, ocupadas, red, permitirRotacion, ampliado);
}

/** Las dos orientaciones de `tamanoAncla` a probar, en un orden sembrado de forma determinista por `semillaId`
 * (Etapa 4, punto 1: variedad también en anclas no cuadradas, ej. `parque` 3x2) — un solo elemento si el ancla
 * es cuadrada, girar no aporta nada. */
function orientacionesDeAncla(semillaId: string, tamanoAncla: TamanoEdificio): { tamano: TamanoEdificio; rotado: boolean }[] {
  if (tamanoAncla.ancho === tamanoAncla.alto) return [{ tamano: tamanoAncla, rotado: false }];
  const girado: TamanoEdificio = { ancho: tamanoAncla.alto, alto: tamanoAncla.ancho };
  const normal = { tamano: tamanoAncla, rotado: false };
  const rotada = { tamano: girado, rotado: true };
  return pseudoAleatorio(hashTexto(`${semillaId}-orientacion-ancla`)) < 0.5 ? [normal, rotada] : [rotada, normal];
}

/** Radio (en celdas, medido centro a centro) al que se prueba la primera ranura a lo largo de una dirección —
 * arranca ya más allá de `separacionMinimaAnclas` para no perder intentos en radios que van a fallar la
 * separación dura, y crece de `FONDO_MANZANA` en `FONDO_MANZANA` (mismo paso que `sitiosPorAtraccionDura`)
 * hasta un tope razonable. */
const RADIO_INICIAL_RANURA = TRAZADO.separacionMinimaAnclas;
const RADIO_MAXIMO_RANURA = TRAZADO.separacionMinimaAnclas * 3;

/**
 * ¿Hay un hueco real para `tamano` a lo largo de `direccion` desde `origen` (centro de la semilla)? Prueba
 * radios crecientes y devuelve el primer rectángulo que no colisiona con nada ya ocupado y respeta
 * `separacionSeguridadAnclas` frente a TODAS las demás anclas — o `null` si ninguno hasta el tope sirve (Etapa
 * 5: esta ranura de la semilla queda descartada, `crearAnclaNueva` prueba la siguiente dirección).
 */
export function huecoEnDireccion(
  origen: Point,
  direccion: Point,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  otrasAnclas: RectanguloCeldas[]
): RectanguloCeldas | null {
  for (let radioCeldas = RADIO_INICIAL_RANURA; radioCeldas <= RADIO_MAXIMO_RANURA; radioCeldas += FONDO_MANZANA) {
    const centro: Point = { x: origen.x + direccion.x * radioCeldas * T, y: origen.y + direccion.y * radioCeldas * T };
    const min: Celda = { col: Math.round(centro.x / T - tamano.ancho / 2), row: Math.round(centro.y / T - tamano.alto / 2) };
    const rect: RectanguloCeldas = { minCol: min.col, minRow: min.row, ancho: tamano.ancho, alto: tamano.alto };

    let libre = true;
    for (let dc = 0; dc < rect.ancho && libre; dc++) {
      for (let dr = 0; dr < rect.alto && libre; dr++) {
        if (ocupadas.has(claveCelda(rect.minCol + dc, rect.minRow + dr))) libre = false;
      }
    }
    if (!libre) continue;
    if (otrasAnclas.some((a) => gapCeldas(rect, a) < TRAZADO.separacionSeguridadAnclas)) continue;
    return rect;
  }
  return null;
}

/**
 * Entre las anclas NO descartadas (`ANCLAS_REALES`, sin `semillaSaturada`, y sin las `excluidas` de esta
 * misma búsqueda), la más cercana al origen del asentamiento — Etapa 5, Lógica 1: un único árbol para TODAS
 * las anclas, sin distinguir tipo. `undefined` solo si no queda ninguna (asentamiento sin anclas en absoluto,
 * o las 8 direcciones de TODAS agotadas — caso límite confirmado por simulación: solo ocurre cuando el 100%
 * del espacio físico disponible ya está ocupado).
 *
 * `anclaLlena` (Lógica 2) NO participa aquí — es un criterio aparte, sin relación con esta selección de
 * semilla (confirmado con el usuario tras una primera corrección que sí las mezclaba). Un ancla puede estar
 * `anclaLlena` y seguir siendo la semilla activa del árbol mientras sus 8 ranuras de crecimiento tengan sitio.
 */
export function semillaActiva(edificios: Edificio[], excluidas: Set<string> = new Set()): Edificio | undefined {
  let mejor: Edificio | undefined;
  let mejorDistancia = Infinity;
  for (const e of edificiosInternos(edificios)) {
    if (!ANCLAS_REALES.has(e.tipo) || e.semillaSaturada || excluidas.has(e.id)) continue;
    const d = distanciaAlOrigen(e.posicion);
    if (d < mejorDistancia) {
      mejor = e;
      mejorDistancia = d;
    }
  }
  return mejor;
}

/** Orden aleatorio (determinista, sembrado por `semilla.id`+intento) de las 8 direcciones ya rotadas para el
 * asentamiento — Etapa 5: la ranura se elige al azar, no en orden fijo N→NE→E... */
function direccionesBarajadas(semillaId: string, asentamientoId: string): Point[] {
  const direcciones = direccionesRotadas(asentamientoId).map((d) => d.vector);
  for (let i = direcciones.length - 1; i > 0; i--) {
    const j = Math.floor(pseudoAleatorio(hashTexto(`${semillaId}-ranura-${i}`)) * (i + 1));
    [direcciones[i], direcciones[j]] = [direcciones[j]!, direcciones[i]!];
  }
  return direcciones;
}

/**
 * Crea una ancla nueva de `tipoAncla` en el árbol único de anclas (Etapa 5, Lógica 1) — recorre la semilla
 * activa (la ancla no saturada más cercana a la raíz, de CUALQUIER tipo, sin distinguir categoría) probando
 * sus 8 direcciones en orden aleatorio; si ninguna de las 8 tiene hueco real (`huecoEnDireccion`), esa semilla
 * se descarta PARA SIEMPRE (`anclasRecienSaturadas`, que el llamante debe persistir como `semillaSaturada` en
 * el array real de edificios) y se prueba la siguiente semilla más cercana. `null` si no queda ninguna semilla
 * disponible en absoluto — caso límite: el asentamiento ya no tiene dónde crecer para este tipo de ancla.
 */
function anguloNormalizado(a: number): number {
  let r = a % (Math.PI * 2);
  if (r > Math.PI) r -= Math.PI * 2;
  if (r < -Math.PI) r += Math.PI * 2;
  return r;
}

/**
 * ¿Ya hay una ancla existente sobre esta ranura (dirección) de `centroSemilla`? Compara el ángulo desde
 * `centroSemilla` hacia cada una de `otrasAnclas` contra `direccion` — si alguna cae casi exacta (margen
 * generoso pero muy por debajo de los 45° entre ranuras, para absorber el redondeo a celda de
 * `huecoEnDireccion`) Y a una distancia dentro del rango de una ranura (`RADIO_INICIAL_RANURA`..
 * `RADIO_MAXIMO_RANURA`, con margen), esa ranura se considera OCUPADA — bug detectado con el laboratorio
 * visual: sin este chequeo, `huecoEnDireccion` simplemente sigue expandiendo el radio en la misma dirección ya
 * usada hasta encontrar hueco más lejos, en vez de repartirse entre las 8 direcciones libres.
 */
function ranuraOcupada(centroSemilla: Point, direccion: Point, otrasAnclas: RectanguloCeldas[]): boolean {
  const anguloDireccion = Math.atan2(direccion.y, direccion.x);
  const EPS_ANGULO = 0.35;
  const MARGEN_RADIO_CELDAS = 2;
  return otrasAnclas.some((ancla) => {
    const centro = centroDeRectangulo(ancla);
    const dx = centro.x - centroSemilla.x;
    const dy = centro.y - centroSemilla.y;
    const distanciaCeldas = Math.hypot(dx, dy) / T;
    if (distanciaCeldas < RADIO_INICIAL_RANURA - MARGEN_RADIO_CELDAS || distanciaCeldas > RADIO_MAXIMO_RANURA + MARGEN_RADIO_CELDAS) return false;
    return Math.abs(anguloNormalizado(Math.atan2(dy, dx) - anguloDireccion)) < EPS_ANGULO;
  });
}

export function crearAnclaNueva(
  asentamientoId: string,
  edificios: Edificio[],
  tipoAncla: EdificioTipo,
  id: string
): { nuevaAncla: Edificio; anclasRecienSaturadas: string[] } | null {
  const tamanoBase = tamanoEdificio(tipoAncla);
  const ocupadas = celdasOcupadas(edificios);
  const excluidas = new Set<string>();
  let semilla = semillaActiva(edificios, excluidas);

  while (semilla) {
    const otrasAnclas = edificiosInternos(edificios)
      .filter((e) => ANCLAS_REALES.has(e.tipo) && e.id !== semilla!.id)
      .map(rectanguloDeEdificio);
    const centroSemilla = centroDeRectangulo(rectanguloDeEdificio(semilla));

    // Ranuras libres primero (a la mínima distancia posible que cada una permita) — solo si NINGUNA de las
    // libres sirve se recurre a una ya usada, expandiendo su radio como último recurso antes de descartar la
    // semilla entera (ver `ranuraOcupada`).
    const direcciones = direccionesBarajadas(semilla.id, asentamientoId);
    const libres = direcciones.filter((d) => !ranuraOcupada(centroSemilla, d, otrasAnclas));
    const usadas = direcciones.filter((d) => ranuraOcupada(centroSemilla, d, otrasAnclas));

    for (const direccion of [...libres, ...usadas]) {
      for (const { tamano, rotado } of orientacionesDeAncla(`${semilla.id}-${id}`, tamanoBase)) {
        const rect = huecoEnDireccion(centroSemilla, direccion, tamano, ocupadas, otrasAnclas);
        if (!rect) continue;
        return {
          nuevaAncla: {
            id,
            tipo: tipoAncla,
            posicion: centroDeRectangulo(rect),
            estado: 'activo',
            ticksRestantes: 0,
            ambito: 'asentamiento',
            rotado,
          },
          anclasRecienSaturadas: [...excluidas],
        };
      }
    }
    excluidas.add(semilla.id);
    semilla = semillaActiva(edificios, excluidas);
  }
  return null;
}

/** El mejor hueco para `tipo`, o `null` si no cabe en ningún sitio. */
export function sitioParaTipo(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  ocupados: Edificio[],
  tipo: EdificioTipo,
  nivelInterno?: number
): { punto: Point; rotado: boolean } | null {
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
  const candidatos = candidatosLibres(ORIGEN_RECT, radioMaximo, tamano, ocupadas, red, distanciaMinima);
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
