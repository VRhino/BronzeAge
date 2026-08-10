// Ruido fractal para los campos continuos del mundo (elevación, fertilidad).
//
// POR QUÉ EXISTE ESTE MÓDULO. Hasta Fase 0.1 los dos campos se construían como suma de octavas de
// `sin(x·f + φx) · cos(y·f + φy)`. Es un producto de sinusoides ALINEADAS A LOS EJES: matemáticamente
// continuo y determinista, pero visualmente un enrejado regular en diagonal, que además se repite igual a
// todas las escalas porque las frecuencias iban en progresión lineal (f, 2f, 3f, 4f) en vez de geométrica.
// El resultado no se lee como terreno bajo NINGÚN sombreado — se intentó, y subir el contraste solo hacía
// más evidente el patrón. El problema no era cómo se dibujaba el mapa, sino cómo se generaba.
//
// La sustitución es ruido de GRADIENTE (Perlin) sumado en octavas (fBm), que es el estándar para terreno
// procedural justamente porque no tiene dirección privilegiada: el valor en cada punto sale de interpolar
// gradientes pseudoaleatorios de una rejilla, no de multiplicar ondas.
//
// Se mantienen intactos los contratos de `worldgen/`:
//  - DETERMINISTA por seed: la tabla de permutación se baraja con el PRNG del mundo.
//  - DATOS PUROS: `CampoRuido` es tabla + números, serializable y comparable; `evaluarRuido` es pura.
//  - Definida en TODO el plano, también fuera de los límites del mapa (los muestreos en anillo del motor
//    pueden salirse del borde), y estrictamente normalizada a 0-1.
//  - Sin dependencias externas.

import type { Point } from '../domain/types';
import type { RandomFn } from './rng';

const TAMANO_TABLA = 256;

/**
 * Gradientes de las esquinas: 8 direcciones UNITARIAS a intervalos de 45°. Que todas tengan la misma
 * longitud importa — el juego de gradientes "clásico" de Perlin en 2D mezcla ejes (longitud 1) y diagonales
 * (longitud √2), y esa desigualdad sesga el resultado hacia las diagonales.
 */
const S = Math.SQRT1_2;
const GRADIENTES: readonly (readonly [number, number])[] = [
  [1, 0],
  [S, S],
  [0, 1],
  [-S, S],
  [-1, 0],
  [-S, -S],
  [0, -1],
  [S, -S],
];

/** Desplazamiento propio de cada octava. Sin esto todas las octavas comparten la misma rejilla y sus
 * vértices coinciden (el valor del ruido es 0 en cada vértice), dejando una retícula de "nudos" visible. */
export interface DesplazamientoOctava {
  dx: number;
  dy: number;
}

/**
 * Campo de ruido fractal. La tabla de permutación es el "mundo" del ruido: barajarla con el PRNG es lo que
 * hace que cada seed produzca un relieve distinto.
 */
export interface CampoRuido {
  /** Permutación de 0..255 DUPLICADA (512 entradas): evita un módulo en cada consulta de esquina. */
  permutacion: number[];
  octavas: number;
  /** Frecuencia de la octava más gruesa, en ciclos por unidad de mapa (1/frecuencia ≈ tamaño de formación). */
  frecuenciaBase: number;
  /** Cuánto sube la frecuencia por octava (2 = cada octava tiene el doble de detalle). */
  lacunaridad: number;
  /** Cuánto baja la amplitud por octava (0.5 = cada octava pesa la mitad que la anterior). */
  persistencia: number;
  desplazamientos: DesplazamientoOctava[];
  /** Suma de amplitudes, precalculada: normaliza el resultado. */
  amplitudTotal: number;
}

export interface OpcionesRuido {
  octavas: number;
  frecuenciaBase: number;
  lacunaridad: number;
  persistencia: number;
}

