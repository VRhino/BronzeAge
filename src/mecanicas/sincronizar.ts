// Los bloques de parámetros de `Docs/Mechanics/` se COPIAN de `constants.ts`, no se escriben a mano (decisión del autor, 2026-10-09): una cifra
// que cambia en el código tiene que cambiar en su documento, y un test lo vigila (`__tests__/sincronizar.test.ts`) para que no se pueda olvidar.
//
// Un bloque se marca así, y lo de dentro lo reescribe `npm run mecanicas`:
//
//   <!-- parametros:MURALLA -->
//   ```json
//   ...
//   ```
//   <!-- /parametros -->
import * as constantesJuego from '../constants';
import * as constantesInfra from '../constantsInfra';
import * as configMundo from '../worldgen/config';
import { REGIONES } from '../worldgen/regiones';
import { WORLDGEN_VERSION } from '../worldgen/types';
import { DURACION_SESION_MS } from '../acceso/servicioAutenticacion';

// Las reglas del juego (`constants.ts`), las de infraestructura (`constantsInfra.ts`) y las congeladas de la generación del mundo (`worldgen/config.ts`)
// se citan igual desde los documentos (más las regiones autoradas y la versión del generador); sus nombres no chocan.
const constantes = { ...constantesJuego, ...constantesInfra, ...configMundo, REGIONES, WORLDGEN_VERSION, ACCESO: { duracionSesionMs: DURACION_SESION_MS } };

const BLOQUE = /<!-- parametros:([A-Za-z0-9_]+) -->\r?\n```json\r?\n[\s\S]*?\r?\n```\r?\n<!-- \/parametros -->/g;

/** El JSON de una constante exportada, tal como queda en el documento. `undefined` si no existe. */
export function parametrosDe(nombre: string): string | undefined {
  const valor = (constantes as Record<string, unknown>)[nombre];
  return valor === undefined ? undefined : JSON.stringify(valor, null, 2);
}

/** El texto con todos sus bloques de parámetros al día, y los nombres que no existen en `constants.ts`. */
export function sincronizar(texto: string): { texto: string; desconocidos: string[] } {
  const desconocidos: string[] = [];
  const nuevo = texto.replace(BLOQUE, (bloque, nombre: string) => {
    const json = parametrosDe(nombre);
    if (json === undefined) {
      desconocidos.push(nombre);
      return bloque;
    }
    return `<!-- parametros:${nombre} -->\n\`\`\`json\n${json}\n\`\`\`\n<!-- /parametros -->`;
  });
  return { texto: nuevo, desconocidos };
}
