// Quién puede ejecutar cada comando (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md, "Matriz de
// autorización por comando"), como datos ejecutables. Una entrada por cada comando de `registro.ts`.
//
// Vive en `session/` —la capa de aplicación DE PARTIDA— porque eso es exactamente lo que es: una regla de
// negocio que cruza los dos dominios del sistema. Necesita ver el juego (¿reside ahí? ¿es el Gobernador?) y
// el acceso (`RolTecnico`), y `session` es el único sitio que alcanza a ambos. Estuvo brevemente en
// `server/` (Fase C2, corregido el mismo día): allí obligaba a reimplementar predicados del motor dentro de
// la capa de infraestructura, duplicando reglas de juego que ya existían en `engine/`. Lo que decide esta
// tabla no cambia si mañana se sustituyen Fastify, el proveedor de identidad o la base de datos.
//
// Las condiciones NO reimplementan reglas: delegan en los predicados de `engine/pertenencia.ts` y
// `engine/faccion.ts`, que son la única definición de residencia, cargo y ciudadanía.
//
// Exhaustividad garantizada en COMPILACIÓN por el tipo indexado `{ [T in TipoComando]: ... }`: añadir un
// comando al registro sin darle fila aquí no compila. `autorizacionComandos.test.ts` lo refuerza comparando
// las claves contra el registro real.
import type { Asentamiento, Caravana, Faccion } from '../../domain/types';
import type { RolTecnico } from '../../acceso/tipos';
import { esCiudadano } from '../../engine/faccion';
import { esResidente, esReyDe, esReyOEmbajadorDe, tieneCargoLocal } from '../../engine/pertenencia';
import { estaEnAsentamiento } from '../../engine/ubicacion';
import type { GameSessionState } from '../estado';
import type { ParamsDe, TipoComando } from './registro';

/**
 * Identidad del actor resuelta para esta partida — lo que aporta la `Membresia` (doc 5), nunca un id que el
 * cliente afirme tener (doc 2, principio 3). `jugadorId` es `null` para roles que no requieren `Jugador`
 * (ej. `administrador_partida` puro).
 *
 * Sin `faccionId`: la Facción del actor se DERIVA de `Faccion.ciudadanosIds`, única fuente de verdad del
 * juego (ver `Membresia` en `acceso/tipos.ts` para el porqué).
 */
export interface ActorDeComando {
  rol: RolTecnico;
  jugadorId: string | null;
}

export type CondicionDominio<T extends TipoComando> = (
  estado: GameSessionState,
  jugadorId: string,
  params: ParamsDe<T>
) => boolean;

export interface EntradaMatriz<T extends TipoComando> {
  /** Cualquiera de estos roles técnicos basta para pasar el primer filtro. */
  rolesPermitidos: RolTecnico[];
  /**
   * Condición de dominio adicional — SOLO se evalúa cuando el actor entra con el rol `'jugador'` (doc 5:
   * "sin restricción si es admin"). Si el comando admite varios roles y el actor entró como
   * `administrador_partida`/`moderador`, no se comprueba: para esos, el rol técnico ya es autoridad completa.
   */
  condicionJugador?: CondicionDominio<T>;
}

// --- Resolutores locales ---
//
// Devuelven `undefined` en vez de rechazar: la comprobación de EXISTENCIA es del comando (`exigirAsentamiento`
// et al. en `ayudas.ts`, con su código `*.no_existe`). Las condiciones de abajo tratan "no existe" como
// autorizado a propósito, para que un id inventado produzca "eso no existe" y no un "no tienes permiso"
// engañoso — sin que ninguna acción no autorizada llegue a aplicarse, porque el comando la rechaza después.

function buscarAsentamiento(estado: GameSessionState, id: string): Asentamiento | undefined {
  return estado.asentamientos.find((a) => a.id === id);
}

function buscarFaccion(estado: GameSessionState, id: string): Faccion | undefined {
  return estado.facciones.find((f) => f.id === id);
}

function buscarCaravana(estado: GameSessionState, id: string): Caravana | undefined {
  return estado.caravanas.find((c) => c.id === id);
}