export function generarCampoRuido(rng: RandomFn, opciones: OpcionesRuido): CampoRuido {
  // Fisher-Yates con el PRNG del mundo: la permutación queda determinada por la seed.
  const base = Array.from({ length: TAMANO_TABLA }, (_, i) => i);
  for (let i = TAMANO_TABLA - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    const tmp = base[i]!;
    base[i] = base[j]!;
    base[j] = tmp;
  }

  const desplazamientos = Array.from({ length: opciones.octavas }, () => ({
    dx: rng() * 256,
    dy: rng() * 256,
  }));

  let amplitudTotal = 0;
  let amplitud = 1;
  for (let i = 0; i < opciones.octavas; i++) {
    amplitudTotal += amplitud;
    amplitud *= opciones.persistencia;
  }

  return { permutacion: [...base, ...base], ...opciones, desplazamientos, amplitudTotal };
}

/** Curva de suavizado de Perlin (6t⁵−15t⁴+10t³): primera y segunda derivada nulas en 0 y 1, así que las
 * celdas del ruido encajan sin discontinuidad visible en la pendiente — que es justo lo que mira el
 * sombreado de relieve. */
function suavizar(t: number): number {
  return t * t * t * (t * (t * 6 - 15) + 10);
}

function interpolar(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/** Aporte de una esquina: su gradiente pseudoaleatorio proyectado sobre el vector que va de ella al punto. */
function aporteEsquina(hash: number, dx: number, dy: number): number {
  const g = GRADIENTES[hash & 7]!;
  return g[0] * dx + g[1] * dy;
}

/**
 * Ruido de gradiente 2D en el punto dado. Rango teórico [−√2/2, +√2/2]. Definido para cualquier coordenada,
 * incluidas negativas: `& 255` sobre el entero envuelve la rejilla en ambos sentidos.
 */
function ruidoGradiente(permutacion: number[], x: number, y: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const xf = x - xi;
  const yf = y - yi;
  const cx = xi & 255;
  const cy = yi & 255;

  const u = suavizar(xf);
  const v = suavizar(yf);

  const filaA = permutacion[cx]!;
  const filaB = permutacion[cx + 1]!;
  const aa = permutacion[filaA + cy]!;
  const ab = permutacion[filaA + cy + 1]!;
  const ba = permutacion[filaB + cy]!;
  const bb = permutacion[filaB + cy + 1]!;

  const superior = interpolar(aporteEsquina(aa, xf, yf), aporteEsquina(ba, xf - 1, yf), u);
  const inferior = interpolar(aporteEsquina(ab, xf, yf - 1), aporteEsquina(bb, xf - 1, yf - 1), u);
  return interpolar(superior, inferior, v);
}

/**
 * Valor del campo en un punto, normalizado a 0-1. Función PURA: mismo campo + mismo punto dan siempre el
 * mismo valor. La normalización usa la cota TEÓRICA del ruido (√2/2 por octava), no el rango observado, para
 * que el 0-1 esté garantizado en todo el plano y no dependa de qué zona se haya muestreado — de ahí que en
 * la práctica los valores se concentren alrededor de 0.5 y los umbrales de clasificación (`ELEVACION`,
 * `BIOMA`) estén calibrados contra la distribución real, no repartidos por el rango nominal.
 */
export function evaluarRuido(campo: CampoRuido, p: Point): number {
  let valor = 0;
  let amplitud = 1;
  let frecuencia = campo.frecuenciaBase;

  for (let i = 0; i < campo.octavas; i++) {
    const desplazamiento = campo.desplazamientos[i]!;
    valor += ruidoGradiente(campo.permutacion, p.x * frecuencia + desplazamiento.dx, p.y * frecuencia + desplazamiento.dy) * amplitud;
    frecuencia *= campo.lacunaridad;
    amplitud *= campo.persistencia;
  }

  const normalizado = valor / (campo.amplitudTotal * S);
  return Math.min(1, Math.max(0, (normalizado + 1) / 2));
}
