// Modelo del DOMINIO DE ACCESO (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md, diseño de Fase A6):
// quién es alguien y qué rol técnico tiene. Deliberadamente separado de los cargos de juego (gobernador,
// rey, etc.), que son dominio de JUEGO y viven en `Faccion`/`Asentamiento` (`domain/`).
//
// Esto es NEGOCIO, no infraestructura: son las reglas de quién puede existir y con qué papel, estables
// aunque cambien Fastify, el proveedor de identidad o la base de datos. Por eso vive en `acceso/` —una capa
// sin dependencias, que `session/` y `server/` pueden consultar— y no dentro de `server/`, donde estaba
// mezclado con los adaptadores que lo implementan (revisión de separación negocio/infraestructura,
// 2026-08-25).
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

// `Jugador` (identidad DENTRO de una partida) está diseñado en el doc 5 pero NO se declara aquí: hoy nada
// lo usaría. `Membresia.jugadorId` ya lleva el vínculo `Usuario` -> id de jugador que el motor consume, y
// una interfaz sin un solo consumidor es peso muerto que envejece mal. Se añadirá cuando haya algo que
// guardar en ella (perfil, fecha de entrada, estado dentro de la partida) — el contrato sigue en el doc.

/**
 * La relación que de verdad se consulta en cada chequeo de autorización (Fase C2) — une `Usuario`,
 * `Jugador`, partida y rol técnico con vigencia ("hasta" ausente = vigente).
 *
 * SIN `faccionId`, a diferencia del diseño original de A6 (doc 5, revisado 2026-08-25). La pertenencia a una
 * Facción es un hecho del JUEGO cuya única fuente de verdad es `Faccion.ciudadanosIds`, que el motor muta en
 * `otorgarCiudadania`, `anexionar` y `fusionar`. Una copia aquí quedaría obsoleta en cuanto una anexión
 * moviera ciudadanos de una Facción a otra: la autorización seguiría razonando sobre una Facción que ya no
 * existe. La autorización la deriva del estado de partida (`session/comandos/autorizacion.ts`) en vez de
 * guardarla; un campo que no hay que creerse es peor que no tenerlo.
 */
export interface Membresia {
  usuarioId: string;
  gameId: string;
  jugadorId: string | null;
  rol: RolTecnico;
  desde: string;
  hasta?: string;
}
