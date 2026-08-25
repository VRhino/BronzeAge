// Matriz de autorización por comando (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md, sección "Matriz
// de autorización por comando") convertida en datos ejecutables. Una entrada por cada uno de los 30 comandos
// de `session/comandos/registro.ts` — `verificar.ts` tiene un test de exhaustividad que falla si alguno se
// queda sin fila aquí, así que esto no puede desincronizarse en silencio del registro real de comandos.
//
// Tabla única en vez de metadatos junto a cada manejador (que era la idea original, ver comentario en
// `session/comandos/tipos.ts`): `RolTecnico`/`Membresia` son conceptos de `server/identidad/`, y `session` no
// puede importar de `server` (frontera de capas). Colocar el metadato junto al manejador habría significado
// mover el propio concepto de rol técnico a `session/`, mezclando dominio de juego con autorización de
// infraestructura — justo la confusión que el doc 5 pide evitar ("Diferencia entre rol técnico y cargo de
// juego"). El precio es el que el comentario original ya señalaba (una tabla paralela puede desincronizarse);
// el test de exhaustividad de `verificar.ts` es la mitigación.
import type { TipoComando } from '../../session/comandos/registro';
import type { ParamsDe } from '../../session/comandos/registro';
import type { GameSessionState } from '../../session/estado';
import type { RolTecnico } from '../identidad/tipos';
import {
  buscarAsentamiento,
  buscarCaravana,
  buscarFaccion,
  esFaccionPropia,
  esReyDe,
  esReyOEmbajadorDe,
  esResidente,
  tieneCargoLocal,
} from './condiciones';

/** Identidad del actor resuelta para ESTA partida — lo que aporta la `Membresia` (Docs 5), no el `Usuario`
 * global. `jugadorId` es `null` para roles que no requieren `Jugador` (ej. `administrador_partida` puro). */
export interface ActorDeComando {
  rol: RolTecnico;
  jugadorId: string | null;
  faccionId: string | null;
}

export type CondicionDominio<T extends TipoComando> = (
  estado: GameSessionState,
  actor: ActorDeComando,
  params: ParamsDe<T>
) => boolean;

export interface EntradaMatriz<T extends TipoComando> {
  /** Cualquiera de estos roles técnicos basta para pasar el primer filtro. */
  rolesPermitidos: RolTecnico[];
  /**
   * Condición de dominio adicional — SOLO se evalúa cuando el rol con el que el actor pasó el filtro anterior
   * es `'jugador'` (doc 5: "sin restricción si es admin"). Si el comando admite varios roles y el actor entró
   * como `administrador_partida`/`moderador`, esta condición NO se comprueba: el rol técnico ya es la
   * autoridad completa para esos casos, según la propia matriz del doc 5.
   */
  condicionJugador?: CondicionDominio<T>;
}

/** Exige `jugadorId` no nulo antes de delegar en una condición que lo necesita — todas las de abajo lo
 * necesitan, porque `condicionJugador` solo se evalúa para el rol `'jugador'`, que siempre tiene `Jugador`. */
function conJugador<T extends TipoComando>(fn: (jugadorId: string, ...resto: Parameters<CondicionDominio<T>>) => boolean): CondicionDominio<T> {
  return (estado, actor, params) => actor.jugadorId !== null && fn(actor.jugadorId, estado, actor, params);
}

