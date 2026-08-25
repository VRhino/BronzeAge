// Puerto de PERSISTENCIA de identidad — deliberadamente separado de `ProveedorIdentidad` (que solo verifica
// credenciales, nunca guarda nada): este puerto recuerda qué `Usuario` corresponde a qué identidad externa,
// y las `Sesion`/`Membresia` vigentes. La implementación en memoria de abajo es la única hoy — sustituirla
// por una respaldada en disco o en una base real (igual que `persistenciaPartida.ts` para el estado de
// partida) es cambiar esta clase sin tocar `servicioAutenticacion.ts`.
//
// Se pierde al reiniciar el proceso: aceptable mientras el único proveedor sea el de desarrollo
// (`proveedorDesarrollo.ts`) y no haya usuarios reales que deban sobrevivir a un reinicio.
import type { IdentidadVinculada, Membresia, Sesion, Usuario } from './tipos';

export interface RepositorioIdentidad {
  obtenerUsuario(usuarioId: string): Usuario | undefined;
  buscarUsuarioPorIdentidadExterna(proveedor: string, sujetoId: string): Usuario | undefined;
  crearUsuario(datos: { creadoEn: string }): Usuario;
  vincularIdentidad(vinculo: IdentidadVinculada): void;
  crearSesion(sesion: Sesion): void;
  buscarSesion(sesionId: string): Sesion | undefined;
  obtenerMembresia(usuarioId: string, gameId: string): Membresia | undefined;
  otorgarMembresia(membresia: Membresia): void;
}

export function crearRepositorioIdentidadEnMemoria(): RepositorioIdentidad {
  const usuarios = new Map<string, Usuario>();
  const identidadesPorClave = new Map<string, string>(); // `${proveedor}:${sujetoId}` -> usuarioId
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
