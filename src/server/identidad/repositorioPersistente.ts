// Adaptador de `RepositorioIdentidad` que persiste en un `AlmacenDeObjetos`: el proceso real
// (`server/index.ts`) lo usa para que usuarios, sesiones, membresías y credenciales locales sobrevivan a un
// reinicio (cierre de Fase C).
//
// NO reimplementa los índices — es `crearRepositorioIdentidadEnMemoria` con dos ganchos: `inicial` (lo que
// se leyó al arrancar) y `alCambiar` (reescribir tras cada mutación). La lógica de find-or-create, "primera
// identidad gana" y las claves compuestas vive en un solo sitio.
//
// Las escrituras se SERIALIZAN en una cola de promesas (misma técnica que `RunnerDePartida`): dos mutaciones
// seguidas no compiten, y `alCambiar` puede ser síncrono aunque la escritura no lo sea. Un fallo de
// escritura no se traga: se guarda en `ultimoError` y `esperarEscrituras()` lo relanza, para que un apagado
// o un test se enteren en vez de perder datos en silencio.
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import type { AlmacenDeObjetos } from '../almacen/almacenDeObjetos';
import { escribirIdentidad, leerIdentidad } from '../persistenciaIdentidad';
import { crearRepositorioIdentidadEnMemoria, type DatosIdentidad } from './repositorioEnMemoria';

export interface RepositorioIdentidadPersistente {
  repositorio: RepositorioIdentidad;
  /** Se resuelve cuando toda escritura encolada hasta este instante ha terminado. Relanza el último fallo de
   * escritura si lo hubo. Para un apagado limpio del proceso y para tests deterministas. */
  esperarEscrituras(): Promise<void>;
}

export async function crearRepositorioIdentidadPersistente(almacen: AlmacenDeObjetos): Promise<RepositorioIdentidadPersistente> {
  const inicial = await leerIdentidad(almacen);

  let cola: Promise<void> = Promise.resolve();
  let ultimoError: unknown;

  const persistir = (datos: DatosIdentidad): void => {
    cola = cola
      .then(() => escribirIdentidad(almacen, datos))
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
