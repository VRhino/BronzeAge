import type { Asentamiento, Edificio, EdificioTipo, Point } from '../domain/types';
import {
  EDIFICIO_CATALOGO,
  EDIFICIO_TAMANO,
  EDIFICIO_TAMANO_POR_DEFECTO,
  PUESTO_MERCADO_FORMA,
  REJILLA_ASENTAMIENTO,
  TRAZADO,
} from '../constants';

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

/** Un rectángulo ya resuelto a coordenadas LOCALES — lo que `ui/canvas.ts` recibe para pintar, sin saber nada
 * de celdas (acoplamiento 0). Sirve igual para la huella de un edificio y para una tirada de calle: desde la
 * Etapa 6 una calle también es un área, no una línea. */
export interface RectanguloLocal {
  x: number;
  y: number;
  ancho: number;
  alto: number;
}

/** Calles (urbanas: forman filas y cierran manzanas) y caminos (rurales: solo conectan Granja/Corral con la
 * ciudad, ver §11 del doc) como conjuntos de claves de CELDA (`"col,row"`, mismo formato que `celdasOcupadas`).
 * Separados porque son clases distintas, no dos grosores del mismo trazo.
 *
 * Etapa 6 (§E6.2): eran claves de ARISTA. Que ahora sean celdas —del mismo espacio que las de los edificios— es
 * lo que permite meterlas en `ocupadas` y hacer que la calle cueste suelo. */
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
 * interno (ver `EDIFICIO_CATALOGO.granja.niveles`); el resto lo tiene fijo en `EDIFICIO_TAMANO`, y un tipo
 * ausente de esa tabla mide `EDIFICIO_TAMANO_POR_DEFECTO`.
 *
 * Ojo con las medidas escritas en los comentarios de este archivo y del catálogo: se acordaron con el usuario
 * en la rejilla ORIGINAL y siguen expresadas así, pero el Paso 1 de la Etapa 6 (doc trazado §E6.11) dobló
 * todas las huellas al partir la celda por la mitad. Lo que era "1x1" son hoy 2x2 celdas, y sigue midiendo lo
 * mismo en unidades locales.
 */
export function tamanoEdificio(tipo: EdificioTipo, nivelInterno?: number): TamanoEdificio {
  if (tipo === 'granja') {
    const niveles = EDIFICIO_CATALOGO.granja.niveles as Record<number, { tamano?: TamanoEdificio }>;
    return niveles[nivelInterno ?? 1]?.tamano ?? EDIFICIO_TAMANO_POR_DEFECTO;
  }
  // Puesto de Mercado: `nivelInterno` no es progresión, identifica qué FORMA tiene esta pieza de la zona.
  if (tipo === 'puestoMercado') return PUESTO_MERCADO_FORMA[nivelInterno ?? 1] ?? EDIFICIO_TAMANO_POR_DEFECTO;
  return EDIFICIO_TAMANO[tipo] ?? EDIFICIO_TAMANO_POR_DEFECTO;
}

export function tamanoDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'rotado'>): TamanoEdificio {
  const tamano = tamanoEdificio(edificio.tipo, edificio.nivelInterno);
  return edificio.rotado ? { ancho: tamano.alto, alto: tamano.ancho } : tamano;
}

/**
 * Centro del rectángulo `tamano` cuya esquina mínima (menor col, menor row) es `celdaMin` — el valor que se
 * guarda en `Edificio.posicion`.
 *
 * De esta fórmula depende que reescalar la rejilla no mueva ninguna partida guardada: doblar `col` y `ancho`
 * a la vez que se parte `T` por la mitad devuelve EXACTAMENTE el mismo punto local
 * (`(2·col + 2·ancho/2)·(T/2) = (col + ancho/2)·T`). Congelado en `escalaRejilla.test.ts` con una tabla
 * generada antes del Paso 1 de la Etapa 6 (doc trazado §E6.11).
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
 * SIN excepciones desde 2026-08-31 (doc trazado §E6.20): el Centro Urbano tenía un caso especial —su `posicion`
 * `(0,0)` era el VÉRTICE de una esquina, no su centro— que a petición del usuario se quitó. Ahora `(0,0)` es su
 * CENTRO real, como en cualquier otro edificio: `centroDeRectangulo(rectanguloDeEdificio(cu))` y `cu.posicion`
 * coinciden, y la ciudad crece simétrica alrededor del origen. Con 6x6 el CU ocupa las columnas -3..2 y las
 * filas -3..2.
 */
