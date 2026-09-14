// Qué puede hacer cada rol técnico sobre una partida (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md,
// Fase C3). Es la política que separa las dos superficies HTTP: `/admin/*` y `/jugador/*`.
//
// Vive en `acceso/` y no en `server/` porque es NEGOCIO: "un moderador no puede descartar una partida" no
// cambia si mañana el transporte deja de ser Fastify. Lo que `server/` decide es cómo se mapea a rutas y
// códigos de estado HTTP.
//
// Ojo a la frontera con `session/comandos/autorizacion.ts`: aquí se decide quién entra a una SUPERFICIE
// (¿puedes usar la consola de administración de esta partida?); allí, quién puede ejecutar un COMANDO
// concreto según su relación de juego con la entidad objetivo. Un `administrador_partida` pasa este filtro
// pero sigue sin poder reclutar tropa en un asentamiento ajeno — son las dos jerarquías ortogonales que el
// doc 5 insiste en no confundir.
import type { Membresia, RolTecnico } from './tipos';

/** Roles que administran UNA partida concreta. `administrador_global` no está aquí: administra la instancia
 * y no necesita `Membresia` en ninguna partida — se trata aparte en `ActorDeInstancia`. */
const ROLES_DE_ADMINISTRACION: readonly RolTecnico[] = ['administrador_partida', 'moderador'];

/**
 * El actor tal y como lo ve la capa de acceso: quién es, si la instancia lo reconoce como administrador
 * global, y qué `Membresia` tiene en la partida objetivo (si alguna, y si sigue vigente).
 */
export interface ActorDeInstancia {
  usuarioId: string;
  esAdministradorGlobal: boolean;
  membresia?: Membresia;
}

/** Una `Membresia` con `hasta` en el pasado está revocada: se conserva por historial, no concede nada. */
export function esVigente(membresia: Membresia, ahora: string): boolean {
  return membresia.hasta === undefined || membresia.hasta > ahora;
}

/**
 * Rol con el que el actor actúa EN ESTA PARTIDA, o `undefined` si no tiene ninguno.
 *
 * La `Membresia` manda sobre `esAdministradorGlobal` — no al revés. Bug real corregido el 2026-08-26 (hito
 * C8): el orden anterior cortocircuitaba a `'administrador_global'` para cualquier actor con acceso técnico
 * global, incluso cuando además tenía una `Membresia` `administrador_partida` en esa partida concreta (la que
 * `otorgarAdministracion()` concede automáticamente a quien la crea) — así que la ÚNICA fila de
 * `MATRIZ_AUTORIZACION` que entonces admitía administración (`['jugador', 'administrador_partida']`, ninguna
 * admitía `'administrador_global'`), devolvía 403 siempre para un administrador, contradiciendo el propio
 * comentario de esa fila ("sin restricción si es admin", doc 5). `esAdministradorGlobal` sigue siendo la
 * autoridad para ENTRAR a la superficie `/admin/*` de una partida sin Membresia (`puedeAdministrar`, más
 * abajo, no usa esta función) — pero el rol CON el que actúa dentro de una partida concreta es el que da su
 * Membresia ahí, si tiene una.
 */
export function rolEnPartida(actor: ActorDeInstancia): RolTecnico | undefined {
  if (actor.membresia) return actor.membresia.rol;
  if (actor.esAdministradorGlobal) return 'administrador_global';
  return undefined;
}

/** Puede usar la superficie `/admin/*` de esta partida. */
export function puedeAdministrar(actor: ActorDeInstancia): boolean {
  return actor.esAdministradorGlobal || (actor.membresia !== undefined && ROLES_DE_ADMINISTRACION.includes(actor.membresia.rol));
}

/** Puede usar la superficie `/jugador/*` de esta partida. Deliberadamente NO lo cumple un administrador: para
 * jugar hace falta ser jugador, tener acceso técnico no es lo mismo que tener un personaje en la partida. */
export function puedeJugar(actor: ActorDeInstancia): boolean {
  return actor.membresia?.rol === 'jugador';
}

/** Crear o reabrir una partida es una operación de INSTANCIA: no hay `Membresia` posible en una partida que
 * todavía no existe, así que solo un administrador global puede hacerlo. */
export function puedeCrearPartida(actor: ActorDeInstancia): boolean {
  return actor.esAdministradorGlobal;
}

/**
 * Operaciones destructivas sobre una partida existente (hoy: `forzar`, que descarta el estado y empieza de
 * cero). El `moderador` queda fuera a propósito — doc 5 lo define como "subset de administrador_partida, sin
 * acceso a balance/regeneración de mundo".
 */
export function puedeDescartarPartida(actor: ActorDeInstancia): boolean {
  return actor.esAdministradorGlobal || actor.membresia?.rol === 'administrador_partida';
}

/**
 * Otorgar o revocar membresías de administración/observación de una partida (cierre de Fase C). Mismo
 * criterio que descartar: un `moderador` administra la partida pero no reparte roles — conceder acceso
 * técnico a otros es competencia de `administrador_partida`/`administrador_global`.
 */
export function puedeGestionarMembresias(actor: ActorDeInstancia): boolean {
  return actor.esAdministradorGlobal || actor.membresia?.rol === 'administrador_partida';
}
