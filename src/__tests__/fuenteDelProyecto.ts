// Utilidad compartida por los tests de FRONTERA de `src/__tests__/`: `arquitectura.test.ts` (dirección de
// dependencias entre capas) y `autoridadTemporal.test.ts` (el núcleo puro no lee reloj ni aleatoriedad). No
// es un `.test.ts` — es infraestructura de test, igual que `engine/__tests__/fixtures.ts`.
//
// Lee el árbol con `node:fs` (este repo es Node puro desde la Fase C — ver la cabecera de `arquitectura.test.ts`).
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const RAIZ_SRC = fileURLToPath(new URL('..', import.meta.url));

/** Todo el código fuente de `src/` como texto plano, indexado por ruta absoluta-desde-raíz
 * (`/src/engine/population.ts`). */
export function leerArbolFuente(directorio = RAIZ_SRC, prefijo = '/src'): Record<string, string> {
  const archivos: Record<string, string> = {};
  for (const entrada of readdirSync(directorio, { withFileTypes: true })) {
    const ruta = `${prefijo}/${entrada.name}`;
    if (entrada.isDirectory()) Object.assign(archivos, leerArbolFuente(`${directorio}/${entrada.name}`, ruta));
    else if (entrada.name.endsWith('.ts')) archivos[ruta] = readFileSync(`${directorio}/${entrada.name}`, 'utf-8');
  }
  return archivos;
}

export const ARCHIVOS_FUENTE = leerArbolFuente();

/** Archivos de PRODUCCIÓN de una capa (`'engine'`, `'constants'`, ...): excluye `__tests__/` y `*.test.ts`,
 * que tienen sus propias conveniencias y no forman parte del contrato de arquitectura. */
export function archivosDeCapa(capa: string): [ruta: string, contenido: string][] {
  const prefijo = capa === 'constants' ? `/src/${capa}.ts` : `/src/${capa}/`;
  return Object.entries(ARCHIVOS_FUENTE).filter(
    ([ruta]) => (ruta === prefijo || ruta.startsWith(prefijo)) && !ruta.includes('/__tests__/') && !ruta.endsWith('.test.ts')
  );
}

/**
 * Neutraliza comentarios de bloque (`/* *​/`, incluidos JSDoc) y de línea (`//`) de un fuente TypeScript
 * sustituyéndolos por espacios — se conservan los saltos de línea y las posiciones, así que `split('\n')`
 * sigue dando los números de línea reales. Sirve para que un escáner de tokens prohibidos no salte por una
 * mención en la documentación (ej. `` `Math.random()` `` en un comentario que explica justo que no se use).
 *
 * Suficiente para su propósito, no un parser: un literal de cadena con `//` en la misma línea que un token
 * prohibido daría un falso negativo. No ocurre hoy en las capas vigiladas (verificado: sin `://` en código de
 * `domain`/`constants`/`worldgen`/`world`/`engine`); si algún día ocurre lo cazan la revisión de código y el
 * propio doc 10 — el guard existe para hacer imposible el error común, no para verificar formalmente.
 */
export function sinComentarios(fuente: string): string {
  return fuente
    .replace(/\/\*[\s\S]*?\*\//g, (bloque) => bloque.replace(/[^\n]/g, ' '))
    .replace(/\/\/[^\n]*/g, (linea) => ' '.repeat(linea.length));
}
