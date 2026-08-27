// Copia de la fusión de siluetas por campo de distancia + marching squares (`src/world/poligonos.ts` del
// backend) — geometría pura, ciega a qué es una Facción o un bosque, igual que el resto de `terreno/`.
//
// Reducida a propósito: el backend también expone `formaPoligono`/`unirPoligonos` para fusionar zonas de
// influencia de Facción, pero esas dependen de la posición de TODOS los asentamientos rivales (entrada
// privilegiada, T2b en doc 9) — no se pueden calcular aquí ni con esta función ni con ninguna otra. Lo único
// que este cliente necesita fusionar es BOSQUES (`bosques.ts`), que son círculos y dato público desde C11a,
// así que solo se copia `formaCirculo` + el motor de fusión (`unirFormas`), no el camino de polígonos.

import type { Point } from './tipos';

interface Caja {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/** Ver `FormaDistancia` en el backend: `distanciaConSigno` responde POSITIVO dentro, NEGATIVO fuera. */
interface FormaDistancia {
  readonly caja: Caja;
  distanciaConSigno(p: Point, banda: number): number;
}

export interface OpcionesUnion {
  paso: number;
  tolerancia?: number;
  areaMinima?: number;
}

const MAX_PUNTOS_REJILLA = 2_000_000;

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

export function formaCirculo(centro: Point, radio: number): FormaDistancia {
  return {
    caja: { minX: centro.x - radio, minY: centro.y - radio, maxX: centro.x + radio, maxY: centro.y + radio },
    distanciaConSigno: (p) => radio - distancia(p, centro),
  };
}

/** Contorno de la unión de `formas`, como lazos CERRADOS. Devuelve `[]` si no hay formas. Ver el original en
 * `src/world/poligonos.ts` para la explicación completa del método (campo de distancia + marching squares). */
export function unirFormas(formas: readonly FormaDistancia[], opciones: OpcionesUnion): Point[][] {
  if (formas.length === 0) return [];

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

  const puntosEstimados = ((maxX - minX) / paso + 2) * ((maxY - minY) / paso + 2);
  if (puntosEstimados > MAX_PUNTOS_REJILLA) paso *= Math.sqrt(puntosEstimados / MAX_PUNTOS_REJILLA);

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

function idHorizontal(i: number, j: number, nx: number): number {
  return (j * nx + i) * 2;
}

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
  const siguiente = new Map<number, number>();

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

      const registrar = (id: number, punto: Point): number => {
        puntoDeArista.set(id, punto);
        return id;
      };
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

function areaFirmada(poligono: readonly Point[]): number {
  let suma = 0;
  for (let i = 0, j = poligono.length - 1; i < poligono.length; j = i++) {
    suma += poligono[j]!.x * poligono[i]!.y - poligono[i]!.x * poligono[j]!.y;
  }
  return suma / 2;
}

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

function distanciaASegmento(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const largoCuadrado = dx * dx + dy * dy;
  if (largoCuadrado === 0) return distancia(p, a);
  const t = Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / largoCuadrado));
  return distancia(p, { x: a.x + t * dx, y: a.y + t * dy });
}

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
