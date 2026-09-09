// Adaptador de `AlmacenDeObjetos` sobre el sistema de archivos: una clave = un archivo bajo `raiz`. Es lo
// que había antes repartido por `persistenciaPartida.ts`, `eventosDePartida.ts`, `auditoria.ts` y
// `persistenciaIdentidad.ts` (cada uno con su `join(directorio, ...)` y su `.tmp`+`rename`), ahora en un
// solo sitio.
//
// Es el adaptador por defecto y el único hoy. Para un despliegue en un free tier con disco efímero (Render,
// Koyeb) o para escalar, se escribe otro adaptador (object storage, SQLite sobre HTTP, Postgres) y se elige
// en `server/index.ts` — nada más cambia.
import { appendFile, mkdir, readdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { AlmacenDeObjetos } from './almacenDeObjetos';

export function crearAlmacenEnDisco(raiz: string): AlmacenDeObjetos {
  const ruta = (clave: string): string => join(raiz, clave);

  return {
    async leer(clave) {
      try {
        return await readFile(ruta(clave), 'utf-8');
      } catch (err) {
        if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null;
        throw err;
      }
    },

    async escribir(clave, contenido) {
      // `.tmp` + `rename` (atómico en el mismo volumen, NTFS y POSIX): un corte a mitad deja la copia
      // temporal a medias, nunca el archivo final. Decidido en el doc 4, era el criterio de
      // `persistenciaPartida`/`persistenciaIdentidad`.
      await mkdir(raiz, { recursive: true });
      const destino = ruta(clave);
      const temporal = `${destino}.tmp`;
      await writeFile(temporal, contenido, 'utf-8');
      await rename(temporal, destino);
    },

    async anexar(clave, contenido) {
      await mkdir(raiz, { recursive: true });
      await appendFile(ruta(clave), contenido, 'utf-8');
    },

    async listar(prefijo) {
      let nombres: string[];
      try {
        nombres = await readdir(raiz);
      } catch (err) {
        // Directorio inexistente = nada escrito todavía en este despliegue, no un error.
        if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
        throw err;
      }
      return prefijo === '' ? nombres : nombres.filter((n) => n.startsWith(prefijo));
    },
  };
}
