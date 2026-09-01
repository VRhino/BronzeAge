// MURALLAS — el trazo del recinto (`Consideraciones/Murallas_Definicion.md`, Paso 1).
//
// Este módulo es GEOMETRÍA PURA: dada una ciudad, devuelve dónde CAERÍA su muralla y cuánto costaría. No
// persiste nada, no cobra nada y no toca `Asentamiento`. La entidad `Recinto`, la obra celda a celda y el
// efecto defensivo llegan en los pasos 2 y 3b del plan.
//
// Vive aparte de `engine/trazado.ts` (ya 1900+ líneas) a propósito: el trazado urbano razona sobre CÓMO CRECE
// la ciudad, y esto razona sobre DÓNDE TERMINA. Solo comparten el vocabulario de celdas.
//
// Por qué el anillo es una dilatación y no un casco convexo ni un círculo: la ciudad es una estrella de
// distritos alrededor de sus anclas, no un disco. Un círculo encerraría campo vacío y dejaría fuera los brazos;
// un casco convexo encerraría los huecos entre brazos. La dilatación morfológica sigue la forma real, que es
// lo único coherente con un trazado emergente.
import type { Asentamiento, CeldaMuro, Edificio, Recinto, RecursoAlmacenado, RecursoTipo } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { MURALLA } from '../constants';
import { descontarRecursos, tieneRecursos } from './almacen';
import { nivelActualDe } from './asentamientoQuery';
import type { EventoCrudo } from '../domain/eventos';
import {
  celdasDeEdificio,
  edificiosInternos,
  esDeAfueras,
  fusionarCeldas,
  integridadDeRecinto,
  redDeCalles,
  type RedDeCalles,
  type TrazadoMuralla,
} from './trazado';

// `CeldaMuro` y `Recinto` viven en `domain/types.ts`: son estado PERSISTIDO (se pagan), no geometría derivada.
export type { CeldaMuro, Recinto };

export interface TrazoRecinto {
  /** El anillo completo, en orden de recorrido desde la puerta principal — el orden en que se levantará. */
  celdas: CeldaMuro[];
  puertas: number;
  torres: number;
  /** Celdas de suelo que el recinto encierra (incluye la franja de ronda). */
  areaEncerrada: number;
  /** Edificios que quedarían DENTRO del recinto. */
  dentro: string[];
  /** Edificios urbanos que quedarían FUERA: el arrabal del día 1 (§9). */
  fuera: string[];
  /** Granjas y Corrales que quedarían DENTRO. Normalmente vacío: el trazo no puede expandirse hasta las
   * afueras (§11.2). Solo se llena cuando la ciudad ha rodeado una por los cuatro costados y el relleno de
   * agujeros se la traga — ver `rellenarAgujeros`. Se expone en vez de esconderse porque es la única grieta
   * de la garantía del enunciado, y una grieta medible es una grieta vigilada. */
  afuerasDentro: string[];
  /** Coste total de levantarlo al nivel pedido, ya con los factores de puerta y torre aplicados. */
  costo: Partial<Record<string, number>>;
}

// --- Conjuntos de celdas, indexados por clave numérica ---
//
// Clave numérica y no `"col,row"`: el Paso 2 de la Etapa 6 midió que las claves de texto eran el 30-40% del
// coste del trazado (75.000 cadenas por barrido) y hubo que reescribirlo sobre índices. No repetimos el error.
const DESPLAZAMIENTO = 1 << 15;
function clave(col: number, row: number): number {
  return (col + DESPLAZAMIENTO) * (1 << 16) + (row + DESPLAZAMIENTO);
}
function columnaDe(k: number): number {
  return Math.floor(k / (1 << 16)) - DESPLAZAMIENTO;
}
function filaDe(k: number): number {
  return (k % (1 << 16)) - DESPLAZAMIENTO;
}

const VECINAS_4: readonly [number, number][] = [
  [0, -1],
  [1, 0],
  [0, 1],
  [-1, 0],
];

/** Dilatación morfológica con vecindad 4 (`pasos` iteraciones). Con vecindad 4 las esquinas salen achaflanadas
 * en vez de en ángulo recto, que es lo que da al recinto su silueta orgánica en vez de la de una caja. */
function dilatar(conjunto: Set<number>, pasos: number): Set<number> {
  let actual = conjunto;
  for (let i = 0; i < pasos; i++) {
    const siguiente = new Set(actual);
    for (const k of actual) {
      const col = columnaDe(k);
      const row = filaDe(k);
      for (const [dc, dr] of VECINAS_4) siguiente.add(clave(col + dc, row + dr));
    }
    actual = siguiente;
  }
  return actual;
}

interface Caja {
  minCol: number;
  minRow: number;
  maxCol: number;
  maxRow: number;
}

function cajaDe(conjunto: Set<number>, margen: number): Caja {
  let minCol = Infinity;
  let minRow = Infinity;
  let maxCol = -Infinity;
  let maxRow = -Infinity;
  for (const k of conjunto) {
    const col = columnaDe(k);
    const row = filaDe(k);
    if (col < minCol) minCol = col;
    if (col > maxCol) maxCol = col;
    if (row < minRow) minRow = row;
    if (row > maxRow) maxRow = row;
  }
  return { minCol: minCol - margen, minRow: minRow - margen, maxCol: maxCol + margen, maxRow: maxRow + margen };
}

/** La componente conexa (vecindad 4) de `conjunto` que contiene a `semillas`. Todo lo que no cuelgue de ahí se
 * descarta: una isla de ciudad separada del Centro Urbano no es algo que este muro vaya a encerrar. */
function componenteDe(conjunto: Set<number>, semillas: Iterable<number>): Set<number> {
  const resultado = new Set<number>();
  const pila: number[] = [];
  for (const s of semillas) {
    if (conjunto.has(s) && !resultado.has(s)) {
      resultado.add(s);
      pila.push(s);
    }
  }
  while (pila.length > 0) {
    const k = pila.pop()!;
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) {
      const v = clave(col + dc, row + dr);
      if (conjunto.has(v) && !resultado.has(v)) {
        resultado.add(v);
        pila.push(v);
      }
    }
  }
  return resultado;
}

