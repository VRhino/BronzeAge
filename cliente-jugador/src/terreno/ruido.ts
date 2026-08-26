// Copia de las funciones de EVALUACIÓN (puras, sin RNG) de `src/worldgen/ruido.ts` del backend — no la
// generación (`generarCampoRuido`, que consume RNG y por tanto es simulación, server-only). El servidor ya
// generó el campo; este cliente solo necesita LEERLO en un punto para pintar el terreno. Ver el README.
import type { CampoRuido, Point } from './tipos';

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

function suavizar(t: number): number {
  return t * t * t * (t * (t * 6 - 15) + 10);
}

function interpolar(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function aporteEsquina(hash: number, dx: number, dy: number): number {
  const g = GRADIENTES[hash & 7]!;
  return g[0] * dx + g[1] * dy;
}

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

/** Valor del campo en un punto, normalizado 0-1. Debe coincidir bit a bit con `evaluarRuido` del servidor —
 * misma tabla de permutación (viaja en `CampoRuido.permutacion`), mismo algoritmo. */
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

/** Como `evaluarRuido`, pero solo con las `octavasMax` primeras octavas — usado por `elevacion.ts` para el
 * suavizado jugable (Fase 0.4.2 del servidor). */
export function evaluarRuidoParcial(campo: CampoRuido, p: Point, octavasMax: number): number {
  const octavas = Math.min(octavasMax, campo.octavas);
  let valor = 0;
  let amplitud = 1;
  let amplitudTotal = 0;
  let frecuencia = campo.frecuenciaBase;

  for (let i = 0; i < octavas; i++) {
    const desplazamiento = campo.desplazamientos[i]!;
    valor += ruidoGradiente(campo.permutacion, p.x * frecuencia + desplazamiento.dx, p.y * frecuencia + desplazamiento.dy) * amplitud;
    amplitudTotal += amplitud;
    frecuencia *= campo.lacunaridad;
    amplitud *= campo.persistencia;
  }

  const normalizado = valor / (amplitudTotal * S);
  return Math.min(1, Math.max(0, (normalizado + 1) / 2));
}
