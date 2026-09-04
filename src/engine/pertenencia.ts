// Predicados de PERTENENCIA: qué relación tiene un jugador con un asentamiento o una Facción — residencia y
// titularidad de cargos. Son reglas de JUEGO (Doc 2.2/2.5), y por eso viven aquí y no en la capa de servidor:
// la autorización de comandos (`session/comandos/autorizacion.ts`) las CONSULTA, no las redefine.
//
// Módulo hoja a propósito (solo importa `domain/types`): lo consumen `faccion.ts`, `cargos.ts`, `politicas.ts`
// y `tropas.ts`, y cualquier import hacia otro módulo de `engine/` crearía un ciclo con alguno de ellos.
//
// Nace de una duplicación medida (revisión de separación negocio/infraestructura, 2026-08-25): el mapa
// `CargoTipo -> campo de Asentamiento.cargos` estaba copiado CUATRO veces (dos en `cargos.ts`, una en
// `politicas.ts`, una más en la capa de autorización), y la regla de residencia TRES (`tropas.ts`,
// `faccion.ts`, autorización). Añadir un cargo nuevo obligaba a acertar en los cuatro sitios.
import type { Asentamiento, CargoTipo, Faccion, RelacionPolitica } from '../domain/types';

/** Único mapa `CargoTipo -> campo de `Asentamiento.cargos``. Si se añade un cargo, el tipo `CargoTipo`
 * (domain/types.ts) obliga a completarlo aquí, y todo lo demás lo hereda. */
export const CAMPO_CARGO: Record<CargoTipo, keyof Asentamiento['cargos']> = {
  gobernador: 'gobernadorId',
  tesorero: 'tesoreroId',
  general: 'generalId',
  maestroObras: 'maestroObrasId',
  sacerdote: 'sacerdoteId',
};

/** Residencia (Doc 2.5): fundar el asentamiento o comprar casa en él son las dos vías, equivalentes a
 * efectos de qué puede hacer el jugador ahí. */
export function esResidente(asentamiento: Asentamiento, jugadorId: string): boolean {
  return asentamiento.jugadoresFundadoresIds.includes(jugadorId) || asentamiento.casasCompradas.includes(jugadorId);
}

/** Reside en ALGÚN asentamiento distinto del indicado — un jugador solo puede residir en uno (Doc 2.1). */
export function resideEnOtroAsentamiento(asentamientos: Asentamiento[], asentamientoId: string, jugadorId: string): boolean {
  return asentamientos.some((a) => a.id !== asentamientoId && esResidente(a, jugadorId));
}

/** El cargo está OCUPADO por alguien (pregunta de regla de juego: "¿hay Gobernador?"). Distinta de
 * `tieneCargoLocal`, que pregunta por un jugador concreto — ver el comentario de ahí. */
export function cargoOcupado(asentamiento: Asentamiento, cargo: CargoTipo): boolean {
  return asentamiento.cargos[CAMPO_CARGO[cargo]] !== null;
}

/**
 * Ese jugador ES el titular del cargo.
 *
 * Ojo a la diferencia con `cargoOcupado`, que es sutil y deliberada: el motor casi siempre pregunta si el
 * puesto está cubierto (regla de juego: una política necesita un cargo que la respalde), mientras que la
 * autorización pregunta si el actor es quien lo ocupa (¿puedes TÚ hacer esto?). Son preguntas distintas y
 * ambas hacen falta; confundirlas dejaría que cualquier residente actuara en nombre del Gobernador.
 */
export function tieneCargoLocal(asentamiento: Asentamiento, cargo: CargoTipo, jugadorId: string): boolean {
  return asentamiento.cargos[CAMPO_CARGO[cargo]] === jugadorId;
}

/** Asigna (o libera, con `null`) el titular de un cargo local. Sin validación: las reglas de quién puede
 * hacerlo viven en `cargos.ts`; esto es solo la escritura del campo correcto. */
export function conCargoLocal(asentamiento: Asentamiento, cargo: CargoTipo, jugadorId: string | null): Asentamiento {
  return { ...asentamiento, cargos: { ...asentamiento.cargos, [CAMPO_CARGO[cargo]]: jugadorId } };
}

export function esReyDe(faccion: Faccion, jugadorId: string): boolean {
  return faccion.reyId === jugadorId;
}

/** Cargos de Facción con autoridad diplomática (Doc 2.2): el Rey, y el Embajador que él designa. */
export function esReyOEmbajadorDe(faccion: Faccion, jugadorId: string): boolean {
  return faccion.reyId === jugadorId || faccion.embajadorId === jugadorId;
}

/**
 * ¿Hay una alianza ACTIVA entre estas dos Facciones? (Doc 2.4).
 *
 * Vivía como función privada de `engine/combate.ts`, donde solo servía para la penalización de reputación por
 * atacar a un Aliado. Sube aquí al aparecer el segundo consumidor con una pregunta distinta: el
 * reabastecimiento en ruta, que deja repostar en una plaza aliada (Doc 5.13). Es un predicado de pertenencia
 * política como los de arriba, y este módulo es hoja (solo importa `domain/types`), así que no crea ciclo con
 * nadie.
 */
export function estanAliadas(relaciones: readonly RelacionPolitica[], aId: string, bId: string): boolean {
  return relaciones.some(
    (r) =>
      r.estado === 'activa' &&
      r.tipo === 'alianza' &&
      ((r.faccionAId === aId && r.faccionBId === bId) || (r.faccionAId === bId && r.faccionBId === aId))
  );
}