/**
 * Rellena los agujeros interiores: toda celda que no pertenezca a `region` y desde la que NO se pueda salir al
 * exterior de la caja pasa a formar parte de ella.
 *
 * Sin esto, un patio encerrado por la ciudad generaría un anillo de muralla INTERIOR alrededor de un hueco, que
 * no es una muralla sino un artefacto.
 *
 * **Y aquí aparece una excepción a §11.2 que solo se ve al escribir el código**: si el tejido urbano ha rodeado
 * por completo a una Granja, el relleno se la traga y la deja INTRAMUROS. Es correcto y deliberado — una granja
 * a la que la ciudad ha crecido alrededor por los cuatro costados ya no es "las afueras", es un huerto urbano.
 * Lo que la regla impide es que el muro se EXPANDA hasta las afueras (§4 paso 1); no puede impedir que la
 * ciudad las envuelva, y fabricar un anillo interior para excluirla sería peor que absorberla. Medido en el
 * Paso 0: solo ocurre a partir de nivel 3, donde entre el 0.3% y el 12% de las celdas urbanas caen más lejos
 * que la granja más cercana.
 */
function rellenarAgujeros(region: Set<number>, caja: Caja): Set<number> {
  const exterior = new Set<number>();
  const pila: number[] = [];
  const empujar = (col: number, row: number) => {
    if (col < caja.minCol || col > caja.maxCol || row < caja.minRow || row > caja.maxRow) return;
    const k = clave(col, row);
    if (region.has(k) || exterior.has(k)) return;
    exterior.add(k);
    pila.push(k);
  };
  for (let col = caja.minCol; col <= caja.maxCol; col++) {
    empujar(col, caja.minRow);
    empujar(col, caja.maxRow);
  }
  for (let row = caja.minRow; row <= caja.maxRow; row++) {
    empujar(caja.minCol, row);
    empujar(caja.maxCol, row);
  }
  while (pila.length > 0) {
    const k = pila.pop()!;
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) empujar(col + dc, row + dr);
  }

  const rellena = new Set(region);
  for (let col = caja.minCol; col <= caja.maxCol; col++) {
    for (let row = caja.minRow; row <= caja.maxRow; row++) {
      const k = clave(col, row);
      if (!region.has(k) && !exterior.has(k)) rellena.add(k);
    }
  }
  return rellena;
}

/**
 * El muro: la capa de celdas inmediatamente EXTERIOR a `region`, con vecindad 4.
 *
 * Que sea la capa de vecindad 4 es lo que lo hace impermeable sin ser gruesa: para entrar en `region` hay que
 * pisar una celda 4-adyacente a ella, y todas están en el muro. El anillo mide una celda y aun así no se puede
 * cruzar — que es justo la propiedad que el modelo de aristas nunca pudo dar.
 */
function anilloDe(region: Set<number>): Set<number> {
  const muro = new Set<number>();
  for (const k of region) {
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) {
      const v = clave(col + dc, row + dr);
      if (!region.has(v)) muro.add(v);
    }
  }
  return muro;
}

/** Celdas de todos los edificios internos, indexadas por id. */
function celdasPorEdificio(edificios: Edificio[]): Map<string, number[]> {
  const mapa = new Map<string, number[]>();
  for (const e of edificios) {
    mapa.set(
      e.id,
      celdasDeEdificio(e).map((c) => clave(c.col, c.row))
    );
  }
  return mapa;
}

const MAX_ITERACIONES_CONFLICTO = 24;

export interface OpcionesTrazo {
  /** Nivel del recinto que se está presupuestando: decide las tarifas y si hay torres (§7). */
  nivel?: number;
}

/**
 * ¿Dónde caería la muralla de este asentamiento, y cuánto costaría? Función PURA y determinista: la misma
 * ciudad da siempre el mismo anillo, las mismas clases y el mismo orden.
 *
 * `null` cuando no hay trazo válido — el gate duro de §E6.5 aplicado a esta capa: un recinto que no se puede
 * cerrar no es un recinto, y fallar aquí es infinitamente más barato que fallar después de haberlo cobrado.
 */
