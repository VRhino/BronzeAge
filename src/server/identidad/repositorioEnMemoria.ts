// Adaptador en memoria del puerto `RepositorioIdentidad` (`acceso/repositorio.ts`). Es INFRAESTRUCTURA: cómo
// se guardan los datos de acceso, no qué significan.
//
// Se pierde al reiniciar el proceso — aceptable para tests y para un despliegue efímero. El proceso real
// (`server/index.ts`) usa `repositorioEnDisco.ts`, que es ESTE adaptador con un `inicial` cargado de disco y
// un `alCambiar` que reescribe el archivo tras cada mutación. Por eso este archivo expone el snapshot
// (`DatosIdentidad`) y el gancho de cambio: para no duplicar la lógica de los índices en dos adaptadores.
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import type { IdentidadVinculada, Membresia, Sesion, Usuario } from '../../acceso/tipos';

/** Todo lo que el dominio de acceso guarda, en forma plana y serializable — lo que viaja a disco. */
export interface DatosIdentidad {
  usuarios: Usuario[];
  /** En orden de vinculación: el primero de cada `usuarioId` es el que resuelve `buscarIdentidadDeUsuario`
   * (ver `vincularIdentidad`). */
  identidades: IdentidadVinculada[];
  sesiones: Sesion[];
  membresias: Membresia[];
}

export interface OpcionesRepositorioEnMemoria {
  /** Estado con el que arranca el repositorio (lo carga `repositorioEnDisco.ts`). Vacío por defecto. */
  inicial?: DatosIdentidad;
  /** Se invoca con el snapshot COMPLETO tras cada mutación. `repositorioEnDisco.ts` lo usa para persistir;
   * el adaptador puro no lo pasa. */
  alCambiar?: (datos: DatosIdentidad) => void;
}

/** Mayor `N` en los ids `usuario-<N>` ya existentes, para que un repositorio recargado no reasigne ids. */
function ultimoContadorDe(usuarios: Usuario[]): number {
  return usuarios.reduce((max, u) => {
    const n = /^usuario-(\d+)$/.exec(u.id);
    return n ? Math.max(max, Number(n[1])) : max;
  }, 0);
}

export function crearRepositorioIdentidadEnMemoria(opciones: OpcionesRepositorioEnMemoria = {}): RepositorioIdentidad {
  const inicial = opciones.inicial;
  const usuarios = new Map<string, Usuario>((inicial?.usuarios ?? []).map((u) => [u.id, u]));
  const identidadesPorClave = new Map<string, string>(); // `${proveedor}:${sujetoId}` -> usuarioId
  const identidadesPorUsuario = new Map<string, IdentidadVinculada>(); // usuarioId -> primer vinculo
  const identidadesEnOrden: IdentidadVinculada[] = [];
  const sesiones = new Map<string, Sesion>((inicial?.sesiones ?? []).map((s) => [s.id, s]));
  const membresias = new Map<string, Membresia>((inicial?.membresias ?? []).map((m) => [`${m.usuarioId}:${m.gameId}`, m]));

  for (const vinculo of inicial?.identidades ?? []) registrarVinculo(vinculo);

  let contadorUsuarios = ultimoContadorDe([...usuarios.values()]);

  function registrarVinculo(vinculo: IdentidadVinculada): void {
    identidadesPorClave.set(`${vinculo.proveedor}:${vinculo.sujetoId}`, vinculo.usuarioId);
    identidadesEnOrden.push(vinculo);
    // Un `Usuario` puede acumular varias identidades (doc 5); el directorio de administradores solo necesita
    // una para resolver `proveedor:sujetoId`, así que se guarda la PRIMERA —la que creó la cuenta— y no la
    // última, para que vincular un proveedor nuevo no cambie quién es administrador.
    if (!identidadesPorUsuario.has(vinculo.usuarioId)) identidadesPorUsuario.set(vinculo.usuarioId, vinculo);
  }

  function snapshot(): DatosIdentidad {
    return {
      usuarios: [...usuarios.values()],
      identidades: [...identidadesEnOrden],
      sesiones: [...sesiones.values()],
      membresias: [...membresias.values()],
    };
  }

  const notificar = opciones.alCambiar ? () => opciones.alCambiar!(snapshot()) : () => {};

  return {
    obtenerUsuario(usuarioId) {
      return usuarios.get(usuarioId);
    },
    buscarUsuarioPorIdentidadExterna(proveedor, sujetoId) {
      const usuarioId = identidadesPorClave.get(`${proveedor}:${sujetoId}`);
      return usuarioId ? usuarios.get(usuarioId) : undefined;
    },
    crearUsuario({ creadoEn }) {
      contadorUsuarios += 1;
      const usuario: Usuario = { id: `usuario-${contadorUsuarios}`, creadoEn, deshabilitado: false };
      usuarios.set(usuario.id, usuario);
      notificar();
      return usuario;
    },
    vincularIdentidad(vinculo) {
      registrarVinculo(vinculo);
      notificar();
    },
    buscarIdentidadDeUsuario(usuarioId) {
      return identidadesPorUsuario.get(usuarioId);
    },
    crearSesion(sesion) {
      sesiones.set(sesion.id, sesion);
      notificar();
    },
    buscarSesion(sesionId) {
      return sesiones.get(sesionId);
    },
    obtenerMembresia(usuarioId, gameId) {
      return membresias.get(`${usuarioId}:${gameId}`);
    },
    otorgarMembresia(membresia) {
      membresias.set(`${membresia.usuarioId}:${membresia.gameId}`, membresia);
      notificar();
    },
    listarMembresiasDePartida(gameId) {
      return [...membresias.values()].filter((m) => m.gameId === gameId);
    },
    revocarMembresia(usuarioId, gameId, hasta) {
      const membresia = membresias.get(`${usuarioId}:${gameId}`);
      if (!membresia) return false;
      membresias.set(`${usuarioId}:${gameId}`, { ...membresia, hasta });
      notificar();
      return true;
    },
  };
}