export function celdaMinimaDeEdificio(edificio: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): Celda {
  const tamano = tamanoDeEdificio(edificio);
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

/**
 * El SUELO ocupado de un asentamiento: edificios **y** red de calles, más la propia red. Es lo que toda
 * colocación necesita, y desde la Etapa 6 las dos cosas no se pueden calcular por separado.
 *
 * Existe como una sola función a propósito. Con las calles sobre celdas, olvidarse de meter la red en
 * `ocupadas` en UNO de los cuatro puntos de colocación bastaría para que ese camino plantara edificios encima
 * de las calles — y el síntoma sería visual y tardío, no un error de tipos. Centralizarlo hace que ese olvido
 * no sea expresable.
 *
 * Exportada porque el LABORATORIO (`lab/src/debugAnclas.ts`) reconstruye el árbol de anclas reproduciendo
 * `huecoEnDireccion` paso a paso, y para que su reproducción coincida con lo que hizo el motor tiene que ver
 * EXACTAMENTE el mismo suelo ocupado. Cuando calculaba la ocupación solo con edificios, las anclas salían con
 * código "?" (padre irreconocible) — el primer bug que encontró el laboratorio al recuperarlo.
 */
/** Añade las celdas de la red a un conjunto de celdas ocupadas ya calculado — para los caminos que reciben la
 * red hecha desde fuera en vez de derivarla ellos (ver `sueloOcupado`, que es la vía normal). */
function conCeldasDeRed(ocupadas: Set<string>, red: RedDeCalles): Set<string> {
  for (const clave of red.calles) ocupadas.add(clave);
  for (const clave of red.caminos) ocupadas.add(clave);
  return ocupadas;
}

export function sueloOcupado(
  asentamientoId: string,
  edificios: Edificio[],
  excluirId?: string
): { ocupadas: Set<string>; red: RedDeCalles } {
  const red = redDeCalles(asentamientoId, edificios);
  const ocupadas = celdasOcupadas(edificios, excluirId);
  for (const clave of red.calles) ocupadas.add(clave);
  for (const clave of red.caminos) ocupadas.add(clave);
  return { ocupadas, red };
}

// --- La red de calles OCUPA CELDAS (Etapa 6, doc trazado §E6.2) ---
//
// Antes corría sobre las ARISTAS de la rejilla, que no ocupan superficie. Eso daba tres garantías "gratis"
// (un edificio nunca sobre una calle, una calle nunca sobre otra, un tramo nunca atravesando un edificio) que
// resultaron ser gratis porque no garantizaban nada real: MEDIDO, el 51% de la red corría por el muro
// compartido de dos edificios pegados —calles de ancho cero, irrealizables en 3D— y solo el 33% de los
// edificios tenía delante algo por lo que se pudiera caminar.
//
// La causa era económica, no geométrica: una arista es GRATIS, así que apelotonarse regalaba calles sin pagar
// suelo. Con celdas la calle cuesta terreno, y ese coste es la única presión capaz de producir hileras y
// manzanas de verdad.
//
// Dos de las tres garantías pasan de gratuitas a obligaciones: las celdas de calle entran en `ocupadas` (nadie
// construye encima) y un corredor tiene que RODEAR los edificios en vez de atravesarlos. La tercera sigue
// siendo gratis: la red es un `Set` de celdas, añadir una repetida es un no-op.

/** Las 4 celdas vecinas ortogonales, en orden FIJO (arriba, derecha, abajo, izquierda). El orden importa: es
 * lo que hace determinista el corredor que traza el BFS, y por tanto la red entera. */
const VECINAS_ORTOGONALES: readonly [number, number][] = [
  [0, -1],
  [1, 0],
  [0, 1],
  [-1, 0],
];

/** Las celdas del anillo de UNA celda de grosor que rodea un rectángulo — el "frente" que un ancla ofrece a sus
 * satélites (§5.2) y la semilla de la red (el anillo del Centro Urbano). */
function anilloDeRectangulo(r: RectanguloCeldas): Celda[] {
  const celdas: Celda[] = [];
  for (let dc = -1; dc <= r.ancho; dc++) {
    celdas.push({ col: r.minCol + dc, row: r.minRow - 1 });
    celdas.push({ col: r.minCol + dc, row: r.minRow + r.alto });
  }
  for (let dr = 0; dr < r.alto; dr++) {
    celdas.push({ col: r.minCol - 1, row: r.minRow + dr });
    celdas.push({ col: r.minCol + r.ancho, row: r.minRow + dr });
  }
  return celdas;
}

/** Celdas ortogonalmente adyacentes a un rectángulo (su anillo sin las cuatro esquinas): por ahí es por donde
 * un edificio da a la calle. Una esquina NO cuenta como frente — no se puede salir por una diagonal. */
function adyacentesARectangulo(r: RectanguloCeldas): Celda[] {
  const celdas: Celda[] = [];
  for (let dc = 0; dc < r.ancho; dc++) {
    celdas.push({ col: r.minCol + dc, row: r.minRow - 1 });
    celdas.push({ col: r.minCol + dc, row: r.minRow + r.alto });
  }
  for (let dr = 0; dr < r.alto; dr++) {
    celdas.push({ col: r.minCol - 1, row: r.minRow + dr });
    celdas.push({ col: r.minCol + r.ancho, row: r.minRow + dr });
  }
  return celdas;
}

/** Empaqueta una celda en un entero, para usarla como clave de `Map`/`Set` sin construir una cadena. El
 * desplazamiento admite coordenadas negativas; el rango cubre de sobra cualquier asentamiento (±32768 celdas
 * frente a las ~60 que ocupa el mayor). Medido: las claves de texto en el trazado del corredor eran el 22% del
 * tiempo de la simulación de balance. */
function claveNumerica(col: number, row: number): number {
  return (col + 32768) * 65536 + (row + 32768);
}

/** Como `tieneFrenteDeCalle`, pero sin construir el array de celdas adyacentes: recorre las cuatro franjas
 * directamente. Se usa en el replay de `redDeCalles`, que lo pregunta una vez por edificio. */
function tieneFrenteDeCalle(r: RectanguloCeldas, red: RedDeCalles): boolean {
  const enRed = (col: number, row: number): boolean => {
    const clave = claveCelda(col, row);
    return red.calles.has(clave) || red.caminos.has(clave);
  };
  for (let dc = 0; dc < r.ancho; dc++) {
    if (enRed(r.minCol + dc, r.minRow - 1)) return true;
    if (enRed(r.minCol + dc, r.minRow + r.alto)) return true;
  }
  for (let dr = 0; dr < r.alto; dr++) {
    if (enRed(r.minCol - 1, r.minRow + dr)) return true;
    if (enRed(r.minCol + r.ancho, r.minRow + dr)) return true;
  }
  return false;
}

/**
 * Corredor de celdas LIBRES que conecta un rectángulo con la red, o `null` si no existe ninguno dentro de
 * `cap`. BFS 4-conexo desde las celdas adyacentes al rectángulo; una celda vale como meta cuando ya toca la
 * red, de modo que al marcar el corredor entero como calle el resultado queda pegado por los dos extremos.
 *
 * Esto es lo que sustituye a `conectarEdificio`, que trazaba una "L" sobre las líneas de la rejilla y **no
 * podía fallar nunca** porque una arista siempre estaba disponible. Con celdas sí puede fallar (a 70% de
 * ocupación local puede no quedar paso), y por eso deja de ser una reparación posterior para convertirse en
 * condición de validez del candidato (§E6.5): si no hay corredor, ese sitio no es un sitio.
 *
 * Determinista por construcción: las celdas de arranque se recorren en el orden de `adyacentesARectangulo` y
 * los vecinos en el de `VECINAS_ORTOGONALES`. El BFS garantiza además que el corredor es de longitud mínima.
 */
/**
 * Añade a `destino` solo las celdas de `candidatas` que quedan CONECTADAS a la red ya existente, sea
 * directamente o a través de otras celdas del mismo grupo. Las que no conectan no se añaden.
 *
 * Existe porque la inducción de conectividad de §E6.4 —"la red arranca conexa y cada edificio o ya la toca o
 * abre un corredor, que es un camino"— **solo cubría el corredor**. El anillo de un ancla y las franjas del
 * retículo se añadían sueltas, y una primera versión de la Etapa 6 dejó 48 de 95 celdas de calle inalcanzables
 * a pie en un asentamiento real: la red se partía en islas. Filtrar por conexión al añadirlas cierra el hueco
 * sin tocar nada más.
 *
 * De paso resulta MÁS fiel al principio rector que añadirlas sin más: una franja del retículo que todavía no
 * llega a la ciudad simplemente no existe todavía, que es justo lo que dice §10 ("la calle entera se va
 * formando conforme la ciudad crece hacia ahí, y si nunca crece, esa calle no llega a existir").
 */
function anadirConectadas(candidatas: Celda[], ocupadas: Set<string>, red: RedDeCalles, destino: Set<string>): void {
  const enRed = (clave: string): boolean => red.calles.has(clave) || red.caminos.has(clave);
  const libres = new Map<string, Celda>();
  for (const c of candidatas) {
    const clave = claveCelda(c.col, c.row);
    if (ocupadas.has(clave) || enRed(clave)) continue;
    libres.set(clave, c);
  }
  if (libres.size === 0) return;

  // La red vacía es el caso raíz: el anillo del Centro Urbano no tiene nada a lo que conectarse todavía.
  const redVacia = red.calles.size === 0 && red.caminos.size === 0;

  const alcanzadas = new Set<string>();
  const cola: Celda[] = [];
  for (const [clave, celda] of libres) {
    let semilla = redVacia;
    if (!semilla) {
      for (const [dc, dr] of VECINAS_ORTOGONALES) {
        if (enRed(claveCelda(celda.col + dc, celda.row + dr))) {
          semilla = true;
          break;
        }
      }
    }
    if (!semilla) continue;
    alcanzadas.add(clave);
    cola.push(celda);
  }

  for (let i = 0; i < cola.length; i++) {
    const actual = cola[i]!;
    for (const [dc, dr] of VECINAS_ORTOGONALES) {
      const clave = claveCelda(actual.col + dc, actual.row + dr);
      if (alcanzadas.has(clave)) continue;
      const vecina = libres.get(clave);
      if (!vecina) continue;
      alcanzadas.add(clave);
      cola.push(vecina);
    }
  }

  for (const clave of alcanzadas) destino.add(clave);
}

function corredorHastaLaRed(
  r: RectanguloCeldas,
  ocupadas: Set<string>,
  red: RedDeCalles,
  cap: number
): string[] | null {
  // Índice numérico -> celda, para que el BFS no construya ni una cadena por celda visitada. Solo las celdas
  // del camino GANADOR se convierten a clave de texto al final, que son unas pocas.
  const previa = new Map<number, number>();
  const colDe = new Map<number, number>();
  const rowDe = new Map<number, number>();
  const cola: number[] = [];
  const pasos: number[] = [];

  const registrar = (col: number, row: number, desde: number, paso: number): void => {
    const clave = claveNumerica(col, row);
    if (previa.has(clave)) return;
    if (ocupadas.has(claveCelda(col, row))) return;
    previa.set(clave, desde);
    colDe.set(clave, col);
    rowDe.set(clave, row);
    cola.push(clave);
    pasos.push(paso);
  };

  const tocaLaRed = (col: number, row: number): boolean => {
    for (const [dc, dr] of VECINAS_ORTOGONALES) {
      const clave = claveCelda(col + dc, row + dr);
      if (red.calles.has(clave) || red.caminos.has(clave)) return true;
    }
    return false;
  };

  // Arranque: las celdas adyacentes al rectángulo, en el orden fijo de `adyacentesARectangulo` — de ahí sale
  // el determinismo del corredor, y con él el de la red entera.
  for (let dc = 0; dc < r.ancho; dc++) {
    registrar(r.minCol + dc, r.minRow - 1, -1, 1);
    registrar(r.minCol + dc, r.minRow + r.alto, -1, 1);
  }
  for (let dr = 0; dr < r.alto; dr++) {
    registrar(r.minCol - 1, r.minRow + dr, -1, 1);
    registrar(r.minCol + r.ancho, r.minRow + dr, -1, 1);
  }

  for (let i = 0; i < cola.length; i++) {
    const clave = cola[i]!;
    const col = colDe.get(clave)!;
    const row = rowDe.get(clave)!;
    if (tocaLaRed(col, row)) {
      const camino: string[] = [];
      let actual: number | undefined = clave;
      while (actual !== undefined && actual !== -1) {
        camino.push(claveCelda(colDe.get(actual)!, rowDe.get(actual)!));
        actual = previa.get(actual);
      }
      return camino;
    }
    const paso = pasos[i]!;
    if (paso >= cap) continue;
    for (const [dc, dr] of VECINAS_ORTOGONALES) {
      const c = col + dc;
      const f = row + dr;
      // Nunca a través del propio edificio.
      if (c >= r.minCol && c < r.minCol + r.ancho && f >= r.minRow && f < r.minRow + r.alto) continue;
      registrar(c, f, clave, paso + 1);
    }
  }
  return null;
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
 * ¿Son AFINES dos tipos de edificio? — regla de agrupación pedida por el usuario (2026-08-31): un edificio, al
 * colocarse, prefiere (como DESEMPATE tras la adyacencia al ancla) el hueco que más lado comparte con los
 * suyos. "Los suyos" = mismo tipo exacto (viviendas con viviendas, puestos con puestos) o misma categoría
 * funcional (fundición/curtiduría/armería/… todas 'industria').
 *
 * Se apoya en `CATEGORIA_POR_TIPO` a propósito: los tipos de ANCLA (centroUrbano, plaza, plazaDeArmas,
 * patioDeGremios…) no están en esa tabla, así que un satélite nunca sale "afín" a su ancla por aquí — la
 * adyacencia al ancla ya es el criterio primario y contarla otra vez la duplicaría.
 */
export function tiposAfines(a: EdificioTipo, b: EdificioTipo): boolean {
  if (a === b) return true;
  const ca = CATEGORIA_POR_TIPO[a];
  return ca !== undefined && ca === CATEGORIA_POR_TIPO[b];
}

/** Celdas ocupadas por edificios afines a `tipo` (`tiposAfines`), excluida `excluirId` (el ancla) — el
 * "territorio de los suyos" contra el que `bordeAfinDe` mide cuánto lado compartiría un candidato. */
function celdasDeTiposAfines(edificios: Edificio[], tipo: EdificioTipo, excluirId: string): Set<string> {
  const set = new Set<string>();
  for (const e of edificiosInternos(edificios)) {
    if (e.id === excluirId || !tiposAfines(tipo, e.tipo)) continue;
    for (const c of celdasDeEdificio(e)) set.add(claveCelda(c.col, c.row));
  }
  return set;
}

/** Largo de lado (en celdas) que `rect` compartiría con edificios afines: cuenta las celdas de `celdasAfines`
 * ortogonalmente adyacentes a su perímetro. Las esquinas no cuentan — no se comparte lado por una diagonal,
 * mismo criterio que `bordeCompartido`. */
function bordeAfinDe(rect: RectanguloCeldas, celdasAfines: Set<string>): number {
  if (celdasAfines.size === 0) return 0;
  let n = 0;
  for (let dc = 0; dc < rect.ancho; dc++) {
    if (celdasAfines.has(claveCelda(rect.minCol + dc, rect.minRow - 1))) n++;
    if (celdasAfines.has(claveCelda(rect.minCol + dc, rect.minRow + rect.alto))) n++;
  }
  for (let dr = 0; dr < rect.alto; dr++) {
    if (celdasAfines.has(claveCelda(rect.minCol - 1, rect.minRow + dr))) n++;
    if (celdasAfines.has(claveCelda(rect.minCol + rect.ancho, rect.minRow + dr))) n++;
  }
  return n;
}

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
function radioMaximoAfueras(radioPotencial: number, tamano: TamanoEdificio): number {
  // El tope acota el CENTRO del edificio, pero la banda tiene que poder CONTENERLO entero: sumarle su media
  // diagonal es lo que permite que un edificio del tamaño que sea llegue con su borde interior hasta el final
  // de la banda, en vez de quedarse a medias porque su centro topa antes.
  //
  // Sin esto la banda de afueras se quedaba corta justo donde importa: mide `anchoBandaAfueras` = 36 unidades
  // y una Granja de nivel 4 mide 36 de lado, así que literalmente no cabía sin retroceder hacia la ciudad —
  // medido: cinco de nueve mejoras acababan más cerca del centro aunque la reubicación ya prefiriera afuera.
  const mediaDiagonal = Math.hypot(tamano.ancho * T, tamano.alto * T) / 2;
  return Math.max(radioPotencial, TRAZADO.radioAfuerasMin + TRAZADO.anchoBandaAfueras) + mediaDiagonal;
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
 * quedan alargados, como una manzana de verdad, y no un cuadrado lleno de callejones interiores para dar
 * salida a los edificios del centro.
 *
 * 2 → 4 en el Paso 1 de la Etapa 6 (§E6.11): sigue siendo "dos hileras de Vivienda espalda con espalda", solo
 * que una Vivienda mide ahora 2 celdas de lado en vez de 1.
 */
export const FONDO_MANZANA = 4;

/**
 * La red de calles y caminos del asentamiento, reconstruida desde cero (§2 del doc: derivada, no persistida).
 * Etapa 6: la red son CELDAS, no aristas.
 *
 * Semilla: el ANILLO DE CELDAS que rodea al Centro Urbano. Nace ya cerrado —el CU es la manzana cero— y ofrece
 * frente de calle en las cuatro direcciones desde el primer tick. Eso es lo que evita el bloqueo que hundió el
 * intento anterior, donde algunos barrios no tenían ningún sitio viable al principio.
 *
 * Después, por cada edificio en orden de construcción:
 *  1. si alguna celda ortogonalmente adyacente ya es red, no se añade nada (el caso común de la ciudad
 *     compacta: el edificio nació dando a una calle que ya existía);
 *  2. si no, se abre un corredor de celdas libres hasta la red (`corredorHastaLaRed`). Puede NO existir, a
 *     diferencia del viejo trazado en "L" sobre aristas — por eso la colocación tiene que garantizarlo antes
 *     (§E6.5), y aquí el replay se limita a reproducir lo que la colocación ya validó;
 *  2.5. si además es un ANCLA REAL (Mercado, Carpintería — §5.2), siembra TODO su anillo, igual que el Centro
 *     Urbano: ese es el frente que sus satélites llenan primero por atracción dura (§5.3);
 *  3. las columnas y filas del RETÍCULO que le quedan pegadas se vuelven calle. Es la versión en celdas del
 *     criterio posicional de §10: el edificio aporta solo SU trozo, y la calle entera se va formando conforme
 *     la ciudad crece hacia ahí. Si nunca crece, esa calle no llega a existir — sigue sin haber plano previo.
 *
 * **Las celdas de calle nunca pisan un edificio**: cada `añadir` comprueba `ocupadas` primero. Un anillo o una
 * franja de retículo pueden salir incompletos, y eso es correcto — un edificio grande desvía la calle
 * localmente, que es exactamente lo que significa "retículo blando" (§E6.8).
 */
export function redDeCalles(asentamientoId: string, edificios: Edificio[]): RedDeCalles {
  const red: RedDeCalles = { calles: new Set(), caminos: new Set() };
  const internos = edificiosInternos(edificios);
  const centro = internos.find((e) => e.tipo === 'centroUrbano');
  if (!centro) return red;

  // Ocupación acumulada: solo los edificios YA procesados. El replay tiene que ver la ciudad como estaba en el
  // momento de construir cada uno, no como está al final — si no, dejaría de ser un crecimiento paso a paso.
  const ocupadas = new Set<string>();
  for (const c of celdasDeEdificio(centro)) ocupadas.add(claveCelda(c.col, c.row));

  anadirConectadas(anilloDeRectangulo(rectanguloDeEdificio(centro)), ocupadas, red, red.calles);

  const maxFila = largoMaxFila(asentamientoId);
  const desfase = Math.floor(pseudoAleatorio(hashTexto(`${asentamientoId}-desfase-manzana`)) * maxFila);
  // Período del retículo: la manzana MÁS su calle. Con aristas la calle no ocupaba nada y el período era el
  // ancho de manzana a secas; ahora la calle es una columna de celdas y hay que contarla (§E6.8).
  const pasoColumna = maxFila + TRAZADO.anchoCalle;
  const pasoFila = FONDO_MANZANA + TRAZADO.anchoCalle;

  for (const edificio of internos) {
    if (edificio.tipo === 'centroUrbano') continue;
    const rect = rectanguloDeEdificio(edificio);
    for (const c of celdasDeEdificio(edificio)) ocupadas.add(claveCelda(c.col, c.row));

    const deAfueras = esDeAfueras(edificio.tipo);
    const destino = deAfueras ? red.caminos : red.calles;

    if (!tieneFrenteDeCalle(rect, red)) {
      const cap = deAfueras ? TRAZADO.capCorredorAfueras : TRAZADO.capCorredorUrbano;
      const corredor = corredorHastaLaRed(rect, ocupadas, red, cap);
      if (corredor) for (const clave of corredor) destino.add(clave);
    }

    if (ANCLAS_REALES.has(edificio.tipo)) {
      anadirConectadas(anilloDeRectangulo(rect), ocupadas, red, red.calles);
    }

    // Un camino rural no forma manzanas (§11): las afueras solo se conectan.
    if (deAfueras) continue;

    // Retículo blando: las franjas de calle que le quedan pegadas a este edificio. `anchoCalle` celdas de
    // grosor hacia afuera en cada lado que caiga sobre una línea del retículo.
    const franja: Celda[] = [];
    for (let g = 0; g < TRAZADO.anchoCalle; g++) {
      if (esBordeDeManzana(rect.minCol - 1 - g, pasoColumna, desfase)) {
        for (let dr = 0; dr < rect.alto; dr++) franja.push({ col: rect.minCol - 1 - g, row: rect.minRow + dr });
      }
      if (esBordeDeManzana(rect.minCol + rect.ancho + g, pasoColumna, desfase)) {
        for (let dr = 0; dr < rect.alto; dr++) franja.push({ col: rect.minCol + rect.ancho + g, row: rect.minRow + dr });
      }
      if (esBordeDeManzana(rect.minRow - 1 - g, pasoFila, desfase)) {
        for (let dc = 0; dc < rect.ancho; dc++) franja.push({ col: rect.minCol + dc, row: rect.minRow - 1 - g });
      }
      if (esBordeDeManzana(rect.minRow + rect.alto + g, pasoFila, desfase)) {
        for (let dc = 0; dc < rect.ancho; dc++) franja.push({ col: rect.minCol + dc, row: rect.minRow + rect.alto + g });
      }
    }
    anadirConectadas(franja, ocupadas, red, red.calles);
  }
  return red;
}

/**
 * La red resuelta a RECTÁNGULOS en coordenadas locales, listos para dibujar — `ui/canvas.ts` nunca ve una
 * celda. Etapa 6: antes eran segmentos (líneas sin grosor), ahora son áreas, porque una calle ocupa suelo.
 *
 * Las celdas se fusionan en tiradas horizontales antes de salir: una avenida de 20 celdas viaja como UN
 * rectángulo y no como 20. Es el mismo criterio de presupuesto de payload del doc 6 que ya obligó a sacar el
 * mapa de las lecturas de estado.
 */
export function rectangulosDeRed(red: RedDeCalles): { calles: RectanguloLocal[]; caminos: RectanguloLocal[] } {
  const fusionar = (celdas: Iterable<string>, excluir?: Set<string>): RectanguloLocal[] => {
    const porFila = new Map<number, number[]>();
    for (const clave of celdas) {
      if (excluir?.has(clave)) continue;
      const coma = clave.indexOf(',');
      const col = Number(clave.slice(0, coma));
      const row = Number(clave.slice(coma + 1));
      const fila = porFila.get(row);
      if (fila) fila.push(col);
      else porFila.set(row, [col]);
    }
    const rects: RectanguloLocal[] = [];
    // Filas y columnas en orden ascendente: la salida es determinista, igual que la red de la que sale.
    for (const row of [...porFila.keys()].sort((a, b) => a - b)) {
      const cols = porFila.get(row)!.sort((a, b) => a - b);
      let inicio = cols[0]!;
      let previa = inicio;
      for (let i = 1; i <= cols.length; i++) {
        const col = cols[i];
        if (col !== undefined && col === previa + 1) {
          previa = col;
          continue;
        }
        rects.push({ x: inicio * T, y: row * T, ancho: (previa - inicio + 1) * T, alto: T });
        if (col === undefined) break;
        inicio = col;
        previa = col;
      }
    }
    return rects;
  };

  return {
    calles: fusionar(red.calles),
    // Una celda que ya es calle no se dibuja también como camino: la calle manda.
    caminos: fusionar(red.caminos, red.calles),
  };
}

export interface TrazadoAsentamiento {
  /** Tiradas de celdas de calle, en coordenadas locales. Etapa 6: son ÁREAS, no líneas. */
  calles: RectanguloLocal[];
  caminos: RectanguloLocal[];
  /** Rectángulo (coords locales) que ocupa cada edificio, por `id`. */
  huellas: Record<string, RectanguloLocal>;
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
  const { calles, caminos } = rectangulosDeRed(redDeCalles(asentamiento.id, asentamiento.edificios));
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
 * ¿Colocarse aquí CONTINÚA UNA FILA en vez de empezar una suelta? Es decir: ¿hay alguna celda de calle
 * delante que ya sirve de fachada a un edificio vecino?
 *
 * Traducción a celdas del criterio de la versión de aristas. Se recorren las celdas de calle que el candidato
 * tendría enfrente y, para cada una, se mira si alguna de SUS otras vecinas ortogonales está ocupada por un
 * edificio: eso es exactamente "otro edificio da a mi misma calle".
 *
 * No es cosmético. Sin preferir la continuación, la ciudad crece como un borrón compacto —cada edificio se
 * pega donde le queda más cerca del ancla— y las hileras nunca llegan al largo que dispara la transversal:
 * medido en su momento, solo 3 de 50 edificios cerraban manzana.
 */
function continuaFila(r: RectanguloCeldas, ocupadas: Set<string>, red: RedDeCalles): boolean {
  for (const c of adyacentesARectangulo(r)) {
    const clave = claveCelda(c.col, c.row);
    if (!red.calles.has(clave) && !red.caminos.has(clave)) continue;
    for (const [dc, dr] of VECINAS_ORTOGONALES) {
      const col = c.col + dc;
      const row = c.row + dr;
      // Las celdas del propio candidato no cuentan: la fila la hace un VECINO, no uno mismo.
      if (col >= r.minCol && col < r.minCol + r.ancho && row >= r.minRow && row < r.minRow + r.alto) continue;
      if (ocupadas.has(claveCelda(col, row))) return true;
    }
  }
  return false;
}

const ORIGEN_RECT: RectanguloCeldas = { minCol: 0, minRow: 0, ancho: 0, alto: 0 };
const ORIGEN: Point = { x: 0, y: 0 };

function distanciaAlOrigen(p: Point): number {
  return Math.hypot(p.x, p.y);
}

/**
 * Distancia del punto `centro` al PUNTO MÁS CERCANO del rectángulo de celdas `[col,row]+tamano`, en unidades
 * locales. 0 si `centro` cae dentro del rectángulo.
 *
 * Es la métrica correcta para un radio VEDADO: lo que no puede entrar en la ciudad es el edificio entero, no
 * su punto medio. Ver `TRAZADO.radioAfuerasMin` y el uso en `candidatosLibres`.
 */
function distanciaBordeAlCentro(col: number, row: number, tamano: TamanoEdificio, centro: Point): number {
  const x0 = col * T;
  const x1 = (col + tamano.ancho) * T;
  const y0 = row * T;
  const y1 = (row + tamano.alto) * T;
  const dx = Math.max(x0 - centro.x, 0, centro.x - x1);
  const dy = Math.max(y0 - centro.y, 0, centro.y - y1);
  return Math.hypot(dx, dy);
}

/**
 * Cuántas celdas OCUPADAS hay dentro de un rectángulo cualquiera, en O(1) — tabla de sumas acumuladas 2D
 * (*summed-area table*) construida una vez por barrido de candidatos.
 *
 * Por qué existe (Paso 1b de la Etapa 6, doc trazado §E6.15): `candidatosLibres` preguntaba "¿cabe aquí?"
 * recorriendo celda a celda la huella y consultando el `Set` con una clave de texto por celda. Eso es
 * `O(ancho×alto)` consultas —y otras tantas cadenas construidas— por CADA posición del barrido. El Paso 1
 * dobló la resolución de la rejilla, lo que multiplica por 4 las posiciones **y** por 4 las celdas de cada
 * huella: los dos factores se multiplican, y la corrida de referencia del laboratorio de balance pasó de
 * 1m40s a 8m08s (×4.9). Con la tabla, "¿cabe?" es una resta de cuatro enteros y el factor cuadrático
 * desaparece.
 *
 * Se construye recorriendo `ocupadas` (unos cientos de entradas en una ciudad real), no la caja entera
 * (decenas de miles de celdas): el coste de montarla es proporcional a lo que hay construido, no al área que
 * se barre.
 *
 * La caja cubre con margen TODAS las consultas que hace el barrido — la huella completa desde la posición más
 * lejana, más una celda de borde para la comprobación de "pared con pared" —, así que nunca hace falta
 * consultar fuera y ninguna consulta necesita recorte.
 */
class OcupacionAcumulada {
  private readonly acum: Int32Array;
  private readonly anchoTabla: number;
  private readonly minCol: number;
  private readonly minRow: number;

  constructor(ocupadas: Set<string>, minCol: number, minRow: number, maxCol: number, maxRow: number) {
    this.minCol = minCol;
    this.minRow = minRow;
    const ancho = maxCol - minCol + 1;
    const alto = maxRow - minRow + 1;
    this.anchoTabla = ancho + 1;
    this.acum = new Int32Array(this.anchoTabla * (alto + 1));

    for (const clave of ocupadas) {
      const coma = clave.indexOf(',');
      const col = Number(clave.slice(0, coma));
      if (col < minCol || col > maxCol) continue;
      const row = Number(clave.slice(coma + 1));
      if (row < minRow || row > maxRow) continue;
      this.acum[(row - minRow + 1) * this.anchoTabla + (col - minCol + 1)] = 1;
    }

    for (let r = 1; r <= alto; r++) {
      const fila = r * this.anchoTabla;
      const filaPrevia = (r - 1) * this.anchoTabla;
      for (let c = 1; c <= ancho; c++) {
        this.acum[fila + c] =
          this.acum[fila + c]! + this.acum[filaPrevia + c]! + this.acum[fila + c - 1]! - this.acum[filaPrevia + c - 1]!;
      }
    }
  }

  /** Celdas ocupadas en `[col, col+ancho) × [row, row+alto)`. 0 significa "el rectángulo está libre". */
  ocupadasEn(col: number, row: number, ancho: number, alto: number): number {
    const c0 = col - this.minCol;
    const r0 = row - this.minRow;
    const c1 = c0 + ancho;
    const r1 = r0 + alto;
    const w = this.anchoTabla;
    return this.acum[r1 * w + c1]! - this.acum[r0 * w + c1]! - this.acum[r1 * w + c0]! + this.acum[r0 * w + c0]!;
  }
}

/**
 * Campo de distancia a la calle (Etapa 6, doc trazado §E6.6): para cada celda LIBRE de la caja, a cuántos
 * pasos está de tocar la red. BFS multiorigen desde todas las celdas de calle a la vez, **una sola pasada por
 * barrido de candidatos**.
 *
 * Por qué así y no un BFS por candidato: `candidatosLibres` produce miles de posiciones por colocación, y un
 * BFS para cada una sería inviable. Invertido, el coste de conexión de un candidato es una consulta de tabla.
 *
 * Sustituye de golpe a tres cosas del modelo de aristas: la reparación posterior `conectarEdificio`, el
 * chequeo de conectividad, y el nivel 0-3 de `Candidato` en su parte de "¿tiene frente?". Y entrega gratis el
 * número que la decisión 4 (§E6.3) necesita: **cuántas celdas de suelo cuesta poner el edificio ahí**.
 *
 * `cap` acota la propagación: más allá de ese número de pasos la celda se considera inalcanzable. Es lo que
 * impide enterrarse dentro de un coágulo (§E6.10) y además mantiene el BFS barato.
 *
 * **Todo el trabajo va sobre MÁSCARAS INDEXADAS, no sobre claves de texto.** La primera versión consultaba
 * `ocupadas`/`red` con una clave `"col,row"` por celda de la caja y por vecina — unas 75.000 cadenas por
 * barrido — y fue la causa de que el Paso 2 duplicara el tiempo de la corrida de balance (medido con
 * `node --cpu-prof`: `tieneFrenteDeCalle` sola era el 16.5%). Las máscaras se rellenan recorriendo los
 * CONJUNTOS (cientos de entradas), no la caja (decenas de miles), igual que `OcupacionAcumulada`.
 */
class DistanciaALaCalle {
  private readonly dist: Int32Array;
  /** 0 = libre · 1 = ocupada (edificio o calle) · 2 = celda de la RED. Una celda de red es siempre ocupada. */
  private readonly mascara: Uint8Array;
  private readonly anchoTabla: number;
  private readonly altoTabla: number;
  private readonly minCol: number;
  private readonly minRow: number;

  constructor(
    ocupadas: Set<string>,
    red: RedDeCalles,
    minCol: number,
    minRow: number,
    maxCol: number,
    maxRow: number,
    cap: number
  ) {
    this.minCol = minCol;
    this.minRow = minRow;
    this.anchoTabla = maxCol - minCol + 1;
    this.altoTabla = maxRow - minRow + 1;
    const total = this.anchoTabla * this.altoTabla;
    this.mascara = new Uint8Array(total);
    this.dist = new Int32Array(total).fill(-1);

    const marcar = (claves: Iterable<string>, valor: number): void => {
      for (const clave of claves) {
        const coma = clave.indexOf(',');
        const col = Number(clave.slice(0, coma));
        if (col < minCol || col > maxCol) continue;
        const row = Number(clave.slice(coma + 1));
        if (row < minRow || row > maxRow) continue;
        this.mascara[(row - minRow) * this.anchoTabla + (col - minCol)] = valor;
      }
    };
    marcar(ocupadas, 1);
    marcar(red.calles, 2);
    marcar(red.caminos, 2);

    // Origen: las celdas LIBRES que tocan la red. Distancia 1 = "una celda de corredor y estoy en la calle";
    // una celda que YA es calle no es candidata a nada, así que no entra en el campo.
    const cola: number[] = [];
    for (let fila = 0; fila < this.altoTabla; fila++) {
      const base = fila * this.anchoTabla;
      for (let c = 0; c < this.anchoTabla; c++) {
        const i = base + c;
        if (this.mascara[i] !== 0) continue;
        const tocaRed =
          (fila > 0 && this.mascara[i - this.anchoTabla] === 2) ||
          (fila < this.altoTabla - 1 && this.mascara[i + this.anchoTabla] === 2) ||
          (c > 0 && this.mascara[i - 1] === 2) ||
          (c < this.anchoTabla - 1 && this.mascara[i + 1] === 2);
        if (!tocaRed) continue;
        this.dist[i] = 1;
        cola.push(i);
      }
    }

    for (let cabeza = 0; cabeza < cola.length; cabeza++) {
      const i = cola[cabeza]!;
      const d = this.dist[i]!;
      if (d >= cap) continue;
      const c = i % this.anchoTabla;
      const fila = (i - c) / this.anchoTabla;
      if (fila > 0) this.propagar(i - this.anchoTabla, d, cola);
      if (fila < this.altoTabla - 1) this.propagar(i + this.anchoTabla, d, cola);
      if (c > 0) this.propagar(i - 1, d, cola);
      if (c < this.anchoTabla - 1) this.propagar(i + 1, d, cola);
    }
  }

  private propagar(j: number, d: number, cola: number[]): void {
    if (this.dist[j] !== -1 || this.mascara[j] !== 0) return;
    this.dist[j] = d + 1;
    cola.push(j);
  }

  /**
   * Coste en celdas de corredor para conectar `r` a la red:
   * - **0** si ya tiene frente de calle (alguna celda adyacente ES de la red);
   * - **N** si hay que abrir N celdas libres para alcanzarla;
   * - **`null`** si es inalcanzable dentro del cap — y entonces esto NO es un sitio válido (§E6.5).
   *
   * Recorre las cuatro franjas adyacentes al rectángulo por aritmética de índices, sin construir ni un array
   * ni una cadena: es la consulta más caliente de todo el barrido.
   */
  costeDesde(r: RectanguloCeldas): number | null {
    let mejor = -1;
    const c0 = r.minCol - this.minCol;
    const f0 = r.minRow - this.minRow;

    const mirar = (c: number, fila: number): boolean => {
      if (c < 0 || c >= this.anchoTabla || fila < 0 || fila >= this.altoTabla) return false;
      const i = fila * this.anchoTabla + c;
      if (this.mascara[i] === 2) return true; // frente de calle real: coste 0, no hay nada mejor
      const d = this.dist[i]!;
      if (d !== -1 && (mejor === -1 || d < mejor)) mejor = d;
      return false;
    };

    for (let dc = 0; dc < r.ancho; dc++) {
      if (mirar(c0 + dc, f0 - 1)) return 0;
      if (mirar(c0 + dc, f0 + r.alto)) return 0;
    }
    for (let dr = 0; dr < r.alto; dr++) {
      if (mirar(c0 - 1, f0 + dr)) return 0;
      if (mirar(c0 + r.ancho, f0 + dr)) return 0;
    }
    return mejor === -1 ? null : mejor;
  }
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
  rotado = false,
  capCorredor = TRAZADO.capCorredorUrbano
): Candidato[] {
  const centro = centroDeRectangulo(referencia);
  const maxCeldas = Math.ceil(radioPotencial / T) + 1;
  const centroCol = Math.round(centro.x / T);
  const centroRow = Math.round(centro.y / T);
  const candidatos: Candidato[] = [];

  const colMin = centroCol - maxCeldas;
  const colMax = centroCol + maxCeldas;
  const rowMin = centroRow - maxCeldas;
  const rowMax = centroRow + maxCeldas;
  // La caja de la tabla desborda el barrido lo justo para cubrir toda consulta posible: la huella completa
  // desde la posición más lejana (`+ tamano`), más una celda de borde a cada lado para "pared con pared".
  const ocupacion = new OcupacionAcumulada(
    ocupadas,
    colMin - 1,
    rowMin - 1,
    colMax + tamano.ancho,
    rowMax + tamano.alto
  );
  // Campo de distancia a la calle (§E6.6): una sola pasada de BFS multiorigen para TODO el barrido, en vez de
  // un BFS por candidato. Convierte "¿puedo conectarme, y a qué coste?" en una consulta O(1).
  const distancias = new DistanciaALaCalle(ocupadas, red, colMin - 1, rowMin - 1, colMax + tamano.ancho, rowMax + tamano.alto, capCorredor);

  for (let col = colMin; col <= colMax; col++) {
    for (let row = rowMin; row <= rowMax; row++) {
      // Sin construir el `Point` todavía: la inmensa mayoría de posiciones del barrido se descartan aquí
      // mismo, y asignarles un objeto para tirarlo es la otra fuente de coste que multiplicó el Paso 1.
      const px = (col + tamano.ancho / 2) * T;
      const py = (row + tamano.alto / 2) * T;
      const distancia = Math.hypot(px - centro.x, py - centro.y);
      if (distancia > radioPotencial) continue;
      // El VETO se mide contra la huella, no contra el centro. Con el centro, una Granja de nivel 4 (12x12
      // celdas) cuyo centro cumpliera el radio metía medio edificio dentro de la ciudad: medido, su borde
      // interior llegaba a 39 con `radioAfuerasMin` = 60, ocupando suelo del casco urbano. Con el borde,
      // "las afueras empiezan en 60" significa que NINGUNA celda del edificio entra de 60 para dentro.
      if (distanciaMinima > 0 && distanciaBordeAlCentro(col, row, tamano, centro) < distanciaMinima) continue;

      if (ocupacion.ocupadasEn(col, row, tamano.ancho, tamano.alto) > 0) continue;

      const min: Celda = { col, row };
      const rect: RectanguloCeldas = { minCol: col, minRow: row, ancho: tamano.ancho, alto: tamano.alto };

      // GATE DURO de la Etapa 6 (§E6.5): un sitio del que no se pueda salir a la calle NO es un sitio. Con las
      // calles sobre aristas esto no podía fallar nunca y por eso se reparaba después de colocar; con celdas sí
      // falla, y repararlo después sería el mismo fallo en silencio que costó la Etapa 5.
      const costeCalle = distancias.costeDesde(rect);
      if (costeCalle === null) continue;

      const punto: Point = { x: px, y: py };
      let nivel: number;
      if (costeCalle === 0) {
        // Ya da a una calle. Nivel 0 si además CONTINÚA UNA FILA (hay un vecino dando a esa misma calle).
        nivel = continuaFila(rect, ocupadas, red) ? 0 : 1;
      } else {
        // Pared con pared: alguna celda inmediatamente adyacente al rectángulo está ocupada. Se pregunta por
        // las cuatro FRANJAS de una celda que lo rodean, cada una en O(1) — antes era una consulta por celda.
        const pegado =
          ocupacion.ocupadasEn(col, row - 1, tamano.ancho, 1) > 0 ||
          ocupacion.ocupadasEn(col, row + tamano.alto, tamano.ancho, 1) > 0 ||
          ocupacion.ocupadasEn(col - 1, row, 1, tamano.alto) > 0 ||
          ocupacion.ocupadasEn(col + tamano.ancho, row, 1, tamano.alto) > 0;
        nivel = pegado ? 2 : 3;
      }

      candidatos.push({ min, punto, nivel, rotado });
    }
  }
  return candidatos;
}

/** Tipos cuyo tamaño NUNCA debe intercambiarse (Etapa 4, punto 1): `granja` es una progresión real (los
 * cuatro tamaños de nivel son distintos edificios, no la misma pieza girada). `puestoMercado` SÍ puede girar
 * (2026-08-31, a petición del usuario): su `nivelInterno` identifica una FORMA relativa, no una orientación
 * absoluta —el propio Mercado que ancla la zona ya nace girado o no (`orientacionesDeAncla`), así que "forma 1"
 * nunca tuvo una orientación fija en el mapa—, y dejarlo girar es lo que permite que un puesto estrecho pegue
 * su lado LARGO al Mercado en vez de salir siempre con la misma cara. Ningún otro tipo tiene tamaño especial
 * (ver `tamanoEdificio`), así que el resto puede ofrecerse girado sin caso especial. */
const TIPOS_SIN_ROTACION = new Set<EdificioTipo>(['granja']);

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

/**
 * Ordena por distancia al origen, con `semillaCandidato` de desempate final.
 *
 * Decorar-ordenar-desdecorar y no calcular dentro del comparador: un comparador se ejecuta `O(n log n)` veces,
 * y tanto `distanciaAlOrigen` (un `Math.hypot`) como `semillaCandidato` (construir una plantilla de texto,
 * pasarla por FNV-1a y por un `Math.sin`) son CONSTANTES por candidato. Calcularlas dentro costaba el 28% del
 * tiempo de la simulación de balance, medido con `node --cpu-prof` (Paso 1b, doc trazado §E6.15) — repartido
 * entre `pseudoAleatorio` 11.9%, `distanciaAlOrigen`+`porDistanciaAlOrigen` 12.5% y `hashTexto` 3.1%.
 */
function porDistanciaAlOrigen(candidatos: Candidato[], masLejos: boolean): Candidato[] {
  const decorados = candidatos.map((c) => ({ c, distancia: distanciaAlOrigen(c.punto), semilla: semillaCandidato(c) }));
  decorados.sort((a, b) => {
    const porDistancia = masLejos ? b.distancia - a.distancia : a.distancia - b.distancia;
    return porDistancia || a.semilla - b.semilla;
  });
  return decorados.map((d) => d.c);
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
  ampliado = false,
  /** Celdas ocupadas por edificios AFINES al que se coloca (`celdasDeTiposAfines`) — desempate secundario
   * tras la adyacencia al ancla: entre dos huecos igual de pegados al ancla gana el que más lado comparte con
   * los suyos (viviendas con viviendas, industria junta…). Vacío = sin preferencia de agrupación. */
  celdasAfines: Set<string> = new Set()
): { punto: Point; rotado: boolean }[] {
  const rectAncla = rectanguloDeEdificio(ancla);
  // §E6.7 — LA TRAMPA de la Etapa 6, y la razón de que este bloque no se pudiera dejar para el Paso 3.
  //
  // Toda ancla siembra su anillo de calle (§5.2). Con las calles sobre CELDAS ese anillo OCUPA las celdas que
  // rodean al ancla, así que `gapCeldas(satelite, ancla) === 0` pasa a ser geométricamente imposible: nada
  // puede tocar el ancla, porque en medio está su calle. Medir contra el ancla desnuda hacía que ningún anillo
  // ofreciera nunca un hueco y la colocación cayera al fallback — exactamente el bug que la Etapa 2 corrigió en
  // su día ("38/38 piezas de Mercado con gap 0; antes algunas quedaban a 2 filas").
  //
  // La formulación que lo arregla sin tocar nada más: medir contra el ancla EXPANDIDA por su anillo. Así
  // "hueco 0" recupera su significado —el satélite mira a su ancla desde el otro lado de la calle— y tanto el
  // bucle de anillos como el desempate por borde compartido siguen valiendo tal cual. Que además es lo
  // correcto en 3D: un satélite pegado sin calle en medio no tendría puerta.
  const anillo = TRAZADO.anchoCalle;
  const rectAnclaConAnillo: RectanguloCeldas = {
    minCol: rectAncla.minCol - anillo,
    minRow: rectAncla.minRow - anillo,
    ancho: rectAncla.ancho + anillo * 2,
    alto: rectAncla.alto + anillo * 2,
  };
  const radioMaximoNucleoCeldas = TRAZADO.separacionMinimaAnclas / 2;

  // Escaneo generoso: cubre el tope de verdad más el propio tamaño del ancla y del satélite, para que ningún
  // candidato válido quede fuera por culpa del radio de escaneo — el tope real lo decide el hueco, más abajo.
  const margenCeldas = radioMaximoNucleoCeldas + Math.max(rectAncla.ancho, rectAncla.alto) + Math.max(tamano.ancho, tamano.alto);
  const candidatosConHueco = candidatosConOrientaciones(rectAncla, margenCeldas * T, tamano, ocupadas, red, 0, permitirRotacion).map((c) => {
    const rectCandidato: RectanguloCeldas = { minCol: c.min.col, minRow: c.min.row, ...tamanoDeCandidato(tamano, c) };
    return { ...c, rectCandidato, hueco: gapCeldas(rectCandidato, rectAnclaConAnillo) };
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

    // Decorar-ordenar-desdecorar, mismo motivo que en `porDistanciaAlOrigen`: `bordeCompartido`,
    // `bordeAfin` y `semillaCandidato` son constantes por candidato y el comparador se ejecuta `O(n log n)`
    // veces.
    //
    // Prioridad del desempate (a petición del usuario, 2026-08-31): 1º hueco al ancla, 2º lado compartido con
    // el ANCLA (regla original), 3º lado compartido con los AFINES —edificios del mismo tipo o categoría— para
    // que la ciudad forme barrios homogéneos (viviendas juntas, industria junta…), 4º semilla determinista.
    return enEsteAnillo
      .filter((c) => c.nivel === mejorNivel)
      .map((c) => ({
        c,
        borde: bordeCompartido(c.rectCandidato, rectAnclaConAnillo),
        bordeAfin: bordeAfinDe(c.rectCandidato, celdasAfines),
        semilla: semillaCandidato(c),
      }))
      .sort((a, b) => a.c.hueco - b.c.hueco || b.borde - a.borde || b.bordeAfin - a.bordeAfin || a.semilla - b.semilla)
      .map((d) => ({ punto: d.c.punto, rotado: d.c.rotado }));
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
  const ocupadas = conCeldasDeRed(celdasOcupadas(edificios), red);
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
  const { ocupadas, red } = sueloOcupado(asentamiento.id, ocupados);
  const aPunto = (candidatos: Candidato[]): { punto: Point; rotado: boolean }[] =>
    candidatos.map((c) => ({ punto: c.punto, rotado: c.rotado }));

  if (esDeAfueras(tipo)) {
    // CAP DE CORREDOR POR CLASE (§E6.10): Granja y Corral viven a `radioAfuerasMin` por diseño y su camino
    // es largo A PROPÓSITO. Con el cap urbano no pasaba NINGÚN candidato y la Granja caía al fallback `(0,0)`,
    // encima del Centro Urbano — el propio §E6.10 avisaba de que "un cap único los rechazaría a todos".
    const candidatos = candidatosLibres(
      ORIGEN_RECT,
      radioMaximoAfueras(asentamiento.radioPotencial, tamano),
      tamano,
      ocupadas,
      red,
      TRAZADO.radioAfuerasMin,
      false,
      TRAZADO.capCorredorAfueras
    );
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
  // Desempate por agrupación (2026-08-31): entre huecos igual de pegados al ancla, el que más lado comparte
  // con edificios afines ya construidos. `ampliado` (Líneas de Producción) lo ignora — esa política reordena
  // los candidatos por distancia a sus insumos, no por vecindad.
  const celdasAfines = ampliado ? new Set<string>() : celdasDeTiposAfines(ocupados, tipo, anclaInstancia.id);
  return sitiosPorAtraccionDura(anclaInstancia, tamano, ocupadas, red, permitirRotacion, ampliado, celdasAfines);
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

/** Radio (en celdas, centro a centro) al que se prueba la primera ranura a lo largo de una dirección — arranca
 * ya más allá de `separacionMinimaAnclas` para no perder intentos en radios que van a fallar la separación
 * dura, y crece de `FONDO_MANZANA` en `FONDO_MANZANA` hasta `radioMaximoRanura()`.
 *
 * Funciones y no `const`: el laboratorio ajusta `TRAZADO.separacionMinimaAnclas` en caliente, y un `const`
 * capturado al cargar el módulo dejaría la búsqueda de ranura anclada al valor viejo (bug ya visto: cambiar
 * la separación movía `radioMaximoNucleo` pero no el rango de ranura). */
function radioInicialRanura(): number {
  return TRAZADO.separacionMinimaAnclas;
}
/** Ver `radioInicialRanura`. Exportada porque un test que quiera saturar una semilla tiene que rellenar MÁS
 * que este radio. */
export function radioMaximoRanura(): number {
  return TRAZADO.separacionMinimaAnclas * 3;
}

/**
 * ¿Hay un hueco real para `tamano` a lo largo de `direccion` desde `origen` (centro de la semilla)? Prueba
 * radios crecientes y devuelve el primer rectángulo que no colisiona con nada ya ocupado y respeta
 * `separacionSeguridadAnclas` frente a TODAS las demás anclas — o `null` si ninguno hasta el tope sirve (Etapa
 * 5: esta ranura de la semilla queda descartada, `crearAnclaNueva` prueba la siguiente dirección).
 *
 * Ojo: esto NO comprueba que un satélite pueda pegarse al ancla ni que el ancla alcance la red — de eso se
 * encarga `asegurarAnclaPara` (construction.ts), que descarta el ancla recién creada si `sitiosPorAtraccionDura`
 * no le encuentra sitio. Ver doc trazado §E6.19.
 */
export function huecoEnDireccion(
  origen: Point,
  direccion: Point,
  tamano: TamanoEdificio,
  ocupadas: Set<string>,
  otrasAnclas: RectanguloCeldas[]
): RectanguloCeldas | null {
  const radioMax = radioMaximoRanura();
  for (let radioCeldas = radioInicialRanura(); radioCeldas <= radioMax; radioCeldas += FONDO_MANZANA) {
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
    if (distanciaCeldas < radioInicialRanura() - MARGEN_RADIO_CELDAS || distanciaCeldas > radioMaximoRanura() + MARGEN_RADIO_CELDAS) return false;
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
  const { ocupadas } = sueloOcupado(asentamientoId, edificios);
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
 * Reubica un edificio cuya huella acaba de crecer (hoy solo Granja, §7 del doc), sin exigir que quepa en su
 * sitio actual.
 *
 * **CRECE HACIA AFUERA, NUNCA HACIA EL CENTRO** (a petición del usuario, 2026-08-31). Entre los huecos
 * válidos se descartan primero los que dejarían el edificio MÁS CERCA del centro de lo que ya estaba, y solo
 * entre los que respetan eso se elige el más cercano a donde estaba — para que la Granja siga junto a sus
 * campos en vez de saltar al otro extremo.
 *
 * El criterio anterior era "el hueco más cercano a donde estaba", sin dirección, y medido: la Granja marchaba
 * hacia dentro a cada mejora (borde interior 88.2 → 65.8 → 57.9 → 39.0 en tres subidas de nivel), comiéndose
 * el suelo del casco urbano. Es además lo que ya pedía §11 para las afueras — "se prefiere siempre el hueco
 * más lejano, así que acompañan al borde de la ciudad a medida que crece"—, que la reubicación no cumplía.
 *
 * LA MEJORA MANDA SOBRE LA DIRECCIÓN: si NINGÚN hueco evita acercarse, se muda igual al más cercano — mejor
 * una Granja algo más adentro que una mejora bloqueada. Devuelve `null` solo cuando no existe hueco alguno
 * para el tamaño nuevo, que es cuando la mejora no puede aplicarse sin romper "ningún edificio encima de
 * otro", lo único que no se negocia.
 */
export function reubicarPorTamano(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  edificio: Edificio,
  todos: Edificio[],
  nivelInternoNuevo: number
): Point | null {
  const tamano = tamanoEdificio(edificio.tipo, nivelInternoNuevo);
  const { ocupadas, red } = sueloOcupado(asentamiento.id, todos, edificio.id);
  const afueras = esDeAfueras(edificio.tipo);
  const distanciaMinima = afueras ? TRAZADO.radioAfuerasMin : 0;
  const radioMaximo = afueras ? radioMaximoAfueras(asentamiento.radioPotencial, tamano) : asentamiento.radioPotencial;
  const capCorredor = afueras ? TRAZADO.capCorredorAfueras : TRAZADO.capCorredorUrbano;
  const candidatos = candidatosLibres(ORIGEN_RECT, radioMaximo, tamano, ocupadas, red, distanciaMinima, false, capCorredor);
  if (candidatos.length === 0) return null;

  // Radio del BORDE INTERIOR actual: lo que hay que no empeorar. Se mide contra la huella y no contra el
  // centro por la misma razón que el veto de afueras — al crecer, el rectángulo se expande hacia los dos
  // lados desde su centro, así que un centro que no se mueve YA mete el edificio más adentro.
  const rectActual = rectanguloDeEdificio(edificio);
  const bordeActual = distanciaBordeAlCentro(rectActual.minCol, rectActual.minRow, { ancho: rectActual.ancho, alto: rectActual.alto }, ORIGEN);
  const tamanoDe = (c: Candidato): TamanoEdificio => (c.rotado ? { ancho: tamano.alto, alto: tamano.ancho } : tamano);

  let mejor: Candidato | null = null;
  let mejorDistancia = Infinity;
  let respaldo = candidatos[0]!;
  let respaldoDistancia = Infinity;
  for (const c of candidatos) {
    const d = Math.hypot(c.punto.x - edificio.posicion.x, c.punto.y - edificio.posicion.y);
    if (d < respaldoDistancia) {
      respaldo = c;
      respaldoDistancia = d;
    }
    if (distanciaBordeAlCentro(c.min.col, c.min.row, tamanoDe(c), ORIGEN) < bordeActual) continue;
    if (d < mejorDistancia) {
      mejor = c;
      mejorDistancia = d;
    }
  }
  return (mejor ?? respaldo).punto;
}