export function trazarRecinto(asentamiento: Asentamiento, opciones: OpcionesTrazo = {}): TrazoRecinto | null {
  const nivel = opciones.nivel ?? 1;
  const internos = edificiosInternos(asentamiento.edificios);
  const centro = internos.find((e) => e.tipo === 'centroUrbano');
  if (!centro) return null;

  const red = redDeCalles(asentamiento.id, asentamiento.edificios);
  const porEdificio = celdasPorEdificio(internos);

  // 1. `permitido`: todo menos las afueras y su vecindad inmediata. No hay tope de radio (§11.2, Paso 0): el
  //    tejido urbano no cabe en ningún disco fijo, así que el único límite es la forma de la propia ciudad.
  const vetadas = new Set<number>();
  for (const e of internos) {
    if (!esDeAfueras(e.tipo)) continue;
    for (const k of porEdificio.get(e.id) ?? []) vetadas.add(k);
  }
  const vetadasConMargen = dilatar(vetadas, 1);

  // 2. `nucleo`: el tejido urbano —edificios y CALLES, nunca caminos: un camino rural arrastraría la región
  //    hasta las granjas— recortado por `permitido` y quedándose con lo que cuelga del Centro Urbano.
  const nucleoBruto = new Set<number>();
  for (const e of internos) {
    if (esDeAfueras(e.tipo)) continue;
    for (const k of porEdificio.get(e.id) ?? []) if (!vetadasConMargen.has(k)) nucleoBruto.add(k);
  }
  for (const c of red.calles) {
    const coma = c.indexOf(',');
    const k = clave(Number(c.slice(0, coma)), Number(c.slice(coma + 1)));
    if (!vetadasConMargen.has(k)) nucleoBruto.add(k);
  }
  const celdasCentro = porEdificio.get(centro.id) ?? [];
  const nucleo = componenteDe(nucleoBruto, celdasCentro);
  if (nucleo.size === 0) return null;

  // 3-5. Región + anillo, con el bucle de resolución de conflictos: si el muro pisa un edificio, se retira esa
  //      vecindad de la región y se recalcula. Cada vuelta solo QUITA celdas, así que termina.
  let semilla = nucleo;
  let region = new Set<number>();
  let muro = new Set<number>();
  let resuelto = false;

  for (let iteracion = 0; iteracion < MAX_ITERACIONES_CONFLICTO; iteracion++) {
    const dilatada = dilatar(semilla, MURALLA.franjaDeRonda);
    const sinVetadas = new Set<number>();
    for (const k of dilatada) if (!vetadasConMargen.has(k)) sinVetadas.add(k);
    const conexa = componenteDe(sinVetadas, celdasCentro);
    if (conexa.size === 0) return null;
    region = rellenarAgujeros(conexa, cajaDe(conexa, 2));
    muro = anilloDe(region);

    // ¿Pisa el anillo algún edificio? (Uno urbano que quedó fuera de la región, o una granja pegada al borde.)
    const conflictivos: string[] = [];
    for (const [id, celdas] of porEdificio) {
      if (celdas.some((k) => muro.has(k))) conflictivos.push(id);
    }
    if (conflictivos.length === 0) {
      resuelto = true;
      break;
    }
    const aQuitar = new Set<number>();
    for (const id of conflictivos) for (const k of porEdificio.get(id) ?? []) aQuitar.add(k);
    const zona = dilatar(aQuitar, MURALLA.franjaDeRonda + 1);
    const recortada = new Set<number>();
    for (const k of semilla) if (!zona.has(k)) recortada.add(k);
    if (recortada.size === semilla.size) return null; // no se puede recortar más: no hay trazo válido
    semilla = componenteDe(recortada, celdasCentro);
    if (semilla.size === 0) return null;
  }
  if (!resuelto) return null;

  // 6. Validación: el anillo tiene que AISLAR de verdad. Se comprueba como lo comprobará el test (§15 nº 1):
  //    un flood fill desde fuera que no cruce muro no puede alcanzar el Centro Urbano.
  if (!aislaElCentro(muro, celdasCentro, cajaDe(muro, 2))) return null;

  // 7-8. Puertas (geométricas) → orden de recorrido → UNA puerta por cruce → torres (espaciadas A LO LARGO
  //      del recorrido). El orden importa: tanto el colapso de puertas como la separación de torres se miden
  //      en celdas DE ANILLO, y eso solo tiene sentido recorriéndolo. Clasificar antes de ordenar hacía que el
  //      paso de torres no significara nada — medido, salían 27-35 torres en un anillo de 120 celdas.
  const enRed = celdasDeRed(red);
  const ordenadas = marcarTorres(
    colapsarPuertas(ordenarAnillo(clasificarPuertas(muro, enRed)), muro, region, enRed),
    region,
    nivel
  );

  const dentro: string[] = [];
  const fuera: string[] = [];
  const afuerasDentro: string[] = [];
  for (const e of internos) {
    const celdasE = porEdificio.get(e.id) ?? [];
    const encerrado = celdasE.length > 0 && celdasE.every((k) => region.has(k));
    if (esDeAfueras(e.tipo)) {
      if (encerrado) afuerasDentro.push(e.id);
    } else if (encerrado) dentro.push(e.id);
    else fuera.push(e.id);
  }

  return {
    celdas: ordenadas,
    puertas: ordenadas.filter((c) => c.clase === 'puerta').length,
    torres: ordenadas.filter((c) => c.clase === 'torre').length,
    areaEncerrada: region.size,
    dentro,
    fuera,
    afuerasDentro,
    costo: costoDeTrazo(ordenadas, nivel),
  };
}

/** ¿El anillo separa de verdad el Centro Urbano del exterior? Flood fill con vecindad 4 desde el borde de la
 * caja, sin cruzar celdas de muro ni de torre — las PUERTAS sí se cruzan, son transitables (§5). */
function aislaElCentro(muro: Set<number>, celdasCentro: number[], caja: Caja): boolean {
  const visitadas = new Set<number>();
  const pila: number[] = [];
  const objetivo = new Set(celdasCentro);
  const empujar = (col: number, row: number) => {
    if (col < caja.minCol || col > caja.maxCol || row < caja.minRow || row > caja.maxRow) return;
    const k = clave(col, row);
    if (muro.has(k) || visitadas.has(k)) return;
    visitadas.add(k);
    pila.push(k);
  };
  for (let col = caja.minCol; col <= caja.maxCol; col++) {
    empujar(col, caja.minRow);
    empujar(col, caja.maxRow);
  }
  for (let row = caja.minRow; row <= caja.maxRow; row++) {
    empujar(caja.minCol, row);
    empujar(caja.maxCol, row);
  }
  while (pila.length > 0) {
    const k = pila.pop()!;
    if (objetivo.has(k)) return false;
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) empujar(col + dc, row + dr);
  }
  return true;
}

/**
 * **Puerta**: toda celda del anillo que pise una calle o un camino que ya existía. No es una recompensa de
 * nivel, es una obligación geométrica (§5): si el muro cortara un corredor sin dejar puerta, la red dejaría de
 * ser un único componente conexo y las granjas quedarían incomunicadas. Así la red no pierde ni una celda.
 */
function celdasDeRed(red: RedDeCalles): Set<number> {
  const enRed = new Set<number>();
  for (const conjunto of [red.calles, red.caminos]) {
    for (const c of conjunto) {
      const coma = c.indexOf(',');
      enRed.add(clave(Number(c.slice(0, coma)), Number(c.slice(coma + 1))));
    }
  }
  return enRed;
}

function clasificarPuertas(muro: Set<number>, enRed: Set<number>): CeldaMuro[] {
  const celdas: CeldaMuro[] = [];
  for (const k of muro) {
    celdas.push({ col: columnaDe(k), row: filaDe(k), clase: enRed.has(k) ? 'puerta' : 'muro' });
  }
  return celdas;
}

/**
 * **Una puerta es UNA celda por cruce.** Marcar como puerta toda celda del anillo que pise la red daba tiradas
 * de 4 y 5 celdas seguidas —detectado jugando con el laboratorio—, porque un camino no siempre CRUZA el muro:
 * a menudo corre pegado a él un tramo antes de salir, y todo ese tramo se pintaba como un portón enorme.
 *
 * El daño no era solo visual: **el nº de puertas es el divisor del bono defensivo** (§16), así que un camino
 * que rozaba el muro cinco celdas dividía la defensa de la ciudad por cinco.
 *
 * Se recorre el anillo (CIRCULARMENTE — el recorrido empieza en una puerta, así que las tiradas parten el
 * final con el principio) y de cada tirada contigua se conserva una sola celda: la que de verdad ATRAVIESA,
 * es decir la que tiene red a un lado dentro del recinto y red al otro lado fuera. Si ninguna lo cumple —el
 * camino solo lamía el muro— se queda la del medio. El resto vuelve a ser muro.
 */
