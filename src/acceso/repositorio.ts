// Puerto de PERSISTENCIA del dominio de acceso — deliberadamente separado de `ProveedorIdentidad` (que solo
// verifica credenciales, nunca guarda nada): este puerto recuerda qué `Usuario` corresponde a qué identidad
// externa, y las `Sesion`/`Membresia` vigentes.
//
// Aquí vive solo el CONTRATO. Las implementaciones son adaptadores y viven en `server/` — hoy
// `server/identidad/repositorioEnMemoria.ts`; mañana, una respaldada en disco o en una base real, sin que
// `servicioAutenticacion.ts` ni ninguna ruta se enteren.
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
