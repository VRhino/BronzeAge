// Entidades de identidad y autorización TÉCNICA (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md,
// diseño de Fase A6). Deliberadamente separadas de los cargos de juego (gobernador, rey, etc.), que son
// dominio y siguen viviendo en `Faccion`/`Asentamiento` (`domain/`) — este archivo es la capa de acceso que
// hoy no existe delante de ellos.
export type RolTecnico =
  | 'jugador'
  | 'administrador_partida'
  | 'moderador'
  | 'administrador_global'
  | 'observador'
  | 'servicio_npc';

/** Cuenta autenticable, externa a cualquier partida concreta. No sabe nada de proveedores de identidad — eso
 * es `IdentidadVinculada`, la relación N:1 que permite que un mismo `Usuario` haya iniciado sesión alguna vez
 * con el proveedor de desarrollo y mañana con uno real, sin perder su historial. */
export interface Usuario {
  id: string;
  creadoEn: string;
  deshabilitado: boolean;
}

/** Vínculo entre un `Usuario` y una identidad externa concreta (`proveedor` + `sujetoId`, doc 5). Varias
 * pueden apuntar al mismo `Usuario` — es lo que permite añadir o retirar proveedores sin fusionar/migrar
 * cuentas a mano. */
export interface IdentidadVinculada {
  usuarioId: string;
  proveedor: string;
  sujetoId: string;
  email?: string;
  vinculadaEn: string;
}

/** Credenciales activas de un `Usuario`. La API resuelve `Sesion -> Usuario` en cada request; nunca confía en
 * un `usuarioId`/`jugadorId` que el cliente afirme tener sin pasar por aquí (doc 2, principio 3). */
export interface Sesion {
  id: string;
  usuarioId: string;
  emitidaEn: string;
  expiraEn: string;
}

/** Identidad DENTRO de una partida — sustituye al `jugadorId` de cadena libre que usa hoy el motor. `id`
 * conserva el mismo valor que ya usa `ActorId` (`session/comandos/tipos.ts`): esto no cambia el dominio,
 * solo le da un dueño (`usuarioId`) verificado. */
export interface Jugador {
  id: string;
  gameId: string;
  usuarioId: string;
  faccionId: string | null;
  creadoEn: string;
}

/** La relación que de verdad se consulta en cada chequeo de autorización (Fase C2) — une `Usuario`,
 * `Jugador`, partida, Facción y rol técnico con vigencia ("hasta" ausente = vigente). */
export interface Membresia {
  usuarioId: string;
  gameId: string;
  jugadorId: string | null;
  faccionId: string | null;
  rol: RolTecnico;
  desde: string;
  hasta?: string;
}
