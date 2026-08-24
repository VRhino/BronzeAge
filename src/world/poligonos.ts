// Fusión de siluetas: dadas N formas que se solapan, devuelve el CONTORNO del conjunto (su unión).
// Geometría pura, igual que `geometria.ts`: no sabe qué es una facción, un bosque ni un asentamiento.
//
// POR QUÉ EXISTE (a petición del usuario, dos problemas que resultaron ser el mismo):
//  1. Las zonas de influencia de una MISMA Facción no se recortan entre sí (ver `computeZonaInfluencia`:
//     el recorte solo aplica contra facciones rivales), así que se dibujaban como discos independientes
//     apilados — se veían las fronteras internas entre asentamientos hermanos, que no existen, y el relleno
//     translúcido se acumulaba más oscuro en los solapes.
//  2. Los bosques son 170 discos de radio 45-120 sobre un mapa de 2000 (ver `BOSQUE`), o sea que se solapan
//     muchísimo: el mapa mostraba un amasijo de círculos con anillos oscuros en cada intersección en vez de
//     manchas de bosque.
// En los dos casos lo que se quiere pintar es una sola silueta con un solo borde.
//
// MÉTODO: campo de distancia con signo + marching squares, NO recorte booleano exacto (Martínez,
// Greiner-Hormann, clipper). Decisión deliberada:
//  - El resultado es solo para DIBUJAR (nadie hace `pointInPolygon` contra la unión; las reglas de juego
//    siguen usando el polígono por asentamiento, ver `engine/zones.ts`), así que un contorno con error
//    sub-píxel es indistinguible del exacto.
//  - Un clipper booleano robusto son cientos de líneas y su parte difícil son justo los casos degenerados
//    que aquí abundan: 170 discos con tangencias, contenciones totales y vértices coincidentes. Este método
//    no tiene casos degenerados — muestrea un campo escalar y le saca las isolíneas.
//  - Sale gratis lo que un clipper cuesta aparte: los agujeros (un anillo de bosques que encierra un claro)
//    aparecen como lazos con orientación opuesta, que es exactamente lo que el relleno `nonzero` del canvas
//    necesita para recortarlos.
// El precio es la resolución: el contorno sigue la rejilla de muestreo. Se controla con `paso` (ver
// `OpcionesUnion`), y como el campo es continuo (distancia, no ocupación 0/1) la interpolación lineal en cada
// celda deja el borde liso, no escalonado.

import type { Point } from '../domain/types';
import { boundingBox, distancia, distanciaASegmento } from './geometria';

