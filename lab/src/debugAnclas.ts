// Overlay de depuración del Laboratorio (fuera del ámbito del juego general) — NO se importa desde
// `main.ts`/`ui/canvas.ts` del juego real. Inspecciona y dibuja encima del trazado real (`drawAsentamiento`,
// ui/canvas.ts) el estado del árbol único de anclas (Etapa 5, Lógica 1: `engine/trazado.ts`), para poder ver
// a simple vista si una semilla reparte sus 5 ranuras antes de pasar a una hija y si toda ancla de árbol
// tiene algún satélite alcanzable.
import type { Edificio, EdificioTipo, Point } from '../../src/domain/types';
import { REJILLA_ASENTAMIENTO, TRAZADO } from '../../src/constants';
import {
  ANCLAS_REALES,
  ANCLA_PRIMARIA_POR_CATEGORIA,
  ANCLA_SATURACION_POR_CATEGORIA,
  CATEGORIA_POR_TIPO,
  celdaMinimaDeEdificio,
  sueloOcupado,
  direccionesRotadas,
  edificiosInternos,
  huecoEnDireccion,
  puntoDeRectangulo,
  semillaActiva,
  tamanoDeEdificio,
  type CategoriaAsentamiento,
  type RectanguloCeldas,
} from '../../src/engine/trazado';

/** Categoría de la que `tipo` es ancla (primaria o de saturación) — inversa de `ANCLA_PRIMARIA_POR_CATEGORIA`/
 * `ANCLA_SATURACION_POR_CATEGORIA`, derivada de ellas para no duplicar la relación a mano. Centro
 * Urbano/Mercado/Carpintería aparecen aquí como ancla PRIMARIA de su categoría; Plaza/Pozo/Parque/Plaza de
 * Armas/Patio de Gremios como anclas de saturación. */
const CATEGORIA_DE_ANCLA: Partial<Record<EdificioTipo, CategoriaAsentamiento>> = {};
for (const [categoria, tipo] of Object.entries(ANCLA_PRIMARIA_POR_CATEGORIA) as [CategoriaAsentamiento, EdificioTipo][]) {
  CATEGORIA_DE_ANCLA[tipo] = categoria;
}
for (const [categoria, tipos] of Object.entries(ANCLA_SATURACION_POR_CATEGORIA) as [CategoriaAsentamiento, EdificioTipo[]][]) {
  for (const tipo of tipos) CATEGORIA_DE_ANCLA[tipo] = categoria;
}

/** Letras para los hijos DIRECTOS de la raíz (Centro Urbano) — a petición del usuario. A partir de la
 * segunda generación el código sigue con un dígito 1-5 en vez de otra letra (ver `construirArbol`). */
const LETRAS_RANURA = ['a', 'b', 'c', 'd', 'e'];

interface NodoArbol {
  padreId: string | null;
  nivel: number;
  codigo: string;
  /** Distancia (en celdas, centro a centro) a su padre directo — "raíz" en la terminología del usuario: la
   * semilla de la que nació esta ancla, no la raíz última del árbol. `null` para Centro Urbano (sin padre). */
  distanciaPadreCeldas: number | null;
}

function centroDeRect(r: RectanguloCeldas): Point {
  return puntoDeRectangulo({ col: r.minCol, row: r.minRow }, { ancho: r.ancho, alto: r.alto });
}

function distanciaAlOrigenLocal(p: Point): number {
  return Math.hypot(p.x, p.y);
}

/**
 * Reconstruye, para cada ancla de árbol (`ANCLAS_REALES`) del asentamiento, su padre real, profundidad y
 * código de posición (Centro Urbano = "R"; hijos directos de la raíz = "a".."e" según la ranura; a partir de
 * ahí se apila un dígito 1-5 por generación: "a1", "a11"...).
 *
 * Un primer intento de esto comparaba solo el ÁNGULO entre dos anclas contra las 5 ranuras rotadas — y
 * resultó ser un espejismo: como todas las anclas del asentamiento comparten el MISMO eje de 5 ranuras,
 * dos anclas SIN relación real (p. ej. dos hermanas de la misma semilla, o dos anclas de una cadena en línea
 * recta del bug de "ranura reutilizada") pueden quedar alineadas por pura coincidencia geométrica y el
 * heurístico les inventaba un parentesco falso. La única forma de estar seguro es no adivinar: se REPRODUCE
 * la búsqueda real de `crearAnclaNueva` para cada ancla, con las piezas EXPORTADAS del motor
 * (`huecoEnDireccion`, la misma función que usa el juego) sobre la ocupación reconstruida hasta ese punto del
 * historial (`edificios` nunca se reordena — orden de creación) — si el resultado coincide EXACTO con la
 * posición real del ancla, ese es su padre y su ranura; si no coincide con ninguna, no se inventa nada.
 */