function colapsarPuertas(anillo: CeldaMuro[], muro: Set<number>, region: Set<number>, enRed: Set<number>): CeldaMuro[] {
  const n = anillo.length;
  if (n === 0) return anillo;

  const atraviesa = (c: CeldaMuro): boolean => {
    let dentro = false;
    let fuera = false;
    for (const [dc, dr] of VECINAS_4) {
      const k = clave(c.col + dc, c.row + dr);
      if (!enRed.has(k)) continue;
      if (region.has(k)) dentro = true;
      else if (!muro.has(k)) fuera = true;
    }
    return dentro && fuera;
  };

  const resultado = anillo.map((c) => ({ ...c }));
  const esPuerta = (i: number) => resultado[((i % n) + n) % n]!.clase === 'puerta';
  const en = (i: number) => resultado[((i % n) + n) % n]!;

  const yaVisto = new Array<boolean>(n).fill(false);
  for (let i = 0; i < n; i++) {
    if (!esPuerta(i) || yaVisto[i]) continue;
    // Retroceder hasta el principio real de la tirada (puede envolver el final del array).
    let inicio = i;
    while (esPuerta(inicio - 1) && ((inicio - 1 + n) % n) !== i) inicio -= 1;
    const indices: number[] = [];
    for (let j = inicio; esPuerta(j) && indices.length < n; j++) {
      const idx = ((j % n) + n) % n;
      if (yaVisto[idx]) break;
      yaVisto[idx] = true;
      indices.push(idx);
    }
    if (indices.length <= 1) continue;
    const elegido = indices.find((idx) => atraviesa(resultado[idx]!)) ?? indices[Math.floor(indices.length / 2)]!;
    for (const idx of indices) if (idx !== elegido) en(idx).clase = 'muro';
  }
  return resultado;
}

/**
 * **Torre**: a partir del nivel 2, recorriendo el anillo ya ordenado, se marca una torre en cada esquina
 * convexa **que esté al menos `pasoTorres` celdas de la anterior**, y se fuerza una si se lleva `pasoTorres`
 * celdas de tramo recto sin ninguna. Sin huella propia — una torre es una celda de muro marcada, y por eso no
 * puede chocar con nada (§6).
 *
 * La separación mínima es lo que impide el resultado que dio la primera versión: la dilatación con vecindad 4
 * achaflana las esquinas, así que un giro produce VARIAS esquinas convexas seguidas y salían torres a pares y
 * a tríos. Una torre cada 8 celdas se lee como una muralla; siete torres seguidas se leen como un error.
 *
 * Una puerta nunca se convierte en torre: son la misma celda y la puerta manda (es lo que mantiene la red
 * conexa).
 */
function marcarTorres(anillo: CeldaMuro[], region: Set<number>, nivel: number): CeldaMuro[] {
  const paso = MURALLA.pasoTorres[nivel];
  if (paso === undefined) return anillo;

  const esEsquinaConvexa = (c: CeldaMuro): boolean => {
    const vertical = region.has(clave(c.col, c.row - 1)) || region.has(clave(c.col, c.row + 1));
    const horizontal = region.has(clave(c.col - 1, c.row)) || region.has(clave(c.col + 1, c.row));
    return vertical && horizontal;
  };

  let desdeLaUltima = paso; // la primera esquina que aparezca ya puede llevar torre
  return anillo.map((c) => {
    if (c.clase === 'puerta') {
      desdeLaUltima += 1;
      return c;
    }
    desdeLaUltima += 1;
    if (desdeLaUltima < paso) return c;
    if (!esEsquinaConvexa(c) && desdeLaUltima < paso * 2) return c;
    desdeLaUltima = 0;
    return { ...c, clase: 'torre' as const };
  });
}

/**
 * El anillo en orden de recorrido, empezando por la puerta de menor `(row, col)` — la "puerta principal" — y
 * siguiendo por vecindad 8 mientras se pueda.
 *
 * Es el orden en que se levantará la obra (Paso 2), y por eso importa que sea un RECORRIDO y no un orden
 * arbitrario: el laboratorio tiene que enseñar el anillo cerrándose desde una puerta, no apareciendo a manchas.
 * Si el anillo tiene un punto grueso donde el paseo se atasca, se completa con las celdas que falten ordenadas
 * de forma determinista — mejor un tramo desordenado que perder celdas.
 */
function ordenarAnillo(celdas: CeldaMuro[]): CeldaMuro[] {
  if (celdas.length === 0) return celdas;
  const porClave = new Map<number, CeldaMuro>();
  for (const c of celdas) porClave.set(clave(c.col, c.row), c);

  const comparar = (a: CeldaMuro, b: CeldaMuro) => a.row - b.row || a.col - b.col;
  const puertas = celdas.filter((c) => c.clase === 'puerta').sort(comparar);
  const inicio = puertas[0] ?? [...celdas].sort(comparar)[0]!;

  const visitadas = new Set<number>();
  const recorrido: CeldaMuro[] = [];
  let actual: CeldaMuro | undefined = inicio;
  while (actual) {
    visitadas.add(clave(actual.col, actual.row));
    recorrido.push(actual);
    let siguiente: CeldaMuro | undefined;
    for (let dr = -1; dr <= 1 && !siguiente; dr++) {
      for (let dc = -1; dc <= 1 && !siguiente; dc++) {
        if (dc === 0 && dr === 0) continue;
        const k = clave(actual.col + dc, actual.row + dr);
        if (!visitadas.has(k)) siguiente = porClave.get(k);
      }
    }
    actual = siguiente;
  }
  if (recorrido.length < celdas.length) {
    for (const c of [...celdas].sort(comparar)) if (!visitadas.has(clave(c.col, c.row))) recorrido.push(c);
  }
  return recorrido;
}

/** Coste de levantar el anillo entero al nivel dado: `Σ tarifa(clase, nivel)`, ni una celda gratis. */
export function costoDeTrazo(celdas: CeldaMuro[], nivel: number): Partial<Record<string, number>> {
  const tarifa = MURALLA.tarifaPorCelda[nivel] ?? {};
  const total: Partial<Record<string, number>> = {};
  for (const celda of celdas) {
    const factor = celda.clase === 'puerta' ? MURALLA.factorPuerta : celda.clase === 'torre' ? MURALLA.factorTorre : 1;
    for (const [recurso, cantidad] of Object.entries(tarifa)) {
      total[recurso] = (total[recurso] ?? 0) + (cantidad ?? 0) * factor;
    }
  }
  for (const recurso of Object.keys(total)) total[recurso] = Math.round(total[recurso]!);
  return total;
}