/** Ciudadano de esa Facción — la pertenencia real del juego, no una copia en la capa de acceso. */
function esFaccionPropia(estado: GameSessionState, jugadorId: string, faccionId: string): boolean {
  const faccion = buscarFaccion(estado, faccionId);
  return faccion === undefined || esCiudadano(faccion, jugadorId);
}

function sinFaccionTodavia(estado: GameSessionState, jugadorId: string): boolean {
  return !estado.facciones.some((f) => esCiudadano(f, jugadorId));
}

/**
 * Es su Facción, o todavía no tiene ninguna. Lo segundo importa para `comprarCasa`: comprar casa es una de
 * las dos vías de ENTRAR en una Facción (Doc 2.5) y está abierta a cualquiera —el motor solo exige no ser ya
 * ciudadano de otra, y el cupo de vivienda la limita—, así que exigir ciudadanía previa la volvería
 * inalcanzable.
 */
function esFaccionPropiaOSinFaccion(estado: GameSessionState, jugadorId: string, faccionId: string): boolean {
  return sinFaccionTodavia(estado, jugadorId) || esFaccionPropia(estado, jugadorId, faccionId);
}

/**
 * Puede fundar para esa Facción: ser ya ciudadano suyo, o —caso de arranque— que la Facción no tenga NINGÚN
 * ciudadano todavía y el actor no pertenezca a ninguna otra.
 *
 * La excepción es estrecha a propósito. Hace falta porque `crearFaccion` deja la Facción con
 * `ciudadanosIds: []` (`engine/faccion.ts`) y fundar es lo que otorga la primera ciudadanía: sin ella, quien
 * crea una Facción no podría fundar en ella y la Facción nacería muerta. Pero no puede ser más ancha: fundar
 * consume el CAP DE FUNDACIÓN de la Facción (limitado por su nivel, Doc 1.7), así que dejar que un
 * desconocido funde en una Facción ajena sería regalarle una vía para agotarle el cupo. Comprar casa, que sí
 * está abierta, no consume nada de eso.
 */
function puedeFundarEn(estado: GameSessionState, jugadorId: string, faccionId: string): boolean {
  const faccion = buscarFaccion(estado, faccionId);
  if (faccion === undefined) return true; // no existe: lo rechaza el comando, no la autorización
  if (esCiudadano(faccion, jugadorId)) return true;
  return faccion.ciudadanosIds.length === 0 && sinFaccionTodavia(estado, jugadorId);
}

/** Ciudadano de la Facción dueña de ese asentamiento. */
function esFaccionDelAsentamiento(estado: GameSessionState, jugadorId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  return asentamiento === undefined || esFaccionPropia(estado, jugadorId, asentamiento.faccionId);
}

function reside(estado: GameSessionState, jugadorId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  return asentamiento === undefined || (esResidente(asentamiento, jugadorId) && presente(estado, jugadorId, asentamientoId));
}

/**
 * Está DENTRO de esa plaza (Doc 1.10.1). Es la segunda mitad de "la ciudadanía habilita, la presencia
 * ejerce" (Doc 2.5), y por eso no es una condición aparte en la matriz sino que se suma a la residencia: los
 * 24 comandos que exigían ser vecino exigían en realidad *ser vecino y estar ahí*, solo que hasta ahora un
 * jugador estaba en todas partes a la vez.
 *
 * La consecuencia buscada: un Gobernador de campaña **sigue siendo** el Gobernador —no pierde el cargo— pero
 * no gobierna desde el camino. Lo ya ordenado sigue corriendo solo; lo que no puede es dar órdenes nuevas.
 *
 * Y por eso la delegación pasa a importar, que era el punto (Doc 2.5).
 */
function presente(estado: GameSessionState, jugadorId: string, asentamientoId: string): boolean {
  return estaEnAsentamiento(estado.jugadores, jugadorId, asentamientoId, estado.asentamientos, estado.ejercitos);
}