function construirArbol(edificios: Edificio[], asentamientoId: string): Map<string, NodoArbol> {
  const internos = edificiosInternos(edificios);
  const direcciones = direccionesRotadas(asentamientoId);
  const nodos = new Map<string, NodoArbol>();
  const anclasVistas: Edificio[] = [];
  const excluidas = new Set<string>();
  // Coincidencia de posición: `huecoEnDireccion` es determinista, así que para el padre/ranura reales el
  // resultado debería caer prácticamente exacto (diferencia de punto flotante) — un candidato equivocado
  // difiere por lo menos una celda entera, muy por encima de este margen.
  const EPS_LOCAL = 0.01;

  for (let i = 0; i < internos.length; i++) {
    const e = internos[i]!;
    if (!ANCLAS_REALES.has(e.tipo)) continue;

    if (e.tipo === 'centroUrbano') {
      nodos.set(e.id, { padreId: null, nivel: 0, codigo: 'R', distanciaPadreCeldas: null });
      anclasVistas.push(e);
      continue;
    }

    // MISMO suelo que vio el motor al crear esta ancla: edificios Y celdas de calle. Con solo los edificios
    // (como hacía antes de la Etapa 6) `huecoEnDireccion` devuelve otra posición y el ancla queda sin padre.
    const ocupadasHastaAqui = sueloOcupado(asentamientoId, internos.slice(0, i)).ocupadas;
    const tamanoE = tamanoDeEdificio(e);
    const candidatos = [...anclasVistas]
      .filter((a) => !excluidas.has(a.id))
      .sort((a, b) => distanciaAlOrigenLocal(a.posicion) - distanciaAlOrigenLocal(b.posicion));

    let asignado: { padreId: string; bucket: number; distanciaCeldas: number } | null = null;
    for (const candidato of candidatos) {
      const otrasAnclas = anclasVistas.filter((a) => a.id !== candidato.id).map(rectDe);
      const centroCandidato = centroDeRect(rectDe(candidato));
      let algunHueco = false;
      for (let b = 0; b < direcciones.length; b++) {
        const rect = huecoEnDireccion(centroCandidato, direcciones[b]!.vector, tamanoE, ocupadasHastaAqui, otrasAnclas);
        if (!rect) continue;
        algunHueco = true;
        const centro = centroDeRect(rect);
        if (Math.hypot(centro.x - e.posicion.x, centro.y - e.posicion.y) < EPS_LOCAL) {
          const distanciaCeldas = Math.hypot(e.posicion.x - centroCandidato.x, e.posicion.y - centroCandidato.y) / REJILLA_ASENTAMIENTO.tamanoCelda;
          asignado = { padreId: candidato.id, bucket: b, distanciaCeldas };
          break;
        }
      }
      if (asignado) break;
      // Las 5 ranuras de `candidato` fallan TODAS en este punto del historial: coherente con que el motor
      // lo hubiera descartado aquí — pasamos al siguiente candidato, más lejano. Si en cambio sí tenía hueco
      // pero en ninguna dirección coincide con `e`, no se descarta (de verdad tenía sitio): seguimos probando
      // el resto de candidatos por si la coincidencia real está más lejos en la lista.
      if (!algunHueco) excluidas.add(candidato.id);
    }

    if (asignado) {
      const nodoPadre = nodos.get(asignado.padreId);
      const nivelPadre = nodoPadre?.nivel ?? 0;
      const codigoPadre = nodoPadre?.codigo ?? 'R';
      const codigo = nivelPadre === 0 ? LETRAS_RANURA[asignado.bucket]! : codigoPadre + String(asignado.bucket + 1);
      nodos.set(e.id, { padreId: asignado.padreId, nivel: nivelPadre + 1, codigo, distanciaPadreCeldas: asignado.distanciaCeldas });
    } else {
      // No debería pasar para un ancla nacida de `crearAnclaNueva` con la reconstrucción de ocupación
      // completa — señal real de discrepancia si aparece, no solo un caso defensivo de adorno.
      nodos.set(e.id, { padreId: null, nivel: 1, codigo: '?', distanciaPadreCeldas: null });
    }
    anclasVistas.push(e);
  }
  return nodos;
}

