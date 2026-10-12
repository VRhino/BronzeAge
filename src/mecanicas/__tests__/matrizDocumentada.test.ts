// `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §5 enumera la matriz de autorización comando a comando. Si se añade o se quita un comando del
// registro y el documento no se toca, este test lo dice: la regla de `Docs/Mechanics` es que cada cambio de comportamiento lleva su documento.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { REGISTRO_COMANDOS } from '../../session/comandos/registro';

const DOCUMENTO = new URL('../../../Docs/Mechanics/Proyecciones_Y_Autorizacion.md', import.meta.url);

/** El texto de la sección 5 (la matriz), sin las demás: un comando citado de pasada en otra sección no cuenta. */
function seccionDeLaMatriz(): string {
  const texto = readFileSync(DOCUMENTO, 'utf8').replace(/\r\n/g, '\n');
  const desde = texto.indexOf('\n## 5. ');
  const hasta = texto.indexOf('\n## 6. ');
  expect(desde).toBeGreaterThan(-1);
  expect(hasta).toBeGreaterThan(desde);
  return texto.slice(desde, hasta);
}

describe('la matriz de autorización documentada', () => {
  it('nombra cada comando del registro en la sección 5', () => {
    const seccion = seccionDeLaMatriz();
    const sinDocumentar = Object.keys(REGISTRO_COMANDOS).filter((tipo) => !seccion.includes(`\`${tipo}\``));
    expect(sinDocumentar).toEqual([]);
  });

  it('no nombra comandos que ya no existen (el nombre de un comando va en una celda de tabla, entre comillas invertidas, en la primera columna)', () => {
    const celdas = [...seccionDeLaMatriz().matchAll(/^\| ((?:`[A-Za-z]+`(?:, | y )?)+)/gm)].flatMap((m) => [...m[1]!.matchAll(/`([A-Za-z]+)`/g)].map((c) => c[1]!));
    // Los predicados y los roles también van entre comillas invertidas en esa columna (§5.9 lista los roles): se descartan los que no son comandos
    // solo si son palabras conocidas de la sección; cualquier otro nombre desconocido es un comando que sobra.
    const ajenos = new Set(['jugador', 'administrador_partida', 'administrador_global', 'moderador']);
    const sobran = celdas.filter((n) => !ajenos.has(n) && !(n in REGISTRO_COMANDOS));
    expect(sobran).toEqual([]);
  });
});
