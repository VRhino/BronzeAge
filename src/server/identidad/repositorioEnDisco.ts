// Adaptador de `RepositorioIdentidad` respaldado en disco: el proceso real (`server/index.ts`) lo usa para
// que usuarios, sesiones y membresías sobrevivan a un reinicio (cierre de Fase C).
//
// NO reimplementa los índices — es `crearRepositorioIdentidadEnMemoria` con dos ganchos: `inicial` (lo que
// se leyó del archivo al arrancar) y `alCambiar` (reescribir el archivo tras cada mutación). La lógica de
// find-or-create, "primera identidad gana" y las claves compuestas vive en un solo sitio.
//
// Las escrituras se SERIALIZAN en una cola de promesas (misma técnica que `RunnerDePartida`): dos mutaciones
// seguidas no compiten por el archivo, y `alCambiar` puede ser síncrono aunque la escritura no lo sea. Un
// fallo de escritura no se traga: se guarda en `ultimoError` y `esperarEscrituras()` lo relanza, para que un
// apagado o un test se enteren en vez de perder datos en silencio.
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import { escribirIdentidad, leerIdentidad } from '../persistenciaIdentidad';
import { crearRepositorioIdentidadEnMemoria, type DatosIdentidad } from './repositorioEnMemoria';

export interface RepositorioIdentidadEnDisco {
  repositorio: RepositorioIdentidad;
  /** Se resuelve cuando toda escritura encolada hasta este instante ha terminado. Relanza el último fallo de
   * escritura si lo hubo. Para un apagado limpio del proceso y para tests deterministas. */
  esperarEscrituras(): Promise<void>;
}

export async function crearRepositorioIdentidadEnDisco(ruta: string): Promise<RepositorioIdentidadEnDisco> {
  const inicial = await leerIdentidad(ruta);

  let cola: Promise<void> = Promise.resolve();
  let ultimoError: unknown;

  const persistir = (datos: DatosIdentidad): void => {
    cola = cola
      .then(() => escribirIdentidad(ruta, datos))
      .catch((err: unknown) => {
        ultimoError = err;
      });
  };

  const repositorio = crearRepositorioIdentidadEnMemoria({ inicial, alCambiar: persistir });

  return {
    repositorio,
    async esperarEscrituras() {
      await cola;
      if (ultimoError !== undefined) {
        const err = ultimoError;
        ultimoError = undefined;
        throw err;
      }
    },
  };
}