export const MATRIZ_AUTORIZACION: { [T in TipoComando]: EntradaMatriz<T> } = {
  // --- fundarAsentamiento, lanzarCaravanaFundacion, desarmarCaravanaFundacion: Facción propia ---
  fundarAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, actor, params) => esFaccionPropia(actor.faccionId, params.faccionId),
  },
  lanzarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) => {
      const origen = buscarAsentamiento(estado, params.origenAsentamientoId);
      return origen === undefined || esFaccionPropia(actor.faccionId, origen.faccionId);
    },
  },
  desarmarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) => {
      const caravana = buscarCaravana(estado, params.caravanaId);
      if (!caravana) return true;
      const origen = buscarAsentamiento(estado, caravana.origenAsentamientoId);
      return origen === undefined || esFaccionPropia(actor.faccionId, origen.faccionId);
    },
  },

  // --- crearFaccion: sin condición de dominio adicional (doc 5, "Preguntas abiertas": límite a un Jugador
  // sin Facción todavía queda sin decidir) ---
  crearFaccion: { rolesPermitidos: ['jugador'] },

  // --- alternarFaccionNpc: jugador(rey) o administrador_partida ---
  alternarFaccionNpc: {
    rolesPermitidos: ['jugador', 'administrador_partida'],
    condicionJugador: (estado, actor, params) =>
      esFaccionPropia(actor.faccionId, params.faccionId) && esReyDe(buscarFaccion(estado, params.faccionId), actor.jugadorId!),
  },

  // --- asignarRey, asignarEmbajador: Facción propia + cargo de rey vigente (o primera asignación) ---
  asignarRey: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) => {
      if (!esFaccionPropia(actor.faccionId, params.faccionId)) return false;
      const faccion = buscarFaccion(estado, params.faccionId);
      return faccion === undefined || faccion.reyId === null || faccion.reyId === actor.jugadorId;
    },
  },
  asignarEmbajador: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) =>
      esFaccionPropia(actor.faccionId, params.faccionId) && esReyDe(buscarFaccion(estado, params.faccionId), actor.jugadorId!),
  },

  // --- asignarCargoLocal: residente; designar 'gobernador' es directo (doc 5: "Fase 0: designación
  // directa"), el resto de cargos los designa el Gobernador vigente ---
  asignarCargoLocal: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      if (!esResidente(asentamiento, jugadorId)) return false;
      if (params.cargo === 'gobernador') return true;
      return tieneCargoLocal(asentamiento, 'gobernador', jugadorId);
    }),
  },

  // --- comprarCasa: Facción propia del asentamiento objetivo — o SIN Facción todavía, porque comprar casa es
  // una de las dos vías de UNIRSE a una (junto a fundarAsentamiento; `engine/faccion.ts` solo bloquea ser
  // ciudadano de OTRA Facción distinta, ver `comprarCasa` ahí). Además: nadie compra una casa en nombre de
  // otro jugador. ---
  comprarCasa: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) => {
      if (actor.jugadorId !== params.jugadorId) return false;
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return asentamiento === undefined || actor.faccionId === null || esFaccionPropia(actor.faccionId, asentamiento.faccionId);
    },
  },

  // --- activarPolitica, anadirEdificioManualmente, quitarDeCola, moverEnCola, mejorarEdificioAhora,
  // alternarAutoConstruccion, calibrarReservaManual: residente + el cargo que ese comando exige ---
  activarPolitica: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, params.cargo, jugadorId);
    }),
  },
  anadirEdificioManualmente: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, params.cargo, jugadorId);
    }),
  },
  quitarDeCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, params.cargo, jugadorId);
    }),
  },
  moverEnCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, params.cargo, jugadorId);
    }),
  },
  mejorarEdificioAhora: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, params.cargo, jugadorId);
    }),
  },
  // Sin `cargo` explícito en `params` (a diferencia de los de arriba) — se exige Gobernador o Maestro de
  // Obras, mismo criterio que el resto del grupo de construcción, ver `session/comandos/construccion.ts`.
  alternarAutoConstruccion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return (
        esResidente(asentamiento, jugadorId) &&
        (tieneCargoLocal(asentamiento, 'gobernador', jugadorId) || tieneCargoLocal(asentamiento, 'maestroObras', jugadorId))
      );
    }),
  },
  // Sin `cargo` explícito — "Requiere Tesorero asignado" (comentario de `construccion.ts`): el cargo exigido
  // es tesorero.
  calibrarReservaManual: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => {
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return esResidente(asentamiento, jugadorId) && tieneCargoLocal(asentamiento, 'tesorero', jugadorId);
    }),
  },

  // --- renombrarAsentamiento: residente (doc 5 deja abierto si debería exigir un cargo específico) ---
  renombrarAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.asentamientoId), jugadorId)),
  },

  // --- romperRelacion, rebelionVasallo, anexionar, fusionar: Facción propia, cargo de rey/embajador ---
  romperRelacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) =>
      esFaccionPropia(actor.faccionId, params.iniciadorFaccionId) &&
      esReyOEmbajadorDe(buscarFaccion(estado, params.iniciadorFaccionId), actor.jugadorId!),
  },
  rebelionVasallo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, actor, params) => {
      const relacion = estado.relaciones.find((r) => r.id === params.relacionId);
      if (!relacion) return true; // lo rechaza el comando (relación inexistente / no es vasallaje)
      // Quien se REBELA es el vasallo (`faccionBId`, ver engine/diplomacia.ts `rebelionVasallo`).
      return esFaccionPropia(actor.faccionId, relacion.faccionBId) && esReyOEmbajadorDe(buscarFaccion(estado, relacion.faccionBId), actor.jugadorId!);
    },
  },
  anexionar: {
    rolesPermitidos: ['jugador'],
    // La Facción absorbente es `faccionAId` (`engine/fusion.ts` `anexionar`) — quien inicia la anexión.
    condicionJugador: (estado, actor, params) =>
      esFaccionPropia(actor.faccionId, params.faccionAId) && esReyOEmbajadorDe(buscarFaccion(estado, params.faccionAId), actor.jugadorId!),
  },
  fusionar: {
    rolesPermitidos: ['jugador'],
    // Fusión consentida por ambos lados: basta con ser rey/embajador de CUALQUIERA de las dos Facciones.
    condicionJugador: (estado, actor, params) =>
      (esFaccionPropia(actor.faccionId, params.faccionAId) && esReyOEmbajadorDe(buscarFaccion(estado, params.faccionAId), actor.jugadorId!)) ||
      (esFaccionPropia(actor.faccionId, params.faccionBId) && esReyOEmbajadorDe(buscarFaccion(estado, params.faccionBId), actor.jugadorId!)),
  },

  // --- proponerRelacion, proponerTrueque: NO están en la tabla del doc 5 (omisión — las 34 filas
  // enumeradas no cubren estos 2 de los 30 comandos reales). Regla inferida, coherente con el resto de la
  // fila de diplomacia/comercio; ver nota añadida al propio doc 5. ---
  proponerRelacion: {
    rolesPermitidos: ['jugador'],
    // `faccionAId` es quien propone, por convención con `romperRelacion.iniciadorFaccionId` (mismo patrón:
    // el primer id de la pareja es el iniciador salvo que el comando diga lo contrario).
    condicionJugador: (estado, actor, params) =>
      esFaccionPropia(actor.faccionId, params.faccionAId) && esReyOEmbajadorDe(buscarFaccion(estado, params.faccionAId), actor.jugadorId!),
  },
  proponerTrueque: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.asentamientoAId), jugadorId)),
  },

  // --- colocarOrdenMercado, crearCaravana, reclutarTropa: residente del asentamiento objetivo ---
  colocarOrdenMercado: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.asentamientoId), jugadorId)),
  },
  crearCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.asentamientoId), jugadorId)),
  },
  reclutarTropa: {
    rolesPermitidos: ['jugador'],
    // Nadie recluta un escuadrón a nombre de otro jugador (`Escuadron.jugadorId` sería el del actor, no el
    // que decida mandar el cliente).
    condicionJugador: (estado, actor, params) =>
      actor.jugadorId === params.jugadorId && esResidente(buscarAsentamiento(estado, params.asentamientoId), actor.jugadorId!),
  },

  // --- iniciarAsedio, combateCampoAbierto, interceptarCaravana, atacarCampamentoBandidos: residente del
  // asentamiento atacante. La matriz del doc 5 añade "escuadrones propios del jugador o de otros residentes
  // autorizados" — no comprobado aquí todavía (requeriría resolver el dueño de cada `escuadronId`); queda
  // como simplificación explícita, sin la cual ningún residente podría nunca ordenar un ataque. ---
  iniciarAsedio: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.atacanteId), jugadorId)),
  },
  interceptarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.atacanteId), jugadorId)),
  },
  atacarCampamentoBandidos: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador((jugadorId, estado, _actor, params) => esResidente(buscarAsentamiento(estado, params.atacanteId), jugadorId)),
  },
  // Batalla de campo abierto entre dos asentamientos cualesquiera: el actor debe residir en AL MENOS uno de
  // los dos lados que está comandando.
  combateCampoAbierto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: conJugador(
      (jugadorId, estado, _actor, params) =>
        esResidente(buscarAsentamiento(estado, params.asentamientoAId), jugadorId) ||
        esResidente(buscarAsentamiento(estado, params.asentamientoBId), jugadorId)
    ),
  },
};
