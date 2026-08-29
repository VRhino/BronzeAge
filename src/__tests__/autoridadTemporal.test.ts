// Guard de AUTORIDAD TEMPORAL Y ALEATORIA (Docs/Arquitectura/10_Modelo_Temporal.md).
//
// Por qué existe: el `arquitectura.test.ts` de al lado congela la DIRECCIÓN de los imports entre capas, pero
// no que cada capa CONTENGA lo que le toca. Por ese hueco, `ctx.momento` se pobló con reloj de pared
// (`new Date()`) desde `server/` y viajó al estado persistido de la partida durante meses sin que ningún test
// lo viera: el de determinismo del motor usa un `momento` derivado del tick, no el del servidor real.
//
// Este guard cierra la clase entera de fallo en el borde: el NÚCLEO PURO (domain/constants/worldgen/world/
// engine) no nombra `Date` ni consume aleatoriedad ambiental; `session` no lee el reloj (recibe `momento`).
// El tiempo y la aleatoriedad ENTRAN como parámetro (`ContextoSimulacion`, `ContextoComando`) y son
// responsabilidad de `server/` — que sí puede leer `Date.now()`, ver `RunnerDePartida.ahora`.
//
// `server/` y `acceso/` quedan FUERA a propósito: `server` es el dueño del reloj de pared (nombres de
// snapshot, TTL de cachés, cálculo del catch-up de ticks) y `acceso` genera tokens de sesión, que no son
// estado de simulación y deben ser impredecibles.
import { describe, expect, it } from 'vitest';
import { archivosDeCapa, sinComentarios } from './fuenteDelProyecto';

/** Capas del núcleo puro de simulación: su salida debe ser función determinista de (seed, estado, entradas). */
const NUCLEO_PURO = ['domain', 'constants', 'worldgen', 'world', 'engine'] as const;

interface Prohibicion {
  patron: RegExp;
  motivo: string;
}

/** Fuentes de no-determinismo prohibidas en el núcleo puro. El motor recibe su aleatoriedad como
 * `RandomFn` inyectada (`ContextoSimulacion.rng`) y su tiempo como `string`/`number` (`momento`). */
const PROHIBIDO_EN_NUCLEO: Prohibicion[] = [
  { patron: /\bDate\b/, motivo: 'el núcleo puro no nombra `Date`: el tiempo entra como ISO 8601 (`momento`) o ms, y se compara lexicográficamente' },
  { patron: /\bMath\s*\.\s*random\b/, motivo: 'usa la `RandomFn` inyectada (`ContextoSimulacion.rng`), nunca la aleatoriedad global' },
  { patron: /\bperformance\s*\.\s*now\b/, motivo: 'reloj de alta resolución: mismo problema que `Date.now()`' },
  { patron: /\bcrypto\s*\.\s*(randomUUID|getRandomValues|randomBytes|randomInt)\b/, motivo: 'aleatoriedad no reproducible: los ids salen de `GeneradorIds` (contador)' },
];

/** `session` es la capa de aplicación de partida: recibe `ContextoComando.momento` ya resuelto. Puede
 * PARSEAR un ISO recibido (`new Date(ctx.momento)`) —hasta que D2 introduzca `Instante` como número y esto
 * también sobre—, pero nunca LEER el reloj. */
const PROHIBIDO_EN_SESSION: Prohibicion[] = [
  { patron: /\bnew\s+Date\s*\(\s*\)/, motivo: '`new Date()` sin argumento lee el reloj de pared; `session` recibe `ctx.momento`' },
  { patron: /\bDate\s*\.\s*now\b/, motivo: '`Date.now()` lee el reloj de pared; `session` recibe `ctx.momento`' },
  { patron: /\bperformance\s*\.\s*now\b/, motivo: 'reloj de alta resolución' },
  { patron: /\bMath\s*\.\s*random\b/, motivo: 'aleatoriedad global: un comando que necesite azar usa `ctx.rng`' },
  { patron: /\bcrypto\s*\.\s*(randomUUID|getRandomValues|randomBytes|randomInt)\b/, motivo: 'aleatoriedad no reproducible' },
];

function violaciones(capas: readonly string[], prohibiciones: Prohibicion[]): string[] {
  const encontradas: string[] = [];
  for (const capa of capas) {
    for (const [ruta, contenido] of archivosDeCapa(capa)) {
      const lineas = sinComentarios(contenido).split('\n');
      lineas.forEach((linea, i) => {
        for (const { patron, motivo } of prohibiciones) {
          if (patron.test(linea)) encontradas.push(`${ruta}:${i + 1}  — ${motivo}\n    > ${linea.trim()}`);
        }
      });
    }
  }
  return encontradas;
}

describe('autoridad temporal y aleatoria (doc 10)', () => {
  it('la lectura del árbol encuentra los archivos del núcleo (si no, el guard no prueba nada)', () => {
    expect(archivosDeCapa('engine').length).toBeGreaterThan(10);
    expect(archivosDeCapa('session').length).toBeGreaterThan(5);
  });

  it('el núcleo puro (domain/constants/worldgen/world/engine) no nombra `Date` ni consume aleatoriedad ambiental', () => {
    const encontradas = violaciones(NUCLEO_PURO, PROHIBIDO_EN_NUCLEO);
    expect(encontradas, `\n${encontradas.join('\n')}\n`).toEqual([]);
  });

  it('`session` no lee el reloj de pared (recibe `ctx.momento`) ni usa aleatoriedad global', () => {
    const encontradas = violaciones(['session'], PROHIBIDO_EN_SESSION);
    expect(encontradas, `\n${encontradas.join('\n')}\n`).toEqual([]);
  });

  it('el guard detecta de verdad una violación inyectada (si esto no falla, los tests de arriba no valen)', () => {
    const fuenteMala = 'export const x = Date.now();\nconst y = Math.random();';
    const lineas = sinComentarios(fuenteMala).split('\n');
    const golpes = PROHIBIDO_EN_NUCLEO.filter((p) => lineas.some((l) => p.patron.test(l)));
    expect(golpes.map((g) => g.motivo)).toHaveLength(2);
  });

  it('el guard NO salta por una mención en un comentario (JSDoc o `//`)', () => {
    const fuenteConComentarios = [
      '/** Nada aquí puede consumir `Math.random()` — rompería la reproducibilidad. */',
      'export function foo() { return 1; } // ni `Date.now()` ni `new Date()`',
    ].join('\n');
    const lineas = sinComentarios(fuenteConComentarios).split('\n');
    const golpes = [...PROHIBIDO_EN_NUCLEO, ...PROHIBIDO_EN_SESSION].filter((p) => lineas.some((l) => p.patron.test(l)));
    expect(golpes).toEqual([]);
  });

  it('`session` SÍ puede parsear un ISO recibido (`new Date(ctx.momento)`) — solo se prohíbe leer el reloj', () => {
    const lineas = sinComentarios('const ms = new Date(ctx.momento).getTime();').split('\n');
    const golpes = PROHIBIDO_EN_SESSION.filter((p) => lineas.some((l) => p.patron.test(l)));
    expect(golpes).toEqual([]);
  });
});