export interface Caja {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/**
 * Una forma vista como campo de distancia. `caja` acota dónde puede influir (fuera de ella la unión ni
 * la consulta) y `distanciaConSigno` responde POSITIVO dentro, NEGATIVO fuera, 0 en el borde.
 *
 * Es una interfaz y no "un polígono" porque las dos fuentes reales tienen coste muy distinto: un círculo
 * responde con una resta (`radio - distancia`) y un polígono necesita recorrer sus aristas. Los bosques son
 * círculos de verdad, y aproximarlos a polígonos solo para poder unirlos costaría ~24x más consultas y encima
 * encogería el radio (un polígono inscrito queda por dentro del círculo).
 */
export interface FormaDistancia {
  readonly caja: Caja;
  /**
   * `banda` es la única franja alrededor del borde donde el valor se usa de verdad: fuera de ella el llamador
   * satura a ±banda de todos modos (ver `unirFormas`). Una forma puede aprovecharlo para salir con una cota
   * barata en vez de calcular la distancia exacta, siempre que el valor que devuelva conserve el signo y esté
   * igual de fuera del rango `(-banda, banda)` que el exacto. Ignorarlo y devolver siempre la distancia real
   * también es válido.
   */
  distanciaConSigno(p: Point, banda: number): number;
}

export interface OpcionesUnion {
  /**
   * Lado de la celda de muestreo, en unidades de mapa. Manda sobre calidad y coste: el error del contorno es
   * del orden de `paso` en las zonas de curvatura fuerte, y el coste crece con 1/paso². Referencia: sobre el
   * mapa de 2000 unidades, `paso` 5-8 da un borde que a escala de pantalla se lee como una curva limpia.
   */
  paso: number;
  /** Tolerancia de simplificación (Douglas-Peucker), en unidades de mapa. 0 = devolver todos los puntos. */
  tolerancia?: number;
  /** Área mínima (unidades²) para conservar un lazo — descarta las motas de un solape casi tangente. */
  areaMinima?: number;
}

/**
 * Techo de puntos de rejilla. No es una optimización sino un seguro: `paso` llega desde el llamador y un
 * valor pequeño sobre un mapa grande pediría cientos de millones de celdas. Al superarlo se agranda `paso`
 * (se pierde detalle) en vez de colgar la pestaña.
 */
const MAX_PUNTOS_REJILLA = 2_000_000;

export function formaCirculo(centro: Point, radio: number): FormaDistancia {
  return {
    caja: { minX: centro.x - radio, minY: centro.y - radio, maxX: centro.x + radio, maxY: centro.y + radio },
    distanciaConSigno: (p) => radio - distancia(p, centro),
  };
}

/**
 * `poligono` debe tener al menos 3 puntos y estar implícitamente cerrado (el último enlaza con el primero).
 *
 * Distancia y test de interior se resuelven en la MISMA pasada sobre las aristas. Escrito así, y no como
 * `pointInPolygon(...) ? d : -d` sobre `distanciaASegmento`, porque este es el punto caliente de toda la
 * fusión: se llama una vez por punto de rejilla y por polígono, y recorrer las aristas dos veces (una para
 * la distancia, otra para el cruce de rayo) costaba justo el doble. Con las zonas de influencia —polígonos
 * de 48 lados y radio de hasta 180— eso era la diferencia entre un recálculo perceptible y uno que no se nota.
 *
 * El descarte por círculo envolvente ataca la otra mitad: la rejilla se recorre por CAJA, y las esquinas de
 * la caja de una silueta redondeada están lejísimos de ella. Salir con una resta ahí evita el recorrido
 * completo de aristas para ~1 de cada 5 muestras.
 */
export function formaPoligono(poligono: readonly Point[]): FormaDistancia {
  const puntos = poligono as Point[];
  const caja = boundingBox(puntos)!;
  const centro: Point = { x: (caja.minX + caja.maxX) / 2, y: (caja.minY + caja.maxY) / 2 };
  let radioEnvolvente = 0;
  for (const p of puntos) {
    const d = distancia(centro, p);
    if (d > radioEnvolvente) radioEnvolvente = d;
  }

  return {
    caja,
    distanciaConSigno: (p, banda) => {
      // Cota por círculo envolvente: la distancia real al polígono es al menos esta, así que en cuanto la
      // cota ya cae fuera de la banda, el valor exacto también lo hace y da igual cuál de los dos se
      // devuelva. Dentro de la banda no vale como atajo (puede quedarse muy corta frente a un lado plano)
      // y se recorren las aristas.
      if (distancia(centro, p) - radioEnvolvente >= banda) return -banda;

      let minimo = Infinity;
      let dentro = false;
      for (let i = 0, j = puntos.length - 1; i < puntos.length; j = i++) {
        const a = puntos[j]!;
        const b = puntos[i]!;
        const d = distanciaASegmento(p, a, b);
        if (d < minimo) minimo = d;
        // Cruce de rayo horizontal hacia -x, idéntico al de `pointInPolygon` (misma aritmética, mismos
        // resultados en los casos límite).
        if (b.y > p.y !== a.y > p.y && p.x < ((a.x - b.x) * (p.y - b.y)) / (a.y - b.y) + b.x) dentro = !dentro;
      }
      return dentro ? minimo : -minimo;
    },
  };
}

/** Atajo para el caso habitual: unir polígonos ya materializados. Los de menos de 3 puntos se ignoran. */
export function unirPoligonos(poligonos: readonly (readonly Point[])[], opciones: OpcionesUnion): Point[][] {
  return unirFormas(
    poligonos.filter((p) => p.length >= 3).map(formaPoligono),
    opciones
  );
}

/**
 * Contorno de la unión de `formas`, como lista de lazos CERRADOS (el último punto enlaza con el primero,
 * no se repite). Los lazos exteriores y los agujeros salen con orientación opuesta, así que dibujarlos todos
 * en un mismo `Path2D`/`beginPath` y rellenar con la regla `nonzero` (la de por defecto en canvas) recorta
 * los agujeros solo.
 *
 * Devuelve `[]` si no hay formas.
 */
export function unirFormas(formas: readonly FormaDistancia[], opciones: OpcionesUnion): Point[][] {
  if (formas.length === 0) return [];

  // Caja global. El margen extra garantiza que el anillo exterior de la rejilla quede SIEMPRE fuera de toda
  // forma: sin él, una silueta que tocara el borde produciría un contorno abierto y el encadenado en lazos
  // (que asume que cada arista tiene exactamente una continuación) se rompería.
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const forma of formas) {
    if (forma.caja.minX < minX) minX = forma.caja.minX;
    if (forma.caja.minY < minY) minY = forma.caja.minY;
    if (forma.caja.maxX > maxX) maxX = forma.caja.maxX;
    if (forma.caja.maxY > maxY) maxY = forma.caja.maxY;
  }

