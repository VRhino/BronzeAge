// Adaptador en memoria del puerto `RepositorioIdentidad` (`acceso/repositorio.ts`). Es INFRAESTRUCTURA: cómo
// se guardan los datos de acceso, no qué significan.
//
// Se pierde al reiniciar el proceso: aceptable mientras el único proveedor activo sea el de desarrollo
// (`proveedorDesarrollo.ts`) y no haya usuarios reales que deban sobrevivir a un reinicio. Sustituirlo por
// una implementación persistente (al estilo de `server/persistenciaPartida.ts` para el estado de partida) es
// escribir otro adaptador de este mismo puerto y cambiar quién lo construye.
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import type { IdentidadVinculada, Membresia, Sesion, Usuario } from '../../acceso/tipos';

export function crearRepositorioIdentidadEnMemoria(): RepositorioIdentidad {
  const usuarios = new Map<string, Usuario>();
  const identidadesPorClave = new Map<string, string>(); // `${proveedor}:${sujetoId}` -> usuarioId
  const identidadesPorUsuario = new Map<string, IdentidadVinculada>(); // usuarioId -> vinculo
  const sesiones = new Map<string, Sesion>();
  const membresias = new Map<string, Membresia>(); // `${usuarioId}:${gameId}` -> Membresia

  let contadorUsuarios = 0;

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
      return usuario;
    },
    vincularIdentidad(vinculo) {
      identidadesPorClave.set(`${vinculo.proveedor}:${vinculo.sujetoId}`, vinculo.usuarioId);
      // Un `Usuario` puede acumular varias identidades (doc 5); el directorio de administradores solo
      // necesita una para resolver `proveedor:sujetoId`, así que se guarda la PRIMERA — la que creó la
      // cuenta — y no la última, para que vincular un proveedor nuevo no cambie quién es administrador.
      if (!identidadesPorUsuario.has(vinculo.usuarioId)) identidadesPorUsuario.set(vinculo.usuarioId, vinculo);
    },
    buscarIdentidadDeUsuario(usuarioId) {
      return identidadesPorUsuario.get(usuarioId);
    },
    crearSesion(sesion) {
      sesiones.set(sesion.id, sesion);
    },
    buscarSesion(sesionId) {
      return sesiones.get(sesionId);
    },
    obtenerMembresia(usuarioId, gameId) {
      return membresias.get(`${usuarioId}:${gameId}`);
    },
    otorgarMembresia(membresia) {
      membresias.set(`${membresia.usuarioId}:${membresia.gameId}`, membresia);
    },
  };
}
