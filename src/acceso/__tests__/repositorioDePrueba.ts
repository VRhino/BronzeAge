// Doble del puerto `RepositorioIdentidad` para los tests de `acceso/`. Existe para que el dominio de acceso
// se pueda probar sin arrastrar el adaptador real (`server/identidad/repositorioEnMemoria.ts`): que un test
// de política necesite infraestructura sería justo la mezcla que la capa `acceso/` viene a deshacer.
//
// Guarda en memoria y devuelve los mismos objetos que recibe (sin clonar), para que un test pueda mutar un
// `Usuario` y simular un baneo aplicado por fuera del flujo de autenticación.
import type { RepositorioIdentidad } from '../repositorio';
import type { Membresia, Sesion, Usuario } from '../tipos';

export function repositorioDePrueba(): RepositorioIdentidad {
  const usuarios = new Map<string, Usuario>();
  const identidades = new Map<string, string>();
  const sesiones = new Map<string, Sesion>();
  const membresias = new Map<string, Membresia>();
  let n = 0;

  return {
    obtenerUsuario: (id) => usuarios.get(id),
    buscarUsuarioPorIdentidadExterna: (proveedor, sujetoId) => {
      const id = identidades.get(`${proveedor}:${sujetoId}`);
      return id ? usuarios.get(id) : undefined;
    },
    crearUsuario: ({ creadoEn }) => {
      const usuario: Usuario = { id: `usuario-${++n}`, creadoEn, deshabilitado: false };
      usuarios.set(usuario.id, usuario);
      return usuario;
    },
    vincularIdentidad: (v) => void identidades.set(`${v.proveedor}:${v.sujetoId}`, v.usuarioId),
    crearSesion: (s) => void sesiones.set(s.id, s),
    buscarSesion: (id) => sesiones.get(id),
    obtenerMembresia: (usuarioId, gameId) => membresias.get(`${usuarioId}:${gameId}`),
    otorgarMembresia: (m) => void membresias.set(`${m.usuarioId}:${m.gameId}`, m),
  };
}