  let paso = Math.max(1e-6, opciones.paso);
  const margen = paso * 4;
  minX -= margen;
  minY -= margen;
  maxX += margen;
  maxY += margen;

  // Ajuste del paso si la rejilla se dispararía (ver `MAX_PUNTOS_REJILLA`).
  const puntosEstimados = ((maxX - minX) / paso + 2) * ((maxY - minY) / paso + 2);
  if (puntosEstimados > MAX_PUNTOS_REJILLA) paso *= Math.sqrt(puntosEstimados / MAX_PUNTOS_REJILLA);

  /**
   * Anchura de la BANDA alrededor del borde de cada forma dentro de la que su distancia se evalúa de verdad;
   * fuera de ella el campo se satura a `-banda` (equivalente a "lejos, fuera"). Debe ser > `paso`: un punto de
   * rejilla vecino de otro que está dentro de la forma no puede tener distancia real menor que `-paso`, así
   * que con `banda = 2·paso` ningún cruce por cero cae nunca en una zona saturada. Es lo que permite recorrer
   * solo la caja de cada forma en vez de la rejilla entera por forma.
   */
  const banda = paso * 2;

  const nx = Math.ceil((maxX - minX) / paso) + 1;
  const ny = Math.ceil((maxY - minY) / paso) + 1;
  const campo = new Float64Array(nx * ny).fill(-banda);

  for (const forma of formas) {
    const i0 = Math.max(0, Math.floor((forma.caja.minX - banda - minX) / paso));
    const i1 = Math.min(nx - 1, Math.ceil((forma.caja.maxX + banda - minX) / paso));
    const j0 = Math.max(0, Math.floor((forma.caja.minY - banda - minY) / paso));
    const j1 = Math.min(ny - 1, Math.ceil((forma.caja.maxY + banda - minY) / paso));
    for (let j = j0; j <= j1; j++) {
      const y = minY + j * paso;
      const fila = j * nx;
      for (let i = i0; i <= i1; i++) {
        const d = forma.distanciaConSigno({ x: minX + i * paso, y }, banda);
        if (d <= -banda) continue;
        const idx = fila + i;
        const valor = d > banda ? banda : d;
        if (valor > campo[idx]!) campo[idx] = valor;
      }
    }
  }

  return trazarIsolineas(campo, nx, ny, minX, minY, paso, opciones);
}

// --- Marching squares ---
//
// Cada celda de la rejilla se clasifica por qué esquinas suyas quedan dentro (campo >= 0) y aporta 0, 1 o 2
// segmentos del contorno. Los extremos de esos segmentos no son puntos sueltos: viven sobre una ARISTA de la
// rejilla, y cada arista es compartida por exactamente 2 celdas. Identificar el extremo por la arista (y no
// por sus coordenadas) es lo que hace que encadenar los segmentos en lazos sea exacto y no dependa de
// comparar floats con tolerancia.

/** Id de la arista horizontal entre los puntos de rejilla (i,j) y (i+1,j). */
function idHorizontal(i: number, j: number, nx: number): number {
  return (j * nx + i) * 2;
}

/** Id de la arista vertical entre los puntos de rejilla (i,j) y (i,j+1). */
function idVertical(i: number, j: number, nx: number): number {
  return (j * nx + i) * 2 + 1;
}

