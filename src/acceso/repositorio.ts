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
  /** Lectura inversa del vínculo: con qué identidad externa entró un `Usuario`. La necesita el directorio de
   * administradores de la instancia (`server/identidad/administradoresGlobales.ts`), que se configura por
   * `proveedor:sujetoId` —lo que un operador conoce— y no por el `usuarioId` interno, que se asigna solo. */
  buscarIdentidadDeUsuario(usuarioId: string): IdentidadVinculada | undefined;
  crearSesion(sesion: Sesion): void;
  buscarSesion(sesionId: string): Sesion | undefined;
  obtenerMembresia(usuarioId: string, gameId: string): Membresia | undefined;
  otorgarMembresia(membresia: Membresia): void;
}
