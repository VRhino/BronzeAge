// Test de arquitectura: congela la dirección de dependencias entre capas descrita en
// Docs/Arquitectura/1_Arquitectura_Actual.md ("Vista global") — falla si algún import nuevo apunta "hacia
// arriba" (motor -> app/ui, dominio -> motor, etc.). Es la verificación de A2
// (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md) convertida en test permanente: hasta ahora esa frontera se
// revisaba a mano (grep) una vez por tarea, y nada impedía que un import posterior la rompiera en silencio.
//
// Importa especialmente al empezar la Fase B (Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md): la
// prueba de que `engine/`/`world/`/`worldgen/`/`domain/` siguen siendo el juego reutilizable sin servidor es
// que `scripts/run-batch-sim.ts` corre sin tocar `app/`/`ui/` — este test es la versión automática de esa
// misma garantía, para que "backend" y "lógica de juego" no se puedan enredar sin que algo lo señale.
//
// Usa `import.meta.glob` (Vite) en vez de `node:fs` a propósito: este es un proyecto 100% navegador/Vite, sin
// `@types/node` como dependencia — leer el árbol de archivos vía Vite evita introducir una dependencia nueva
// solo para este test.
import { describe, expect, it } from 'vitest';

/** Todo el código fuente de producción como texto plano, indexado por ruta absoluta-desde-raíz
 * (`/src/engine/population.ts`). `eager: true` los carga todos de una vez (son ~150, nada pesado). */
const ARCHIVOS_FUENTE = import.meta.glob('/src/**/*.ts', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

/**
 * Capas hacia las que cada capa puede importar, además de sí misma (siempre permitido, no hace falta
 * listarlo). `main` es la raíz de composición (`src/main.ts`): la única capa que puede importar de `app` y
 * `ui` a la vez. `lab` es la herramienta de desarrollo de `laboratorio.html` (Docs/Arquitectura/
 * 1_Arquitectura_Actual.md no la describe como parte de la partida real) — consume el motor directamente,
 * sin pasar por `app`.
 */
const CAPAS_PERMITIDAS: Record<string, string[]> = {
  domain: [],
  constants: ['domain'],
  worldgen: ['domain', 'constants'],
  world: ['domain', 'worldgen', 'constants'],
  engine: ['domain', 'worldgen', 'world', 'constants'],
  ui: ['domain', 'world', 'constants'],
  app: ['domain', 'worldgen', 'world', 'engine', 'constants'],
  lab: ['domain', 'worldgen', 'world', 'engine', 'ui', 'constants'],
  main: ['domain', 'worldgen', 'world', 'engine', 'app', 'ui', 'constants'],
};

/** Capa de una ruta absoluta-desde-raíz (`/src/engine/population.ts` -> `'engine'`, `/src/constants.ts` ->
 * `'constants'`): su primer segmento tras `/src/`, sin extensión. */
function capaDe(rutaDesdeRaiz: string): string {
  const primerSegmento = rutaDesdeRaiz.replace(/^\/src\//, '').split('/')[0]!;
  return primerSegmento.endsWith('.ts') ? primerSegmento.slice(0, -3) : primerSegmento;
}

/** Resuelve un especificador de import relativo (`'../world/mapa'`, `'./asentamientoQuery'`) contra la ruta
 * absoluta-desde-raíz del archivo que importa, sin `node:path` — solo manipulación de segmentos. */
function resolverEspecificador(rutaDesdeArchivo: string, especificador: string): string {
  const partes = rutaDesdeArchivo.split('/').slice(0, -1); // directorio del archivo, sin su propio nombre
  for (const segmento of especificador.split('/')) {
    if (segmento === '.' || segmento === '') continue;
    if (segmento === '..') partes.pop();
    else partes.push(segmento);
  }
  return partes.join('/');
}

/** Especificadores de import relativos (`./`, `../`) de un archivo — estáticos y dinámicos, cubre tanto
 * `import` como `export ... from`. Los paquetes de `node_modules` no forman parte de esta frontera. */
function importsRelativos(contenido: string): string[] {
  const specs: string[] = [];
  for (const m of contenido.matchAll(/\bfrom\s+['"]([^'"]+)['"]/g)) specs.push(m[1]!);
  for (const m of contenido.matchAll(/\bimport\(\s*['"]([^'"]+)['"]\s*\)/g)) specs.push(m[1]!);
  return specs.filter((s) => s.startsWith('.'));
}

/** Archivos de producción (excluye `__tests__` y `*.test.ts`: los tests tienen sus propias conveniencias —
 * fixtures compartidas, etc. — y no son parte del contrato de arquitectura) de una capa dada. */
function archivosDeCapa(capa: string): [ruta: string, contenido: string][] {
  const prefijo = capa === 'constants' || capa === 'main' ? `/src/${capa}.ts` : `/src/${capa}/`;
  return Object.entries(ARCHIVOS_FUENTE).filter(
    ([ruta]) => (ruta === prefijo || ruta.startsWith(prefijo)) && !ruta.includes('/__tests__/') && !ruta.endsWith('.test.ts')
  );
}

describe('fronteras de arquitectura entre capas', () => {
  it('el glob de Vite encuentra archivos fuente reales (si esto falla, el resto del test no prueba nada)', () => {
    expect(Object.keys(ARCHIVOS_FUENTE).length).toBeGreaterThan(50);
    expect(archivosDeCapa('engine').length).toBeGreaterThan(10);
  });

  it('ninguna capa importa de una capa que no tiene permitida explícitamente', () => {
    const violaciones: string[] = [];

    for (const [capa, permitidas] of Object.entries(CAPAS_PERMITIDAS)) {
      for (const [ruta, contenido] of archivosDeCapa(capa)) {
        for (const spec of importsRelativos(contenido)) {
          const capaDestino = capaDe(resolverEspecificador(ruta, spec));
          if (capaDestino === capa) continue; // importar dentro de la propia capa siempre es válido
          if (!permitidas.includes(capaDestino)) {
            violaciones.push(`${ruta} (capa '${capa}') importa de '${spec}' (capa '${capaDestino}', no permitida)`);
          }
        }
      }
    }

    expect(violaciones, `\n${violaciones.join('\n')}`).toEqual([]);
  });

  it('el motor (engine/world/worldgen/domain) sigue corriendo sin `app`/`ui` — invariante mínima para batch/servidor', () => {
    // Espejo explícito, en lenguaje de negocio, de lo que la regla genérica de arriba ya comprueba en
    // detalle: si esto falla, `scripts/run-batch-sim.ts` (que solo importa engine/world/worldgen/domain,
    // igual que hará el futuro backend) dejaría de poder correr sin `app`/`ui`.
    for (const capaMotor of ['engine', 'world', 'worldgen', 'domain']) {
      for (const [ruta, contenido] of archivosDeCapa(capaMotor)) {
        for (const spec of importsRelativos(contenido)) {
          const capaDestino = capaDe(resolverEspecificador(ruta, spec));
          expect(['app', 'ui', 'main', 'lab'], `${ruta} importa de '${spec}'`).not.toContain(capaDestino);
        }
      }
    }
  });
});