function trazarIsolineas(
  campo: Float64Array,
  nx: number,
  ny: number,
  minX: number,
  minY: number,
  paso: number,
  opciones: OpcionesUnion
): Point[][] {
  const puntoDeArista = new Map<number, Point>();
  /** Arista de entrada -> arista de salida. El contorno se orienta con el INTERIOR a la izquierda del avance. */
  const siguiente = new Map<number, number>();

  /** Corte sobre la arista horizontal (i,j)-(i+1,j), interpolado linealmente al valor 0 del campo. */
  const cruceHorizontal = (i: number, j: number): Point => {
    const a = campo[j * nx + i]!;
    const b = campo[j * nx + i + 1]!;
    return { x: minX + (i + a / (a - b)) * paso, y: minY + j * paso };
  };
  const cruceVertical = (i: number, j: number): Point => {
    const a = campo[j * nx + i]!;
    const b = campo[(j + 1) * nx + i]!;
    return { x: minX + i * paso, y: minY + (j + a / (a - b)) * paso };
  };

  for (let j = 0; j < ny - 1; j++) {
    for (let i = 0; i < nx - 1; i++) {
      const supIzq = campo[j * nx + i]!;
      const supDer = campo[j * nx + i + 1]!;
      const infDer = campo[(j + 1) * nx + i + 1]!;
      const infIzq = campo[(j + 1) * nx + i]!;
      const caso = (supIzq >= 0 ? 1 : 0) | (supDer >= 0 ? 2 : 0) | (infDer >= 0 ? 4 : 0) | (infIzq >= 0 ? 8 : 0);
      if (caso === 0 || caso === 15) continue;

      // El mismo corte calculado desde la celda vecina da el MISMO float (misma fórmula, mismas entradas),
      // así que reescribir la entrada del mapa es inofensivo.
      const registrar = (id: number, punto: Point): number => {
        puntoDeArista.set(id, punto);
        return id;
      };
      // Las 4 aristas de esta celda, con su id global y su punto de corte (solo se materializa el que se use).
      const arriba = () => registrar(idHorizontal(i, j, nx), cruceHorizontal(i, j));
      const derecha = () => registrar(idVertical(i + 1, j, nx), cruceVertical(i + 1, j));
      const abajo = () => registrar(idHorizontal(i, j + 1, nx), cruceHorizontal(i, j + 1));
      const izquierda = () => registrar(idVertical(i, j, nx), cruceVertical(i, j));
      const conectar = (desde: number, hasta: number): void => {
        siguiente.set(desde, hasta);
      };

      switch (caso) {
        case 1: conectar(izquierda(), arriba()); break;
        case 2: conectar(arriba(), derecha()); break;
        case 3: conectar(izquierda(), derecha()); break;
        case 4: conectar(derecha(), abajo()); break;
        case 6: conectar(arriba(), abajo()); break;
        case 7: conectar(izquierda(), abajo()); break;
        case 8: conectar(abajo(), izquierda()); break;
        case 9: conectar(abajo(), arriba()); break;
        case 11: conectar(abajo(), derecha()); break;
        case 12: conectar(derecha(), izquierda()); break;
        case 13: conectar(derecha(), arriba()); break;
        case 14: conectar(arriba(), izquierda()); break;
        // Sillas de montar: dos esquinas opuestas dentro y las otras dos fuera. Las dos formas de unir los 4
        // cortes son geométricamente válidas y hay que desempatar con información extra — el promedio de las
        // 4 esquinas aproxima el valor del campo en el centro de la celda: si es "dentro", las dos esquinas
        // interiores están conectadas por el medio; si es "fuera", son dos islas separadas. Sin este desempate
        // aparecen mordiscos en los cuellos estrechos entre dos siluetas que casi se tocan.
        case 5: {
          if ((supIzq + supDer + infDer + infIzq) / 4 >= 0) {
            conectar(derecha(), arriba());
            conectar(izquierda(), abajo());
          } else {
            conectar(izquierda(), arriba());
            conectar(derecha(), abajo());
          }
          break;
        }
        case 10: {
          if ((supIzq + supDer + infDer + infIzq) / 4 >= 0) {
            conectar(arriba(), izquierda());
            conectar(abajo(), derecha());
          } else {
            conectar(arriba(), derecha());
            conectar(abajo(), izquierda());
          }
          break;
        }
      }
    }
  }

  // Encadenado: `siguiente` es una permutación sobre las aristas cortadas (cada una entra en un segmento y
  // sale de otro, en la celda vecina), así que recorrerla descompone el conjunto en ciclos = lazos cerrados.
  const visitadas = new Set<number>();
  const tolerancia = opciones.tolerancia ?? 0;
  const areaMinima = opciones.areaMinima ?? 0;
  const lazos: Point[][] = [];
  for (const inicio of siguiente.keys()) {
    if (visitadas.has(inicio)) continue;
    const lazo: Point[] = [];
    let actual: number | undefined = inicio;
    while (actual !== undefined && !visitadas.has(actual)) {
      visitadas.add(actual);
      lazo.push(puntoDeArista.get(actual)!);
      actual = siguiente.get(actual);
    }
    if (lazo.length < 3) continue;
    if (areaMinima > 0 && Math.abs(areaFirmada(lazo)) < areaMinima) continue;
    lazos.push(tolerancia > 0 ? simplificar(lazo, tolerancia) : lazo);
  }
  return lazos;
}

