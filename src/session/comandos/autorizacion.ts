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
import { faccionDeCaravana } from '../../engine/expansion';
import { esResidente, esReyDe, esReyOEmbajadorDe, puedeReclutarEn, tieneCargoLocal } from '../../engine/pertenencia';
import { estaEnAsentamiento } from '../../engine/ubicacion';
import { indiceTropa } from '../../engine/tropa';
import type { GameSessionState } from '../estado';
import type { ParamsDe, TipoComando } from './registro';

/**
 * Identidad del actor resuelta para esta partida — lo que aporta la `Membresia` (doc 5), nunca un id que el
 * cliente afirme tener (doc 2, principio 3). `heroeId` es `null` para roles que no requieren `Jugador`
 * (ej. `administrador_partida` puro).
 *
 * Sin `faccionId`: la Facción del actor se DERIVA de `Faccion.ciudadanosIds`, única fuente de verdad del
 * juego (ver `Membresia` en `acceso/tipos.ts` para el porqué).
 */
export interface ActorDeComando {
  rol: RolTecnico;
  heroeId: string | null;
}

export type CondicionDominio<T extends TipoComando> = (
  estado: GameSessionState,
  heroeId: string,
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
function esFaccionPropia(estado: GameSessionState, heroeId: string, faccionId: string): boolean {
  const faccion = buscarFaccion(estado, faccionId);
  return faccion === undefined || esCiudadano(faccion, heroeId);
}

/**
 * Residente presente en el asentamiento B de ese acuerdo — el que TIENE que contestar (Doc 3.2).
 *
 * Un acuerdo inexistente se deja pasar, mismo criterio fail-open que el resto de resolutores de este archivo:
 * lo rechaza el propio comando con `acuerdo.no_existe`, que dice mucho mas que un "no autorizado".
 */
function resideEnElLadoQueContesta(estado: GameSessionState, heroeId: string, acuerdoId: string): boolean {
  const acuerdo = estado.acuerdos.find((a) => a.id === acuerdoId);
  return acuerdo === undefined || reside(estado, heroeId, acuerdo.asentamientoBId);
}

/** Residente del asentamiento de origen de la caravana (revamp, Doc 3.13): componerla y reservarla es cosa de casa. */
function resideEnOrigenDeCaravana(estado: GameSessionState, heroeId: string, caravanaId: string): boolean {
  const caravana = buscarCaravana(estado, caravanaId);
  return caravana === undefined || reside(estado, heroeId, caravana.origenAsentamientoId);
}

/**
 * Operar una caravana `'aparcada'` en una plaza anfitriona (Ocupacion §2.3d): ser residente de SU ORIGEN
 * —sigue siendo tuya— y estar PRESENTE en la plaza donde está aparcada (no en el origen, del que marchaste).
 */
function puedeOperarCaravanaAparcada(estado: GameSessionState, heroeId: string, caravanaId: string, asentamientoId: string): boolean {
  const caravana = buscarCaravana(estado, caravanaId);
  const origen = caravana && buscarAsentamiento(estado, caravana.origenAsentamientoId);
  if (!caravana || !origen) return true; // lo rechaza el comando con un código que dice más
  return esResidente(origen, heroeId) && presente(estado, heroeId, asentamientoId);
}

function reside(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  return asentamiento === undefined || (esResidente(asentamiento, heroeId) && presente(estado, heroeId, asentamientoId));
}
/** Reclutar (revisión 2026-09-08): residir aquí, O ser ciudadano de la Facción del asentamiento y que este lo
 * permita — en ambos casos, estando presente. El detalle "solo reponer si no resides" lo hace el motor. */
function puedeReclutarEnPlaza(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (!asentamiento) return true;
  const faccionJugador = estado.facciones.find((f) => f.ciudadanosIds.includes(heroeId));
  return (
    puedeReclutarEn(asentamiento, heroeId, faccionJugador?.id ?? '') !== 'no' &&
    presente(estado, heroeId, asentamientoId)
  );
}
/** Sacar tropa a campaña: residir aquí —ahí está tu campamento, Doc 5.15.2— y estar presente. Mismo criterio
 * que el gate del motor en `movilizarEjercito`. */
function puedeMoverTropaDe(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (!asentamiento) return true;
  return esResidente(asentamiento, heroeId) && presente(estado, heroeId, asentamientoId);
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
function presente(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  return estaEnAsentamiento(estado.heroes, heroeId, asentamientoId, estado.asentamientos, estado.ejercitos);
}

/**
 * Todos los escuadrones indicados que EXISTEN pertenecen al actor (Doc 5, fila de combate: "escuadrones
 * propios del jugador"). Viven en su héroe (`Heroe.escuadrones`), así que el dueño es quien los guarda: lo
 * decidió el motor al reclutar y el cliente no puede falsearlo.
 *
 * Un `escuadronId` que no existe se deja pasar — lo rechaza el propio comando de combate, mismo criterio
 * fail-open que el resto de resolutores de este archivo. Lo que se corta es comprometer el escuadrón de OTRO
 * residente del mismo asentamiento: la delegación "de otros residentes autorizados" del doc 5 necesita un
 * mecanismo de mando (un General al que se le ceden tropas) que la Fase 0 no tiene, así que hoy cada jugador
 * solo manda lo suyo.
 */
function comandaEscuadrones(estado: GameSessionState, heroeId: string, escuadronIds: string[]): boolean {
  const tropa = indiceTropa(estado.heroes);
  return escuadronIds.every((id) => {
    const escuadron = tropa.get(id);
    return escuadron === undefined || escuadron.heroeId === heroeId;
  });
}

/** ¿El jugador va en ese ejército, con escuadrones o sin ellos (un viajero solo también va, Doc 5.12.1)? Un ejército
 * inexistente se deja pasar — lo rechaza el propio comando, mismo criterio fail-open que el resto de resolutores de este
 * archivo. */
function participaEnEjercito(estado: GameSessionState, heroeId: string, ejercitoId: string): boolean {
  const ejercito = estado.ejercitos.find((e) => e.id === ejercitoId);
  if (!ejercito) return true;
  if (ejercito.participantes.some((p) => p.heroeId === heroeId)) return true;
  const heroe = estado.heroes.find((h) => h.id === heroeId);
  return (heroe?.escuadrones ?? []).some((e) => e.contenedor.tipo === 'ejercito' && e.contenedor.ejercitoId === ejercitoId);
}

/** Reside en el asentamiento, está DENTRO, y ostenta ahí el cargo indicado. */
function residenteConCargo(estado: GameSessionState, heroeId: string, asentamientoId: string, cargo: Parameters<typeof tieneCargoLocal>[1]): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (!asentamiento) return true;
  return esResidente(asentamiento, heroeId) && presente(estado, heroeId, asentamientoId) && tieneCargoLocal(asentamiento, cargo, heroeId);
}

/** Rey de la Facción dueña de la plaza (y ciudadano de ella). Una plaza que no existe se deja pasar al dominio. */
function esReyDeLaPlaza(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  const faccion = asentamiento && buscarFaccion(estado, asentamiento.faccionId);
  return !asentamiento || (!!faccion && esCiudadano(faccion, heroeId) && esReyDe(faccion, heroeId));
}

/** Ciudadano de esa Facción y además Rey o Embajador suyo — autoridad diplomática (Doc 2.2). */
function conAutoridadDiplomatica(estado: GameSessionState, heroeId: string, faccionId: string): boolean {
  const faccion = buscarFaccion(estado, faccionId);
  if (!faccion) return true;
  return esCiudadano(faccion, heroeId) && esReyOEmbajadorDe(faccion, heroeId);
}

/** Rey de la Facción dueña de ese asentamiento (Doc 2.2: designar Gobernador es potestad exclusiva del Rey,
 * a petición del usuario 2026-09-10). Es un acto de nivel Facción, como designar Embajador: NO exige que el
 * Rey resida ni esté presente en la plaza. */
function esReyDelAsentamiento(estado: GameSessionState, heroeId: string, asentamientoId: string): boolean {
  const asentamiento = buscarAsentamiento(estado, asentamientoId);
  if (!asentamiento) return true;
  const faccion = buscarFaccion(estado, asentamiento.faccionId);
  if (!faccion) return true;
  return esCiudadano(faccion, heroeId) && esReyDe(faccion, heroeId);
}

export const MATRIZ_AUTORIZACION: { [T in TipoComando]: EntradaMatriz<T> } = {
  // --- Fundación y expansión (D30): con la caravana de un campamento (`fundar`) o la de una plaza ---
  // Fundar con la caravana de un campamento: el titular, por sí mismo; quién es el titular lo mira el comando (D11).
  fundar: { rolesPermitidos: ['jugador'] },
  // Lanzar la Caravana de Fundación desde una plaza: residente presente, como todo lo que sale de casa (el lanzador es el titular).
  lanzarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => reside(estado, heroeId, params.origenAsentamientoId),
  },
  // Desarmarla: ciudadano de su Facción; que además sea el titular lo mira el comando.
  desarmarCaravanaFundacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const caravana = buscarCaravana(estado, params.caravanaId);
      return caravana === undefined || esFaccionPropia(estado, heroeId, faccionDeCaravana(caravana, estado.asentamientos) ?? '');
    },
  },

  // --- crearFaccion/solicitarIngreso/dejarFaccion: sin condición de dominio adicional aquí — a diferencia del
  // resto de la matriz, "1 jugador, 1 Facción" y el cooldown de creación son reglas de NEGOCIO de la partida
  // (qué transición es válida), no de AUTORIZACIÓN (quién puede intentarla): cualquier `jugador` puede
  // intentar los tres, y el propio comando rechaza con su código si no toca (`faccion.ya_pertenece`,
  // `faccion.cooldown_creacion`, `faccion.no_pertenece`) — mismo criterio que nombre vacío/duplicado en
  // `crearFaccion.ts`. Resuelve la pregunta que este archivo dejaba abierta (doc 5, "Preguntas abiertas").
  // Ninguno de los tres acepta un `heroeId`/`faccionId` de OTRO en `params` con el que suplantar: el actor
  // siempre es `ctx.actor`, nunca algo que el cliente pueda mandar.
  crearFaccion: { rolesPermitidos: ['jugador'] },
  solicitarIngreso: { rolesPermitidos: ['jugador'] },
  // Responder a una solicitud de ingreso es del Rey de esa Facción, y solo suyo (D31, D46).
  responderSolicitud: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => estado.facciones.find((f) => f.id === params.faccionId)?.reyId === heroeId,
  },
  dejarFaccion: { rolesPermitidos: ['jugador'] },

  // --- Cargos de Facción ---
  // Crear el propio héroe es lo único que puede hacer un jugador que todavía no tiene (`verificarAutorizacion`).
  crearHeroe: { rolesPermitidos: ['jugador'] },
  // Los comandos del héroe sobre sí mismo no reciben un `heroeId`: actúan siempre sobre el del actor.
  repartirPuntos: { rolesPermitidos: ['jugador'] },
  guardarLoadout: { rolesPermitidos: ['jugador'] },
  borrarLoadout: { rolesPermitidos: ['jugador'] },
  // Que resida donde guarnece y el cupo los comprueba el motor, que sabe dónde está su campamento.
  asignarGuarnicion: { rolesPermitidos: ['jugador'] },
  // El almacén personal es del héroe del actor; que sea Líder de su columna lo comprueba el motor.
  guardarEnAlmacenPersonal: { rolesPermitidos: ['jugador'] },
  sacarDelAlmacenPersonal: { rolesPermitidos: ['jugador'] },
  retirarGuarnicion: { rolesPermitidos: ['jugador'] },
  // Las Facciones NPC las crea el admin, ya asentadas; ninguna Facción de jugador pasa a la IA.
  asignarRey: {
    rolesPermitidos: ['jugador'],
    // Primera asignación abierta a cualquier ciudadano (el motor ya exige que el designado lo sea); una vez
    // hay Rey, solo él puede traspasar el cargo.
    condicionJugador: (estado, heroeId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      if (!faccion) return true;
      return esCiudadano(faccion, heroeId) && (faccion.reyId === null || esReyDe(faccion, heroeId));
    },
  },
  // La capital la designa el Rey de la Facción (Doc 2.2).
  designarCapital: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      return !faccion || (esCiudadano(faccion, heroeId) && esReyDe(faccion, heroeId));
    },
  },
  asignarEmbajador: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      return faccion === undefined || (esCiudadano(faccion, heroeId) && esReyDe(faccion, heroeId));
    },
  },

  // --- Cargos locales: designar Gobernador es potestad EXCLUSIVA del Rey de la Facción (Doc 2.2, a petición
  // del usuario 2026-09-10 — antes lo hacía cualquier residente); los demás cargos los designa el Gobernador
  // vigente, residente y presente. ---
  asignarCargoLocal: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      params.cargo === 'gobernador'
        ? esReyDelAsentamiento(estado, heroeId, params.asentamientoId)
        : residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador'),
  },

  // Cambiar de residencia: nadie a nombre de otro; y el destino tiene que ser de la propia Facción (el
  // jugador YA es ciudadano de una — el motor lo exige). El resto de
  // condiciones (hueco de vivienda, permiso, no residir ya ahí) las valida `cambiarResidencia`.
  // Reclutar en el campamento donde reside el actor: la residencia y el pago los comprueba el motor.
  reclutarEnCampamento: { rolesPermitidos: ['jugador'] },
  // La tropa prestada (D25, D45, D80): residencia y presencia las comprueban el comando y el motor.
  pedirPrestamo: { rolesPermitidos: ['jugador'] },
  reponerPrestamo: { rolesPermitidos: ['jugador'] },
  // Alijos (D60-D63): quién puede abrirlos y dónde hay que estar, lo comprueba el motor.
  abrirAlijo: { rolesPermitidos: ['jugador'] },
  // Comprar en el mercado del campamento donde reside el actor: la residencia y el pago los comprueba el motor.
  comprarEnCampamento: { rolesPermitidos: ['jugador'] },
  // La refundación desde un campamento: dónde está, la Facción sin plazas y el fondo los comprueban el comando y el motor.
  aportarARefundacion: { rolesPermitidos: ['jugador'] },
  retirarDeRefundacion: { rolesPermitidos: ['jugador'] },
  comprarCaravanaDeRefundacion: { rolesPermitidos: ['jugador'] },
  // Residir en un campamento de mercenarios: cualquier héroe, por sí mismo y sin pedir permiso a su Facción (Doc 2.5).
  residirEnCampamento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // Dejar la casa: cada uno la suya.
  dejarResidencia: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  cambiarResidencia: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      if (heroeId !== params.heroeId) return false;
      const destino = buscarAsentamiento(estado, params.destinoId);
      return destino === undefined || estado.facciones.some((f) => f.id === destino.faccionId && f.ciudadanosIds.includes(heroeId));
    },
  },

  // --- Construcción y gestión local: residente + el cargo que exige cada comando ---
  activarPolitica: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  anadirEdificioManualmente: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  quitarDeCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  moverEnCola: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  mejorarEdificioAhora: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  comprometerRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  // Sin `cargo` en `params`: abandonar un recinto es SOLO del Gobernador (§8 del doc de murallas), a
  // diferencia de comprometer, que también admite al Maestro de Obras.
  abandonarRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador'),
  },
  mejorarRecinto: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, params.cargo),
  },
  // Sin `cargo` en `params`: pedir la subida de nivel es SOLO del Gobernador (Doc 4.5, decisión del usuario
  // 2026-09-26) — es una decisión de gobierno que compromete el almacén del asentamiento, no de obra.
  solicitarAscenso: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador'),
  },
  // Adoptar tecnología es del Rey (Doc 6.5). Que esté en la capital y pueda pagar lo comprueba el motor.
  adoptarTecnologia: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const faccion = buscarFaccion(estado, params.faccionId);
      return faccion === undefined || esReyDe(faccion, heroeId);
    },
  },
  // Comprar a un Aeda es del Rey o del Gobernador de la plaza, estando en ella (Doc 6.7). Que haya un Aeda y el oro lo comprueba el motor.
  comprarTecnologiaAeda: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const plaza = buscarAsentamiento(estado, params.asentamientoId);
      const faccion = plaza && buscarFaccion(estado, plaza.faccionId);
      return !plaza || !faccion || ((esReyDe(faccion, heroeId) || tieneCargoLocal(plaza, 'gobernador', heroeId)) && presente(estado, heroeId, plaza.id));
    },
  },
  // Sin `cargo` en `params`, a diferencia de los de arriba: autoridad sobre la cola de construcción es del
  // Gobernador o del Maestro de Obras (`CargoConstructor`, ver `construccion.ts`).
  /** Enganchar o soltar el tren de suministros lo decide quien va en la columna, igual que replegarla. */
  adjuntarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },
  soltarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },
  /** Cargar y entregar las decide quien va en la columna: es SU viaje (Doc 5.13.3). */
  cargarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },
  entregarDeCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },
  /** Abrir el almacén a un aliado es política de la plaza: Gobernador (manda) o Tesorero (custodia el stock). */
  alternarReabastecerAliados: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador') ||
      residenteConCargo(estado, heroeId, params.asentamientoId, 'tesorero'),
  },
  alternarAutoConstruccion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador') ||
      residenteConCargo(estado, heroeId, params.asentamientoId, 'maestroObras'),
  },
  // Qué producen los talleres lo deciden los mismos que la auto-construcción.
  alternarReceta: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador') ||
      residenteConCargo(estado, heroeId, params.asentamientoId, 'maestroObras'),
  },
  // La reserva manual es del Tesorero (el comando ya rechaza si el cargo está vacante).
  calibrarReservaManual: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => residenteConCargo(estado, heroeId, params.asentamientoId, 'tesorero'),
  },
  // Doc 5 deja abierto si debería exigir un cargo concreto; hoy basta con residir.
  renombrarAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => reside(estado, heroeId, params.asentamientoId),
  },

  // --- Diplomacia: Facción propia + autoridad de Rey/Embajador ---
  proponerRelacion: {
    rolesPermitidos: ['jugador'],
    // `faccionAId` es quien propone, por la misma convención que `romperRelacion.iniciadorFaccionId`.
    condicionJugador: (estado, heroeId, params) => conAutoridadDiplomatica(estado, heroeId, params.faccionAId),
  },
  romperRelacion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => conAutoridadDiplomatica(estado, heroeId, params.iniciadorFaccionId),
  },
  // Declara Rey o Embajador de `faccionAId` (Doc 2); la paz la ofrece cualquiera de las dos Facciones en guerra.
  declararGuerra: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => conAutoridadDiplomatica(estado, heroeId, params.faccionAId),
  },
  proponerPaz: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => conAutoridadDiplomatica(estado, heroeId, params.faccionId),
  },
  rebelionVasallo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => {
      const relacion = estado.relaciones.find((r) => r.id === params.relacionId);
      // Quien se rebela es el VASALLO (`faccionBId`, ver `engine/diplomacia.ts`), no el señor.
      return relacion === undefined || conAutoridadDiplomatica(estado, heroeId, relacion.faccionBId);
    },
  },
  anexionar: {
    rolesPermitidos: ['jugador'],
    // La absorbente es `faccionAId` (`engine/fusion.ts`) — quien inicia la anexión.
    condicionJugador: (estado, heroeId, params) => conAutoridadDiplomatica(estado, heroeId, params.faccionAId),
  },
  fusionar: {
    rolesPermitidos: ['jugador'],
    // Fusión consentida por ambos lados: basta tener autoridad en cualquiera de las dos Facciones.
    condicionJugador: (estado, heroeId, params) =>
      conAutoridadDiplomatica(estado, heroeId, params.faccionAId) || conAutoridadDiplomatica(estado, heroeId, params.faccionBId),
  },

  // --- Comercio: residente del asentamiento objetivo ---
  proponerTrueque: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => reside(estado, heroeId, params.asentamientoAId),
  },
  // Contestar es cosa del lado RECEPTOR (B): quien propuso ya dijo lo suyo, y dejarle aceptar su propia
  // propuesta devolveria el pacto unilateral que este comando existe para quitar.
  aceptarTrueque: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnElLadoQueContesta(estado, heroeId, params.acuerdoId),
  },
  rechazarTrueque: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnElLadoQueContesta(estado, heroeId, params.acuerdoId),
  },
  colocarOrdenMercado: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => reside(estado, heroeId, params.asentamientoId),
  },
  crearCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => reside(estado, heroeId, params.asentamientoId),
  },
  agregarCarroCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnOrigenDeCaravana(estado, heroeId, params.caravanaId),
  },
  comprarAnimalCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnOrigenDeCaravana(estado, heroeId, params.caravanaId),
  },
  reservarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnOrigenDeCaravana(estado, heroeId, params.caravanaId),
  },
  prepararCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId && resideEnOrigenDeCaravana(estado, heroeId, params.caravanaId),
  },
  cancelarCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnOrigenDeCaravana(estado, heroeId, params.caravanaId),
  },
  moverCarroCaravana: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => resideEnOrigenDeCaravana(estado, heroeId, params.desdeCaravanaId),
  },
  // Caravana 'aparcada' (Ocupacion §2.3d): residente de SU ORIGEN, presente en la plaza que la hospeda.
  moverCargaCaravanaAparcada: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId && puedeOperarCaravanaAparcada(estado, heroeId, params.caravanaId, params.asentamientoId),
  },
  enviarCaravanaAlOrigen: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId && puedeOperarCaravanaAparcada(estado, heroeId, params.caravanaId, params.asentamientoId),
  },
  // Comerciar en el mostrador de OTRO no exige residencia ni presencia dentro: exige estar alli con la
  // columna, y eso lo comprueba el motor (`comerciarEnPlaza`), que es donde vive la regla. Aqui solo se corta
  // que nadie opere en nombre de otro.
  comerciarEnPlaza: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },

  // --- Militar: residente del asentamiento atacante + solo puede comprometer SUS PROPIOS escuadrones
  // (`comandaEscuadrones`, doc 5 "escuadrones propios del jugador"). Mandar los de otro residente del mismo
  // asentamiento sigue fuera: necesita un mecanismo de cesión de tropas que la Fase 0 no tiene. ---
  reclutarTropa: {
    rolesPermitidos: ['jugador'],
    // Nadie recluta a nombre de otro: `Escuadron.heroeId` sería el del actor, no el que mande el cliente.
    // Residir → escuadrón nuevo; plaza de tu Facción con permiso, estando presente → solo reponer (lo acota
    // el motor). Ver `puedeReclutarEn`, Doc 5.4/5.8.
    condicionJugador: (estado, heroeId, params) => heroeId === params.heroeId && puedeReclutarEnPlaza(estado, heroeId, params.asentamientoId),
  },
  iniciarAsedio: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      reside(estado, heroeId, params.atacanteId) && comandaEscuadrones(estado, heroeId, params.escuadronIds),
  },
  // --- Presencia (Doc 1.10). Nadie sale, entra ni vuelve a salir a nombre de otro, así que la condición
  // común es `heroeId === actor`. Salir al mundo añade lo mismo que movilizar (residencia + mando de los
  // propios escuadrones); entrar y salir de una plaza NO exigen residencia — justamente el caso interesante
  // es la plaza ajena— y su geometría la comprueba el motor, que es quien sabe dónde está la columna. ---
  salirAlMundo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId &&
      reside(estado, heroeId, params.asentamientoId) &&
      comandaEscuadrones(estado, heroeId, params.escuadronIds),
  },
  entrarEnAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // --- Presencia (Doc 1.10.6): cada uno entra y sale del mundo por sí mismo. ---
  conectarse: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  desconectarse: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  salirDeAsentamiento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // Campamentos de mercenarios (D76, D77): cualquiera entra y sale por sí mismo; la puerta la mira el motor.
  entrarEnCampamento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  salirDelCampamento: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // `guarnecer` (Ocupacion §2.3): como `entrarEnAsentamiento` — la geometría (ejército en la puerta de una
  // plaza de su Facción) la valida el motor, que sabe dónde está la columna. Aquí solo que no actúe por otro.
  guarnecer: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  marcharA: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // La puerta es del Gobernador de la plaza o del Rey de su Facción (Doc 1.10.5): cualquiera de los dos puede
  // cambiarla, plaza a plaza, y manda el último. El Rey no tiene que estar presente: es un acto de Facción.
  fijarPuerta: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId &&
      (residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador') || esReyDeLaPlaza(estado, heroeId, params.asentamientoId)),
  },
  vetarJugador: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId && residenteConCargo(estado, heroeId, params.asentamientoId, 'gobernador'),
  },
  // El menú de interacción (Doc 5.12.3). La geometría —estar en el anillo— la comprueba el motor, que es
  // quien sabe dónde está cada cosa; aquí solo que nadie mira a nombre de otro.
  inspeccionar: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  atacar: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  perseguir: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  dejarDePerseguir: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // --- Batallas de Unity (doc 02 §3.1). Nadie se une a nombre de otro; cancelar es de quien la inició, y el admin
  // puede siempre. Una batalla inexistente se deja pasar: la rechaza el comando con `batalla.no_existe`. ---
  unirseABatalla: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  cancelarBatalla: {
    rolesPermitidos: ['jugador', 'administrador_partida', 'administrador_global'],
    condicionJugador: (estado, heroeId, params) => {
      const batalla = estado.batallas.find((b) => b.id === params.battleId);
      return batalla === undefined || batalla.iniciadaPor === heroeId;
    },
  },
  // --- Composición de una columna compartida (Doc 5.14). Nadie se une, se separa ni cede el mando a nombre
  // de otro. Lo demás —ir dentro, ser el Líder, la distancia— lo comprueba el motor, que es quien sabe
  // dónde está cada columna y quién la manda. ---
  unirseEnCampo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  responderPeticionDeUnion: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  separarseDelEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  cederLiderazgo: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (_estado, heroeId, params) => heroeId === params.heroeId,
  },
  // --- Ejércitos (Doc 5.12): salir de campaña es sacar TUS escuadrones de TU asentamiento, así que la
  // condición es la misma pareja que el resto de lo militar (residencia + mando de los propios escuadrones).
  // Nadie moviliza a nombre de otro: `heroeId` tiene que ser el actor, igual que en `reclutarTropa`. ---
  movilizarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId &&
      puedeMoverTropaDe(estado, heroeId, params.asentamientoId) &&
      comandaEscuadrones(estado, heroeId, params.escuadronIds),
  },
  unirseAEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) =>
      heroeId === params.heroeId &&
      reside(estado, heroeId, params.asentamientoId) &&
      comandaEscuadrones(estado, heroeId, params.escuadronIds),
  },
  // Replegar y estacionar mandan sobre el ejército entero, no sobre escuadrones sueltos: basta con tener
  // tropa dentro. El mando compartido de una coalición (quién decide cuando hay varios jugadores) necesita
  // un mecanismo de cesión que la Fase 0 no tiene — hoy cualquier participante puede ordenar el repliegue.
  // Cancelar la marcha es del Líder, y solo suyo (Doc 5.14.3). Aquí se exige ir dentro, que es la condición
  // barata; que además seas el Líder lo comprueba el motor, que es quien sabe quién manda esa columna.
  replegarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },
  estacionarEjercito: {
    rolesPermitidos: ['jugador'],
    condicionJugador: (estado, heroeId, params) => participaEnEjercito(estado, heroeId, params.ejercitoId),
  },

  // Entre dos asentamientos: el actor debe residir en al menos uno de los lados, y comandar sus propios
  // escuadrones en cada lado donde resida. Escoger qué escuadrones del OTRO lado participan es una
  // simplificación del comando en sí (Fase 0: el combate se resuelve en una sola llamada), no de esta matriz.
};

export type MotivoDenegacion = 'rol_insuficiente' | 'condicion_dominio' | 'fuera_del_mundo';

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
    // Sin héroe todavía, lo único que puede hacer es crearlo (doc 02 §4.2).
    if (actor.heroeId === null) return tipo === 'crearHeroe' ? { autorizado: true } : { autorizado: false, motivo: 'condicion_dominio' };
    // Fuera del mundo no se da ninguna orden (Doc 1.10.6): lo único que se puede hacer es volver.
    const heroeId = actor.heroeId;
    if (tipo !== 'conectarse' && estado.heroes.find((h) => h.id === heroeId)?.fuera) return { autorizado: false, motivo: 'fuera_del_mundo' };
    if (entrada.condicionJugador && !entrada.condicionJugador(estado, actor.heroeId, params)) {
      return { autorizado: false, motivo: 'condicion_dominio' };
    }
  }
  return { autorizado: true };
}