// `TrazadoMuralla` vive en `trazado.ts` junto al resto del contrato de dibujo: es lo mismo que consume el
// cliente, tanto si viene de un recinto real como de un presupuesto todavía sin comprometer.
export type { TrazadoMuralla };

/** Convierte un trazo en el contrato de dibujo. Lo usan igual el presupuesto del laboratorio (recinto aún no
 * comprometido, `integridad` 0) y —desde el Paso 2— un recinto real. */
export function trazadoDeRecinto(trazo: TrazoRecinto, nivel: number, integridad = 0): TrazadoMuralla {
  const de = (clase: CeldaMuro['clase']) => fusionarCeldas(trazo.celdas.filter((c) => c.clase === clase));
  // `planificado` vacío a propósito: un presupuesto no ha comprometido NADA, así que no hay obra pendiente
  // que dibujar. La distinción visual importa — punteado = propuesta, contorno rojo = comprometido y sin
  // levantar, sólido = levantado.
  return { nivel, integridad, muro: de('muro'), puertas: de('puerta'), torres: de('torre'), planificado: [] };
}

// --- El recinto como entidad: comprometer, ocupar suelo, abrir puertas ---
//
// Hasta aquí todo era una CONSULTA. Lo que sigue es lo que convierte el trazo en algo que el resto del motor
// tiene que respetar, y es la diferencia entre un dibujo y una muralla: sin esto, la colocación de edificios
// sigue siendo ciega al anillo y planta casas encima de él (observado por el usuario en el laboratorio).

export class RecintoInvalidoError extends Error {}

/**
 * Compromete un recinto: congela el trazo y lo mete en el estado del asentamiento. A partir de aquí sus celdas
 * OCUPAN SUELO —todas, levantadas o no (decisión 7)— y sus puertas son huecos transitables. El motor entero
 * deja de poder ignorarlo.
 *
 * **Comprometer es gratis.** Lo que cuesta es levantarlo, y se paga celda a celda mientras la obra avanza
 * (`avanzarObraDeRecintos`). Así una muralla no vacía el almacén de golpe —sus costes están en el orden del
 * Palacio o la Maravilla— y "anillo a medio cerrar" pasa a ser un estado normal y visible en vez de un
 * artefacto.
 */
export function comprometerRecinto(
  asentamiento: Asentamiento,
  nivel: number,
  instante: Instante,
  id?: string
): Asentamiento {
  const existentes = asentamiento.recintos ?? [];
  if (existentes.some((r) => r.avance < r.celdas.length - 1)) {
    throw new RecintoInvalidoError('Ya hay un recinto en obra: hay que terminarlo o abandonarlo antes de trazar otro.');
  }
  // Gate de AMPLIACIÓN (§10): al menos `arrabalMinimo` edificios urbanos fuera del recinto exterior actual.
  // Sin esto ampliar sería spam y el muro dejaría de significar una decisión. "El recinto exterior actual
  // tiene que estar completo" (la otra mitad del gate de §10) ya la exige, para CUALQUIER recinto, el chequeo
  // de arriba (`existentes.some(...)`) — este de aquí solo añade lo que ese no cubre.
  if (existentes.length > 0) {
    const anterior = existentes[existentes.length - 1]!;
    const extramuros = edificiosExtramurosDe(asentamiento, anterior);
    if (extramuros < MURALLA.arrabalMinimo) {
      throw new RecintoInvalidoError(
        `Ampliar exige al menos ${MURALLA.arrabalMinimo} edificios extramuros (hay ${extramuros}).`
      );
    }
  }
  const trazo = trazarRecinto(asentamiento, { nivel });
  if (!trazo) throw new RecintoInvalidoError('No hay trazo válido: el anillo no se puede cerrar en esta ciudad.');
  if (existentes.length > 0 && !contieneA(trazo, existentes[existentes.length - 1]!)) {
    throw new RecintoInvalidoError('Una ampliación tiene que contener por completo al recinto anterior.');
  }
  const recinto: Recinto = {
    id: id ?? `recinto-${asentamiento.id}-${existentes.length + 1}`,
    nivel,
    celdas: trazo.celdas,
    avance: -1,
    comprometidoEn: instante,
  };
  return { ...asentamiento, recintos: [...existentes, recinto] };
}

/** ¿Hay alguien con ese cargo asignado? Copia deliberada de la misma comprobación de `engine/construction.ts`
 * (`cargoOcupado`, no exportada) en vez de importarla: `construction.ts` ya importa `avanzarObraDeRecintos` de
 * ESTE módulo, así que el import inverso crearía un ciclo. Dos líneas duplicadas salen más baratas que un
 * ciclo de módulos. */
function cargoOcupado(asentamiento: Asentamiento, cargo: 'gobernador' | 'maestroObras'): string | null {
  return cargo === 'gobernador' ? asentamiento.cargos.gobernadorId : asentamiento.cargos.maestroObrasId;
}

/**
 * Compromete un recinto por decisión MANUAL de Gobernador o Maestro de Obras (§8 del doc, mismo camino que
 * `anadirEdificioManualmente` con Barracón/Galería/Mercado): valida cargo y nivel de asentamiento ANTES de
 * trazar, y delega en `comprometerRecinto` para el resto. Es la única puerta de entrada de la mecánica a una
 * partida real — el laboratorio sigue llamando a `comprometerRecinto` directo porque sus asentamientos de
 * prueba no siempre tienen cargos asignados, y no debería necesitarlos para probar geometría.
 */
export function comprometerRecintoManualmente(
  asentamiento: Asentamiento,
  cargo: 'gobernador' | 'maestroObras',
  nivel: number,
  instante: Instante,
  id?: string
): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new RecintoInvalidoError(`Se necesita un ${cargo} asignado para trazar un recinto.`);
  }
  const nivelOperativo = nivelActualDe(asentamiento);
  if (nivelOperativo < MURALLA.nivelMinimoConstruccion) {
    throw new RecintoInvalidoError(`Requiere nivel de asentamiento ${MURALLA.nivelMinimoConstruccion} (actual: ${nivelOperativo}).`);
  }
  return comprometerRecinto(asentamiento, nivel, instante, id);
}

