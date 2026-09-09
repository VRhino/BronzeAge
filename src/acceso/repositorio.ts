// Puerto de PERSISTENCIA del dominio de acceso — deliberadamente separado de `ProveedorIdentidad` (que solo
// verifica credenciales, nunca guarda nada): este puerto recuerda qué `Usuario` corresponde a qué identidad
// externa, y las `Sesion`/`Membresia` vigentes.
//
// Aquí vive solo el CONTRATO. Las implementaciones son adaptadores y viven en `server/`:
// `repositorioEnMemoria.ts` (tests y despliegue efímero) y `repositorioPersistente.ts` (el proceso real, mismo
// adaptador de memoria con carga/guardado de un JSON atómico). Una base de datos real sería otro adaptador
// más, sin que `servicioAutenticacion.ts` ni ninguna ruta se enteren.
import type { CredencialLocal, IdentidadVinculada, Membresia, Sesion, Usuario } from './tipos';

export interface RepositorioIdentidad {
  obtenerUsuario(usuarioId: string): Usuario | undefined;
  buscarUsuarioPorIdentidadExterna(proveedor: string, sujetoId: string): Usuario | undefined;
  crearUsuario(datos: { creadoEn: string }): Usuario;
  vincularIdentidad(vinculo: IdentidadVinculada): void;
  /** Lectura inversa del vínculo: con qué identidad externa entró un `Usuario`. La necesita el directorio de
   * administradores de la instancia (`server/identidad/administradoresGlobales.ts`), que se configura por
   * `proveedor:sujetoId` —lo que un operador conoce— y no por el `usuarioId` interno, que se asigna solo. */
  buscarIdentidadDeUsuario(usuarioId: string): IdentidadVinculada | undefined;
  /** Secreto de una cuenta local por nick normalizado, para el proveedor `clave`. `undefined` si ese nick no
   * está registrado — es lo que distingue "alta nueva" de "verificar" en el flujo de registro/login. */
  buscarCredencialLocal(nick: string): CredencialLocal | undefined;
  guardarCredencialLocal(credencial: CredencialLocal): void;
  crearSesion(sesion: Sesion): void;
  buscarSesion(sesionId: string): Sesion | undefined;
  obtenerMembresia(usuarioId: string, gameId: string): Membresia | undefined;
  otorgarMembresia(membresia: Membresia): void;
  /** Todas las membresías de una partida —vigentes y revocadas— para que la superficie de administración
   * pueda listarlas y gestionarlas (Fase C3, cierre de Fase C). El filtro de vigencia lo aplica quien lo
   * consume (`esVigente`, `acceso/rolesDePartida.ts`). */
  listarMembresiasDePartida(gameId: string): Membresia[];
  /** Revoca una membresía poniéndole `hasta` (no la borra: el historial se conserva, doc 5). `false` si no
   * había ninguna para ese usuario+partida. */
  revocarMembresia(usuarioId: string, gameId: string, hasta: string): boolean;
}
