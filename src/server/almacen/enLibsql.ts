// Adaptador de `AlmacenDeObjetos` sobre libSQL (SQLite en red): una tabla KV genérica `objetos(clave,
// contenido)`. Es la persistencia para cualquier host con disco efímero — el URL apunta a un Turso, a un
// sqld propio, o a un `file:`/`":memory:"` local para pruebas.
//
// El puerto encaja casi sin fricción porque el modelo ya era "blobs pequeños con clave":
//   - `escribir` = UPSERT.
//   - `anexar`   = UPSERT con `contenido || excluded.contenido` — append NATIVO, sin releer el blob (a
//     diferencia de un object storage sin append). Seguro con un solo escritor por clave, que es el caso.
//   - `listar`   = `LIKE 'prefijo%'`, con `_`/`%`/`\` del prefijo escapados.
//
// NO gestiona la concurrencia optimista de `guardarPartida`: esa sigue siendo leer→comparar→escribir en
// `persistenciaPartida.ts`, serializada por `RunnerDePartida` igual que con el disco.
import { createClient, type Config } from '@libsql/client';
import type { AlmacenDeObjetos } from './almacenDeObjetos';

const CREAR_TABLA = 'CREATE TABLE IF NOT EXISTS objetos (clave TEXT PRIMARY KEY, contenido TEXT NOT NULL)';

/** `config` se pasa tal cual a `@libsql/client` — `{ url, authToken? }`. Es `async` porque crea la tabla al
 * conectar; `server/index.ts` ya es async. */
export async function crearAlmacenEnLibsql(config: Config): Promise<AlmacenDeObjetos> {
  const cliente = createClient(config);
  await cliente.execute(CREAR_TABLA);

  return {
    async leer(clave) {
      const r = await cliente.execute({ sql: 'SELECT contenido FROM objetos WHERE clave = ?', args: [clave] });
      return r.rows.length === 0 ? null : String(r.rows[0]!.contenido);
    },

    async escribir(clave, contenido) {
      await cliente.execute({
        sql: 'INSERT INTO objetos (clave, contenido) VALUES (?, ?) ON CONFLICT(clave) DO UPDATE SET contenido = excluded.contenido',
        args: [clave, contenido],
      });
    },

    async anexar(clave, contenido) {
      await cliente.execute({
        sql: 'INSERT INTO objetos (clave, contenido) VALUES (?, ?) ON CONFLICT(clave) DO UPDATE SET contenido = objetos.contenido || excluded.contenido',
        args: [clave, contenido],
      });
    },

    async listar(prefijo) {
      const patron = `${prefijo.replace(/[\\%_]/g, (c) => `\\${c}`)}%`;
      const r = await cliente.execute({ sql: "SELECT clave FROM objetos WHERE clave LIKE ? ESCAPE '\\'", args: [patron] });
      return r.rows.map((fila) => String(fila.clave));
    },
  };
}