/**
 * Abandona un recinto por decisión MANUAL — solo del Gobernador (§8 del doc: a diferencia de comprometer, que
 * también admite al Maestro de Obras, abandonar es una decisión de gobierno, no de obra). Delega en
 * `abandonarRecinto` para las reglas de fondo (incompleto, sin devolución).
 */
export function abandonarRecintoManualmente(asentamiento: Asentamiento, recintoId: string): Asentamiento {
  if (!cargoOcupado(asentamiento, 'gobernador')) {
    throw new RecintoInvalidoError('Se necesita un gobernador asignado para abandonar un recinto.');
  }
  return abandonarRecinto(asentamiento, recintoId);
}

/**
 * Empieza a mejorar un recinto de nivel (§7): reinicia `avance` a -1 y marca `mejorandoA`, así que la MISMA
 * obra progresiva de `avanzarObraDeRecintos` recorre otra vez el anillo entero desde la puerta principal,
 * pagando esta vez la tarifa de MEJORA de cada celda (`MURALLA.tarifaPorCelda[mejorandoA]`). Gratis empezarla
 * — igual que comprometer — porque lo que cuesta es la obra, no la decisión.
 *
 * **No se puede mejorar un recinto incompleto**: primero se cierra el anillo, después se sube de nivel — un
 * muro medio empalizada y medio piedra no significa nada y multiplicaría los estados a probar.
 */
export function iniciarMejoraDeRecinto(asentamiento: Asentamiento, recintoId: string): Asentamiento {
  const recintos = asentamiento.recintos ?? [];
  const recinto = recintos.find((r) => r.id === recintoId);
  if (!recinto) throw new RecintoInvalidoError(`No existe el recinto ${recintoId}.`);
  if (recinto.avance < recinto.celdas.length - 1) {
    throw new RecintoInvalidoError('No se puede mejorar un recinto incompleto: hay que cerrar el anillo primero.');
  }
  if (recinto.mejorandoA !== undefined) {
    throw new RecintoInvalidoError('Ya hay una mejora en curso para este recinto.');
  }
  if (recinto.nivel >= MURALLA.nivelMaximo) {
    throw new RecintoInvalidoError(`Este recinto ya está en su nivel máximo (${MURALLA.nivelMaximo}).`);
  }
  const actualizado: Recinto = { ...recinto, mejorandoA: recinto.nivel + 1, avance: -1 };
  return { ...asentamiento, recintos: recintos.map((r) => (r.id === recintoId ? actualizado : r)) };
}

/** Versión MANUAL de `iniciarMejoraDeRecinto` (§8: mismo camino que comprometer — Gobernador o Maestro de
 * Obras, es una decisión de obra, no de gobierno). */
export function iniciarMejoraDeRecintoManualmente(
  asentamiento: Asentamiento,
  cargo: 'gobernador' | 'maestroObras',
  recintoId: string
): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new RecintoInvalidoError(`Se necesita un ${cargo} asignado para mejorar un recinto.`);
  }
  return iniciarMejoraDeRecinto(asentamiento, recintoId);
}

/** ¿El trazo nuevo envuelve al recinto anterior? Invariante de §10: **el área encerrada nunca decrece**. Se
 * comprueba con el mismo flood fill que valida el cierre — si desde fuera del anillo nuevo se llega a una
 * celda del viejo sin cruzarlo, es que el viejo se quedó fuera. */
function contieneA(trazo: TrazoRecinto, anterior: Recinto): boolean {
  const muro = new Set(trazo.celdas.map((c) => clave(c.col, c.row)));
  const objetivo = new Set(anterior.celdas.map((c) => clave(c.col, c.row)));
  // ESTRICTAMENTE dentro: si el anillo nuevo comparte una sola celda con el viejo, no lo está envolviendo —
  // lo está calcando. Sin esta línea, ampliar sobre una ciudad que no ha crecido devolvía el mismo trazo y se
  // aceptaba como "ampliación", duplicando el recinto encima de sí mismo.
  for (const k of objetivo) if (muro.has(k)) return false;
  const caja = cajaDe(muro, 2);
  const visitadas = new Set<number>();
  const pila: number[] = [];
  const empujar = (col: number, row: number) => {
    if (col < caja.minCol || col > caja.maxCol || row < caja.minRow || row > caja.maxRow) return;
    const k = clave(col, row);
    if (muro.has(k) || visitadas.has(k)) return;
    visitadas.add(k);
    pila.push(k);
  };
  for (let col = caja.minCol; col <= caja.maxCol; col++) {
    empujar(col, caja.minRow);
    empujar(col, caja.maxRow);
  }
  for (let row = caja.minRow; row <= caja.maxRow; row++) {
    empujar(caja.minCol, row);
    empujar(caja.maxCol, row);
  }
  while (pila.length > 0) {
    const k = pila.pop()!;
    if (objetivo.has(k)) return false;
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) empujar(col + dc, row + dr);
  }
  return true;
}

// --- La obra: se levanta celda a celda, pagando sobre la marcha ---

export interface PayloadRecintoCompletado {
  recintoId: string;
  nivel: number;
  celdas: number;
  puertas: number;
}

/** Coste de UNA celda según su clase y el nivel del recinto. Una puerta es una casa-puerta y una torre una
 * torre: no cuestan lo mismo que un tramo de muro llano. */
function costoDeCelda(celda: CeldaMuro, nivel: number): Partial<Record<string, number>> {
  const tarifa = MURALLA.tarifaPorCelda[nivel] ?? {};
  const factor = celda.clase === 'puerta' ? MURALLA.factorPuerta : celda.clase === 'torre' ? MURALLA.factorTorre : 1;
  const total: Partial<Record<string, number>> = {};
  for (const [recurso, cantidad] of Object.entries(tarifa)) total[recurso] = (cantidad ?? 0) * factor;
  return total;
}