/** Área con signo (fórmula del zapatero). El SIGNO distingue lazo exterior de agujero: salen opuestos. */
export function areaFirmada(poligono: readonly Point[]): number {
  let suma = 0;
  for (let i = 0, j = poligono.length - 1; i < poligono.length; j = i++) {
    suma += poligono[j]!.x * poligono[i]!.y - poligono[i]!.x * poligono[j]!.y;
  }
  return suma / 2;
}

/**
 * Douglas-Peucker sobre un lazo CERRADO. Un lazo no tiene extremos que anclar, así que se parte en dos
 * polilíneas por el punto más lejano del primero (el "diámetro" del lazo) y se simplifica cada mitad; sin ese
 * corte, DP con extremos coincidentes colapsaría el lazo entero.
 *
 * Es decimación, no suavizado: nunca mueve un punto, solo descarta los que están a menos de `tolerancia` de
 * la cuerda que los salta. Con `tolerancia` por debajo del paso de rejilla el contorno queda visualmente
 * idéntico con una fracción de los puntos — que es lo que importa cuando esto se traza en cada frame.
 */
function simplificar(lazo: Point[], tolerancia: number): Point[] {
  if (lazo.length < 4) return lazo;

  let opuesto = 0;
  let maxima = -1;
  for (let i = 1; i < lazo.length; i++) {
    const d = distancia(lazo[0]!, lazo[i]!);
    if (d > maxima) {
      maxima = d;
      opuesto = i;
    }
  }
  if (opuesto === 0) return lazo;

  const cerrado = [...lazo, lazo[0]!];
  const conservar = new Uint8Array(cerrado.length);
  conservar[0] = 1;
  conservar[opuesto] = 1;
  conservar[cerrado.length - 1] = 1;
  simplificarTramo(cerrado, 0, opuesto, tolerancia, conservar);
  simplificarTramo(cerrado, opuesto, cerrado.length - 1, tolerancia, conservar);

  const resultado = lazo.filter((_, i) => conservar[i] === 1);
  return resultado.length >= 3 ? resultado : lazo;
}

/** Pila explícita en vez de recursión: un lazo puede traer miles de puntos y el peor caso de DP tiene
 * profundidad O(n), suficiente para desbordar la pila del navegador. */
function simplificarTramo(puntos: Point[], desde: number, hasta: number, tolerancia: number, conservar: Uint8Array): void {
  const pendientes: [number, number][] = [[desde, hasta]];
  while (pendientes.length > 0) {
    const [inicio, fin] = pendientes.pop()!;
    if (fin <= inicio + 1) continue;
    let indice = -1;
    let maxima = tolerancia;
    for (let i = inicio + 1; i < fin; i++) {
      const d = distanciaASegmento(puntos[i]!, puntos[inicio]!, puntos[fin]!);
      if (d > maxima) {
        maxima = d;
        indice = i;
      }
    }
    if (indice < 0) continue;
    conservar[indice] = 1;
    pendientes.push([inicio, indice], [indice, fin]);
  }
}
