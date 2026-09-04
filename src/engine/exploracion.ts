// Lo que una Facción ha llegado a ver alguna vez, como ÁREA (niebla de guerra, Paso 2 —
// `Consideraciones/Niebla_De_Guerra_Definicion.md` §4).
//
// Es la mitad de la niebla que separa el estado 1 del 2: "nunca he estado ahí" (tapado) de "ya lo vi alguna
// vez" (visible, aunque sea con la última foto). Lo que se ve AHORA no se guarda aquí ni en ningún sitio: se
// deriva en cada proyección de dónde están tus plazas y tus columnas (`seVeAhora`).
//
// **Por qué una rejilla y no una lista de puntos**: lo explorado es una superficie, no un conjunto de sitios.
// Preguntarle "¿he estado en (817, 1204)?" a una lista de posiciones históricas obligaría a recorrerla entera
// comparando distancias, y esa lista crecería sin techo con cada tick jugado. La rejilla tiene un tamaño
// máximo fijo y responde en O(1).
//
// **Por qué en HEXADECIMAL y no como lista de celdas**: sobre el mundo de 2000 con celdas de 25 hay 6.400
// celdas, y una Facción que las haya explorado todas serían 6.400 pares `"col,row"` — unos 45 KB por Facción
// en cada snapshot, treinta veces al escribir la partida. Un bit por celda son 800 bytes, 1.600 caracteres
// hex. Se eligió hex y no base64 para no depender de `Buffer` ni de `btoa` (este módulo es aritmética pura,
// como el resto del motor) y porque un volcado hex se sigue pudiendo leer a ojo en un snapshot.
import type { Point } from '../domain/types';
import { EXPLORACION } from '../constants';

/**
 * Celdas exploradas, un bit por celda, en hexadecimal. Cadena vacía = nada explorado, que es lo que vale una
 * Facción sin registro — de ahí que las partidas guardadas antes de esta mecánica no necesiten migración.
 *
 * Es un alias de `string` a propósito y no un tipo nominal: viaja en el snapshot y en la proyección como
 * JSON, y envolverlo obligaría a serializar/deserializar en cada frontera sin ganar nada.
 */
export type Exploracion = string;

export const SIN_EXPLORAR: Exploracion = '';

/** Discretización del mundo. Se DERIVA de los límites del mapa en cada uso: no se guarda, porque duplicarla
 * en el estado abriría la puerta a que una partida cargada la tuviera desfasada respecto a su propio mapa. */
export interface Rejilla {
  columnas: number;
  filas: number;
  tamanoCelda: number;
}

export function rejillaDe(limites: { ancho: number; alto: number }): Rejilla {
  const tamanoCelda = EXPLORACION.tamanoCelda;
  return {
    columnas: Math.ceil(limites.ancho / tamanoCelda),
    filas: Math.ceil(limites.alto / tamanoCelda),
    tamanoCelda,
  };
}

function bytesDe(rejilla: Rejilla): number {
  return Math.ceil((rejilla.columnas * rejilla.filas) / 8);
}

function aBytes(exploracion: Exploracion, rejilla: Rejilla): Uint8Array {
  const bytes = new Uint8Array(bytesDe(rejilla));
  // Se lee solo lo que quepa: una rejilla más pequeña que la cadena (mapa distinto) trunca en vez de romper.
  const pares = Math.min(bytes.length, Math.floor(exploracion.length / 2));
  for (let i = 0; i < pares; i++) bytes[i] = Number.parseInt(exploracion.slice(i * 2, i * 2 + 2), 16) || 0;
  return bytes;
}

function aTexto(bytes: Uint8Array): Exploracion {
  let salida = '';
  for (const byte of bytes) salida += byte.toString(16).padStart(2, '0');
  return salida;
}

/** Índice de bit de la celda que contiene el punto, o `null` si cae fuera del mapa. */
function indiceDe(punto: Point, rejilla: Rejilla): number | null {
  const columna = Math.floor(punto.x / rejilla.tamanoCelda);
  const fila = Math.floor(punto.y / rejilla.tamanoCelda);
  if (columna < 0 || fila < 0 || columna >= rejilla.columnas || fila >= rejilla.filas) return null;
  return fila * rejilla.columnas + columna;
}

export function estaExplorado(exploracion: Exploracion, rejilla: Rejilla, punto: Point): boolean {
  const indice = indiceDe(punto, rejilla);
  if (indice === null) return false;
  const posicion = indice >> 3;
  if (posicion * 2 + 2 > exploracion.length) return false;
  const byte = Number.parseInt(exploracion.slice(posicion * 2, posicion * 2 + 2), 16) || 0;
  return (byte & (1 << (indice & 7))) !== 0;
}

