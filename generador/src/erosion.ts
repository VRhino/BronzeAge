// Erosión por gotas (experimento/rios, fase 5; Beyer 2015): miles de gotas caen al azar, bajan por la ladera, arrancan tierra donde
// corren deprisa y la dejan donde se frenan. Abre cárcavas en las laderas y suaviza los pies. Se simula una vez sobre una rejilla y se
// guarda lo que cambió cada celda (`delta`), que `evaluarElevacion` suma interpolado: datos planos, como el resto del campo.
import type { Point } from './tipos';
import { createRng, derivarSeed } from './rng';
import { ELEVACION } from './config';

/** Sobre la rejilla de celdas del artefacto (4 u, 48 m: sus alturas ya están medidas). `gotasPorCelda` gotas, cada una de `vida` pasos. Inercia, capacidad de arrastre, erosión, depósito y
 * evaporación como en Beyer; `radio` celdas de pincel al arrancar. El cambio se acota a ±`tope` m y se apaga cerca de los ríos (a
 * `margenRio` m de su orilla: más que la diagonal de la celda, para que el cauce no tome nada al interpolar; en `transicion` m), del agua y de la costa. */
export const EROSION = {
  gotasPorCelda: 0.3, vida: 30, inercia: 0.05, capacidad: 4, capacidadMinima: 0.01, arranque: 0.3, deposito: 0.3,
  evaporacion: 0.01, gravedad: 4, radio: 2, tope: 10, margenRio: 60, transicion: 120,
};

export interface CampoErosion { celda: number; columnas: number; filas: number; delta: Float32Array }

/** La elevación `e` en `p` con la erosión (interpolada entre centros de celda). */
export function aplicarErosion(c: CampoErosion, p: Point, e: number): number {
  const fx = Math.min(Math.max(p.x / c.celda - 0.5, 0), c.columnas - 1.001), fy = Math.min(Math.max(p.y / c.celda - 0.5, 0), c.filas - 1.001);
  const i = Math.floor(fx), j = Math.floor(fy), a = fx - i, b = fy - j, k = j * c.columnas + i, d = c.delta;
  return e + (d[k]! * (1 - a) + d[k + 1]! * a) * (1 - b) + (d[k + c.columnas]! * (1 - a) + d[k + c.columnas + 1]! * a) * b;
}

/** Simula la erosión sobre la rejilla `celdas` (sus alturas, normalizadas: 0-1 = 0-1000 m, en el centro de cada celda de `lado`).
 * `peso` (0-1, con la altura del punto) dice cuánto puede cambiar cada punto: 0 en los ríos, el agua y la costa. */
export function crearErosion(seed: number, celdas: { lado: number; columnas: number; filas: number; altura: Float64Array }, peso: (p: Point, e: number) => number): CampoErosion {
  const E = { ...EROSION, celda: celdas.lado }, { columnas, filas } = celdas, n = columnas * filas;
  // alturas en celdas (la pendiente sale directa: celdas de subida por celda de avance)
  const H = new Float64Array(n), W = new Float32Array(n);
  for (let j = 0; j < filas; j++) for (let i = 0; i < columnas; i++) {
    const p = { x: (i + 0.5) * E.celda, y: (j + 0.5) * E.celda }, k = j * columnas + i;
    const e = celdas.altura[k]!;
    H[k] = (e * 1000) / E.celda; W[k] = peso(p, e);
  }
  const original = Float64Array.from(H);
  // el pincel: celdas a menos de `radio`, con su peso
  const pincel: [number, number, number][] = [];
  for (let dy = -E.radio; dy <= E.radio; dy++) for (let dx = -E.radio; dx <= E.radio; dx++) {
    const d = Math.hypot(dx, dy);
    if (d <= E.radio) pincel.push([dx, dy, 1 - d / E.radio]);
  }
  const sumaPincel = pincel.reduce((s, [, , w]) => s + w, 0);
  const alturaYPendiente = (x: number, y: number) => {
    const i = Math.floor(x), j = Math.floor(y), u = x - i, v = y - j, k = j * columnas + i;
    const a = H[k]!, b = H[k + 1]!, c = H[k + columnas]!, d = H[k + columnas + 1]!;
    return { h: a * (1 - u) * (1 - v) + b * u * (1 - v) + c * (1 - u) * v + d * u * v, gx: (b - a) * (1 - v) + (d - c) * v, gy: (c - a) * (1 - u) + (d - b) * u };
  };
  const rng = createRng(derivarSeed(seed, 'erosion-gotas-1'));
  const gotas = Math.round(n * E.gotasPorCelda);
  for (let g = 0; g < gotas; g++) {
    let x = rng() * (columnas - 2), y = rng() * (filas - 2), dx = 0, dy = 0, velocidad = 1, agua = 1, sedimento = 0;
    for (let paso = 0; paso < E.vida; paso++) {
      const i = Math.floor(x), j = Math.floor(y), u = x - i, v = y - j, k = j * columnas + i;
      const antes = alturaYPendiente(x, y);
      dx = dx * E.inercia - antes.gx * (1 - E.inercia); dy = dy * E.inercia - antes.gy * (1 - E.inercia);
      const l = Math.hypot(dx, dy);
      if (l < 1e-9) break;
      dx /= l; dy /= l; x += dx; y += dy;
      if (x < 0 || y < 0 || x >= columnas - 1 || y >= filas - 1) break;
      const sube = alturaYPendiente(x, y).h - antes.h;
      const capacidad = Math.max(-sube * velocidad * agua * E.capacidad, E.capacidadMinima);
      if (sedimento > capacidad || sube > 0) {
        // deja tierra en las cuatro esquinas de la celda de donde viene
        const deja = sube > 0 ? Math.min(sube, sedimento) : (sedimento - capacidad) * E.deposito;
        sedimento -= deja;
        H[k] = H[k]! + deja * (1 - u) * (1 - v); H[k + 1] = H[k + 1]! + deja * u * (1 - v);
        H[k + columnas] = H[k + columnas]! + deja * (1 - u) * v; H[k + columnas + 1] = H[k + columnas + 1]! + deja * u * v;
      } else {
        // arranca con el pincel, sin cavar más hondo de lo que baja
        const arranca = Math.min((capacidad - sedimento) * E.arranque, -sube);
        for (const [ox, oy, w] of pincel) {
          const ci = i + ox, cj = j + oy;
          if (ci < 0 || cj < 0 || ci >= columnas || cj >= filas) continue;
          const c = cj * columnas + ci, quita = Math.min(H[c]!, (arranca * w) / sumaPincel);
          H[c] = H[c]! - quita; sedimento += quita;
        }
      }
      velocidad = Math.sqrt(Math.max(0, velocidad * velocidad + sube * E.gravedad));
      agua *= 1 - E.evaporacion;
    }
  }
  const delta = new Float32Array(n), tope = E.tope / E.celda;
  // acotado, con su peso, y sin bajar nunca un punto de tierra hasta el agua
  for (let k = 0; k < n; k++) {
    const d = (Math.max(-tope, Math.min(tope, H[k]! - original[k]!)) * W[k]! * E.celda) / 1000, e = celdas.altura[k]!;
    delta[k] = e > ELEVACION.umbralAgua + 0.005 ? Math.max(d, ELEVACION.umbralAgua + 0.005 - e) : d;
  }
  return { celda: E.celda, columnas, filas, delta };
}