/**
 * Todos los escuadrones indicados que EXISTEN en ese asentamiento pertenecen al actor (Doc 5, fila de
 * combate: "escuadrones propios del jugador"). `Escuadron.jugadorId` es la única fuente de verdad del dueño
 * (`domain/types.ts`), la mutó el motor al reclutar y el cliente no puede falsearla.
 *
 * Un `escuadronId` que no existe se deja pasar — lo rechaza el propio comando de combate, mismo criterio
 * fail-open que el resto de resolutores de este archivo. Lo que se corta es comprometer el escuadrón de OTRO
 * residente del mismo asentamiento: la delegación "de otros residentes autorizados" del doc 5 necesita un
 * mecanismo de mando (un General al que se le ceden tropas) que la Fase 0 no tiene, así que hoy cada jugador
 * solo manda lo suyo.
 */
function comandaEscuadrones(estado: GameSessionState, jugadorId: string, asentamientoId: string, escuadronIds: string[]): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (asentamiento === undefined) return true; // no existe: lo rechaza el comando
  return escuadronIds.every((id) => {
    const escuadron = asentamiento.escuadrones.find((e) => e.id === id);
    return escuadron === undefined || escuadron.jugadorId === jugadorId;
  });
}

/** ¿El jugador tiene algún escuadrón dentro de ese ejército? Un ejército inexistente se deja pasar — lo
 * rechaza el propio comando, mismo criterio fail-open que el resto de resolutores de este archivo. */
function participaEnEjercito(estado: GameSessionState, jugadorId: string, ejercitoId: string): boolean {
  const ejercito = estado.ejercitos.find((e) => e.id === ejercitoId);
  if (ejercito === undefined) return true;
  return ejercito.escuadrones.some((e) => e.jugadorId === jugadorId);
}

/** Reside en el asentamiento, está DENTRO, y ostenta ahí el cargo indicado. */
function residenteConCargo(estado: GameSessionState, jugadorId: string, asentamientoId: string, cargo: Parameters<typeof tieneCargoLocal>[1]): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (!asentamiento) return true;
  return esResidente(asentamiento, jugadorId) && presente(estado, jugadorId, asentamientoId) && tieneCargoLocal(asentamiento, cargo, jugadorId);
}

/** Ciudadano de esa Facción y además Rey o Embajador suyo — autoridad diplomática (Doc 2.2). */
function conAutoridadDiplomatica(estado: GameSessionState, jugadorId: string, faccionId: string): boolean {
  const faccion = buscarFaccion(estado, faccionId);
  if (!faccion) return true;
  return esCiudadano(faccion, jugadorId) && esReyOEmbajadorDe(faccion, jugadorId);
}