/**
 * Avanza la obra de los recintos incompletos: hasta `celdasPorMinuto` celdas por tick, EN EL ORDEN DEL
 * RECORRIDO, pagando cada una al levantarla. Sirve para las DOS obras progresivas del recinto —construir y
 * mejorar (§7)— porque son la MISMA obra: un recinto con `mejorandoA` definido paga la tarifa de mejora en
 * vez de la de construcción, pero recorre el anillo exactamente igual.
 *
 * Si no hay materiales —respetando la reserva de mantenimiento, igual que cualquier otra obra automática— la
 * obra simplemente **no avanza ese tick**. No falla, no se cancela y no acumula deuda: una muralla parada es
 * una muralla parada, y se ve, porque el anillo se queda a medio cerrar.
 *
 * No ocupa un hueco de `maximoEnConstruccionSimultanea` ni entra en la cola de edificios: es una obra pública
 * del asentamiento, y meter 100 entradas en una cola con tope 8 haría ilegibles el panel y el orden de pago.
 *
 * `edificios` es opcional (`[]` por defecto) y solo hace falta para terminar una MEJORA: al completarla hay
 * que reclasificar torres sobre el interior real de la ciudad (`regionInteriorDeRecinto`), y eso necesita el
 * Centro Urbano. Los tests que ejercitan solo construcción (sin mejora) no lo necesitan.
 */
export function avanzarObraDeRecintos(
  recintos: readonly Recinto[],
  almacen: Record<string, RecursoAlmacenado>,
  reserva: Partial<Record<RecursoTipo, number>>,
  instante: Instante,
  edificios: Edificio[] = []
): { recintos: Recinto[]; almacen: Record<string, RecursoAlmacenado>; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  let almacenActual = almacen;
  const resultado = recintos.map((recinto) => {
    if (recinto.avance >= recinto.celdas.length - 1) return recinto;
    const nivelDePago = recinto.mejorandoA ?? recinto.nivel;
    let avance = recinto.avance;
    for (let i = 0; i < MURALLA.celdasPorMinuto && avance < recinto.celdas.length - 1; i++) {
      const costo = costoDeCelda(recinto.celdas[avance + 1]!, nivelDePago);
      if (!tieneRecursos(almacenActual, costo) || !respetaReserva(almacenActual, costo, reserva)) break;
      almacenActual = descontarRecursos(almacenActual, costo);
      avance += 1;
    }
    if (avance === recinto.avance) return recinto;
    const completo = avance >= recinto.celdas.length - 1;
    if (!completo) return { ...recinto, avance };

    if (recinto.mejorandoA !== undefined) {
      // Mejora terminada: sube de nivel y reclasifica torres SOBRE EL MISMO ANILLO (§6: "no se retraza
      // nada" — las esquinas y tramos no cambian, solo cuántas de sus celdas de muro pasan a ser torre).
      const nivelNuevo = recinto.mejorandoA;
      const region = regionInteriorDeRecinto(edificios, recinto);
      const celdasFinal = region ? marcarTorres(recinto.celdas, region, nivelNuevo) : recinto.celdas;
      const puertas = celdasFinal.filter((c) => c.clase === 'puerta').length;
      eventos.push({
        codigo: 'construccion.recinto_mejorado',
        mensaje: `Muralla mejorada a nivel ${nivelNuevo}: ${celdasFinal.length} celdas y ${puertas} puertas.`,
        payload: { recintoId: recinto.id, nivel: nivelNuevo, celdas: celdasFinal.length, puertas } satisfies PayloadRecintoCompletado,
      });
      const { mejorandoA: _mejorandoA, ...sinMejora } = recinto;
      return { ...sinMejora, celdas: celdasFinal, nivel: nivelNuevo, avance };
    }

    const puertas = recinto.celdas.filter((c) => c.clase === 'puerta').length;
    eventos.push({
      codigo: 'construccion.recinto_completado',
      mensaje: `Muralla cerrada: ${recinto.celdas.length} celdas y ${puertas} puertas (nivel ${recinto.nivel}).`,
      payload: { recintoId: recinto.id, nivel: recinto.nivel, celdas: recinto.celdas.length, puertas } satisfies PayloadRecintoCompletado,
    });
    return { ...recinto, avance, completadoEn: instante };
  });
  return { recintos: resultado, almacen: almacenActual, eventos };
}

/** La misma regla que `puedeIniciarConstruccion` aplica a la cola: la obra nunca puede comerse la reserva que
 * Mantenimiento y la comida van a cobrar en los próximos ticks. Sin excepciones por recurso — a diferencia de
 * la Granja con el trigo, la muralla no es la vía para recuperar nada. */
function respetaReserva(
  almacen: Record<string, RecursoAlmacenado>,
  costo: Partial<Record<string, number>>,
  reserva: Partial<Record<RecursoTipo, number>>
): boolean {
  return Object.entries(costo).every(([recurso, cantidad]) => {
    const reservaRecurso = reserva[recurso as RecursoTipo] ?? 0;
    if (reservaRecurso <= 0) return true;
    return (almacen[recurso]?.cantidad ?? 0) - (cantidad ?? 0) >= reservaRecurso;
  });
}

/**
 * Abandona un recinto INCOMPLETO: se borra entero —celdas ya levantadas incluidas— y su suelo queda libre,
 * **sin devolución de materiales**.
 *
 * Existe como escotilla de seguridad, no como mecánica de juego: sin ella, una obra atascada por falta de
 * piedra bloquearía su franja de suelo para siempre y podría dejar sin sitio a una ciudad entera. Un recinto
 * COMPLETO no se puede abandonar — en Fase 0 no existe destrucción de nada construido.
 */
export function abandonarRecinto(asentamiento: Asentamiento, recintoId: string): Asentamiento {
  const recinto = (asentamiento.recintos ?? []).find((r) => r.id === recintoId);
  if (!recinto) throw new RecintoInvalidoError(`No existe el recinto ${recintoId}.`);
  if (recinto.avance >= recinto.celdas.length - 1) {
    throw new RecintoInvalidoError('Un recinto terminado no se puede abandonar.');
  }
  return { ...asentamiento, recintos: (asentamiento.recintos ?? []).filter((r) => r.id !== recintoId) };
}