/** ¿Tiene `ancla` algún satélite YA CONSTRUIDO de su categoría — mismo radio que `sitiosPorAtraccionDura`
 * (`separacionMinimaAnclas / 2`)? Distinto de `anclaLlena` (que mira si le queda sitio para MÁS, no si
 * alguna vez llegó a tener uno) — la señal directa de "ancla huérfana". */
function tieneAlgunSatelite(ancla: Edificio, internos: Edificio[]): boolean {
  const categoria = CATEGORIA_DE_ANCLA[ancla.tipo];
  if (!categoria) return true;
  const radio = TRAZADO.separacionMinimaAnclas / 2;
  const rectAncla = rectDe(ancla);
  return internos.some((e) => e.id !== ancla.id && !ANCLAS_REALES.has(e.tipo) && categoriaDeSatelite(e.tipo) === categoria && gapCeldasLocal(rectDe(e), rectAncla) <= radio);
}

// --- Geometría mínima local (no exportada por trazado.ts, se reconstruye desde piezas ya públicas) ---

function rectDe(e: Pick<Edificio, 'tipo' | 'nivelInterno' | 'posicion' | 'rotado'>): RectanguloCeldas {
  const min = celdaMinimaDeEdificio(e);
  const tamano = tamanoDeEdificio(e);
  return { minCol: min.col, minRow: min.row, ancho: tamano.ancho, alto: tamano.alto };
}

function gapCeldasLocal(a: RectanguloCeldas, b: RectanguloCeldas): number {
  const gapCols = Math.max(0, a.minCol - (b.minCol + b.ancho), b.minCol - (a.minCol + a.ancho));
  const gapRows = Math.max(0, a.minRow - (b.minRow + b.alto), b.minRow - (a.minRow + a.alto));
  return Math.max(gapCols, gapRows);
}

function categoriaDeSatelite(tipo: EdificioTipo): CategoriaAsentamiento | undefined {
  return CATEGORIA_POR_TIPO[tipo];
}

/** Ticks de gracia antes de marcar una ancla como huérfana — evita que una recién nacida parpadee en rojo
 * mientras su satélite todavía está en cola/construcción. */
const GRACIA_TICKS_HUERFANA = 20;

export interface FilaAncla {
  id: string;
  tipo: EdificioTipo;
  posicion: Point;
  padreId: string | null;
  codigo: string;
  nivel: number;
  /** Distancia en celdas a su padre directo ("raíz" en la terminología del usuario) — `null` para Centro
   * Urbano. Sirve para ver de un vistazo el bug de "ranura reutilizada": si dos hijas del mismo padre y
   * misma ranura (mismo prefijo de código antes del último dígito) tienen distancias muy distintas, es la
   * cadena expandiéndose en línea recta en vez de repartirse entre las 5 ranuras. */
  distanciaPadreCeldas: number | null;
  esSemillaActiva: boolean;
  semillaSaturada: boolean;
  anclaLlena: boolean;
  huerfana: boolean;
  edad: number;
}

/**
 * Filas para el panel lateral del laboratorio: una por cada ancla de árbol presente en el asentamiento
 * (incluye Centro Urbano/Mercado/Carpintería — también nacen y crecen por el árbol único, ver
 * `construirArbol`). `nacimientos` es el registro que lleva `main.ts` de en qué tick apareció cada id — las
 * anclas nacen ya `activo` (no pasan por cola), así que no llevan tick de nacimiento propio en el dominio.
 */
export function inspeccionarAnclas(edificios: Edificio[], asentamientoId: string, tick: number, nacimientos: Map<string, number>): FilaAncla[] {
  const internos = edificiosInternos(edificios);
  const activa = semillaActiva(edificios, new Set());
  const arbol = construirArbol(edificios, asentamientoId);
  return internos
    .filter((e) => ANCLAS_REALES.has(e.tipo))
    .map((e) => {
      const tickNacimiento = nacimientos.get(e.id) ?? tick;
      const edad = tick - tickNacimiento;
      const nodo = arbol.get(e.id) ?? { padreId: null, nivel: 0, codigo: '?', distanciaPadreCeldas: null };
      return {
        id: e.id,
        tipo: e.tipo,
        posicion: e.posicion,
        padreId: nodo.padreId,
        codigo: nodo.codigo,
        nivel: nodo.nivel,
        distanciaPadreCeldas: nodo.distanciaPadreCeldas,
        esSemillaActiva: activa?.id === e.id,
        semillaSaturada: e.semillaSaturada ?? false,
        anclaLlena: e.anclaLlena ?? false,
        huerfana: edad >= GRACIA_TICKS_HUERFANA && !tieneAlgunSatelite(e, internos),
        edad,
      };
    });
}