export const MATRIZ_AUTORIZACION: { [T in TipoComando]: EntradaMatriz<T> } = {
  // --- Fundación y expansión: Facción propia (con el caso de arranque, ver `puedeFundarEn`) ---
  // El fundador es el propio actor: el comando ya no acepta una lista de fundadores (ver
  // `fundarAsentamiento.ts`), así que fundar otorga ciudadanía a quien ejecuta, y a nadie más.
  fundarAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => puedeFundarEn(estado, jugadorId, params.faccionId),
  },
  lanzarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => esFaccionDelAsentamiento(estado, jugadorId, params.origenAsentamientoId),
  },
  desarmarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => {
      const caravana = buscarCaravana(estado, params.caravanaId);
      return caravana === undefined || esFaccionDelAsentamiento(estado, jugadorId, caravana.origenAsentamientoId);
    },
  },

  // --- crearFaccion/unirseAFaccion/dejarFaccion: sin condición de dominio adicional aquí — a diferencia del
  // resto de la matriz, "1 jugador, 1 Facción" y el cooldown de creación son reglas de NEGOCIO de la partida
  // (qué transición es válida), no de AUTORIZACIÓN (quién puede intentarla): cualquier `jugador` puede
  // intentar los tres, y el propio comando rechaza con su código si no toca (`faccion.ya_pertenece`,
  // `faccion.cooldown_creacion`, `faccion.no_pertenece`) — mismo criterio que nombre vacío/duplicado en
  // `crearFaccion.ts`. Resuelve la pregunta que este archivo dejaba abierta (doc 5, "Preguntas abiertas").
  // Ninguno de los tres acepta un `jugadorId`/`faccionId` de OTRO en `params` con el que suplantar: el actor
  // siempre es `ctx.actor`, nunca algo que el cliente pueda mandar.
  crearFaccion: { rolesPermitidos: ['jugador'] },
  unirseAFaccion: { rolesPermitidos: ['jugador'] },
  dejarFaccion: { rolesPermitidos: ['jugador'] },

  // --- Cargos de Facción ---
  alternarFaccionNpc: {
    rolesPermitidos: ['jugador', 'administrador_partida'],
    condicionJugador: (estado, jugadorId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      return faccion === undefined || (esCiudadano(faccion, jugadorId) && esReyDe(faccion, jugadorId));
    },
  },
  asignarRey: {
    rolesPermitidos: ['jugador'],
    // Primera asignación abierta a cualquier ciudadano (el motor ya exige que el designado lo sea); una vez
    // hay Rey, solo él puede traspasar el cargo.
    condicionJugador: (estado, jugadorId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      if (!faccion) return true;
      return esCiudadano(faccion, jugadorId) && (faccion.reyId === null || esReyDe(faccion, jugadorId));
    },
  },
  asignarEmbajador: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      return faccion === undefined || (esCiudadano(faccion, jugadorId) && esReyDe(faccion, jugadorId));
    },
  },

  // --- Cargos locales: designar Gobernador es directo entre residentes (Doc 2.2, "Fase 0: designación
  // directa"); los demás cargos los designa el Gobernador vigente. ---
  asignarCargoLocal: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      params.cargo === 'gobernador'
        ? reside(estado, jugadorId, params.asentamientoId)
        : residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador'),
  },

  // --- comprarCasa: nadie compra en nombre de otro. La Facción del asentamiento debe ser la propia, O el
  // jugador no ser ciudadano de ninguna todavía — comprar casa es una de las dos vías de UNIRSE a una
  // (junto a fundar), y `engine/faccion.ts` solo bloquea ser ciudadano de OTRA distinta. ---
  comprarCasa: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => {
      if (jugadorId !== params.jugadorId) return false;
      const asentamiento = buscarAsentamiento(estado, params.asentamientoId);
      return asentamiento === undefined || esFaccionPropiaOSinFaccion(estado, jugadorId, asentamiento.faccionId);
    },
  },

  // --- Construcción y gestión local: residente + el cargo que exige cada comando ---
  activarPolitica: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  anadirEdificioManualmente: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  quitarDeCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  moverEnCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  mejorarEdificioAhora: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  comprometerRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  // Sin `cargo` en `params`: abandonar un recinto es SOLO del Gobernador (§8 del doc de murallas), a
  // diferencia de comprometer, que también admite al Maestro de Obras.
  abandonarRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador'),
  },
  mejorarRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, params.cargo),
  },
  // Sin `cargo` en `params`, a diferencia de los de arriba: autoridad sobre la cola de construcción es del
  // Gobernador o del Maestro de Obras (`CargoConstructor`, ver `construccion.ts`).
  /** Enganchar o soltar el tren de suministros lo decide quien va en la columna, igual que replegarla. */
  adjuntarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },
  soltarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },
  /** Cargar y entregar las decide quien va en la columna: es SU viaje (Doc 5.13.3). */
  cargarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },
  entregarDeCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },
  /** Abrir el almacén a un aliado es política de la plaza: Gobernador (manda) o Tesorero (custodia el stock). */
  alternarReabastecerAliados: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador') ||
      residenteConCargo(estado, jugadorId, params.asentamientoId, 'tesorero'),
  },
  alternarAutoConstruccion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador') ||
      residenteConCargo(estado, jugadorId, params.asentamientoId, 'maestroObras'),
  },
  // La reserva manual es del Tesorero (el comando ya rechaza si el cargo está vacante).
  calibrarReservaManual: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => residenteConCargo(estado, jugadorId, params.asentamientoId, 'tesorero'),
  },
  // Doc 5 deja abierto si debería exigir un cargo concreto; hoy basta con residir.
  renombrarAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => reside(estado, jugadorId, params.asentamientoId),
  },

  // --- Diplomacia: Facción propia + autoridad de Rey/Embajador ---
  proponerRelacion: {
    rolesPermitidos: ['jugador'],
    // `faccionAId` es quien propone, por la misma convención que `romperRelacion.iniciadorFaccionId`.
    condicionJugador: (estado, jugadorId, params) => conAutoridadDiplomatica(estado, jugadorId, params.faccionAId),
  },
  romperRelacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => conAutoridadDiplomatica(estado, jugadorId, params.iniciadorFaccionId),
  },
  rebelionVasallo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => {
      const relacion = estado.relaciones.find((r) => r.id === params.relacionId);
      // Quien se rebela es el VASALLO (`faccionBId`, ver `engine/diplomacia.ts`), no el señor.
      return relacion === undefined || conAutoridadDiplomatica(estado, jugadorId, relacion.faccionBId);
    },
  },
  anexionar: {
    rolesPermitidos: ['jugador'],
    // La absorbente es `faccionAId` (`engine/fusion.ts`) — quien inicia la anexión.
    condicionJugador: (estado, jugadorId, params) => conAutoridadDiplomatica(estado, jugadorId, params.faccionAId),
  },
  fusionar: {
    rolesPermitidos: ['jugador'],
    // Fusión consentida por ambos lados: basta tener autoridad en cualquiera de las dos Facciones.
    condicionJugador: (estado, jugadorId, params) =>
      conAutoridadDiplomatica(estado, jugadorId, params.faccionAId) || conAutoridadDiplomatica(estado, jugadorId, params.faccionBId),
  },

  // --- Comercio: residente del asentamiento objetivo ---
  proponerTrueque: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => reside(estado, jugadorId, params.asentamientoAId),
  },
  colocarOrdenMercado: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => reside(estado, jugadorId, params.asentamientoId),
  },
  crearCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => reside(estado, jugadorId, params.asentamientoId),
  },

  // --- Militar: residente del asentamiento atacante + solo puede comprometer SUS PROPIOS escuadrones
  // (`comandaEscuadrones`, doc 5 "escuadrones propios del jugador"). Mandar los de otro residente del mismo
  // asentamiento sigue fuera: necesita un mecanismo de cesión de tropas que la Fase 0 no tiene. ---
  reclutarTropa: {
    rolesPermitidos: ['jugador'],
    // Nadie recluta a nombre de otro: `Escuadron.jugadorId` sería el del actor, no el que mande el cliente.
    condicionJugador: (estado, jugadorId, params) => jugadorId === params.jugadorId && reside(estado, jugadorId, params.asentamientoId),
  },
  iniciarAsedio: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      reside(estado, jugadorId, params.atacanteId) && comandaEscuadrones(estado, jugadorId, params.atacanteId, params.escuadronIds),
  },
  atacarCampamentoBandidos: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      reside(estado, jugadorId, params.atacanteId) && comandaEscuadrones(estado, jugadorId, params.atacanteId, params.escuadronIds),
  },
  // --- Presencia (Doc 1.10). Nadie sale, entra ni vuelve a salir a nombre de otro, así que la condición
  // común es `jugadorId === actor`. Salir al mundo añade lo mismo que movilizar (residencia + mando de los
  // propios escuadrones); entrar y salir de una plaza NO exigen residencia — justamente el caso interesante
  // es la plaza ajena— y su geometría la comprueba el motor, que es quien sabe dónde está la columna. ---
  salirAlMundo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      jugadorId === params.jugadorId &&
      reside(estado, jugadorId, params.asentamientoId) &&
      comandaEscuadrones(estado, jugadorId, params.asentamientoId, params.escuadronIds),
  },
  entrarEnAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  salirDeAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  marcharA: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  // La puerta es del Gobernador (Doc 1.10.5), igual que designar cargos: mismo cargo, misma condición.
  fijarPoliticaDeAcceso: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      jugadorId === params.jugadorId && residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador'),
  },
  vetarJugador: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      jugadorId === params.jugadorId && residenteConCargo(estado, jugadorId, params.asentamientoId, 'gobernador'),
  },
  // El menú de interacción (Doc 5.12.3). La geometría —estar en el anillo— la comprueba el motor, que es
  // quien sabe dónde está cada cosa; aquí solo que nadie mira a nombre de otro.
  inspeccionar: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  // --- Composición de una columna compartida (Doc 5.14). Nadie se une, se separa ni cede el mando a nombre
  // de otro. Lo demás —ir dentro, ser el Líder, la distancia— lo comprueba el motor, que es quien sabe
  // dónde está cada columna y quién la manda. ---
  unirseEnCampo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  responderPeticionDeUnion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  separarseDelEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  cederLiderazgo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, jugadorId, params) => jugadorId === params.jugadorId,
  },
  // --- Ejércitos (Doc 5.12): salir de campaña es sacar TUS escuadrones de TU asentamiento, así que la
  // condición es la misma pareja que el resto de lo militar (residencia + mando de los propios escuadrones).
  // Nadie moviliza a nombre de otro: `jugadorId` tiene que ser el actor, igual que en `reclutarTropa`. ---
  movilizarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      jugadorId === params.jugadorId &&
      reside(estado, jugadorId, params.asentamientoId) &&
      comandaEscuadrones(estado, jugadorId, params.asentamientoId, params.escuadronIds),
  },
  unirseAEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) =>
      jugadorId === params.jugadorId &&
      reside(estado, jugadorId, params.asentamientoId) &&
      comandaEscuadrones(estado, jugadorId, params.asentamientoId, params.escuadronIds),
  },
  // Replegar y estacionar mandan sobre el ejército entero, no sobre escuadrones sueltos: basta con tener
  // tropa dentro. El mando compartido de una coalición (quién decide cuando hay varios jugadores) necesita
  // un mecanismo de cesión que la Fase 0 no tiene — hoy cualquier participante puede ordenar el repliegue.
  // Cancelar la marcha es del Líder, y solo suyo (Doc 5.14.3). Aquí se exige ir dentro, que es la condición
  // barata; que además seas el Líder lo comprueba el motor, que es quien sabe quién manda esa columna.
  replegarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },
  estacionarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, jugadorId, params) => participaEnEjercito(estado, jugadorId, params.ejercitoId),
  },

  // Entre dos asentamientos: el actor debe residir en al menos uno de los lados, y comandar sus propios
  // escuadrones en cada lado donde resida. Escoger qué escuadrones del OTRO lado participan es una
  // simplificación del comando en sí (Fase 0: el combate se resuelve en una sola llamada), no de esta matriz.
};