/**
 * Marca como explorado todo lo que un ojo situado en `centro` alcanza con `radio`.
 *
 * Una celda cuenta como vista cuando lo está su CENTRO, no cuando la roza el borde del círculo. Es
 * conservador en la dirección correcta: la niebla se retira un poco más despacio de lo que en rigor se ve, en
 * vez de destapar terreno que nadie llegó a mirar.
 *
 * Con una excepción: **la celda donde está el propio ojo se marca siempre**, alcance el radio a su centro o
 * no. Sin ella, un ojo de radio pequeño plantado cerca del borde de su celda no marcaba ni el suelo que
 * pisaba, que es la única forma en que la regla del centro produce un resultado absurdo.
 *
 * Devuelve la MISMA cadena si no había nada nuevo que marcar. No es una micro-optimización: es lo que hace
 * que un tick sin exploración nueva —el caso normal de una Facción que no se mueve— no ensucie el estado con
 * un objeto distinto cada minuto.
 */
export function marcarVisto(
  exploracion: Exploracion,
  rejilla: Rejilla,
  centro: Point,
  radio: number
): Exploracion {
  const bytes = aBytes(exploracion, rejilla);
  const media = rejilla.tamanoCelda / 2;
  const desde = {
    columna: Math.max(0, Math.floor((centro.x - radio) / rejilla.tamanoCelda)),
    fila: Math.max(0, Math.floor((centro.y - radio) / rejilla.tamanoCelda)),
  };
  const hasta = {
    columna: Math.min(rejilla.columnas - 1, Math.floor((centro.x + radio) / rejilla.tamanoCelda)),
    fila: Math.min(rejilla.filas - 1, Math.floor((centro.y + radio) / rejilla.tamanoCelda)),
  };

  let cambio = false;
  const marcar = (indice: number): void => {
    const posicion = indice >> 3;
    const mascara = 1 << (indice & 7);
    if ((bytes[posicion]! & mascara) !== 0) return;
    bytes[posicion] = bytes[posicion]! | mascara;
    cambio = true;
  };

  const propia = indiceDe(centro, rejilla);
  if (propia !== null) marcar(propia);

  for (let fila = desde.fila; fila <= hasta.fila; fila++) {
    for (let columna = desde.columna; columna <= hasta.columna; columna++) {
      const dx = columna * rejilla.tamanoCelda + media - centro.x;
      const dy = fila * rejilla.tamanoCelda + media - centro.y;
      if (dx * dx + dy * dy > radio * radio) continue;

      marcar(fila * rejilla.columnas + columna);
    }
  }

  return cambio ? aTexto(bytes) : exploracion;
}

/**
 * Lo explorado tal como viaja a un cliente: el bitmap MÁS la geometría con que se descifra.
 *
 * Van juntos a propósito. El cliente necesita el tamaño de celda para saber qué tapa cada bit, y si tuviera
 * que ir a buscarlo a `GET /v1/balance` bastaría una versión de más para que pintase la niebla desplazada
 * sobre el mapa sin que nada fallara de forma visible. La máscara y su geometría son un solo dato.
 */
export interface NieblaProyectada {
  /** Lado de una celda, en unidades de mapa. */
  tamanoCelda: number;
  columnas: number;
  filas: number;
  /** Un bit por celda, en hexadecimal, recorriendo el mundo fila a fila desde (0,0). 1 = explorado. El bit
   * de la celda `(columna, fila)` es el `fila * columnas + columna`, contando desde el bit MENOS significativo
   * de cada byte, y cada byte son dos caracteres hex. */
  celdas: Exploracion;
}

export function proyectarNiebla(exploracion: Exploracion, rejilla: Rejilla): NieblaProyectada {
  return { tamanoCelda: rejilla.tamanoCelda, columnas: rejilla.columnas, filas: rejilla.filas, celdas: exploracion };
}

/** Cuántas celdas hay marcadas. Para tests y métricas del laboratorio — ninguna regla de juego lo consulta. */
export function celdasExploradas(exploracion: Exploracion): number {
  let total = 0;
  for (let i = 0; i + 1 < exploracion.length; i += 2) {
    let byte = Number.parseInt(exploracion.slice(i, i + 2), 16) || 0;
    while (byte !== 0) {
      total += byte & 1;
      byte >>= 1;
    }
  }
  return total;
}