/**
 * Dibuja, ENCIMA del trazado ya pintado por `drawAsentamiento` (ui/canvas.ts): las 5 ranuras rotadas de cada
 * ancla de árbol (mismas para todo el asentamiento — la rotación es por asentamiento, no por ancla, ver
 * `direccionesRotadas`), para ver a simple vista si dos hijas de una misma semilla cayeron en el mismo rayo
 * (bug de "crecimiento en línea recta"); un anillo dorado sobre la semilla activa; un borde rojo punteado
 * sobre las semillas ya saturadas (Lógica 1); y un "!" rojo sobre cualquier ancla huérfana.
 *
 * `aPantalla`/`escala` deben ser EXACTAMENTE los mismos que usó `drawAsentamiento` para este frame (misma
 * fórmula, `ui/canvas.ts:750-757`) — si no, el overlay queda desalineado del trazado.
 */
export function dibujarOverlayAnclas(
  ctx: CanvasRenderingContext2D,
  filas: FilaAncla[],
  asentamientoId: string,
  aPantalla: (p: Point) => Point,
  escala: number,
  hoveredId: string | null = null
): void {
  const rayos = direccionesRotadas(asentamientoId);
  // Mismo tope que `RADIO_MAXIMO_RANURA` en trazado.ts — solo para que el rayo se vea, no afecta a nada real.
  const largoLocal = TRAZADO.separacionMinimaAnclas * 3 * REJILLA_ASENTAMIENTO.tamanoCelda;

  for (const fila of filas) {
    const centro = aPantalla(fila.posicion);

    ctx.strokeStyle = 'rgba(120, 200, 255, 0.22)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const rayo of rayos) {
      const haciaLocal: Point = {
        x: fila.posicion.x + rayo.vector.x * largoLocal,
        y: fila.posicion.y + rayo.vector.y * largoLocal,
      };
      const hasta = aPantalla(haciaLocal);
      ctx.moveTo(centro.x, centro.y);
      ctx.lineTo(hasta.x, hasta.y);
    }
    ctx.stroke();

    const radioAnillo = Math.max(10, 6 * escala);
    if (fila.esSemillaActiva) {
      ctx.strokeStyle = '#ffd23f';
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      ctx.arc(centro.x, centro.y, radioAnillo, 0, Math.PI * 2);
      ctx.stroke();
    } else if (fila.semillaSaturada) {
      ctx.strokeStyle = 'rgba(220, 70, 70, 0.6)';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([2, 2]);
      ctx.beginPath();
      ctx.arc(centro.x, centro.y, radioAnillo, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    ctx.fillStyle = 'rgba(232, 226, 208, 0.85)';
    ctx.font = '10px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.fillText(fila.codigo, centro.x, centro.y - radioAnillo - 2);

    if (fila.huerfana) {
      ctx.fillStyle = '#ff4d4d';
      ctx.font = 'bold 14px system-ui, sans-serif';
      ctx.fillText('!', centro.x, centro.y - radioAnillo - 14);
    }

    if (fila.id === hoveredId) {
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      ctx.arc(centro.x, centro.y, radioAnillo + 5, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
}

/** Ancla bajo el cursor en el canvas del mapa (coords YA en pantalla, mismas que `aPantalla` produce) — para
 * el resaltado cruzado mapa↔árbol del laboratorio. `radioPx` es el margen de tolerancia del hit-test. */
export function anclaEnPosicion(filas: FilaAncla[], aPantalla: (p: Point) => Point, x: number, y: number, radioPx = 14): string | null {
  let mejor: { id: string; distancia: number } | null = null;
  for (const fila of filas) {
    const centro = aPantalla(fila.posicion);
    const distancia = Math.hypot(centro.x - x, centro.y - y);
    if (distancia <= radioPx && (!mejor || distancia < mejor.distancia)) mejor = { id: fila.id, distancia };
  }
  return mejor?.id ?? null;
}