/**
 * El interior de un recinto YA CONGELADO — misma definición que `region` en `trazarRecinto` (§4), pero
 * reconstruida a partir de las celdas ya fijadas del anillo en vez de recalculada del trazado actual de la
 * ciudad. Dos usos: medir el área encerrada (`areaEncerradaDeRecinto`) y reclasificar torres al mejorar de
 * nivel (`completarMejora`, más abajo — necesita saber qué lado de cada celda de muro es "adentro" para
 * `esEsquinaConvexa`, igual que en el trazado original).
 *
 * Inundación desde una celda del Centro Urbano (que por construcción siempre cae dentro), bloqueada por TODAS
 * las celdas del anillo — incluidas las puertas. A diferencia de `redDeCalles`, donde una puerta es un hueco
 * por el que la red puede cruzar, aquí bloquea a propósito: `region` (lo que calcula `trazarRecinto`) es el
 * tejido urbano de ANTES de dilatar el muro hacia afuera, así que nunca incluye ninguna celda del anillo.
 * Dejar la puerta abierta aquí fugaría la inundación hacia el exterior del anillo entero (medido: 1440 contra
 * las 799 esperadas en una ciudad de fixture) — la puerta es un hueco para quien camina, no un agujero en la
 * CONTABILIDAD del interior. `null` si el asentamiento ya no tiene Centro Urbano (no debería pasar nunca; ver
 * `trazarRecinto`).
 */
function regionInteriorDeRecinto(edificios: Edificio[], recinto: Recinto): Set<number> | null {
  const centro = edificiosInternos(edificios).find((e) => e.tipo === 'centroUrbano');
  if (!centro) return null;
  const semilla = celdasDeEdificio(centro)[0];
  if (!semilla) return null;

  const muro = new Set<number>(recinto.celdas.map((c) => clave(c.col, c.row)));
  const caja = cajaDe(muro, 1);

  const vistas = new Set<number>([clave(semilla.col, semilla.row)]);
  const pila: number[] = [clave(semilla.col, semilla.row)];
  while (pila.length > 0) {
    const k = pila.pop()!;
    const col = columnaDe(k);
    const row = filaDe(k);
    for (const [dc, dr] of VECINAS_4) {
      const nCol = col + dc;
      const nRow = row + dr;
      if (nCol < caja.minCol || nCol > caja.maxCol || nRow < caja.minRow || nRow > caja.maxRow) continue;
      const nk = clave(nCol, nRow);
      if (muro.has(nk) || vistas.has(nk)) continue;
      vistas.add(nk);
      pila.push(nk);
    }
  }
  return vistas;
}

/**
 * Área que un recinto encierra, en celdas — misma definición que `TrazoRecinto.areaEncerrada` (§4). Da el
 * MISMO número en cualquier tick posterior al compromiso, porque el anillo no cambia jamás (§5.1) — por eso
 * sirve para la medición en batch del eje fortaleza↔metrópoli (Paso 2c, §18): el área de un recinto ya
 * cerrado es un hecho fijo, no algo que haya que rastrear en el momento en que se comprometió.
 */
export function areaEncerradaDeRecinto(asentamiento: Asentamiento, recinto: Recinto): number | null {
  return regionInteriorDeRecinto(asentamiento.edificios, recinto)?.size ?? null;
}

/**
 * Edificios URBANOS (no Granja/Corral, que están fuera del recinto por diseño desde el Paso 0 y no cuentan
 * como arrabal) que quedan FUERA del interior de `recinto` — el "extramuros" del gate de ampliación (§10).
 */
function edificiosExtramurosDe(asentamiento: Asentamiento, recinto: Recinto): number {
  const region = regionInteriorDeRecinto(asentamiento.edificios, recinto);
  if (!region) return 0;
  let cuenta = 0;
  for (const e of edificiosInternos(asentamiento.edificios)) {
    if (esDeAfueras(e.tipo)) continue;
    const dentro = celdasDeEdificio(e).every((c) => region.has(clave(c.col, c.row)));
    if (!dentro) cuenta++;
  }
  return cuenta;
}

// --- Paso 3b: defensa y upkeep — la razón de ser (§0, §16) ---

/**
 * Multiplicador defensivo del recinto EXTERIOR (el último de la lista — el que de verdad hay que atravesar
 * para llegar al Centro Urbano; los recintos interiores de una ampliación ya no defienden nada por sí solos,
 * son historia en piedra, §10). `1` (neutro) si no hay ningún recinto: un asentamiento sin muralla combate
 * exactamente como hoy.
 *
 * `1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas` (§16) — el "1 +" garantiza que un muro
 * NUNCA perjudique al defensor, `integridad` hace que un anillo a medio cerrar casi no defienda (se entra por
 * el hueco de obra), y dividir por el nº de puertas es lo que hace real el eje fortaleza↔metrópoli (§0):
 * amurallar pronto da un embudo barato con pocas puertas, amurallar tarde protege más ciudad repartiendo la
 * ventaja entre más frentes.
 */
export function multiplicadorDefensivoDeRecintos(recintos: readonly Recinto[]): number {
  const recinto = recintos[recintos.length - 1];
  if (!recinto) return 1;
  const bono = MURALLA.bonoDefensaPorNivel[recinto.nivel];
  if (bono === undefined) return 1;
  const puertas = recinto.celdas.filter((c) => c.clase === 'puerta').length;
  if (puertas === 0) return 1; // no debería pasar nunca (§4 fuerza al menos una), pero /0 sería peor que nada
  return 1 + (bono - 1) * integridadDeRecinto(recinto) / puertas;
}

/**
 * Upkeep de TODOS los recintos del asentamiento, por tick — la mitad de "difícil de obtener **y de
 * mantener**" (§0). Solo cuenta celdas YA LEVANTADAS (`avance + 1`): lo que todavía no existe no cuesta
 * mantenerlo. Usa el `nivel` ACTUAL del recinto (nunca `mejorandoA`): mientras una mejora está en obra, lo que
 * hay en pie sigue siendo el nivel viejo, y eso es lo que hay que mantener hasta que la mejora termine.
 *
 * Deliberadamente SIN el factor de población/distancia de `calcularCostoMantenimiento` — el muro no cuesta
 * más porque la ciudad crezca, cuesta lo que pesan sus propias celdas.
 */
export function upkeepDeRecintos(recintos: readonly Recinto[]): Partial<Record<string, number>> {
  const total: Partial<Record<string, number>> = {};
  for (const recinto of recintos) {
    const celdasLevantadas = recinto.avance + 1;
    if (celdasLevantadas <= 0) continue;
    const tarifa = MURALLA.upkeepPorCelda[recinto.nivel] ?? {};
    for (const [recurso, cantidad] of Object.entries(tarifa)) {
      total[recurso] = (total[recurso] ?? 0) + (cantidad ?? 0) * celdasLevantadas;
    }
  }
  return total;
}