export type MotivoDenegacion = 'rol_insuficiente' | 'condicion_dominio';

export type ResultadoAutorizacion = { autorizado: true } | { autorizado: false; motivo: MotivoDenegacion };

/** Filtro de rol técnico primero; la condición de dominio solo si el actor entra como `'jugador'`. */
export function verificarAutorizacion<T extends TipoComando>(
  tipo: T,
  params: ParamsDe<T>,
  estado: GameSessionState,
  actor: ActorDeComando
): ResultadoAutorizacion {
  const entrada = MATRIZ_AUTORIZACION[tipo];
  if (!entrada.rolesPermitidos.includes(actor.rol)) return { autorizado: false, motivo: 'rol_insuficiente' };

  if (actor.rol === 'jugador') {
    // El rol `'jugador'` sin `Jugador` asociado es un estado imposible por construcción (`Membresia` de ese
    // rol siempre lo trae); si llegara, denegar es lo correcto: no hay a quién atribuir la acción.
    if (actor.jugadorId === null) return { autorizado: false, motivo: 'condicion_dominio' };
    if (entrada.condicionJugador && !entrada.condicionJugador(estado, actor.jugadorId, params)) {
      return { autorizado: false, motivo: 'condicion_dominio' };
    }
  }
  return { autorizado: true };
}
