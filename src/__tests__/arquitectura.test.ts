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
// La lectura del árbol de `src/` vive en `fuenteDelProyecto.ts`, compartida con `autoridadTemporal.test.ts`
// (el guard de "el núcleo puro no lee reloj ni aleatoriedad", Docs/Arquitectura/10_Modelo_Temporal.md). Antes
// se leía con `import.meta.glob` (Vite) porque el repo era "100% navegador/Vite, sin `@types/node`" — premisa
// que dejó de ser cierta al separar el cliente a su propio proyecto (Fase C).
import { describe, expect, it } from 'vitest';
import { ARCHIVOS_FUENTE, archivosDeCapa } from './fuenteDelProyecto';

/**
 * Capas hacia las que cada capa puede importar, además de sí misma (siempre permitido, no hace falta
 * listarlo).
 *
 * Desde la Fase C este repositorio es **solo servidor**: las capas de navegador (`app`, `ui`, `main`, `lab`)
 * se extrajeron a `cliente/`, que es un proyecto aparte con su propio `package.json`/`tsconfig` y está
 * pensado para inicializar su propio repositorio. Aquí ya no existen, y por eso no aparecen — pero la regla
 * que las mantenía fuera del motor sigue viva en el test de abajo ("el motor no importa del lado cliente"),
 * porque lo que protegía era que `engine`/`world`/`worldgen`/`domain` siguieran siendo juego reutilizable sin
 * servidor NI navegador.
 */
const CAPAS_PERMITIDAS: Record<string, string[]> = {
  domain: [],
  constants: ['domain'],
  worldgen: ['domain', 'constants'],
  world: ['domain', 'worldgen', 'constants'],
  engine: ['domain', 'worldgen', 'world', 'constants'],
  // `acceso` es el DOMINIO DE ACCESO (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md): `Usuario`,
  // `Sesion`, `Rol`, `Membresia` y los puertos que los sirven. Es negocio, no infraestructura — las reglas
  // de quién puede existir y con qué papel no cambian aunque se sustituyan Fastify, el proveedor de
  // identidad o la base de datos, y esos adaptadores viven en `server/`. Sin dependencias a propósito: no
  // conoce el juego (un `Usuario` existe fuera de cualquier partida) ni el transporte.
  acceso: [],
  // `contratos` es la forma del cable con Conquest (Docs/Coordinacion/02 §6): tipos, schema y fixtures, sin reglas.
  // Solo mira el dominio (los catálogos cerrados) y las constantes (el catálogo de tropas se genera de ahí).
  contratos: ['domain', 'constants'],
  // `session` es la capa de aplicación DE PARTIDA (Docs/Arquitectura/7_Diseno_GameSession.md): la partida
  // como estado + reglas, síncrona y sin E/S. No conoce HTTP ni disco — de eso se encarga `server`. Ve
  // `acceso` porque la autorización de comandos (`comandos/autorizacion.ts`) cruza ambos dominios: qué rol
  // técnico tiene el actor Y qué relación de juego guarda con la entidad objetivo. Ve `contratos` porque una
  // batalla de Unity congela y guarda su ticket con la forma del contrato (`session/batallas.ts`).
  session: ['domain', 'worldgen', 'world', 'engine', 'constants', 'acceso', 'contratos'],
  // `server` es la capa de aplicación DE PROCESO backend (Node — `fs`, HTTP, futuro WebSocket): todo lo que
  // `session` no puede tener porque es deliberadamente síncrona y sin E/S (doc 7 §2). Persistencia de
  // partida, `RunnerDePartida`, la API y los ADAPTADORES de los puertos de `acceso` (proveedor de identidad,
  // repositorio, parseo de cabeceras).
  server: ['domain', 'worldgen', 'world', 'engine', 'session', 'constants', 'acceso', 'contratos'],
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

describe('fronteras de arquitectura entre capas', () => {
  it('la lectura del arbol encuentra archivos fuente reales (si esto falla, el resto del test no prueba nada)', () => {
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

  it('el negocio no depende de la infraestructura: nada fuera de `server` importa de `server`', () => {
    // Espejo en lenguaje de negocio de la regla genérica de arriba, para la frontera que más cara sale
    // equivocar (revisión de separación negocio/infraestructura, 2026-08-25): `acceso` (dominio de acceso),
    // `session` (aplicación de partida) y el motor definen QUÉ debe pasar; `server` decide CÓMO se sirve
    // (Fastify, disco, cabeceras HTTP, qué proveedor de identidad está activo). Si esto falla, cambiar de
    // framework o de proveedor deja de ser un cambio local y empieza a arrastrar reglas de juego con él.
    const violaciones: string[] = [];
    for (const capa of ['domain', 'worldgen', 'world', 'engine', 'acceso', 'session']) {
      for (const [ruta, contenido] of archivosDeCapa(capa)) {
        for (const spec of importsRelativos(contenido)) {
          if (capaDe(resolverEspecificador(ruta, spec)) === 'server') violaciones.push(`${ruta} importa de '${spec}'`);
        }
      }
    }
    expect(violaciones, `\n${violaciones.join('\n')}`).toEqual([]);
  });

  it('`acceso` no depende de NADA: es dominio puro, sin juego ni transporte', () => {
    // Un `Usuario` existe fuera de cualquier partida, y su autenticación no sabe de HTTP. Que esta capa
    // siga sin imports es lo que permite probarla con dobles (ver `acceso/__tests__/`) y sustituir
    // proveedor o almacenamiento sin tocarla.
    for (const [ruta, contenido] of archivosDeCapa('acceso')) {
      const fuera = importsRelativos(contenido).filter((spec) => capaDe(resolverEspecificador(ruta, spec)) !== 'acceso');
      expect(fuera, `${ruta} importa ${fuera.join(', ')}`).toEqual([]);
    }
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
